//! Payload-free diagnostics. Bounded capture; persistence never runs on telemetry's path.
use crate::{BackendService, operations::Failure};
mod content;
mod model;
pub(crate) mod operations;
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
        // Nearest-rank percentiles share one convention, including tiny samples.
        let percentile = |p: usize| {
            values
                .get((values.len() * p).div_ceil(100).saturating_sub(1))
                .copied()
                .unwrap_or(0)
        };
        json!([self.count, percentile(50), percentile(95), self.maximum])
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
    incident_sizes: BTreeMap<u64, usize>,
    last_telemetry_content: Option<u64>,
    last_configuration_content: Option<(u64, Option<u64>, Option<u64>, u64)>,
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
            this.incident_sizes = incidents
                .iter()
                .map(|i| {
                    (
                        i.id,
                        serde_json::to_vec(i).map_or(MAX_INCIDENT_BYTES, |b| b.len()) + 128,
                    )
                })
                .collect();
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
        application_generation: u64,
        operation: Option<u64>,
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
            json!({"code":code,"detail":operations::configuration_error(error),"application_generation":application_generation,"operation":operation}),
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
        self.event_with_content(now, epoch, event, data, persist, None);
    }
    fn event_with_content(
        &mut self,
        now: u64,
        epoch: i64,
        event: &'static str,
        data: Value,
        persist: bool,
        content: Option<Content>,
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
                } else if matches!(
                    key,
                    Key::SchemaFingerprint | Key::RequestedSchemaFingerprint
                ) {
                    value
                        .as_str()
                        .filter(|v| fingerprint_valid(v))
                        .map(|v| Datum::Fingerprint(v.to_owned()))
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
        if kind == Kind::Metadata
            && let Some(version) = data.get("package").and_then(Value::as_str)
            && let Some(version) =
                float_view_application_runtime::coordinator::compatibility::parse_wire_version(
                    version,
                )
        {
            facts.insert(Key::PackageMajor, Datum::Count(u64::from(version.major)));
            facts.insert(Key::PackageMinor, Datum::Count(u64::from(version.minor)));
            if let Some(patch) = version.patch {
                facts.insert(Key::PackagePatch, Datum::Count(u64::from(patch)));
            }
            let suffix = match version.suffix.as_str() {
                "beta2" => Code::Beta2,
                "beta3" => Code::Beta3,
                _ => Code::StableRelease,
            };
            facts.insert(Key::PackageSuffix, Datum::Code(suffix));
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
            content,
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
        let encoded_len = encoded.len();
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
        let configuration_request = kind == Kind::Request
            && matches!(
                record.facts.get(&Key::RequestKind),
                Some(Datum::Code(Code::Configuration))
            );
        let configuration_content = kind == Kind::Content
            && record.content.as_ref().is_some_and(|c| {
                matches!(
                    c.category,
                    ContentKind::ConfigurationCurrent
                        | ContentKind::ConfigurationDefault
                        | ContentKind::ConfigurationWrite
                )
            });
        let coalesced_failure = (configuration_request
            || configuration_content
            || matches!(
                kind,
                Kind::ApiRequestFailure
                    | Kind::ConfigurationOperation
                    | Kind::RuntimeFailure
                    | Kind::MutationFailure
                    | Kind::ConfigurationFailure
            ))
            && self
                .last_capture
                .is_some_and(|at| now.saturating_sub(at) < 60_000);
        if (self.capture_until.is_some_and(|until| now <= until) || coalesced_failure)
            && let Some(i) = self.incidents.back_mut()
        {
            if i.events.len() < 768 {
                *self.incident_sizes.entry(i.id).or_default() += encoded_len + 1;
                i.events.push(record);
            } else {
                i.clipped = true;
            }
        }
        self.trim_incidents();
    }
    fn trim_incidents(&mut self) {
        while self.incidents.len() > MAX_INCIDENTS
            || self.incident_sizes.values().sum::<usize>() > MAX_RETAINED_INCIDENT_BYTES
        {
            let Some(old) = self.incidents.pop_front() else {
                self.incident_sizes.clear();
                break;
            };
            self.incident_sizes.remove(&old.id);
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
        self.incident_sizes.insert(
            id,
            serde_json::to_vec(self.incidents.back().expect("new incident"))
                .map_or(MAX_INCIDENT_BYTES, |b| b.len())
                + 128,
        );
        self.trim_incidents();
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
            if let Some(old) = self.incidents.pop_front() {
                self.incident_sizes.remove(&old.id);
            }
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
        if method.starts_with("diagnostics.")
            || (matches!(method, "session.export.status" | "session.export.cancel")
                && matches!(failure, Failure::NotFound))
        {
            // A progress poll may precede HTTP export registration.
            return;
        }
        // Method strings come from untrusted envelopes; retain only known API
        // names, never parameters, peer text, IDs or formatted error messages.
        let method = ApiMethod::parse(method).unwrap_or(ApiMethod::Unknown);
        self.record_configuration_contents(None);
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
    fn export_namespaces_transport_and_protocol_generations_without_collisions() {
        use std::io::Read;
        let t = Temp::new();
        let root = t.0.join("product");
        fs::create_dir(&root).unwrap();
        let mut s = BackendService::open(&root).unwrap();
        for (transport, application) in [(9001, 9001), (9001, 9002), (9002, 9003)] {
            s.diagnostics.generation = transport;
            s.diagnostics.event(
                0,
                1000,
                "request",
                json!({"application_generation":application,"operation":application,"blocking_operation":transport,"configuration_operation":application,"outcome":"sent_only"}),
                true,
            );
        }
        let completion = s.begin_diagnostic_report(1, None).unwrap().run().unwrap();
        let preview = s.complete_diagnostic_report(completion).unwrap();
        assert!(
            preview
                .summary
                .contains("application_generation identifies")
        );
        let (transfer, _) = s.diagnostic_export(1, &preview.report_id).unwrap();
        let path = s.present_export(1, transfer, 0).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(fs::read(path).unwrap())).unwrap();
        let mut text = String::new();
        zip.by_name("events.jsonl")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        let records = text
            .lines()
            .map(|row| serde_json::from_str::<Value>(row).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 3);
        for (record, (transport, application)) in records.iter().zip([(1, 1), (1, 2), (2, 3)]) {
            assert_eq!(record["facts"]["generation"], transport);
            assert_eq!(record["facts"]["application_generation"], application);
            assert_eq!(record["facts"]["outcome"], "sent_only");
            assert_eq!(record["facts"]["operation"], application);
            assert_eq!(record["facts"]["blocking_operation"], transport);
            assert_eq!(record["facts"]["configuration_operation"], application);
        }
        assert!(!text.contains("9001") && !text.contains("9002") && !text.contains("9003"));
    }
    #[test]
    fn metric_percentiles_are_ordered_for_empty_small_and_bounded_samples() {
        assert_eq!(Metric::default().summary(), json!([0, 0, 0, 0]));
        let mut metric = Metric::default();
        metric.add(281);
        assert_eq!(metric.summary(), json!([1, 281, 281, 281]));
        metric.add(5476);
        assert_eq!(metric.summary(), json!([2, 281, 5476, 5476]));
        for len in 1..=300_u64 {
            let mut metric = Metric::default();
            for value in (1..=len).rev() {
                metric.add(value);
            }
            let summary = metric.summary();
            let values = summary.as_array().unwrap();
            assert_eq!(values[0], len);
            let p50 = values[1].as_u64().unwrap();
            let p95 = values[2].as_u64().unwrap();
            assert!(p50 <= p95 && p95 <= len);
            assert!(metric.values.len() <= 256);
        }
    }
    #[test]
    fn every_scheduler_completion_outcome_survives_the_privacy_boundary() {
        let mut d = Diagnostics::default();
        for outcome in [
            "sent",
            "response",
            "sent_only",
            "timeout",
            "queue_full",
            "cancelled",
            "stale",
        ] {
            d.event(0, 1000, "request", json!({"outcome": outcome}), true);
            let record: Value = serde_json::from_str(&d.ring.back().unwrap().1).unwrap();
            assert_eq!(record["facts"]["outcome"], outcome);
        }
        d.event(0, 1000, "request", json!({"outcome":"PRIVATE"}), true);
        assert!(!d.ring.back().unwrap().1.contains("PRIVATE"));
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
    // Populate the production runtime from frozen wire/schema fixtures, without an OS radio.
    fn configuration_service(root: &std::path::Path, patch: u8) -> BackendService {
        use float_view_application_runtime::{Command as R, coordinator as c};
        let mut s = BackendService::open(root).unwrap();
        let apply =
            |s: &mut BackendService, command| s.runtime.apply(R::Application(command)).unwrap();
        apply(
            &mut s,
            c::Command::Begin(c::BeginFacts {
                now_ms: 10,
                settings: c::telemetry::Settings::new(1000, 5000).unwrap(),
                preferred_controller_can_id: None,
                routing_hint: None,
                continuity: c::discovery::Continuity::default(),
                preferred_bms_can_id: None,
            }),
        );
        let generation = s.runtime.snapshot().application.generation;
        let mut firmware = vec![0, 6, 6];
        firmware.extend_from_slice(b"THOR300\0");
        firmware.extend_from_slice(&[7; 12]);
        firmware.extend_from_slice(&[1, 0, 0, 1, 1, 0, 1, 0]);
        firmware.extend_from_slice(b"Float\0");
        firmware.extend_from_slice(&123_u32.to_be_bytes());
        let mut info = vec![0; 61];
        info[..5].copy_from_slice(&[36, 101, 0, 2, 0]);
        info[5..12].copy_from_slice(b"refloat");
        info[25..28].copy_from_slice(&[1, 3, patch]);
        for (payload, now_ms) in [(firmware, 20), (info, 30)] {
            apply(
                &mut s,
                c::Command::Receive {
                    generation,
                    payload,
                    now_ms,
                    observed_at_epoch_ms: 1000 + now_ms as i64,
                },
            );
        }
        let schema = if patch == 0 {
            include_bytes!("../../../fixtures/refloat-configuration/schemas/1.3.0.qcompress")
                .as_slice()
        } else {
            include_bytes!("../../../fixtures/refloat-configuration/schemas/1.3.1-beta2.qcompress")
                .as_slice()
        };
        apply(
            &mut s,
            c::Command::Activate {
                generation,
                now_ms: 40,
                session_started: true,
                cached_configuration_schema: Some(schema.to_vec()),
            },
        );
        let mut payload = vec![93, 0];
        payload.extend_from_slice(include_bytes!(
            "../../../fixtures/refloat-configuration/values/1.3.0-current.bin"
        ));
        apply(
            &mut s,
            c::Command::Receive {
                generation,
                payload,
                now_ms: 50,
                observed_at_epoch_ms: 1050,
            },
        );
        assert!(
            s.runtime
                .snapshot()
                .application
                .configuration
                .current
                .is_some()
        );
        s.policy.exchange.now = 100;
        s.policy.exchange.epoch = 1100;
        s
    }

    fn write_operation(s: &BackendService) -> crate::operations::Operation {
        use crate::facts::ConfigurationValue as F;
        use float_view_application_runtime::coordinator::configuration_codec::ConfigurationValue as V;
        let config = s.runtime.snapshot().application.configuration;
        let identity = config.identity.unwrap();
        let current = config.current.unwrap();
        crate::operations::Operation::WriteConfiguration {
            package_version: identity.package_version.into(),
            fingerprint: identity.fingerprint.into(),
            signature: identity.wire_signature,
            revision: current.revision,
            values: current
                .values
                .into_iter()
                .map(|(key, value)| {
                    (
                        key,
                        match value {
                            V::Decimal(v) => F::Double(v),
                            V::Integer(v) => F::Integer(v),
                            V::Boolean(v) => F::Boolean(v),
                            V::String(v) => F::String(v),
                        },
                    )
                })
                .collect(),
        }
    }

    fn latest(s: &BackendService, kind: &str) -> Value {
        s.diagnostics
            .ring
            .iter()
            .rev()
            .map(|(_, row)| serde_json::from_str::<Value>(row).unwrap())
            .find(|r| r["kind"] == kind)
            .unwrap()["facts"]
            .clone()
    }

    #[test]
    fn diagnostic_write_rejections_distinguish_identity_revision_and_imu_authority() {
        use crate::operations::Operation;
        for mismatch in [
            "package_matches",
            "schema_matches",
            "signature_matches",
            "revision_matches",
        ] {
            let t = Temp::new();
            let mut s = configuration_service(&t.0, 0);
            let mut request = write_operation(&s);
            let Operation::WriteConfiguration {
                package_version,
                fingerprint,
                signature,
                revision,
                ..
            } = &mut request
            else {
                unreachable!()
            };
            match mismatch {
                "package_matches" => *package_version = "PRIVATE-package".into(),
                "schema_matches" => *fingerprint = "PRIVATE-fingerprint".into(),
                "signature_matches" => *signature ^= 1,
                _ => *revision += 1,
            }
            assert!(matches!(s.dispatch(1, request), Err(Failure::InvalidState)));
            let facts = latest(&s, "runtime_failure");
            assert_eq!(facts["layer"], "configuration");
            assert_eq!(facts["code"], "InvalidState");
            assert_eq!(facts[mismatch], false);
            assert_eq!(facts["current_loaded"], true);
            for other in [
                "package_matches",
                "schema_matches",
                "signature_matches",
                "revision_matches",
            ] {
                assert_eq!(facts[other], other != mismatch);
            }
            assert!(
                !serde_json::to_string(&s.diagnostics.ring)
                    .unwrap()
                    .contains("PRIVATE")
            );
        }
        let t = Temp::new();
        let mut s = configuration_service(&t.0, 1);
        assert!(matches!(
            s.dispatch(1, write_operation(&s)),
            Err(Failure::InvalidState)
        ));
        let facts = latest(&s, "runtime_failure");
        assert_eq!(facts["layer"], "mutation");
        assert_eq!(facts["write_block"], "imu_timeout_unconfirmed");
        assert_eq!(facts["revision_matches"], true);
    }

    #[test]
    fn diagnostic_operation_busy_timeout_and_cancellation_keep_correlation() {
        use crate::operations::Operation;
        let t = Temp::new();
        let mut s = configuration_service(&t.0, 0);
        assert!(matches!(
            s.dispatch(
                1,
                Operation::ReadConfiguration {
                    defaults: false,
                    refresh: true
                }
            ),
            Ok(crate::Dispatch::Pending(_))
        ));
        let submitted = latest(&s, "configuration_operation");
        let id = submitted["operation"].as_u64().unwrap();
        assert!(matches!(
            s.dispatch(1, write_operation(&s)),
            Err(Failure::InvalidState)
        ));
        let busy = latest(&s, "configuration_operation");
        assert_eq!(busy["code"], "service_busy");
        assert_eq!(busy["blocking_operation"], id);
        assert_eq!(busy["blocking_kind"], "read_current");
        let (timer, due) = s
            .policy
            .timers
            .iter()
            .find_map(|(timer, entry)| {
                matches!(entry.kind, crate::policy::Timer::Request(p) if p == id)
                    .then_some((*timer, entry.due))
            })
            .unwrap();
        s.apply_os(crate::os::Envelope {
            lifetime: s.policy.exchange.lifetime,
            sequence: s.policy.exchange.sequence + 1,
            monotonic_ms: due,
            epoch_ms: 1000 + due as i64,
            fact: crate::os::Fact::Timer { id: timer },
        })
        .unwrap();
        let completed = latest(&s, "configuration_operation");
        assert_eq!(completed["operation"], id);
        assert_eq!(completed["outcome"], "failed");
        assert_eq!(completed["duration_ms"], 3250);
        assert!(
            s.diagnostics
                .ring
                .iter()
                .any(|(_, r)| r.contains("request_timeout"))
        );
        assert!(s.pending.is_empty());
        // A connection replacement cancels accepted work with its cause retained.
        let t = Temp::new();
        let mut s = configuration_service(&t.0, 0);
        s.dispatch(
            1,
            Operation::ReadConfiguration {
                defaults: true,
                refresh: true,
            },
        )
        .unwrap();
        s.fail_kernel_requests("connection_replaced");
        assert!(
            s.diagnostics
                .ring
                .iter()
                .any(|(_, r)| r.contains("connection_replaced"))
        );
        assert!(s.pending.is_empty());
    }

    #[test]
    fn diagnostic_metadata_distinguishes_stable_and_audited_beta_without_peer_text() {
        for (version, suffix) in [
            ("1.3.1", "stable"),
            ("1.3.1-beta2", "beta2"),
            ("1.3.1-beta3", "beta3"),
        ] {
            let mut d = Diagnostics::default();
            d.metadata(0, 1000, json!({"package":version}));
            let record: Value = serde_json::from_str(&d.ring.back().unwrap().1).unwrap();
            assert_eq!(record["facts"]["package_major"], 1);
            assert_eq!(record["facts"]["package_minor"], 3);
            assert_eq!(record["facts"]["package_patch"], 1);
            assert_eq!(record["facts"]["package_suffix"], suffix);
            d.metadata(1, 1001, json!({"package":"1.3.1-PRIVATE"}));
            let record: Value = serde_json::from_str(&d.ring.back().unwrap().1).unwrap();
            assert!(record["facts"].get("package_suffix").is_none());
            assert!(!serde_json::to_string(&d.ring).unwrap().contains("PRIVATE"));
        }
    }

    #[test]
    fn diagnostic_failure_context_fits_limits_with_maximum_counters() {
        let t = Temp::new();
        let mut s = configuration_service(&t.0, 0);
        let mut request = write_operation(&s);
        if let crate::operations::Operation::WriteConfiguration { revision, .. } = &mut request {
            *revision += 1;
        }
        assert!(s.dispatch(1, request).is_err());
        let mut facts = latest(&s, "runtime_failure");
        for key in [
            "application_generation",
            "operation",
            "current_revision",
            "base_revision",
            "generation",
        ] {
            facts[key] = json!(9_007_199_254_740_991_u64);
        }
        facts["write_block"] = json!("imu_timeout_unconfirmed");
        facts["configuration_pending"] = json!("configuration_write");
        facts["mutation_pending"] = json!("configuration_write");
        s.diagnostics.sequence = 9_007_199_254_740_990;
        let before = s.diagnostics.ring.len();
        s.diagnostics
            .event(9_007_199_254_740_991, 1100, "runtime_failure", facts, true);
        // The new timestamp evicts earlier ring rows. A dropped over-size record would leave it empty.
        assert!(before > 0);
        assert_eq!(s.diagnostics.ring.len(), 1);
        let record: Record = serde_json::from_str(&s.diagnostics.ring.back().unwrap().1).unwrap();
        assert_eq!(record.elapsed_ms, 9_007_199_254_740_991);
        assert!(record.valid());
    }

    #[test]
    fn diagnostic_late_failure_context_survives_export_and_remains_bounded() {
        use std::io::Read;
        let t = Temp::new();
        let mut s = configuration_service(&t.0, 0);
        s.diagnostics.capture(100, 1100, "reconnect", false);
        s.policy.exchange.now = 20100;
        s.policy.exchange.epoch = 21100;
        s.diagnostics.tick(20100, 21100);
        let mut request = write_operation(&s);
        if let crate::operations::Operation::WriteConfiguration {
            revision, values, ..
        } = &mut request
        {
            *revision += 1;
            values.insert(
                "PRIVATE-field".into(),
                crate::facts::ConfigurationValue::String("PRIVATE-value".into()),
            );
        }
        assert!(s.dispatch(1, request).is_err());
        s.diagnostics.configuration_failure(20100,21100,&float_view_application_runtime::coordinator::configuration::ServiceError::Codec(float_view_application_runtime::coordinator::configuration_codec::ConfigurationError::InvalidValue), s.runtime.snapshot().application.generation.0, None);
        assert!(s.diagnostics.capture_until.is_none());
        assert!(
            s.diagnostics.incidents[0]
                .events
                .iter()
                .any(|r| r.kind == Kind::RuntimeFailure)
        );
        let completion = s.begin_diagnostic_report(1, None).unwrap().run().unwrap();
        let preview = s.complete_diagnostic_report(completion).unwrap();
        let (transfer, _) = s.diagnostic_export(1, &preview.report_id).unwrap();
        let path = s.present_export(1, transfer, 0).unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
        let mut text = String::new();
        zip.by_name("events.jsonl")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        let records: Vec<Record> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(records.iter().all(Record::valid));
        let failure = records
            .iter()
            .find(|r| r.kind == Kind::RuntimeFailure)
            .unwrap();
        assert!(matches!(
            failure.facts.get(&Key::RevisionMatches),
            Some(Datum::Flag(false))
        ));
        assert!(text.contains("invalid_value"));
        assert!(!text.contains("PRIVATE"));
        // Late storms retain numeric caps and disclose clipping.
        for _ in 0..1000 {
            s.operation_event(
                json!({"kind":"configuration_write"}),
                "rejected",
                Some("service_busy"),
            );
        }
        assert_eq!(s.diagnostics.incidents[0].events.len(), 768);
        assert!(s.diagnostics.incidents[0].clipped);
        assert!(s.diagnostics.incidents[0].valid());
    }
    fn captured_records(s: &BackendService) -> Vec<Record> {
        s.diagnostics
            .ring
            .iter()
            .map(|(_, row)| serde_json::from_str(row).unwrap())
            .collect()
    }
    fn content_bytes(records: &[Record], category: ContentKind) -> Vec<u8> {
        let first = records
            .iter()
            .find(|r| r.content.as_ref().is_some_and(|c| c.category == category))
            .unwrap();
        let Datum::Count(id) = first.facts[&Key::ContentId] else {
            panic!("content id")
        };
        let mut chunks = records
            .iter()
            .filter(|r| matches!(r.facts.get(&Key::ContentId),Some(Datum::Count(i)) if *i==id))
            .collect::<Vec<_>>();
        chunks.sort_by_key(|r| match r.facts[&Key::Offset] {
            Datum::Count(n) => n,
            _ => panic!("offset"),
        });
        let mut bytes = Vec::new();
        for r in chunks {
            assert_eq!(
                serde_json::to_value(&r.facts[&Key::Offset]).unwrap(),
                bytes.len()
            );
            let hex = &r.content.as_ref().unwrap().hex;
            bytes.extend(
                (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()),
            );
        }
        bytes
    }
    #[test]
    fn configuration_contents_and_actual_identities_survive_sanitized_zip_export() {
        use std::io::Read;
        let t = Temp::new();
        let mut s = configuration_service(&t.0, 0);
        let identity = s
            .runtime
            .snapshot()
            .application
            .configuration
            .identity
            .unwrap();
        let private_hex = "e".repeat(64);
        let mut request = write_operation(&s);
        if let crate::operations::Operation::WriteConfiguration {
            revision,
            values,
            fingerprint,
            ..
        } = &mut request
        {
            *revision += 1;
            *fingerprint = private_hex.clone();
            values.insert(
                "PRIVATE-unknown-field".into(),
                crate::facts::ConfigurationValue::String("PRIVATE-token".into()),
            );
        }
        assert!(matches!(s.dispatch(1, request), Err(Failure::InvalidState)));
        let records = captured_records(&s);
        assert!(
            !serde_json::to_string(&records)
                .unwrap()
                .contains(&private_hex)
        );
        let current = content_bytes(&records, ContentKind::ConfigurationCurrent);
        let draft = content_bytes(&records, ContentKind::ConfigurationWrite);
        let values: Value = serde_json::from_slice(&current).unwrap();
        assert!(!values.as_object().unwrap().is_empty());
        assert_eq!(serde_json::from_slice::<Value>(&draft).unwrap(), values);
        assert!(!String::from_utf8(draft).unwrap().contains("PRIVATE"));
        assert!(records.iter().any(|r| matches!(r.facts.get(&Key::SchemaFingerprint),Some(Datum::Fingerprint(v)) if v==identity.fingerprint)));
        assert!(records.iter().all(Record::valid));
        let completion = s.begin_diagnostic_report(1, None).unwrap().run().unwrap();
        let preview = s.complete_diagnostic_report(completion).unwrap();
        let (transfer, _) = s.diagnostic_export(1, &preview.report_id).unwrap();
        let path = s.present_export(1, transfer, 0).unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
        let mut jsonl = String::new();
        zip.by_name("events.jsonl")
            .unwrap()
            .read_to_string(&mut jsonl)
            .unwrap();
        assert!(!jsonl.contains(&private_hex));
        let exported = jsonl
            .lines()
            .map(|line| serde_json::from_str::<Record>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            content_bytes(&exported, ContentKind::ConfigurationCurrent),
            current
        );
        let mut manifest = String::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        let manifest: Value = serde_json::from_str(&manifest).unwrap();
        assert_eq!(manifest["version"], 2);
        assert!(
            !manifest["omissions"]
                .as_array()
                .unwrap()
                .contains(&json!("configuration"))
        );
    }
    #[test]
    fn content_chunks_disclose_truncation_and_old_records_remain_readable() {
        let mut d = Diagnostics::default();
        let bytes = vec![0xab; MAX_CONTENT_BYTES + 99];
        d.content_event(
            0,
            1000,
            "diagnostic_content",
            json!({}),
            (ContentKind::ConfigurationWrite, &bytes),
            true,
        );
        let records = d
            .ring
            .iter()
            .map(|(_, row)| serde_json::from_str::<Record>(row).unwrap())
            .collect::<Vec<_>>();
        assert!(records.iter().all(Record::valid));
        assert_eq!(
            content_bytes(&records, ContentKind::ConfigurationWrite),
            bytes[..MAX_CONTENT_BYTES]
        );
        assert!(
            records
                .iter()
                .all(|r| matches!(r.facts.get(&Key::Truncated), Some(Datum::Flag(true))))
        );
        let record: Record = serde_json::from_str(
            r#"{"run":1,"sequence":1,"elapsed_ms":0,"kind":"link","facts":{"state":"ready"}}"#,
        )
        .unwrap();
        assert!(record.content.is_none());
        assert!(record.valid());
        let mut invalid = record;
        invalid.content = Some(Content {
            category: ContentKind::PacketRx,
            hex: "PRIVATE".into(),
        });
        assert!(!invalid.valid());
    }
    #[test]
    fn telemetry_contents_include_numbers_without_extension_names_or_observation_times() {
        use float_view_application_runtime::{Command, Effect, coordinator as c};
        let t = Temp::new();
        let mut s = configuration_service(&t.0, 0);
        let generation = s.runtime.snapshot().application.generation;
        let mut limits = vec![14];
        limits.extend_from_slice(&0x2EFD_0142_u32.to_be_bytes());
        limits.extend_from_slice(&[0; 4]);
        for v in [100_f32, -100., 50., -20.] {
            limits.extend_from_slice(&v.to_be_bytes());
        }
        s.runtime
            .apply(Command::Application(c::Command::Receive {
                generation,
                payload: limits,
                now_ms: 110,
                observed_at_epoch_ms: 1110,
            }))
            .unwrap();
        let document: Value = serde_json::from_str(include_str!(
            "../../../fixtures/refloat/detailed-source-provenance.json"
        ))
        .unwrap();
        let hex = document["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == "1.3.0-command-33-fast-float32")
            .unwrap()["payloadHex"]
            .as_str()
            .unwrap();
        let payload = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let effects = s
            .runtime
            .apply(Command::Application(c::Command::Receive {
                generation,
                payload,
                now_ms: 120,
                observed_at_epoch_ms: 1120,
            }))
            .unwrap();
        let mut sample = effects
            .into_iter()
            .find_map(|e| match e {
                Effect::Application(c::Effect::PublishTelemetry(v)) => Some(v),
                _ => None,
            })
            .unwrap();
        sample.extensions.push(c::telemetry::ExtensionValue {
            name: "PRIVATE-location-name".into(),
            value: 123.456,
        });
        s.record_telemetry_contents(&sample);
        let records = captured_records(&s);
        let bytes = content_bytes(&records, ContentKind::Telemetry);
        let values: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(values["pack_voltage_v"], json!(sample.pack_voltage_v));
        for key in [
            "extensions",
            "latitude",
            "longitude",
            "observed_at_epoch_ms",
            "slow_observed_at_epoch_ms",
        ] {
            assert!(values.get(key).is_none());
        }
        assert!(!String::from_utf8(bytes).unwrap().contains("PRIVATE"));
        let before = s.diagnostics.ring.len();
        s.record_telemetry_contents(&sample);
        assert_eq!(s.diagnostics.ring.len(), before);
    }
    #[test]
    fn in_memory_retention_keeps_ten_then_evicts_and_expiry_releases_the_budget() {
        let mut d = Diagnostics::default();
        for id in 1..=11_u64 {
            let now = id * 60_001;
            d.event(
                now,
                1000 + now as i64,
                "link",
                json!({"state":"failed"}),
                true,
            );
            d.capture(now, 1000 + now as i64, "connection_failure", false);
        }
        assert_eq!(d.incidents.len(), 10);
        assert_eq!(d.incidents.front().unwrap().id, 2);
        assert_eq!(d.incident_sizes.len(), 10);
        d.tick(1_000_000, RETENTION_MS + 1_000_000);
        assert!(d.incidents.is_empty());
        assert!(d.incident_sizes.is_empty());
    }
    #[test]
    fn current_context_export_includes_configuration_without_creating_an_incident() {
        use std::io::Read;
        let t = Temp::new();
        let mut s = configuration_service(&t.0, 0);
        let revision = s.service_revision();
        assert!(s.diagnostics.incidents.is_empty());
        let completion = s.begin_diagnostic_report(1, None).unwrap().run().unwrap();
        let preview = s.complete_diagnostic_report(completion).unwrap();
        let (transfer, _) = s.diagnostic_export(1, &preview.report_id).unwrap();
        let path = s.present_export(1, transfer, 0).unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
        let mut jsonl = String::new();
        zip.by_name("events.jsonl")
            .unwrap()
            .read_to_string(&mut jsonl)
            .unwrap();
        let records = jsonl
            .lines()
            .map(|line| serde_json::from_str::<Record>(line).unwrap())
            .collect::<Vec<_>>();
        let values: Value =
            serde_json::from_slice(&content_bytes(&records, ContentKind::ConfigurationCurrent))
                .unwrap();
        assert!(!values.as_object().unwrap().is_empty());
        assert!(s.diagnostics.incidents.is_empty());
        assert_eq!(s.service_revision(), revision);
    }
}
