//! Persisted single-job coordination. Workers never borrow the catalog owner.
use crate::{
    Error, ProductStore, Result, SessionLease,
    boards::{timestamp, validate_id},
    identifier,
};
use float_view_session_sqlite::{PruneError, PruneInterrupter, SessionPruner, SessionReader};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Maintenance {
    pub operation_id: String,
    pub kind: String,
    pub scope: String,
    pub session_id: Option<String>,
    pub state: String,
    pub target_resolution_seconds: u64,
    pub upper_session_id: Option<String>,
    pub last_session_id: Option<String>,
    pub completed_sessions: u64,
    pub skipped_sessions: u64,
    pub retained_samples: u64,
    pub reclaimable_bytes: u64,
    pub error: Option<String>,
}
impl Maintenance {
    fn terminal(&self) -> bool {
        matches!(self.state.as_str(), "completed" | "cancelled" | "failed")
    }
}
#[derive(Clone, Default)]
pub struct MaintenanceCancellation {
    cancelled: Arc<AtomicBool>,
    interrupt: Arc<Mutex<Option<PruneInterrupter>>>,
}
impl MaintenanceCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        if let Ok(interrupt) = self.interrupt.lock()
            && let Some(interrupt) = interrupt.as_ref()
        {
            interrupt.interrupt();
        }
    }
    fn check(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}
