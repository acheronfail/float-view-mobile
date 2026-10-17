//! Payload-free diagnostics. Bounded capture; persistence never runs on telemetry's path.
use crate::{BackendService, operations::Failure};
mod model;
mod report;
mod store;
use model::*;
pub use model::{Feature, UiCategory, UiError};
pub(crate) use report::ReportFile;
pub use report::{Preview, ReportCompletion, ReportWork};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::atomic::Ordering,
};
use store::Sink;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UiReport {
    pub snapshots: u64,
    pub max_sample_age_ms: u64,
    pub max_delivery_gap_ms: u64,
    pub max_handler_ms: u64,
}
impl UiReport {
    pub fn valid(&self) -> bool {
        self.snapshots <= 10_000
            && self.max_sample_age_ms <= 86_400_000
            && self.max_delivery_gap_ms <= 86_400_000
            && self.max_handler_ms <= 60_000
    }
}

#[derive(Default)]
struct Metric {
    count: u64,
    maximum: u64,
    values: VecDeque<u64>,
}
impl Metric {
    fn add(&mut self, value: u64) {
        self.count = self.count.saturating_add(1);
        self.maximum = self.maximum.max(value);
        if self.values.len() == 256 {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }
    fn summary(&self) -> Value {
        let mut values: Vec<_> = self.values.iter().copied().collect();
        values.sort_unstable();
        json!([
            self.count,
            values.get(values.len() / 2).copied().unwrap_or(0),
            values
                .get(values.len().saturating_sub(1) * 95 / 100)
                .copied()
                .unwrap_or(0),
            self.maximum
        ])
    }
}
#[derive(Default)]
pub(crate) struct Diagnostics {
    sink: Option<Sink>,
    last_summary: u64,
    last_capture: Option<u64>,
    ring: VecDeque<(u64, String)>,
    pending: Vec<Record>,
    context: BTreeMap<Kind, Record>,
    pending_epoch: i64,
    metrics: BTreeMap<&'static str, Metric>,
    last_sample: Option<(u64, u64, u64, i64)>,
    pub last_ui_report: Option<u64>,
    writes: BTreeMap<u64, u64>,
    sequence: u64,
    run: u64,
    pub(crate) generation: u64,
    epoch: i64,
    incidents: VecDeque<Incident>,
    next_incident: u64,
    capture_until: Option<u64>,
    last_incident_save: u64,
    last_prune: u64,
    last_manual: Option<u64>,
    ui_limits: BTreeMap<u64, (u64, u32)>,
    ui_window: (u64, u32),
    reports: BTreeMap<u64, report::Prepared>,
    report_generation: u64,
    report_slots: [std::sync::Weak<ReportFile>; 2],
}
impl Diagnostics {
    pub fn write_started(&mut self, operation: u64, now: u64, epoch: i64, bytes: usize) {
        if self.writes.len() >= 128 {
            self.writes.clear();
        }
        self.writes.insert(operation, now);
        self.event(
            now,
            epoch,
            "ble_write_start",
            json!({"operation":operation,"bytes":bytes}),
            false,
        );
    }
    pub fn write_progress(
        &mut self,
        operation: u64,
        now: u64,
        epoch: i64,
        progress: crate::ble::Progress,
    ) {
        let duration = self
            .writes
            .get(&operation)
            .map(|start| now.saturating_sub(*start));
        if let Some(value) = duration {
            self.metric("ble_write_ms", value);
        }
        self.event(
            now,
            epoch,
            "ble_write",
            json!({"operation":operation,"progress":progress,"since_start_ms":duration}),
            false,
        );
        if progress != crate::ble::Progress::Submitted {
            self.writes.remove(&operation);
        }
    }
    pub(crate) fn disconnect_client(&mut self, owner: u64) {
        self.reports.retain(|_, r| r.owner != owner);
        for slot in &self.report_slots {
            if let Some(file) = slot.upgrade()
                && file.owner == owner
            {
                file.cancelled.store(true, Ordering::Relaxed);
            }
        }
        self.ui_limits.remove(&owner);
    }
    pub fn open(path: &std::path::Path) -> Self {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let mut bytes = [0u8; 8];
        let _ = getrandom::fill(&mut bytes);
        let mut this = Self {
            run: u64::from_le_bytes(bytes) & 9_007_199_254_740_991,
            epoch,
            next_incident: 1,
            ..Self::default()
        };
        if let Ok((store, incidents)) = store::Store::open(path, epoch) {
            this.next_incident = incidents
                .iter()
                .map(|i| i.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            this.incidents = incidents.into();
            this.sink = Sink::new(store).ok();
        }
        this
    }
    pub fn metadata(&mut self, now: u64, epoch: i64, metadata: Value) {
        self.event(now, epoch, "metadata", metadata, true);
    }
    pub fn configuration_failure(
        &mut self,
        now: u64,
        epoch: i64,
        error: &float_view_application_runtime::coordinator::configuration::ServiceError,
    ) {
        use float_view_application_runtime::coordinator::configuration::ServiceError as E;
        let code = match error {
            E::Unavailable => "Unavailable",
            E::InvalidState => "InvalidState",
            E::StaleResponse => "StaleResponse",
            E::UnauditedSchema => "UnauditedSchema",
            E::MalformedResponse => "MalformedResponse",
            E::Conflict => "Conflict",
            E::Codec(_) => "Codec",
        };
        self.event(
            now,
            epoch,
            "configuration_failure",
            json!({"code":code}),
            true,
        );
        self.capture(now, epoch, "configuration_failure", true);
    }
    pub fn metric(&mut self, key: &'static str, value: u64) {
        if self.metrics.contains_key(key) || self.metrics.len() < 24 {
            self.metrics.entry(key).or_default().add(value);
        }
    }
    // The caller's object is never persisted. Closed keys, codes, counts and
    // flags are reconstructed here, including when a producer is adversarial.
    pub(crate) fn event(
        &mut self,
        now: u64,
        epoch: i64,
        event: &'static str,
        data: Value,
        persist: bool,
    ) {
        let Some(kind) = Kind::parse(event) else {
            return;
        };
        self.epoch = epoch;
        self.sequence = self.sequence.saturating_add(1);
        let mut facts = BTreeMap::new();
        if self.generation > 0 {
            facts.insert(Key::Generation, Datum::Count(self.generation));
        }
        if let Some(map) = data.as_object() {
            for (key, value) in map.iter().take(32) {
                let Some(key) = Key::parse(key) else { continue };
                let value = if let Some(v) = value.as_u64().filter(|v| *v <= 9_007_199_254_740_991)
                {
                    Some(Datum::Count(v))
                } else if let Some(v) = value.as_bool() {
                    Some(Datum::Flag(v))
                } else {
                    value.as_str().and_then(|value| {
                        Code::parse(value)
                            .map(Datum::Code)
                            .or_else(|| ApiMethod::parse(value).map(Datum::Method))
                    })
                };
                if let Some(value) = value {
                    facts.insert(key, value);
                }
            }
        }
        if kind == Kind::Summary
            && let Some(array) = data
                .get("count_p50_p95_max")
                .and_then(|v| serde_json::from_value::<[u64; 4]>(v.clone()).ok())
        {
            facts.insert(Key::Percentiles, Datum::Summary(array));
        }
        if kind == Kind::Metadata
            && let Some(version) = data.get("package").and_then(Value::as_str)
            && version.len() <= 16
            && version.bytes().all(|c| c.is_ascii_digit() || c == b'.')
        {
            for (key, part) in [Key::PackageMajor, Key::PackageMinor, Key::PackagePatch]
                .into_iter()
                .zip(version.split('.'))
            {
                if let Ok(n) = part.parse::<u16>() {
                    facts.insert(key, Datum::Count(u64::from(n)));
                }
            }
        }
        if kind == Kind::Native
            && let Some(version) = data.get("os_version").and_then(Value::as_str)
            && version.len() <= 24
            && version.bytes().all(|c| c.is_ascii_digit() || c == b'.')
        {
            for (key, part) in [Key::OsMajor, Key::OsMinor, Key::OsPatch]
                .into_iter()
                .zip(version.split('.'))
            {
                if let Ok(n) = part.parse::<u32>() {
                    facts.insert(key, Datum::Count(u64::from(n)));
                }
            }
        }
        let record = Record {
            run: self.run,
            sequence: self.sequence,
            elapsed_ms: now,
            kind,
            facts,
        };
        if !record.valid() {
            return;
        }
        while self.ring.len() >= MAX_EVENTS
            || self
                .ring
                .front()
                .is_some_and(|(at, _)| now.saturating_sub(*at) > 30_000)
        {
            self.ring.pop_front();
        }
        if matches!(kind, Kind::Metadata | Kind::Native | Kind::Bluetooth) {
            self.context.insert(kind, record.clone());
        }
        let encoded = serde_json::to_string(&record).unwrap_or_default();
        self.ring.push_back((now, encoded));
        if persist || self.capture_until.is_some() {
            self.pending.push(record.clone());
            self.pending_epoch = epoch;
            if self.pending.len() >= 32 {
                self.flush();
            }
        }
        // Keep coalesced API failures in the exported incident even after its
        // follow-up window has closed. Do not extend that window or its budget.
        let coalesced_failure = kind == Kind::ApiRequestFailure
            && self
                .last_capture
                .is_some_and(|at| now.saturating_sub(at) < 60_000);
        if (self.capture_until.is_some_and(|until| now <= until) || coalesced_failure)
            && let Some(i) = self.incidents.back_mut()
        {
            if i.events.len() < 768 {
                i.events.push(record);
            } else {
                i.clipped = true;
            }
        }
    }
    pub fn capture(&mut self, now: u64, epoch: i64, reason: &'static str, _force: bool) {
        let Some(reason) = Reason::parse(reason) else {
            return;
        };
        let _ = self.capture_incident(now, epoch, reason, false);
    }
    fn capture_incident(
        &mut self,
        now: u64,
        epoch: i64,
        reason: Reason,
        manual: bool,
    ) -> Result<u64, Failure> {
        self.tick(now, epoch);
        if manual
            && self
                .last_manual
                .is_some_and(|at| now.saturating_sub(at) < 10_000)
        {
            return Err(Failure::Unavailable);
        }
        if manual {
            self.last_manual = Some(now);
        }
        if (self.capture_until.is_some()
            || (!manual
                && self
                    .last_capture
                    .is_some_and(|at| now.saturating_sub(at) < 60_000)))
            && let Some(i) = self.incidents.back_mut()
        {
            i.occurrences = i.occurrences.saturating_add(1);
            if let Some(s) = &self.sink {
                s.incident(i.clone());
            }
            return Ok(i.id);
        }
        self.last_capture = Some(now);
        self.capture_until = Some(now.saturating_add(10_000));
        let id = self.next_incident.max(1);
        self.next_incident = id.saturating_add(1);
        let events = self
            .ring
            .iter()
            .filter(|(at, _)| now.saturating_sub(*at) <= 30_000)
            .filter_map(|(_, s)| serde_json::from_str(s).ok())
            .collect();
        if self.incidents.len() == MAX_INCIDENTS {
            self.incidents.pop_front();
        }
        self.incidents.push_back(Incident {
            id,
            created_ms: epoch,
            reason,
            complete: false,
            clipped: self.ring.len() == MAX_EVENTS,
            occurrences: 1,
            events,
            context: self.context.values().cloned().collect(),
        });
        if let Some(s) = &self.sink {
            s.incident(self.incidents.back().unwrap().clone());
        }
        Ok(id)
    }
    pub fn sample(&mut self, now: u64, epoch: i64, sequence: u64, elapsed: u64, riding: bool) {
        if let Some((previous, previous_sequence, previous_elapsed, previous_epoch)) =
            self.last_sample
        {
            let interval = now.saturating_sub(previous);
            self.metric("sample_interval_ms", interval);
            if interval > 500 {
                self.metric("gaps_over_500_ms", interval);
            }
            if interval > 1000 {
                self.metric("gaps_over_1000_ms", interval);
            }
            if sequence <= previous_sequence
                || elapsed < previous_elapsed
                || (epoch - previous_epoch - interval as i64).abs() > 1000
            {
                self.event(now, epoch, "clock_or_sequence_discontinuity", json!({"sequence":sequence,"previous_sequence":previous_sequence,"elapsed_ms":elapsed,"previous_elapsed_ms":previous_elapsed}), true);
            }
            if riding && interval > 500 {
                self.event(now, epoch, "telemetry_gap", json!({"interval_ms":interval,"sequence":sequence,"previous_sequence":previous_sequence}), true);
                self.capture(now, epoch, "telemetry_gap", false);
            }
        }
        self.last_sample = Some((now, sequence, elapsed, epoch));
        self.event(
            now,
            epoch,
            "sample",
            json!({"sequence":sequence,"sample_elapsed_ms":elapsed,"riding":riding}),
            false,
        );
    }
    pub fn tick(&mut self, now: u64, epoch: i64) {
        self.epoch = epoch;
        self.flush();
        if now.saturating_sub(self.last_prune) >= 60_000 {
            self.last_prune = now;
            if let Some(s) = &self.sink {
                s.prune(epoch);
            }
        }

        if self.capture_until.is_some() && now.saturating_sub(self.last_incident_save) >= 1000 {
            self.last_incident_save = now;
            if let Some(i) = self.incidents.back()
                && let Some(s) = &self.sink
            {
                s.incident(i.clone());
            }
        }
        if self.capture_until.is_some_and(|until| now >= until) {
            self.capture_until = None;
            if let Some(i) = self.incidents.back_mut() {
                i.complete = true;
                if let Some(s) = &self.sink {
                    s.incident(i.clone());
                }
            }
        }
        while self
            .incidents
            .front()
            .is_some_and(|i| epoch.saturating_sub(i.created_ms) > RETENTION_MS)
        {
            self.incidents.pop_front();
        }
        self.reports.retain(|_, r| now < r.expires);
        if now.saturating_sub(self.last_summary) < 10_000 {
            return;
        }
        self.summary(now, epoch);
    }
    fn summary(&mut self, now: u64, epoch: i64) {
        let metrics = std::mem::take(&mut self.metrics);
        let dropped = self
            .sink
            .as_ref()
            .map_or(0, |s| s.dropped.load(Ordering::Relaxed));
        // Separate bounded rows avoid silently losing a large summary.
        for (name, metric) in metrics {
            self.event(now, epoch, "summary", json!({"window_ms":now.saturating_sub(self.last_summary),"metric":name,"count_p50_p95_max":metric.summary(),"dropped_records":dropped}), true);
        }
        self.last_summary = now;
        self.flush();
    }
    fn flush(&mut self) {
        if !self.pending.is_empty() {
            let rows = std::mem::take(&mut self.pending);
            if let Some(sink) = &self.sink {
                sink.rows(rows, self.pending_epoch);
            }
        }
    }
    pub fn finish(&mut self, now: u64, epoch: i64) {
        self.summary(now, epoch);
        if let Some(i) = self.incidents.back()
            && let Some(s) = &self.sink
        {
            s.incident(i.clone());
        }
        self.last_sample = None;
    }
}
impl BackendService {
    pub fn record_operation_failure(&mut self, method: &str, failure: &Failure) {
        if method.starts_with("diagnostics.") {
            return;
        }
        // Method strings come from untrusted envelopes; retain only known API
        // names, never parameters, peer text, IDs or formatted error messages.
        let method = ApiMethod::parse(method).unwrap_or(ApiMethod::Unknown);
        let now = self.policy.exchange.now;
        let epoch = self.policy.exchange.epoch;
        self.diagnostics.event(
            now,
            epoch,
            "backend_request_failure",
            json!({"method": method, "code": failure}),
            true,
        );
        self.diagnostics
            .capture(now, epoch, "backend_request_failure", false);
    }

