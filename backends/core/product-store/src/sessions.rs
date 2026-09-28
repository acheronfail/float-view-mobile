use crate::{
    Error, ProductStore, Result,
    boards::{timestamp, validate_history, validate_id},
    identifier, json, limits, root,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use float_view_session_sqlite::{
    CommitMarker, ReaderError, SessionMetadata, SessionReader, SessionWriter, WriterError,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_id: String,
    pub job_id: String,
    pub device_history_id: String,
    pub device_snapshot: serde_json::Value,
    pub profile_snapshot: serde_json::Value,
    pub profile_revision: u64,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub state: String,
    pub integrity: String,
    pub committed_sequence: u64,
    pub committed_event_sequence: u64,
    pub committed_elapsed_ms: u64,
    pub recorded_sample_count: u64,
    pub stored_sample_count: u64,
    pub event_count: u64,
    pub history_revision: u64,
    pub riding_elapsed_ms: u64,
    pub total_elapsed_ms: u64,
    pub distance_m: Option<f64>,
    pub max_speed_mps: Option<f64>,
    pub max_duty_cycle: Option<f64>,
    pub committed_location_sequence: u64,
    pub committed_location_elapsed_ms: u64,
    pub recorded_location_count: u64,
    pub stored_location_count: u64,
    pub location_status: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SessionPage {
    pub items: Vec<Session>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    epoch: String,
    revision: u64,
    history: Option<String>,
    completed_only: bool,
    started: i64,
    id: String,
}
impl ProductStore {
    pub fn catalog_revision(&self) -> u64 {
        self.catalog_revision
    }
    pub fn create_session(
        &mut self,
        board_id: &str,
        at_ms: i64,
        available_bytes: u64,
    ) -> Result<Session> {
        timestamp(at_ms)?;
        if available_bytes < 262_144_000 {
            return Err(Error::Full);
        }
        let active: Option<String> = self
            .catalog
            .query_row(
                "SELECT session_id FROM session_catalog WHERE state='active' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if active.is_some() {
            return Err(Error::Capacity);
        }
        let board = self.board(board_id)?;
        let profile = self.profile(board_id)?;
        let settings = self.settings()?;
        let id = identifier()?;
        let job = identifier()?;
        let filename = format!("session-{id}.sqlite");
        let path = self.root.join(&filename);
        let revision = crate::next_revision(self.catalog_revision)?;
        SessionWriter::create(
            &path,
            SessionMetadata {
                expected_riding_persistence_ms: settings.riding_persist_interval_ms,
                expected_idle_persistence_ms: settings.idle_persist_interval_ms,
                committed_at_epoch_ms: at_ms,
            },
        )?
        .close()?;
        let snapshot = serde_json::json!({"savedDeviceId":board.saved_device_id,"deviceHistoryId":board.device_history_id,"displayName":board.display_name()});
        let insert=self.catalog.execute("INSERT INTO session_catalog(session_id,job_id,device_history_id,device_snapshot_json,profile_snapshot_json,profile_revision,file_name,started_at_ms,state,integrity,requested_idle_ms,effective_idle_ms,requested_riding_ms,effective_riding_ms) VALUES (?,?,?,?,?,?,?,?,'active','complete',?,?,?,?)",params![id,job,board.device_history_id,json(&snapshot,limits::MAX_PROFILE_BYTES)?,json(&profile.values,limits::MAX_PROFILE_BYTES)?,profile.revision,filename,at_ms,settings.idle_persist_interval_ms,settings.idle_persist_interval_ms,settings.riding_persist_interval_ms,settings.riding_persist_interval_ms]);
        if let Err(e) = insert {
            remove_session_files(&path)?;
            return Err(e.into());
        }
        self.catalog_revision = revision;
        self.session(&id)
    }
    pub fn session(&self, id: &str) -> Result<Session> {
        validate_id(id)?;
        self.catalog
            .query_row(
                &format!("{SELECT} WHERE session_id=? AND pending_delete=0"),
                [id],
                row,
            )
            .optional()?
            .ok_or(Error::NotFound)
    }
    /// Resolve only a catalog-owned basename, never a caller-supplied filesystem path.
    pub fn session_path(&self, id: &str) -> Result<PathBuf> {
        validate_id(id)?;
        let name: String = self
            .catalog
            .query_row(
                "SELECT file_name FROM session_catalog WHERE session_id=? AND pending_delete=0",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(Error::NotFound)?;
        if name != format!("session-{id}.sqlite") || name.contains(['/', '\\']) {
            return Err(Error::UnsafePath);
        }
        let path = self.root.join(name);
        root::require_kind(&path, false)?;
        for suffix in ["-wal", "-shm"] {
            let p = PathBuf::from(format!("{}{suffix}", path.display()));
            if p.symlink_metadata().is_ok() {
                root::require_kind(&p, false)?;
            }
        }
        Ok(path)
    }
    pub fn reconcile_session(&mut self, id: &str) -> Result<CommitMarker> {
        let mut reader = SessionReader::open(self.session_path(id)?)?;
        let marker = reader.marker()?;
        self.apply_session_marker(id, &marker)?;
        Ok(marker)
    }
    /// Catalog update follows the session commit. Failure must never roll back
    /// or delete the committed session; the caller reconciles before publication.
    pub fn apply_session_marker(&mut self, id: &str, m: &CommitMarker) -> Result<()> {
        validate_id(id)?;
        let n=self.catalog.execute("UPDATE session_catalog SET committed_sequence=?,committed_event_sequence=?,committed_elapsed_ms=?,stored_sample_count=?,sample_count=?,event_count=?,history_revision=?,riding_elapsed_ms=?,total_elapsed_ms=?,distance_m=?,max_speed_mps=?,max_duty_cycle=?,max_speed_source_sequence=?,max_duty_source_sequence=?,last_sample_elapsed_ms=?,last_speed_mps=?,committed_location_sequence=?,committed_location_elapsed_ms=?,location_count=?,recorded_location_count=?,stored_location_count=?,location_status=?,integrity=CASE WHEN integrity='dirty' THEN 'complete' ELSE integrity END WHERE session_id=? AND pending_delete=0 AND committed_sequence<=? AND history_revision<=? AND committed_event_sequence<=? AND committed_location_sequence<=?",params![m.committed_sample_sequence,m.committed_event_sequence,m.committed_elapsed_ms,m.stored_sample_count,m.recorded_sample_count,m.committed_event_count,m.history_revision,m.riding_elapsed_ms,m.committed_elapsed_ms,m.distance_m,m.maximum_speed_kph.map(|v|v.abs()/3.6),m.maximum_duty_cycle.map(f64::abs),m.maximum_speed_source_sequence,m.maximum_duty_source_sequence,m.committed_elapsed_ms,m.last_speed_kph.map(|v|v.abs()/3.6),m.committed_location_sequence,m.committed_location_elapsed_ms,m.recorded_location_count,m.recorded_location_count,m.stored_location_count,m.last_location_status,id,m.committed_sample_sequence,m.history_revision,m.committed_event_sequence,m.committed_location_sequence])?;
        if n != 1 {
            return Err(Error::NotFound);
        }
        Ok(())
    }
    pub fn sessions(
        &self,
        history: Option<&str>,
        limit: usize,
        cursor: Option<&str>,
        completed_only: bool,
    ) -> Result<SessionPage> {
        if !(1..=limits::MAX_SESSION_PAGE).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        if let Some(h) = history {
            validate_history(h)?;
        }
        let c = cursor
            .map(|raw| {
                if raw.len() > 1024 {
                    return Err(Error::InvalidInput);
                }
                let c: Cursor =
                    serde_json::from_slice(&STANDARD.decode(raw).map_err(|_| Error::InvalidInput)?)
                        .map_err(|_| Error::InvalidInput)?;
                if c.history.as_deref() != history || c.completed_only != completed_only {
                    return Err(Error::InvalidInput);
                }
                if c.epoch != self.cursor_epoch || c.revision != self.catalog_revision {
                    return Err(Error::Conflict {
                        revision: self.catalog_revision,
                    });
                }
                timestamp(c.started)?;
                validate_id(&c.id)?;
                Ok(c)
            })
            .transpose()?;
        let mut statement=self.catalog.prepare(&format!("{SELECT} WHERE pending_delete=0 AND (? IS NULL OR device_history_id=?) AND (?=0 OR state<>'active') AND (? IS NULL OR started_at_ms<? OR (started_at_ms=? AND session_id<?)) ORDER BY started_at_ms DESC,session_id DESC LIMIT ?"))?;
        let started = c.as_ref().map(|c| c.started);
        let key = c.as_ref().map(|c| &c.id);
        let mut items = statement
            .query_map(
                params![
                    history,
                    history,
                    completed_only,
                    started,
                    started,
                    started,
                    key,
                    limit + 1
                ],
                row,
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let more = items.len() > limit;
        if more {
            items.pop();
        }
        let next_cursor = if more {
            items
                .last()
                .map(|last| {
                    Ok::<_, Error>(STANDARD.encode(json(
                        &Cursor {
                            epoch: self.cursor_epoch.clone(),
                            revision: self.catalog_revision,
                            history: history.map(str::to_owned),
                            completed_only,
                            started: last.started_at_ms,
                            id: last.session_id.clone(),
                        },
                        768,
                    )?))
                })
                .transpose()?
        } else {
            None
        };
        Ok(SessionPage { items, next_cursor })
    }
}
const SELECT: &str = "SELECT session_id,job_id,device_history_id,device_snapshot_json,profile_snapshot_json,profile_revision,started_at_ms,ended_at_ms,state,integrity,committed_sequence,committed_event_sequence,committed_elapsed_ms,sample_count,stored_sample_count,event_count,history_revision,riding_elapsed_ms,total_elapsed_ms,distance_m,max_speed_mps,max_duty_cycle,committed_location_sequence,committed_location_elapsed_ms,recorded_location_count,stored_location_count,location_status FROM session_catalog";
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    fn snapshot(r: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<serde_json::Value> {
        let text: String = r.get(index)?;
        if text.len() > limits::MAX_PROFILE_BYTES {
            return Err(rusqlite::Error::InvalidQuery);
        }
        serde_json::from_str(&text).map_err(|_| rusqlite::Error::InvalidQuery)
    }
    Ok(Session {
        session_id: r.get(0)?,
        job_id: r.get(1)?,
        device_history_id: r.get(2)?,
        device_snapshot: snapshot(r, 3)?,
        profile_snapshot: snapshot(r, 4)?,
        profile_revision: r.get(5)?,
        started_at_ms: r.get(6)?,
        ended_at_ms: r.get(7)?,
        state: r.get(8)?,
        integrity: r.get(9)?,
        committed_sequence: r.get(10)?,
        committed_event_sequence: r.get(11)?,
        committed_elapsed_ms: r.get(12)?,
        recorded_sample_count: r.get(13)?,
        stored_sample_count: r.get(14)?,
        event_count: r.get(15)?,
        history_revision: r.get(16)?,
        riding_elapsed_ms: r.get(17)?,
        total_elapsed_ms: r.get(18)?,
        distance_m: r.get(19)?,
        max_speed_mps: r.get(20)?,
        max_duty_cycle: r.get(21)?,
        committed_location_sequence: r.get(22)?,
        committed_location_elapsed_ms: r.get(23)?,
        recorded_location_count: r.get(24)?,
        stored_location_count: r.get(25)?,
        location_status: r.get(26)?,
    })
}
pub(crate) fn remove_session_files(path: &std::path::Path) -> Result<()> {
    for suffix in ["-wal", "-shm", ""] {
        let p = PathBuf::from(format!("{}{suffix}", path.display()));
        if p.symlink_metadata().is_ok() {
            root::require_kind(&p, false)?;
            std::fs::remove_file(p)?;
        }
    }
    Ok(())
}
impl From<WriterError> for Error {
    fn from(e: WriterError) -> Self {
        match e {
            WriterError::Busy => Self::Busy,
            WriterError::Full => Self::Full,
            WriterError::Corrupt | WriterError::Incompatible => Self::Corrupt,
            WriterError::InvalidInput | WriterError::TargetExists => Self::InvalidInput,
            WriterError::Io => Self::Io,
            WriterError::Database => Self::Database,
        }
    }
}
impl From<ReaderError> for Error {
    fn from(e: ReaderError) -> Self {
        match e {
            ReaderError::Busy => Self::Busy,
            ReaderError::Corrupt | ReaderError::Incompatible => Self::Corrupt,
            ReaderError::NotFound => Self::NotFound,
            ReaderError::InvalidInput => Self::InvalidInput,
            ReaderError::Io => Self::Io,
            ReaderError::Cancelled => Self::Busy,
            ReaderError::Database => Self::Database,
        }
    }
}