/// Move to a bounded blocking worker after releasing the service owner lock.
pub struct MaintenanceWork {
    operation_id: String,
    session_id: String,
    previous: Option<String>,
    resolution: u64,
    preview: bool,
    lease: SessionLease,
    cancellation: MaintenanceCancellation,
}
pub struct MaintenanceCompletion {
    work: MaintenanceWork,
    result: Result<WorkResult>,
}
struct WorkResult {
    retained: u64,
    reclaimable: u64,
    skipped: bool,
}
impl MaintenanceWork {
    pub fn cancellation(&self) -> MaintenanceCancellation {
        self.cancellation.clone()
    }
    pub fn run(self, at_ms: i64) -> MaintenanceCompletion {
        let result = self.execute(at_ms);
        if let Ok(mut interrupt) = self.cancellation.interrupt.lock() {
            *interrupt = None;
        }
        MaintenanceCompletion { work: self, result }
    }
    fn execute(&self, at_ms: i64) -> Result<WorkResult> {
        timestamp(at_ms)?;
        self.cancellation.check()?;
        let mut reader = SessionReader::open(self.lease.path())?;
        let metadata = reader.metadata()?;
        let marker = reader.marker()?;
        drop(reader);
        // A crash after session commit but before job progress must not prune twice.
        if metadata.retained_resolution_seconds == self.resolution {
            return Ok(WorkResult {
                retained: marker.stored_sample_count,
                reclaimable: 0,
                skipped: true,
            });
        }
        let mut pruner = SessionPruner::open(self.lease.path()).map_err(map_prune)?;
        *self
            .cancellation
            .interrupt
            .lock()
            .map_err(|_| Error::Database)? = Some(pruner.interrupter());
        self.cancellation.check()?;
        let plan = pruner.preview(self.resolution).map_err(map_prune)?;
        let preview = plan.preview();
        self.cancellation.check()?;
        if !self.preview {
            pruner.commit(plan, at_ms).map_err(map_prune)?;
        }
        pruner.close().map_err(map_prune)?;
        // Space reclamation follows logical commit and cannot roll it back.
        if !self.preview && self.cancellation.check().is_ok() {
            let _ = self.compact();
        }
        Ok(WorkResult {
            retained: preview.retained_sample_count,
            reclaimable: preview.estimated_reclaimable_bytes,
            skipped: preview.skipped,
        })
    }
    fn compact(&self) -> Result<()> {
        let db = Connection::open_with_flags(self.lease.path(), OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        db.busy_timeout(std::time::Duration::from_millis(
            crate::limits::SQLITE_BUSY_MS,
        ))?;
        let cancelled = self.cancellation.cancelled.clone();
        db.progress_handler(1000, Some(move || cancelled.load(Ordering::Acquire)));
        self.cancellation.check()?;
        db.execute_batch("PRAGMA incremental_vacuum(256)")?;
        Ok(())
    }
}
fn map_prune(e: PruneError) -> Error {
    match e {
        PruneError::InvalidInput => Error::InvalidInput,
        PruneError::NotFound => Error::NotFound,
        PruneError::Incompatible | PruneError::Corrupt => Error::Corrupt,
        PruneError::Busy | PruneError::Stale => Error::Busy,
        PruneError::Cancelled => Error::Cancelled,
        PruneError::Io => Error::Io,
        PruneError::Database => Error::Database,
    }
}
impl ProductStore {
    pub fn maintenance_recorded_samples(&self) -> Result<u64> {
        let Some(job) = self.maintenance()? else {
            return Ok(0);
        };
        Ok(self.catalog.query_row("SELECT COALESCE(SUM(sample_count),0) FROM session_catalog WHERE pending_delete=0 AND state<>'active' AND ((?='one' AND session_id=?) OR (?='all' AND session_id<=?))",params![job.scope,job.session_id,job.scope,job.upper_session_id],|r|r.get(0))?)
    }
    pub fn maintenance(&self) -> Result<Option<Maintenance>> {
        Ok(self.catalog.query_row("SELECT operation_id,kind,scope,session_id,state,target_resolution_seconds,upper_session_id,last_session_id,completed_sessions,skipped_sessions,retained_samples,reclaimable_bytes,error FROM session_maintenance_job WHERE singleton=1", [], |r| Ok(Maintenance {
            operation_id:r.get(0)?,kind:r.get(1)?,scope:r.get(2)?,session_id:r.get(3)?,state:r.get(4)?,target_resolution_seconds:r.get(5)?,upper_session_id:r.get(6)?,last_session_id:r.get(7)?,completed_sessions:r.get(8)?,skipped_sessions:r.get(9)?,retained_samples:r.get(10)?,reclaimable_bytes:r.get(11)?,error:r.get(12)?
        })).optional()?)
    }
    pub fn begin_maintenance(
        &mut self,
        session_id: Option<&str>,
        resolution: u64,
        preview: bool,
        at_ms: i64,
    ) -> Result<Maintenance> {
        timestamp(at_ms)?;
        if !(1..=300).contains(&resolution) {
            return Err(Error::InvalidInput);
        }
        if self.maintenance()?.is_some_and(|j| !j.terminal()) {
            return Err(Error::Busy);
        }
        if let Some(id) = session_id
            && self.session(id)?.state == "active"
        {
            return Err(Error::ActiveSession);
        }
        let upper: Option<String> = self.catalog.query_row("SELECT MAX(session_id) FROM session_catalog WHERE pending_delete=0 AND state<>'active'",[],|r|r.get(0))?;
        let job = Maintenance {
            operation_id: identifier()?,
            kind: if preview { "prune_preview" } else { "prune" }.into(),
            scope: if session_id.is_some() { "one" } else { "all" }.into(),
            session_id: session_id.map(str::to_owned),
            state: "pending".into(),
            target_resolution_seconds: resolution,
            upper_session_id: upper,
            last_session_id: None,
            completed_sessions: 0,
            skipped_sessions: 0,
            retained_samples: 0,
            reclaimable_bytes: 0,
            error: None,
        };
        self.save_maintenance(&job, at_ms)?;
        Ok(job)
    }
    fn save_maintenance(&self, j: &Maintenance, at_ms: i64) -> Result<()> {
        self.catalog.execute("INSERT OR REPLACE INTO session_maintenance_job(singleton,operation_id,kind,scope,session_id,state,target_resolution_seconds,upper_session_id,last_session_id,completed_sessions,skipped_sessions,retained_samples,reclaimable_bytes,error,updated_at_ms) VALUES(1,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![j.operation_id,j.kind,j.scope,j.session_id,j.state,j.target_resolution_seconds,j.upper_session_id,j.last_session_id,j.completed_sessions,j.skipped_sessions,j.retained_samples,j.reclaimable_bytes,j.error,at_ms])?;
        Ok(())
    }
    pub fn cancel_maintenance(&mut self, operation_id: &str, at_ms: i64) -> Result<Maintenance> {
        timestamp(at_ms)?;
        validate_id(operation_id)?;
        let mut job = self
            .maintenance()?
            .filter(|j| j.operation_id == operation_id)
            .ok_or(Error::NotFound)?;
        if !job.terminal() {
            job.state = if job.state == "running" {
                "cancel_requested"
            } else {
                "cancelled"
            }
            .into();
            self.save_maintenance(&job, at_ms)?;
        }
        Ok(job)
    }
    /// One-session keyset batch; independent of public catalog cursor revisions.
    pub fn next_maintenance_work(&mut self, at_ms: i64) -> Result<Option<MaintenanceWork>> {
        timestamp(at_ms)?;
        let Some(mut job) = self.maintenance()? else {
            return Ok(None);
        };
        if job.terminal() {
            return Ok(None);
        }
        if job.state != "pending" {
            return Err(Error::Busy);
        }
        let id = if job.scope == "one" {
            if job.last_session_id.is_none() {
                job.session_id.clone()
            } else {
                None
            }
        } else {
            self.catalog.query_row("SELECT session_id FROM session_catalog WHERE pending_delete=0 AND state<>'active' AND session_id>? AND session_id<=? ORDER BY session_id LIMIT 1",params![job.last_session_id.as_deref().unwrap_or(""), job.upper_session_id],|r|r.get::<_,String>(0)).optional()?
        };
        let Some(id) = id else {
            job.state = "completed".into();
            self.save_maintenance(&job, at_ms)?;
            return Ok(None);
        };
        let lease = self.acquire_session_lease(&id, true)?;
        job.state = "running".into();
        self.save_maintenance(&job, at_ms)?;
        Ok(Some(MaintenanceWork {
            operation_id: job.operation_id,
            session_id: id,
            previous: job.last_session_id,
            resolution: job.target_resolution_seconds,
            preview: job.kind == "prune_preview",
            lease,
            cancellation: MaintenanceCancellation::default(),
        }))
    }
    pub fn complete_maintenance_work(
        &mut self,
        completion: MaintenanceCompletion,
        at_ms: i64,
    ) -> Result<Maintenance> {
        timestamp(at_ms)?;
        let work = &completion.work;
        let mut job = self
            .maintenance()?
            .filter(|j| {
                j.operation_id == work.operation_id
                    && j.last_session_id == work.previous
                    && matches!(j.state.as_str(), "running" | "cancel_requested")
            })
            .ok_or(Error::Busy)?;
        // Even a failed worker may have committed. Keep the lease until reconciled.
        self.reconcile_session(&work.session_id)?;
        match completion.result {
            Ok(value) => {
                job.completed_sessions = crate::next_revision(job.completed_sessions)?;
                job.skipped_sessions = job
                    .skipped_sessions
                    .checked_add(u64::from(value.skipped))
                    .ok_or(Error::Capacity)?;
                job.retained_samples = job
                    .retained_samples
                    .checked_add(value.retained)
                    .filter(|v| *v <= crate::limits::MAX_REVISION)
                    .ok_or(Error::Capacity)?;
                job.reclaimable_bytes = job
                    .reclaimable_bytes
                    .checked_add(value.reclaimable)
                    .filter(|v| *v <= crate::limits::MAX_REVISION)
                    .ok_or(Error::Capacity)?;
                job.last_session_id = Some(work.session_id.clone());
                job.state = if job.state == "cancel_requested" {
                    "cancelled"
                } else {
                    "pending"
                }
                .into();
            }
            Err(Error::Cancelled) => job.state = "cancelled".into(),
            Err(error) => {
                job.state = "failed".into();
                job.error = Some(error.to_string());
            }
        }
        self.save_maintenance(&job, at_ms)?;
        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod support {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../session-sqlite/tests/support/mod.rs"
        ));
    }
    use float_view_refloat_telemetry_service::RideState;
    use float_view_session_sqlite::SessionWriter;
    #[test]
    fn interrupted_compaction_preserves_committed_prune_and_invalidates_cursor() {
        let dir = support::TestDirectory::new();
        let mut store = ProductStore::open(dir.path()).unwrap();
        store
            .update_settings(0, r#"{"minimumSessionDistance":0}"#)
            .unwrap();
        let board = store
            .ensure_board("local", None, Some(&[1; 12]), 1)
            .unwrap();
        let mut ids = Vec::new();
        for at in 1..=2 {
            let s = store
                .create_session(&board.saved_device_id, at, u64::MAX)
                .unwrap();
            let mut writer =
                SessionWriter::open(store.session_path(&s.session_id).unwrap()).unwrap();
            let samples: Vec<_> = (1..=100)
                .map(|i| {
                    let mut s = support::sample(i, (i - 1) * 1000);
                    s.ride_state = RideState::Riding;
                    s
                })
                .collect();
            writer.append_samples(&samples).unwrap();
            writer.close().unwrap();
            store
                .close_session(&s.session_id, 100_000, "disconnect")
                .unwrap();
            ids.push(s.session_id);
        }
        let cursor = store
            .sessions(None, 1, None, true)
            .unwrap()
            .next_cursor
            .unwrap();
        store
            .begin_maintenance(Some(&ids[0]), 10, false, 1)
            .unwrap();
        let work = store.next_maintenance_work(2).unwrap().unwrap();
        // Commit through the real kernel, then simulate interruption before the
        // optional compaction phase with the worker's real cancellation handle.
        let mut pruner = SessionPruner::open(work.lease.path()).unwrap();
        let plan = pruner.preview(10).unwrap();
        let preview = plan.preview();
        pruner.commit(plan, 3).unwrap();
        pruner.close().unwrap();
        work.cancellation.cancel();
        assert_eq!(work.compact(), Err(Error::Cancelled));
        let marker = SessionReader::open(dir.file(&format!("session-{}.sqlite", ids[0])))
            .unwrap()
            .marker()
            .unwrap();
        assert_eq!(marker.history_revision, 1);
        assert!(marker.stored_sample_count < marker.recorded_sample_count);
        store
            .complete_maintenance_work(
                MaintenanceCompletion {
                    work,
                    result: Ok(WorkResult {
                        retained: preview.retained_sample_count,
                        reclaimable: preview.estimated_reclaimable_bytes,
                        skipped: preview.skipped,
                    }),
                },
                4,
            )
            .unwrap();
        assert!(store.sessions(None, 1, Some(&cursor), true).is_err());
        assert_eq!(store.session(&ids[0]).unwrap().history_revision, 1);
        drop(store);
        assert_eq!(
            ProductStore::open(dir.path())
                .unwrap()
                .session(&ids[0])
                .unwrap()
                .history_revision,
            1
        );
    }
}