    pub fn record_snapshot_timing(&mut self, duration_us: u64) {
        self.diagnostics.metric("snapshot_us", duration_us);
    }
    pub fn report_ui_diagnostics(&mut self, report: UiReport) -> Result<(), Failure> {
        if !report.valid() {
            return Err(Failure::InvalidRequest);
        }
        let now = self.policy.exchange.now;
        if self
            .diagnostics
            .last_ui_report
            .is_some_and(|last| now.saturating_sub(last) < 5_000)
        {
            return Ok(());
        }
        self.diagnostics.last_ui_report = Some(now);
        self.diagnostics.event(
            now,
            self.policy.exchange.epoch,
            "ui_delivery",
            json!(report),
            true,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::Duration,
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "float-diagnostic-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn all_backend_failures_are_closed_correlated_and_capture_without_a_session() {
        let t = Temp::new();
        let root = t.0.join("product");
        fs::create_dir(&root).unwrap();
        let mut s = BackendService::open(&root).unwrap();
        for failure in [
            Failure::InvalidRequest,
            Failure::InvalidState,
            Failure::PermissionRequired,
            Failure::NotFound,
            Failure::Conflict,
            Failure::Unavailable,
            Failure::Internal,
        ] {
            s.record_operation_failure("refloat.tune.apply", &failure);
        }
        s.record_operation_failure("PRIVATE-123", &Failure::InvalidRequest);
        let records = s
            .diagnostics
            .ring
            .iter()
            .map(|(_, row)| serde_json::from_str::<Value>(row).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 8);
        assert_eq!(records[0]["kind"], "backend_request_failure");
        assert_eq!(records[0]["facts"]["method"], "refloat.tune.apply");
        assert_eq!(records[4]["facts"]["code"], "conflict");
        assert_eq!(records[7]["facts"]["method"], "unknown");
        assert!(!serde_json::to_string(&records).unwrap().contains("PRIVATE"));
        assert_eq!(s.diagnostics.incidents.len(), 1);
        assert!(s.diagnostics.incidents[0].valid());
        let before = s.diagnostics.ring.len();
        s.record_operation_failure("diagnostics.prepare", &Failure::Internal);
        assert_eq!(s.diagnostics.ring.len(), before);
        // A method added to the public API must be added to the closed vocabulary.
        let contract = include_str!("../../../../frontend/src/lib/backend/contract.ts");
        for line in contract
            .split("export interface BackendMethods {")
            .nth(1)
            .unwrap()
            .lines()
        {
            let Some(method) = line
                .trim()
                .strip_prefix('\'')
                .and_then(|line| line.split("':").next())
            else {
                continue;
            };
            if !method.starts_with("diagnostics.") {
                assert!(ApiMethod::parse(method).is_some(), "missing {method}");
            }
        }
    }
    #[test]
    fn coalesced_backend_failure_remains_exportable_after_the_post_window() {
        let mut d = Diagnostics::default();
        d.event(
            0,
            1000,
            "backend_request_failure",
            json!({"method":"refloat.tune.apply","code":"conflict"}),
            true,
        );
        d.capture(0, 1000, "backend_request_failure", false);
        d.tick(11_000, 12_000);
        assert!(d.capture_until.is_none());
        d.event(
            20_000,
            21_000,
            "backend_request_failure",
            json!({"method":"refloat.tune.apply","code":"invalid_state"}),
            true,
        );
        d.capture(20_000, 21_000, "backend_request_failure", false);
        assert_eq!(d.incidents.len(), 1);
        assert!(d.capture_until.is_none());
        assert_eq!(
            serde_json::to_value(&d.incidents[0].events.last().unwrap().facts[&Key::Code]).unwrap(),
            "invalid_state"
        );
    }
    #[test]
    fn privacy_boundary_rebuilds_closed_facts_and_caps_context() {
        let mut d = Diagnostics::default();
        for n in 0..1000 {
            d.event(n,1000,"request",json!({"generation":1,"outcome":"timeout","kind":"fast","address":"PRIVATE-123","code":"PRIVATE-123","queue_ms":23,"name":"PRIVATE-123"}),false);
        }
        assert_eq!(d.ring.len(), 256);
        let text = serde_json::to_string(&d.ring).unwrap();
        assert!(!text.contains("PRIVATE"));
        assert!(text.contains("timeout"));
        d.capture(1000, 1000, "request_delay", true);
        assert_eq!(d.incidents.len(), 1);
        assert!(d.incidents[0].clipped);
        assert!(d.incidents[0].valid());
    }
    #[test]
    fn windows_coalesce_rate_limit_and_finish_without_a_session() {
        let mut d = Diagnostics::default();
        let first = d.capture_incident(0, 1000, Reason::Manual, true).unwrap();
        assert!(d.capture_incident(1, 1001, Reason::Manual, true).is_err());
        d.capture(2, 1002, "connection_failure", true);
        assert_eq!(d.incidents[0].occurrences, 2);
        d.event(
            3,
            1003,
            "link",
            json!({"generation":2,"state":"failed"}),
            true,
        );
        d.tick(10001, 11001);
        assert!(d.incidents[0].complete);
        assert_eq!(d.incidents[0].id, first);
        d.capture(10002, 11002, "connection_failure", true);
        assert_eq!(d.incidents.len(), 1);
        d.capture(60001, 61001, "connection_failure", true);
        assert_eq!(d.incidents.len(), 2);
    }
    #[test]
    fn persistent_incident_reopens_incomplete_and_does_not_touch_product_data() {
        let t = Temp::new();
        let root = t.0.join("diagnostics");
        fs::write(t.0.join("catalog.sqlite"), b"PRIVATE PRODUCT").unwrap();
        let mut d = Diagnostics::open(&root);
        assert!(d.sink.is_some());
        let epoch = d.epoch;
        d.event(1, epoch, "link", json!({"state":"failed"}), true);
        d.capture(2, epoch + 1, "connection_failure", true);
        drop(d);
        let reopened = Diagnostics::open(&root);
        assert_eq!(reopened.incidents.len(), 1);
        assert!(!reopened.incidents[0].complete);
        assert_eq!(
            fs::read(t.0.join("catalog.sqlite")).unwrap(),
            b"PRIVATE PRODUCT"
        );
    }
    #[test]
    fn unknown_layout_and_version_are_preserved() {
        let t = Temp::new();
        let root = t.0.join("diagnostics");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("unknown"), b"DO NOT DELETE").unwrap();
        assert!(Diagnostics::open(&root).sink.is_none());
        assert_eq!(fs::read(root.join("unknown")).unwrap(), b"DO NOT DELETE");
        fs::remove_file(root.join("unknown")).unwrap();
        fs::write(root.join("version"), b"999\n").unwrap();
        assert!(Diagnostics::open(&root).sink.is_none());
        assert_eq!(fs::read(root.join("version")).unwrap(), b"999\n");
    }
    #[test]
    fn export_is_immutable_bounded_and_clear_invalidates_handles() {
        let t = Temp::new();
        let root = t.0.join("product");
        fs::create_dir(&root).unwrap();
        let mut s = BackendService::open(&root).unwrap();
        s.diagnostics.event(
            0,
            1000,
            "link",
            json!({"state":"failed","token":"PRIVATE-SECRET"}),
            true,
        );
        let capture = s.diagnostic_capture().unwrap();
        let id = capture["incidentId"].as_str().unwrap();
        let completed = s
            .begin_diagnostic_report(1, Some(id))
            .unwrap()
            .run()
            .unwrap();
        let preview = s.complete_diagnostic_report(completed).unwrap();
        s.diagnostics
            .event(1, 1001, "foreground", json!({"foreground":true}), true);
        assert!(matches!(
            s.diagnostic_export(2, &preview.report_id),
            Err(Failure::NotFound)
        ));
        let (transfer, _) = s.diagnostic_export(1, &preview.report_id).unwrap();
        let path = s.present_export(1, transfer, 0).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(bytes.len() < MAX_REPORT_BYTES as usize);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        assert_eq!(archive.len(), 3);
        use std::io::Read;
        let mut report = String::new();
        archive
            .by_name("report.md")
            .unwrap()
            .read_to_string(&mut report)
            .unwrap();
        assert_eq!(report, preview.summary);
        let mut events = String::new();
        archive
            .by_name("events.jsonl")
            .unwrap()
            .read_to_string(&mut events)
            .unwrap();
        assert!(!events.contains("PRIVATE"));
        assert!(!events.contains("foreground"));
        let clear = s.diagnostic_clear().unwrap();
        assert!(clear.recv_timeout(Duration::from_secs(2)).unwrap());
        assert!(matches!(
            s.diagnostic_export(1, &preview.report_id),
            Err(Failure::NotFound)
        ));
        assert!(path.exists());
        s.finish_presentation(transfer).unwrap();
        assert!(!path.exists());
        assert!(
            s.diagnostic_list()["incidents"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn clearing_rejects_late_preparation_and_old_writer_work() {
        let t = Temp::new();
        let root = t.0.join("product");
        fs::create_dir(&root).unwrap();
        let mut s = BackendService::open(&root).unwrap();
        s.diagnostic_capture().unwrap();
        let work = s.begin_diagnostic_report(1, None).unwrap();
        let ack = s.diagnostic_clear().unwrap();
        assert!(ack.recv_timeout(Duration::from_secs(2)).unwrap());
        assert!(matches!(
            s.complete_diagnostic_report(work.run().unwrap()),
            Err(Failure::NotFound)
        ));
        drop(s);
        let s = BackendService::open(&root).unwrap();
        assert!(s.diagnostics.incidents.is_empty());
    }
    #[test]
    fn prepared_capacity_includes_workers_and_pinned_presentations() {
        let t = Temp::new();
        let root = t.0.join("product");
        fs::create_dir(&root).unwrap();
        let mut s = BackendService::open(&root).unwrap();
        let a = s.begin_diagnostic_report(1, None).unwrap();
        let b = s.begin_diagnostic_report(1, None).unwrap();
        assert!(matches!(
            s.begin_diagnostic_report(1, None),
            Err(Failure::Unavailable)
        ));
        drop(a);
        drop(b);
        assert!(s.begin_diagnostic_report(1, None).is_ok());
    }
    #[test]
    fn replacement_releases_only_owned_preview_and_disconnect_rejects_late_work() {
        let t = Temp::new();
        let root = t.0.join("product");
        fs::create_dir(&root).unwrap();
        let mut s = BackendService::open(&root).unwrap();
        let completed = s.begin_diagnostic_report(1, None).unwrap().run().unwrap();
        let preview = s.complete_diagnostic_report(completed).unwrap();
        assert!(matches!(
            s.replace_diagnostic_report(2, &preview.report_id),
            Err(Failure::NotFound)
        ));
        s.replace_diagnostic_report(1, &preview.report_id).unwrap();
        let work = s.begin_diagnostic_report(1, None).unwrap();
        s.disconnect_file_client(1);
        assert!(matches!(work.run(), Err(Failure::NotFound)));
    }
    #[test]
    fn old_incident_keeps_original_compatibility_context() {
        let t = Temp::new();
        let root = t.0.join("product");
        fs::create_dir(&root).unwrap();
        let mut s = BackendService::open(&root).unwrap();
        s.diagnostics.generation = 10;
        s.diagnostics
            .metadata(0, 1000, json!({"hardware":"THOR300","firmware_major":6}));
        let captured = s.diagnostic_capture().unwrap();
        s.diagnostics.generation = 20;
        s.diagnostics.metadata(
            1,
            1001,
            json!({"hardware":"VESC Express T","firmware_major":7}),
        );
        let work = s
            .begin_diagnostic_report(1, captured["incidentId"].as_str())
            .unwrap();
        let preview = s.complete_diagnostic_report(work.run().unwrap()).unwrap();
        assert!(preview.summary.contains("THOR300"));
        assert!(!preview.summary.contains("VESC Express T"));
        assert!(preview.summary.contains("\"generation\": 1"));
    }
}
