use crate::{
    Error, ProductStore, Result, Session,
    boards::{timestamp, validate_id},
    limits, root,
    sessions::remove_session_files,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(crate) type Leases = Arc<Mutex<BTreeMap<String, LeaseCount>>>;
#[derive(Default)]
pub(crate) struct LeaseCount {
    pub readers: usize,
    pub exclusive: bool,
}
/// A worker retains this lease until its file access has actually ended.
pub struct SessionLease {
    id: String,
    path: PathBuf,
    leases: Leases,
    _root_lock: crate::root::RootLock,
}
impl SessionLease {
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}
impl Drop for SessionLease {
    fn drop(&mut self) {
        if let Ok(mut leases) = self.leases.lock()
            && let Some(n) = leases.get_mut(&self.id)
        {
            n.readers -= 1;
            if n.readers == 0 {
                leases.remove(&self.id);
            }
        }
    }
}
impl ProductStore {
    pub fn lease_session(&self, id: &str) -> Result<SessionLease> {
        self.acquire_session_lease(id, false)
    }
    pub(crate) fn acquire_session_lease(&self, id: &str, exclusive: bool) -> Result<SessionLease> {
        let path = self.session_path(id)?;
        let mut leases = self.leases.lock().map_err(|_| Error::Database)?;
        if leases.values().map(|n| n.readers).sum::<usize>() >= 6 {
            return Err(Error::Capacity);
        }
        if leases.get(id).is_some_and(|n| exclusive || n.exclusive) {
            return Err(Error::Busy);
        }
        let count = leases.entry(id.into()).or_default();
        count.readers += 1;
        count.exclusive = exclusive;
        Ok(SessionLease {
            id: id.into(),
            path,
            leases: self.leases.clone(),
            _root_lock: self.root_lock.clone(),
        })
    }
    /// Call after the runtime has detached/checkpointed its writer.
    pub fn close_session(&mut self, id: &str, at_ms: i64, reason: &str) -> Result<Option<Session>> {
        timestamp(at_ms)?;
        if reason.is_empty() || reason.len() > 128 {
            return Err(Error::InvalidInput);
        }
        let current = self.session(id)?;
        if current.state != "active" {
            return Ok(Some(current));
        }
        if at_ms < current.started_at_ms {
            return Err(Error::InvalidInput);
        }
        self.reconcile_session(id)?;
        let retain = self.should_retain(id)?;
        let revision = crate::next_revision(self.catalog_revision)?;
        self.catalog.execute("UPDATE session_catalog SET state='completed',ended_at_ms=?,close_reason=? WHERE session_id=? AND state='active'",params![at_ms,reason,id])?;
        self.catalog_revision = revision;
        if retain {
            Ok(Some(self.session(id)?))
        } else {
            self.delete_session(id)?;
            Ok(None)
        }
    }
    fn should_retain(&self, id: &str) -> Result<bool> {
        let session = self.session(id)?;
        let db =
            Connection::open_with_flags(self.session_path(id)?, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        db.busy_timeout(std::time::Duration::from_millis(limits::SQLITE_BUSY_MS))?;
        let riding: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sample WHERE ride_state='riding' AND sequence<=?)",
            [session.committed_sequence],
            |r| r.get(0),
        )?;
        let marked: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM session_event WHERE kind='session_marker' AND sequence<=?)", [session.committed_event_sequence], |r| r.get(0))?;
        Ok(marked
            || (riding
                && session.distance_m.unwrap_or(0.) >= self.settings()?.minimum_session_distance))
    }
    pub fn delete_session(&mut self, id: &str) -> Result<()> {
        validate_id(id)?;
        if self
            .leases
            .lock()
            .map_err(|_| Error::Database)?
            .get(id)
            .is_some_and(|n| n.exclusive)
        {
            return Err(Error::Busy);
        }
        let current: Option<(String, bool)> = self
            .catalog
            .query_row(
                "SELECT state,pending_delete FROM session_catalog WHERE session_id=?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((state, pending)) = current else {
            return Ok(());
        };
        if state == "active" {
            return Err(Error::ActiveSession);
        }
        if !pending {
            let revision = crate::next_revision(self.catalog_revision)?;
            self.catalog.execute(
                "UPDATE session_catalog SET pending_delete=1 WHERE session_id=?",
                [id],
            )?;
            self.catalog_revision = revision;
        }
        self.cleanup_deletes()
    }
    pub fn cancel_pending_delete(&mut self, id: &str) -> Result<bool> {
        validate_id(id)?;
        let filename: Option<String> = self
            .catalog
            .query_row(
                "SELECT file_name FROM session_catalog WHERE session_id=? AND pending_delete=1 AND EXISTS(SELECT 1 FROM saved_device WHERE saved_device.device_history_id=session_catalog.device_history_id)",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        let Some(filename) = filename else {
            return Ok(false);
        };
        let path = self.owned_session_path(id, &filename)?;
        if !path.try_exists()? {
            return Ok(false);
        }
        root::require_kind(&path, false)?;
        let revision = crate::next_revision(self.catalog_revision)?;
        self.catalog.execute(
            "UPDATE session_catalog SET pending_delete=0 WHERE session_id=? AND pending_delete=1",
            [id],
        )?;
        self.catalog_revision = revision;
        Ok(true)
    }
    pub fn forget_board(&mut self, id: &str) -> Result<()> {
        let board = self.board(id)?;
        let leases = self.leases.lock().map_err(|_| Error::Database)?;
        for (session, _) in leases.iter().filter(|(_, lease)| lease.exclusive) {
            if self.session(session)?.device_history_id == board.device_history_id {
                return Err(Error::Busy);
            }
        }
        drop(leases);
        let tx = self
            .catalog
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM session_catalog WHERE device_history_id=? AND state='active')",[&board.device_history_id],|r|r.get(0))?;
        if active {
            return Err(Error::ActiveSession);
        }
        let revision = crate::next_revision(self.catalog_revision)?;
        tx.execute(
            "UPDATE session_catalog SET pending_delete=1 WHERE device_history_id=?",
            [&board.device_history_id],
        )?;
        let mut settings = crate::settings::read(&tx)?;
        if settings.auto_connect_saved_device_id.as_deref() == Some(id) {
            settings.auto_connect_saved_device_id = None;
            settings.revision = crate::next_revision(settings.revision)?;
            tx.execute(
                "UPDATE settings SET revision=?,json=? WHERE singleton=1",
                params![
                    settings.revision,
                    crate::json(&settings, limits::MAX_SETTINGS_BYTES)?
                ],
            )?;
        }
        tx.execute("DELETE FROM saved_device WHERE saved_device_id=?", [id])?;
        tx.commit()?;
        self.catalog_revision = revision;
        self.cleanup_deletes()
    }
    /// Repeatable bounded keyset batches. Leases prevent deleting files being
    /// read/exported; no reader or OS presentation callback owns catalog mutation.
    pub fn cleanup_deletes(&mut self) -> Result<()> {
        let mut after = String::new();
        loop {
            let rows = {
                let mut s=self.catalog.prepare("SELECT session_id,file_name FROM session_catalog WHERE pending_delete=1 AND session_id>? ORDER BY session_id LIMIT ?")?;
                s.query_map(params![after, limits::RECOVERY_BATCH], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
            };
            if rows.is_empty() {
                break;
            }
            for (id, file) in rows {
                after = id.clone();
                if self
                    .leases
                    .lock()
                    .map_err(|_| Error::Database)?
                    .contains_key(&id)
                {
                    continue;
                }
                let path = self.owned_session_path(&id, &file)?;
                remove_session_files(&path)?;
                self.catalog.execute(
                    "DELETE FROM session_catalog WHERE session_id=? AND pending_delete=1",
                    [id],
                )?;
            }
        }
        Ok(())
    }
    fn owned_session_path(&self, id: &str, name: &str) -> Result<PathBuf> {
        if name != format!("session-{id}.sqlite") || name.contains(['/', '\\']) {
            return Err(Error::UnsafePath);
        }
        Ok(self.root.join(name))
    }
    pub(crate) fn recover(&mut self) -> Result<()> {
        self.cleanup_deletes()?;
        let mut after = String::new();
        loop {
            let rows = {
                let mut s=self.catalog.prepare("SELECT session_id,state FROM session_catalog WHERE pending_delete=0 AND session_id>? ORDER BY session_id LIMIT ?")?;
                s.query_map(params![after, limits::RECOVERY_BATCH], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
            };
            if rows.is_empty() {
                break;
            }
            for (id, state) in rows {
                after = id.clone();
                self.reconcile_session(&id)?;
                if state == "active" {
                    let retain = self.should_retain(&id)?;
                    self.catalog.execute("UPDATE session_catalog SET state='interrupted',integrity='recovered',close_reason='process_interruption' WHERE session_id=? AND state='active'",[&id])?;
                    if !retain {
                        self.delete_session(&id)?;
                    }
                }
            }
        }
        self.catalog.execute("UPDATE session_maintenance_job SET state=CASE state WHEN 'cancel_requested' THEN 'cancelled' ELSE 'pending' END WHERE singleton=1 AND state IN ('running','cancel_requested')",[])?;
        Ok(())
    }
}
