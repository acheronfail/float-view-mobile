//! Payload-free diagnostics. Bounded capture; persistence never runs on telemetry's path.
use crate::{BackendService, operations::Failure};
use float_view_application_runtime::session::DiagnosticWriter;
use float_view_product_store::SessionLease;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

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

struct Sink {
    sender: Option<mpsc::SyncSender<Vec<String>>>,
    finished: mpsc::Receiver<()>,
    dropped: Arc<AtomicU64>,
}
impl Sink {
    fn new(lease: SessionLease) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Vec<String>>(8);
        let (done, finished) = mpsc::sync_channel(1);
        let dropped = Arc::new(AtomicU64::new(0));
        let losses = dropped.clone();
        std::thread::Builder::new()
            .name("float-view-diagnostics".into())
            .spawn(move || {
                let mut writer = DiagnosticWriter::open(lease.path()).ok();
                while let Ok(batch) = receiver.recv() {
                    let mut success = false;
                    for _ in 0..3 {
                        if let Some(writer) = writer.as_mut() {
                            match writer.append(&batch) {
                                Ok(()) => {
                                    success = true;
                                    break;
                                }
                                Err(float_view_application_runtime::session::WriterError::Busy) => {
                                    std::thread::sleep(Duration::from_millis(5))
                                }
                                Err(_) => break,
                            }
                        } else {
                            break;
                        }
                    }
                    if !success {
                        losses.fetch_add(batch.len() as u64, Ordering::Relaxed);
                    }
                }
                drop(writer);
                drop(lease);
                let _ = done.send(());
            })?;
        Ok(Self {
            sender: Some(sender),
            finished,
            dropped,
        })
    }
    fn send(&self, batch: Vec<String>) {
        let count = batch.len() as u64;
        if self
            .sender
            .as_ref()
            .is_none_or(|s| s.try_send(batch).is_err())
        {
            self.dropped.fetch_add(count, Ordering::Relaxed);
        }
    }
    fn finish(mut self) {
        self.sender.take();
        // Only at ride close, after polling and the telemetry writer have stopped.
        let _ = self.finished.recv_timeout(Duration::from_secs(2));
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
    start: u64,
    last_summary: u64,
    last_capture: Option<u64>,
    ring: VecDeque<(u64, String)>,
    pending: Vec<String>,
    metrics: BTreeMap<&'static str, Metric>,
    last_sample: Option<(u64, u64, u64, i64)>,
    pub last_ui_report: Option<u64>,
    writes: BTreeMap<u64, u64>,
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
    pub fn start(&mut self, lease: SessionLease, now: u64, epoch: i64, metadata: Value) {
        self.finish(now, epoch);
        *self = Self {
            sink: Sink::new(lease).ok(),
            start: now,
            last_summary: now,
            ..Self::default()
        };
        self.event(now, epoch, "metadata", metadata, true);
        self.flush();
    }
    pub fn metric(&mut self, key: &'static str, value: u64) {
        if self.sink.is_some() && (self.metrics.contains_key(key) || self.metrics.len() < 24) {
            self.metrics.entry(key).or_default().add(value);
        }
    }
    pub fn event(&mut self, now: u64, epoch: i64, event: &'static str, data: Value, persist: bool) {
        if self.sink.is_none() {
            return;
        }
        let record = json!({"v":1,"at_ms":epoch,"elapsed_ms":now.saturating_sub(self.start),"event":event,"data":data}).to_string();
        if record.len() > 1024 {
            if let Some(s) = &self.sink {
                s.dropped.fetch_add(1, Ordering::Relaxed);
            }
            return;
        }
        if persist {
            self.pending.push(record);
            if self.pending.len() >= 32 {
                self.flush();
            }
        } else {
            while self.ring.len() >= 256
                || self
                    .ring
                    .front()
                    .is_some_and(|(at, _)| now.saturating_sub(*at) > 30_000)
            {
                self.ring.pop_front();
            }
            self.ring.push_back((now, record));
        }
    }
    pub fn capture(&mut self, now: u64, epoch: i64, reason: &'static str, force: bool) {
        if !force
            && self
                .last_capture
                .is_some_and(|at| now.saturating_sub(at) < 10_000)
        {
            return;
        }
        self.last_capture = Some(now);
        self.event(now, epoch, "capture", json!({"reason":reason}), true);
        self.flush();
        while let Some((_, record)) = self.ring.pop_front() {
            self.pending.push(record);
            if self.pending.len() >= 32 {
                self.flush();
            }
        }
        self.flush();
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
        if self.sink.is_none() || now.saturating_sub(self.last_summary) < 10_000 {
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
            let records = std::mem::take(&mut self.pending);
            if let Some(sink) = &self.sink {
                sink.send(records);
            }
        }
    }
    pub fn finish(&mut self, now: u64, epoch: i64) {
        if self.sink.is_some() {
            self.capture(now, epoch, "ride_closed", true);
            self.summary(now, epoch);
            if let Some(sink) = self.sink.take() {
                sink.finish();
            }
        }
    }
}
impl BackendService {
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
    #[test]
    fn full_or_disconnected_sink_counts_dropped_records_without_blocking() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let (_done, finished) = mpsc::sync_channel(1);
        let dropped = Arc::new(AtomicU64::new(0));
        let sink = Sink {
            sender: Some(sender),
            finished,
            dropped: dropped.clone(),
        };
        sink.send(vec!["{}".into()]);
        sink.send(vec!["{}".into(), "{}".into()]);
        assert_eq!(dropped.load(Ordering::Relaxed), 2);
        drop(receiver);
        sink.send(vec!["{}".into()]);
        assert_eq!(dropped.load(Ordering::Relaxed), 3);
    }
    #[test]
    fn metrics_and_reports_are_bounded() {
        let mut metric = Metric::default();
        for n in 0..1000 {
            metric.add(n);
        }
        assert_eq!(metric.values.len(), 256);
        assert_eq!(metric.summary(), json!([1000, 872, 986, 999]));
        assert!(
            !UiReport {
                snapshots: 10001,
                max_sample_age_ms: 0,
                max_delivery_gap_ms: 0,
                max_handler_ms: 0
            }
            .valid()
        );
    }
    #[test]
    fn worker_flushes_marker_capture_and_summaries_into_export() {
        let path =
            std::env::temp_dir().join(format!("float-view-diagnostics-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        let mut store = float_view_product_store::ProductStore::open(&path).unwrap();
        let board = store
            .ensure_board("PRIVATE-DEVICE", Some("Private Name"), Some(&[7; 12]), 1)
            .unwrap();
        let session = store
            .create_session(&board.saved_device_id, 1, u64::MAX)
            .unwrap();
        let mut capture = Diagnostics::default();
        capture.start(
            store.lease_session(&session.session_id).unwrap(),
            100,
            1000,
            json!({"build":"test"}),
        );
        capture.sample(200, 1100, 1, 100, true);
        capture.sample(1900, 2800, 2, 1800, true);
        capture.event(2000, 2900, "ride_marker", json!({"sequence":1}), true);
        capture.capture(2000, 2900, "ride_marker", true);
        capture.metric("append_us", 123);
        capture.tick(10100, 11000);
        capture.finish(10200, 11100);
        // Readers/exports share the exact same database; inspect through its archive.
        let mut writer = float_view_application_runtime::session::SessionWriter::open(
            store.session_path(&session.session_id).unwrap(),
        )
        .unwrap();
        writer
            .append_ride_marker("00000000-0000-4000-8000-000000000001", 2000, 2900, false)
            .unwrap();
        writer.close().unwrap();
        store
            .close_session(&session.session_id, 11100, "test")
            .unwrap();
        let export = store
            .prepare_export(
                &session.session_id,
                float_view_product_store::ExportFormat::Archive,
            )
            .unwrap()
            .run(&float_view_product_store::FileCancellation::default())
            .unwrap();
        let bytes = std::fs::read(export.path()).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        for expected in [
            "telemetry_gap",
            "ride_marker",
            "append_us",
            "count_p50_p95_max",
        ] {
            assert!(text.contains(expected), "{expected}");
        }
        assert!(!text.contains("PRIVATE-DEVICE"));
        drop(export);
        drop(store);
        std::fs::remove_dir_all(path).unwrap();
    }
}
