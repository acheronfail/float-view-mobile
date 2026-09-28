//! Portable archive encoding and validation, outside the service mutation lock.
use crate::{
    Error, FileCancellation, ImportSink, ProductStore, Result, Session, SessionLease, StagedFile,
    boards, identifier, json, limits, root,
};
use float_view_session_sqlite::{CommitMarker, SESSION_SCHEMA, SessionReader};
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
    params,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    fs::{self, File},
    sync::Arc,
};
const META: &str =
    "CREATE TABLE float_view_archive_metadata(key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL)";
const MAX_MANIFEST: usize = 65_536;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    session_id: String,
    device_history_id: String,
    device_snapshot: DeviceSnapshot,
    board_profile_snapshot: Map<String, Value>,
    board_profile_revision: u64,
    started_at_ms: i64,
    ended_at_ms: Option<i64>,
    state: String,
    integrity: String,
    committed_sequence: u64,
    sample_count: u64,
    event_count: u64,
    requested_idle_ms: u64,
    effective_idle_ms: u64,
    requested_riding_ms: u64,
    effective_riding_ms: u64,
    close_reason: Option<String>,
    riding_elapsed_ms: u64,
    total_elapsed_ms: u64,
    distance_m: Option<f64>,
    max_speed_mps: Option<f64>,
    max_duty_cycle: Option<f64>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeviceSnapshot {
    saved_device_id: String,
    device_history_id: String,
    display_name: String,
}
impl Manifest {
    fn validate(&self) -> Result<()> {
        boards::validate_id(&self.session_id)?;
        boards::validate_history(&self.device_history_id)?;
        boards::validate_id(&self.device_snapshot.saved_device_id)?;
        if self.device_snapshot.device_history_id != self.device_history_id
            || self.device_snapshot.display_name.len() > 128
            || self.started_at_ms < 0
            || self.ended_at_ms.is_some_and(|v| v < self.started_at_ms)
            || !matches!(self.state.as_str(), "completed" | "interrupted")
            || !matches!(self.integrity.as_str(), "complete" | "recovered" | "dirty")
            || self.close_reason.as_ref().is_some_and(|v| v.len() > 128)
        {
            return Err(Error::InvalidInput);
        }
        for v in [
            self.board_profile_revision,
            self.committed_sequence,
            self.sample_count,
            self.event_count,
            self.riding_elapsed_ms,
            self.total_elapsed_ms,
        ] {
            if v > limits::MAX_REVISION {
                return Err(Error::InvalidInput);
            }
        }
        for v in [
            self.requested_idle_ms,
            self.effective_idle_ms,
            self.requested_riding_ms,
            self.effective_riding_ms,
        ] {
            if !(1..=float_view_session_sqlite::MAX_PERSISTENCE_CADENCE_MS).contains(&v) {
                return Err(Error::InvalidInput);
            }
        }
        if [self.distance_m, self.max_speed_mps, self.max_duty_cycle]
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || *v < 0.)
        {
            return Err(Error::InvalidInput);
        }
        json(&self.board_profile_snapshot, limits::MAX_PROFILE_BYTES)?;
        boards::validate_profile(&self.board_profile_snapshot)
    }
}
pub struct ArchiveImport {
    stage: StagedFile,
    manifest: Manifest,
    marker: CommitMarker,
}
impl ImportSink {
    /// Run validation on a bounded worker, never under the service owner lock.
    pub fn validate(self, cancel: &FileCancellation) -> Result<ArchiveImport> {
        let stage = self.finish()?;
        cancel.check()?;
        let db = open(&stage, cancel)?;
        validate_schema(&db, true)?;
        let version:String=db.query_row("SELECT value FROM float_view_archive_metadata WHERE key='format_version' AND length(value)=1",[],|r|r.get(0))?;
        let text:String=db.query_row("SELECT value FROM float_view_archive_metadata WHERE key='session' AND length(CAST(value AS BLOB))<=?",[MAX_MANIFEST],|r|r.get(0))?;
        let count: usize = db.query_row(
            "SELECT COUNT(*) FROM (SELECT 1 FROM float_view_archive_metadata LIMIT 3)",
            [],
            |r| r.get(0),
        )?;
        if version != "1" || count != 2 {
            return Err(Error::InvalidInput);
        }
        let manifest: Manifest = serde_json::from_str(&text).map_err(|_| Error::InvalidInput)?;
        manifest.validate()?;
        db.execute_batch("DROP TABLE float_view_archive_metadata; PRAGMA journal_mode=WAL; PRAGMA wal_checkpoint(TRUNCATE)")?;
        db.close().map_err(|(_, e)| Error::from(e))?;
        cancel.check()?;
        let mut reader = SessionReader::open(&stage.path)?;
        let marker = reader.marker()?;
        let metadata = reader.metadata()?;
        drop(reader);
        let db = open(&stage, cancel)?;
        let counts:(u64,u64,u64)=db.query_row("SELECT (SELECT COUNT(*) FROM sample),(SELECT COUNT(*) FROM session_event),(SELECT COUNT(*) FROM location)",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        let maxima:(u64,u64,u64)=db.query_row("SELECT (SELECT COALESCE(MAX(sequence),0) FROM sample),(SELECT COALESCE(MAX(sequence),0) FROM session_event),(SELECT COALESCE(MAX(location_sequence),0) FROM location)",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        if maxima.0 != marker.committed_sample_sequence
            || maxima.1 != marker.committed_event_sequence
            || maxima.2 > marker.committed_location_sequence
        {
            return Err(Error::InvalidInput);
        }
        let fk: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
            [],
            |r| r.get(0),
        )?;
        if fk
            || marker.stored_sample_count == 0
            || counts
                != (
                    marker.stored_sample_count,
                    marker.committed_event_count,
                    marker.stored_location_count,
                )
            || manifest.sample_count != marker.recorded_sample_count
            || manifest.event_count != marker.committed_event_count
            || manifest.committed_sequence != marker.committed_sample_sequence
            || manifest.effective_idle_ms != metadata.expected_idle_persistence_ms
            || manifest.effective_riding_ms != metadata.expected_riding_persistence_ms
        {
            return Err(Error::InvalidInput);
        }
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        drop(db);
        cancel.check()?;
        Ok(ArchiveImport {
            stage,
            manifest,
            marker,
        })
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Archive,
    Csv,
}
pub struct ArchiveExport {
    source: SessionLease,
    stage: StagedFile,
    manifest: Manifest,
    format: ExportFormat,
    gps: bool,
}
impl ArchiveExport {
    pub fn run(mut self, cancel: &FileCancellation) -> Result<StagedFile> {
        cancel.check()?;
        root::require_kind(self.source.path(), false)?;
        let source =
            Connection::open_with_flags(self.source.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        configure(&source, cancel)?;
        match self.format {
            ExportFormat::Archive => {
                let mut dest = open(&self.stage, cancel)?;
                {
                    let backup = Backup::new(&source, &mut dest)?;
                    loop {
                        cancel.check()?;
                        match backup.step(16)? {
                            StepResult::Done => break,
                            StepResult::More => {}
                            _ => return Err(Error::Busy),
                        }
                        if self.stage.len()? > limits::MAX_EXPORT_BYTES {
                            return Err(Error::Capacity);
                        }
                    }
                }
                dest.execute_batch(META)?;
                dest.execute("INSERT INTO float_view_archive_metadata(key,value) VALUES ('format_version','1'),('session',?)",[json(&self.manifest,MAX_MANIFEST)?])?;
                dest.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
                dest.close().map_err(|(_, e)| Error::from(e))?;
            }
            ExportFormat::Csv => crate::csv::write(&source, &self.stage, self.gps, cancel)?,
        }
        cancel.check()?;
        if self.stage.len()? > limits::MAX_EXPORT_BYTES {
            return Err(Error::Capacity);
        }
        let suffix = if self.format == ExportFormat::Archive {
            "floatview"
        } else {
            "csv"
        };
        let destination = self.stage.path.with_extension(suffix);
        fs::rename(&self.stage.path, &destination)?;
        self.stage.path = destination;
        File::open(&self.stage.path)?.sync_all()?;
        Ok(self.stage)
    }
}
impl ProductStore {
    pub fn prepare_export(&self, id: &str, format: ExportFormat) -> Result<ArchiveExport> {
        let s = self.session(id)?;
        if s.state == "active" {
            return Err(Error::ActiveSession);
        }
        let (idle,eidle,riding,eriding,reason):(u64,u64,u64,u64,Option<String>)=self.catalog.query_row("SELECT requested_idle_ms,effective_idle_ms,requested_riding_ms,effective_riding_ms,close_reason FROM session_catalog WHERE session_id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        let manifest = Manifest {
            session_id: s.session_id,
            device_history_id: s.device_history_id,
            device_snapshot: serde_json::from_value(s.device_snapshot)
                .map_err(|_| Error::Corrupt)?,
            board_profile_snapshot: serde_json::from_value(s.profile_snapshot)
                .map_err(|_| Error::Corrupt)?,
            board_profile_revision: s.profile_revision,
            started_at_ms: s.started_at_ms,
            ended_at_ms: s.ended_at_ms,
            state: s.state,
            integrity: s.integrity,
            committed_sequence: s.committed_sequence,
            sample_count: s.recorded_sample_count,
            event_count: s.event_count,
            requested_idle_ms: idle,
            effective_idle_ms: eidle,
            requested_riding_ms: riding,
            effective_riding_ms: eriding,
            close_reason: reason,
            riding_elapsed_ms: s.riding_elapsed_ms,
            total_elapsed_ms: s.total_elapsed_ms,
            distance_m: s.distance_m,
            max_speed_mps: s.max_speed_mps,
            max_duty_cycle: s.max_duty_cycle,
        };
        manifest.validate()?;
        let source = self.lease_session(id)?;
        if source.path().metadata()?.len() > limits::MAX_EXPORT_BYTES {
            return Err(Error::Capacity);
        }
        Ok(ArchiveExport {
            source,
            stage: self.stage(true, limits::MAX_EXPORT_BYTES * 2)?,
            manifest,
            format,
            gps: self.settings()?.include_gps_location_when_exporting,
        })
    }
    /// A short owner transaction attaches a validated, service-issued file.
    pub fn attach_import(&mut self, mut import: ArchiveImport) -> Result<Session> {
        if import.stage.root != self.root || !Arc::ptr_eq(&import.stage.budget, &self.file_budget) {
            return Err(Error::InvalidInput);
        }
        let active: bool = self.catalog.query_row(
            "SELECT EXISTS(SELECT 1 FROM session_catalog WHERE state='active')",
            [],
            |r| r.get(0),
        )?;
        if active {
            return Err(Error::ActiveSession);
        }
        let id = identifier()?;
        let job = identifier()?;
        let file = format!("session-{id}.sqlite");
        let destination = self.root.join(&file);
        if destination.try_exists()? {
            return Err(Error::Capacity);
        }
        let revision = crate::next_revision(self.catalog_revision)?;
        let m = &import.manifest;
        root::require_kind(&import.stage.path, false)?;
        // Rename before the transaction; the lease cleans failed attachment.
        // Startup removes an unreferenced owned session after a crash here.
        fs::rename(&import.stage.path, &destination)?;
        import.stage.path = destination;
        File::open(&self.root)?.sync_all()?;
        let tx = self
            .catalog
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        boards::import_board_on(
            &tx,
            &m.device_history_id,
            Some(&m.device_snapshot.display_name),
            &json(&m.board_profile_snapshot, limits::MAX_PROFILE_BYTES)?,
            m.started_at_ms,
        )?;
        tx.execute("INSERT INTO session_catalog(session_id,job_id,device_history_id,device_snapshot_json,profile_snapshot_json,profile_revision,file_name,started_at_ms,ended_at_ms,state,integrity,requested_idle_ms,effective_idle_ms,requested_riding_ms,effective_riding_ms,close_reason) VALUES(?,?,?,?,?,?,?,?,?,'completed','complete',?,?,?,?,?)",params![id,job,m.device_history_id,json(&m.device_snapshot,limits::MAX_PROFILE_BYTES)?,json(&m.board_profile_snapshot,limits::MAX_PROFILE_BYTES)?,m.board_profile_revision,file,m.started_at_ms,m.ended_at_ms,m.requested_idle_ms,m.effective_idle_ms,m.requested_riding_ms,m.effective_riding_ms,m.close_reason])?;
        crate::sessions::apply_marker(&tx, &id, &import.marker)?;
        let committed = tx.commit();
        // A commit error can have an uncertain durable outcome. Preserve the
        // file; startup reconciles referenced files and removes true orphans.
        import.stage.owned = false;
        committed?;
        self.catalog_revision = revision;
        self.session(&id)
    }
}
fn configure(db: &Connection, cancel: &FileCancellation) -> Result<()> {
    db.busy_timeout(std::time::Duration::from_millis(limits::SQLITE_BUSY_MS))?;
    db.pragma_update(None, "trusted_schema", false)?;
    let cancel = cancel.clone();
    db.progress_handler(1000, Some(move || cancel.interrupted()));
    Ok(())
}
fn open(stage: &StagedFile, cancel: &FileCancellation) -> Result<Connection> {
    root::require_kind(&stage.path, false)?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let p = std::path::PathBuf::from(format!("{}{suffix}", stage.path.display()));
        if p.symlink_metadata().is_ok() {
            root::require_kind(&p, false)?;
        }
    }
    let db = Connection::open_with_flags(&stage.path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    configure(&db, cancel)?;
    Ok(db)
}
fn validate_schema(db: &Connection, archive: bool) -> Result<()> {
    if db.query_row("PRAGMA quick_check(1)", [], |r| r.get::<_, String>(0))? != "ok" {
        return Err(Error::Corrupt);
    }
    let expected = Connection::open_in_memory()?;
    expected.execute_batch(SESSION_SCHEMA)?;
    if archive {
        expected.execute_batch(META)?;
    }
    fn schema(db: &Connection) -> Result<Vec<(String, String, String)>> {
        let mut s=db.prepare("SELECT type,name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name LIMIT 65")?;
        let mut rows = s.query([])?;
        let mut output = Vec::new();
        while let Some(r) = rows.next()? {
            if output.len() == 64 {
                return Err(Error::InvalidInput);
            }
            let sql = r.get_ref(2)?.as_str().map_err(|_| Error::InvalidInput)?;
            if sql.len() > 32768 {
                return Err(Error::InvalidInput);
            }
            let mut quoted = None;
            let normalized: String = sql
                .chars()
                .filter_map(|c| {
                    if let Some(q) = quoted {
                        if c == q {
                            quoted = None;
                        }
                        Some(c)
                    } else if matches!(c, '\'' | '"' | '`') {
                        quoted = Some(c);
                        Some(c)
                    } else if c.is_whitespace() {
                        None
                    } else {
                        Some(c.to_ascii_lowercase())
                    }
                })
                .collect();
            output.push((r.get(0)?, r.get(1)?, normalized));
        }
        Ok(output)
    }
    if schema(db)? != schema(&expected)? {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
