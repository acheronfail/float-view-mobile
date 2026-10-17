use super::*;
use crate::BackendService;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub report_id: String,
    pub summary: String,
    pub complete: bool,
    pub clipped: bool,
    pub dropped_records: u64,
    pub expires_in_ms: u64,
}
pub(crate) struct ReportFile {
    path: PathBuf,
    _lease: store::Lease,
    pub(crate) owner: u64,
    pub(crate) cancelled: std::sync::atomic::AtomicBool,
}
impl ReportFile {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn len(&self) -> float_view_product_store::Result<u64> {
        store::safe(&self.path, false).map_err(|_| float_view_product_store::Error::Io)?;
        Ok(self.path.metadata()?.len())
    }
    pub fn read(&self, offset: u64, target: &mut [u8]) -> float_view_product_store::Result<usize> {
        if target.len() > 65_536 || offset > self.len()? {
            return Err(float_view_product_store::Error::InvalidInput);
        }
        let mut f = File::open(&self.path)?;
        f.seek(SeekFrom::Start(offset))?;
        Ok(f.read(target)?)
    }
}
impl Drop for ReportFile {
    fn drop(&mut self) {
        if store::safe(&self.path, false).is_ok() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
pub(super) struct Prepared {
    pub owner: u64,
    pub expires: u64,
    pub generation: u64,
    pub preview: Preview,
    pub file: Arc<ReportFile>,
}
pub struct ReportWork {
    prepared: Prepared,
    events: Vec<Record>,
    reason: Reason,
    occurrences: u64,
    context: Vec<Record>,
    _permit: crate::files::Permit,
}
pub struct ReportCompletion(Prepared);
impl ReportWork {
    pub fn run(mut self) -> Result<ReportCompletion, Failure> {
        if self.prepared.file.cancelled.load(Ordering::Relaxed) {
            return Err(Failure::NotFound);
        }
        let id = self.prepared.preview.report_id.clone();
        // Every source record has already passed the closed serializer. Recheck
        // loaded incident records before emitting any file.
        if self.events.len() > 768 || self.events.iter().any(|e| !e.valid()) {
            return Err(Failure::Unavailable);
        }
        let mut transport_generations = BTreeMap::new();
        let mut application_generations = BTreeMap::new();
        for record in self.events.iter_mut().chain(self.context.iter_mut()) {
            record.run = 1;
            for (key, aliases) in [
                (Key::Generation, &mut transport_generations),
                (Key::ApplicationGeneration, &mut application_generations),
            ] {
                if let Some(Datum::Count(g)) = record.facts.get_mut(&key) {
                    let next = aliases.len() as u64 + 1;
                    *g = *aliases.entry(*g).or_insert(next);
                }
            }
        }
        let build = env!("FLOAT_VIEW_BUILD_REVISION");
        let version = option_env!("FLOAT_VIEW_DIAGNOSTIC_VERSION").unwrap_or("unknown");
        let channel = if version.contains("nightly") {
            "nightly"
        } else if version.contains("rc") {
            "candidate"
        } else if version == "unknown" {
            "unknown"
        } else {
            "stable"
        };
        let reason = serde_json::to_value(self.reason).map_err(|_| Failure::Internal)?;
        let summary = format!(
            "Float View diagnostic report {id}\nBuild: {build}\nVersion: {version}\nChannel: {channel}\nDevice model: unknown\nReason: {}\nEvents: {}\nOccurrences: {}\nComplete capture: {}\nContext clipped: {}\nDropped records: {}\nController/motor/battery: unknown unless recognized facts appear in events.\n\nIncludes structured connection, protocol, lifecycle and timing facts.\nExcludes routes, device identifiers, raw packets, configurations and telemetry values.\n",
            reason.as_str().unwrap_or("unknown"),
            self.events.len(),
            self.occurrences,
            self.prepared.preview.complete,
            self.prepared.preview.clipped,
            self.prepared.preview.dropped_records
        );
        let context = serde_json::to_string_pretty(
            &self
                .context
                .iter()
                .map(|r| json!({"kind":r.kind,"facts":r.facts}))
                .collect::<Vec<_>>(),
        )
        .map_err(|_| Failure::Internal)?;
        let summary = format!(
            "{summary}\nConnection correlation: generation identifies the transport connection; application_generation identifies the controller protocol generation. Both are report-local.\n\nAvailable environment/compatibility facts:\n{context}\n"
        );
        if summary.len() > 8192 {
            return Err(Failure::Unavailable);
        }
        self.prepared.preview.summary = summary.clone();
        let manifest = json!({"version":1,"reportId":id,"complete":self.prepared.preview.complete,"clipped":self.prepared.preview.clipped,"droppedRecords":self.prepared.preview.dropped_records,"eventCount":self.events.len(),"preWindowMs":30000,"postWindowMs":10000,"run":"run-1","omissions":["locations","identifiers","rawPackets","configuration","telemetryValues","rawErrors"],"maxReportBytes":MAX_REPORT_BYTES});
        let path = self.prepared.file.path();
        let f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| Failure::Unavailable)?;
        let mut zip = zip::ZipWriter::new(f);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o600);
        let mut events = Vec::new();
        let first = self.events.first().map_or(0, |e| e.elapsed_ms);
        // A report contains one run's incident. Generations are local ordinals,
        // never controller identifiers; export re-bases them within this report.
        for mut r in self.events {
            r.elapsed_ms = r.elapsed_ms.saturating_sub(first);
            serde_json::to_writer(&mut events, &r).map_err(|_| Failure::Internal)?;
            events.push(b'\n');
        }
        for (name, bytes) in [
            ("report.md", summary.into_bytes()),
            ("events.jsonl", events),
            (
                "manifest.json",
                serde_json::to_vec(&manifest).map_err(|_| Failure::Internal)?,
            ),
        ] {
            zip.start_file(name, options)
                .map_err(|_| Failure::Unavailable)?;
            zip.write_all(&bytes).map_err(|_| Failure::Unavailable)?;
        }
        let file = zip.finish().map_err(|_| Failure::Unavailable)?;
        file.sync_all().map_err(|_| Failure::Unavailable)?;
        if file.metadata().map_err(|_| Failure::Unavailable)?.len() > MAX_REPORT_BYTES {
            return Err(Failure::Unavailable);
        }
        Ok(ReportCompletion(self.prepared))
    }
}
impl BackendService {
    pub fn diagnostic_list(&mut self) -> Value {
        let now = self.monotonic_ms();
        let epoch = self.epoch_ms();
        self.diagnostics.tick(now, epoch);
        let d = &self.diagnostics;
        json!({"persistence":if d.sink.as_ref().is_some_and(|s|s.available.load(Ordering::Relaxed)){"available"}else{"memoryOnly"},"droppedRecords":d.sink.as_ref().map_or(0,|s|s.dropped.load(Ordering::Relaxed)),"incidents":d.incidents.iter().rev().map(|i|json!({"incidentId":i.id.to_string(),"reason":i.reason,"complete":i.complete,"clipped":i.clipped,"ageMs":epoch.saturating_sub(i.created_ms).max(0) as u64})).collect::<Vec<_>>()})
    }
    pub fn diagnostic_capture(&mut self) -> Result<Value, Failure> {
        let now = self.monotonic_ms();
        let epoch = self.epoch_ms();
        let id = self
            .diagnostics
            .capture_incident(now, epoch, Reason::Manual, true)?;
        Ok(json!({"incidentId":id.to_string(),"capturing":true}))
    }
    pub fn begin_diagnostic_report(
        &mut self,
        owner: u64,
        incident: Option<&str>,
    ) -> Result<ReportWork, Failure> {
        let now = self.monotonic_ms();
        let epoch = self.epoch_ms();
        self.diagnostics.tick(now, epoch);
        let permit = self.files.permit().map_err(crate::store_failure)?;
        let d = &mut self.diagnostics;
        if d.reports.len() >= 2 {
            return Err(Failure::Unavailable);
        }
        let (events, reason, complete, clipped, occurrences, context) = if let Some(id) = incident {
            let id = identifier(id)?;
            let i = d
                .incidents
                .iter()
                .find(|i| i.id == id)
                .ok_or(Failure::NotFound)?;
            (
                i.events.clone(),
                i.reason,
                i.complete,
                i.clipped,
                i.occurrences,
                i.context.clone(),
            )
        } else {
            (
                d.ring
                    .iter()
                    .filter(|(at, _)| now.saturating_sub(*at) <= 30_000)
                    .filter_map(|(_, r)| serde_json::from_str(r).ok())
                    .collect(),
                Reason::Manual,
                true,
                d.ring.len() == MAX_EVENTS,
                1,
                d.context.values().cloned().collect(),
            )
        };
        let sink = d
            .sink
            .as_ref()
            .filter(|s| s.available.load(Ordering::Relaxed))
            .ok_or(Failure::Unavailable)?;
        let slot = d
            .report_slots
            .iter()
            .position(|s| s.upgrade().is_none())
            .ok_or(Failure::Unavailable)?;
        // Reserve before spawning work. The file lease pins its slot even after
        // a handle is cleared or native presentation outlives the frontend.
        let file = Arc::new(ReportFile {
            path: sink.root.join(format!("float-view-diagnostics-{slot}.zip")),
            _lease: sink.lease.clone(),
            owner,
            cancelled: std::sync::atomic::AtomicBool::new(false),
        });
        d.report_slots[slot] = Arc::downgrade(&file);
        let mut random = [0u8; 8];
        getrandom::fill(&mut random).map_err(|_| Failure::Unavailable)?;
        let report_id = (u64::from_le_bytes(random) & 9_007_199_254_740_991).max(1);
        if d.reports.contains_key(&report_id) {
            return Err(Failure::Unavailable);
        }
        let prepared = Prepared {
            owner,
            expires: now.saturating_add(REPORT_TTL),
            generation: d.report_generation,
            preview: Preview {
                report_id: report_id.to_string(),
                summary: String::new(),
                complete,
                clipped,
                dropped_records: sink.dropped.load(Ordering::Relaxed),
                expires_in_ms: REPORT_TTL,
            },
            file,
        };
        Ok(ReportWork {
            prepared,
            events,
            reason,
            occurrences,
            context,
            _permit: permit,
        })
    }
    pub fn complete_diagnostic_report(
        &mut self,
        completion: ReportCompletion,
    ) -> Result<Preview, Failure> {
        let prepared = completion.0;
        if prepared.file.cancelled.load(Ordering::Relaxed)
            || prepared.generation != self.diagnostics.report_generation
            || self.monotonic_ms() >= prepared.expires
        {
            return Err(Failure::NotFound);
        }
        let preview = prepared.preview.clone();
        let id = identifier(&preview.report_id)?;
        self.diagnostics.reports.insert(id, prepared);
        Ok(preview)
    }
    pub fn replace_diagnostic_report(&mut self, owner: u64, id: &str) -> Result<(), Failure> {
        let id = identifier(id)?;
        if self
            .diagnostics
            .reports
            .get(&id)
            .is_none_or(|r| r.owner != owner)
        {
            return Err(Failure::NotFound);
        }
        self.diagnostics.reports.remove(&id);
        Ok(())
    }
    pub fn diagnostic_export(&mut self, owner: u64, id: &str) -> Result<(u64, String), Failure> {
        let now = self.monotonic_ms();
        let report = self
            .diagnostics
            .reports
            .get(&identifier(id)?)
            .filter(|r| r.owner == owner && now < r.expires)
            .ok_or(Failure::NotFound)?;
        let name = report
            .file
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(Failure::Internal)?
            .to_owned();
        let file = report.file.clone();
        let transfer = self
            .adopt_diagnostic_file(owner, file)
            .map_err(crate::store_failure)?;
        Ok((transfer, name))
    }
    pub fn diagnostic_clear(&mut self) -> Result<std::sync::mpsc::Receiver<bool>, Failure> {
        let d = &mut self.diagnostics;
        d.report_generation = d.report_generation.saturating_add(1);
        d.reports.clear();
        d.incidents.clear();
        d.ring.clear();
        d.pending.clear();
        d.context.clear();
        d.capture_until = None;
        d.last_capture = None;
        d.last_manual = None;
        let receiver = if let Some(s) = &d.sink {
            s.clear().ok_or(Failure::Unavailable)?
        } else {
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            let _ = tx.send(true);
            rx
        };
        Ok(receiver)
    }
    pub fn diagnostic_ui_error(&mut self, client: u64, error: UiError) -> Result<(), Failure> {
        let now = self.monotonic_ms();
        let epoch = self.epoch_ms();
        let d = &mut self.diagnostics;
        if d.ui_limits.len() >= 64 && !d.ui_limits.contains_key(&client) {
            return Err(Failure::Unavailable);
        }
        if now.saturating_sub(d.ui_window.0) >= 60_000 {
            d.ui_window = (now, 0);
            d.ui_limits.clear();
        }
        let limit = d.ui_limits.entry(client).or_insert((now, 0));
        if now.saturating_sub(limit.0) >= 60_000 {
            *limit = (now, 0);
        }
        if limit.1 >= 10 || d.ui_window.1 >= 30 {
            return Err(Failure::Unavailable);
        }
        limit.1 += 1;
        d.ui_window.1 += 1;
        d.event(now,epoch,"ui_error",json!({"category":error.category,"feature":error.feature,"line":error.line,"column":error.column}),true);
        d.capture(now, epoch, "ui_failure", false);
        Ok(())
    }
}
fn identifier(s: &str) -> Result<u64, Failure> {
    if s.len() > 20 || s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Failure::InvalidRequest);
    }
    s.parse()
        .ok()
        .filter(|id| *id > 0)
        .ok_or(Failure::InvalidRequest)
}
