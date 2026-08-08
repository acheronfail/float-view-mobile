use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{CatalogStore, SESSION_SQL, StoreError};

pub const MAX_SESSION_BATCH_SAMPLES: usize = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct SessionSample {
    pub sequence: u64,
    pub observed_at_millis: i64,
    pub slow_observed_at_millis: Option<i64>,
    pub monotonic_elapsed_millis: u64,
    pub ride_state: String,
    pub package_state: Option<String>,
    pub package_mode: Option<String>,
    pub raw_pack_voltage_v: Option<f64>,
    pub duty_cycle: Option<f64>,
    pub speed_kph: Option<f64>,
    pub extension_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewSession<'a> {
    pub session_id: &'a str,
    pub job_id: &'a str,
    pub device_history_id: &'a str,
    pub device_snapshot_json: &'a str,
    pub profile_snapshot_json: &'a str,
    pub profile_revision: u64,
    pub started_at_millis: i64,
    pub requested_idle_millis: u64,
    pub effective_idle_millis: u64,
    pub requested_riding_millis: u64,
    pub effective_riding_millis: u64,
}

pub struct SessionStore {
    catalog: Arc<CatalogStore>,
    writers: Mutex<HashMap<String, Connection>>,
}

impl SessionStore {
    #[must_use]
    pub fn new(catalog: impl Into<Arc<CatalogStore>>) -> Self {
        Self {
            catalog: catalog.into(),
            writers: Mutex::new(HashMap::new()),
        }
    }

    #[must_use]
    pub fn catalog(&self) -> &CatalogStore {
        &self.catalog
    }

    #[must_use]
    pub fn catalog_arc(&self) -> Arc<CatalogStore> {
        Arc::clone(&self.catalog)
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn create(&self, value: &NewSession<'_>) -> Result<(), StoreError> {
        validate_session(value)?;
        let file_name = format!("session-{}.sqlite", value.session_id);
        let path = self.catalog.root.join(&file_name);
        let mut connection = Connection::open(&path)?;
        connection.execute_batch(SESSION_SQL)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO session_metadata(singleton, expected_riding_persistence_ms, expected_idle_persistence_ms) VALUES (1, ?1, ?2)",
            params![value.effective_riding_millis, value.effective_idle_millis],
        )?;
        transaction.execute(
            "INSERT INTO commit_marker(singleton, committed_at_ms) VALUES (1, ?1)",
            [value.started_at_millis],
        )?;
        transaction.commit()?;

        let mut catalog = self
            .catalog
            .writer
            .lock()
            .map_err(|_| StoreError::Poisoned)?;
        let transaction = catalog.transaction()?;
        let active: Option<String> = transaction
            .query_row(
                "SELECT session_id FROM session_catalog WHERE state = 'active' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if active.is_some() {
            drop(transaction);
            drop(catalog);
            let _ = std::fs::remove_file(path);
            return Err(StoreError::Conflict {
                current_revision: 0,
            });
        }
        transaction.execute(
            "INSERT INTO session_catalog(session_id, job_id, device_history_id, device_snapshot_json, profile_snapshot_json, profile_revision, file_name, started_at_ms, state, integrity, requested_idle_ms, effective_idle_ms, requested_riding_ms, effective_riding_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', 'pending', ?9, ?10, ?11, ?12)",
            params![value.session_id, value.job_id, value.device_history_id, value.device_snapshot_json, value.profile_snapshot_json, value.profile_revision, file_name, value.started_at_millis, value.requested_idle_millis, value.effective_idle_millis, value.requested_riding_millis, value.effective_riding_millis],
        )?;
        transaction.commit()?;
        drop(catalog);
        self.writers()?
            .insert(value.session_id.to_owned(), connection);
        Ok(())
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn append_samples(
        &self,
        session_id: &str,
        samples: &[SessionSample],
        committed_at_millis: i64,
    ) -> Result<(), StoreError> {
        if samples.is_empty()
            || samples.len() > MAX_SESSION_BATCH_SAMPLES
            || committed_at_millis < 0
        {
            return Err(StoreError::Invalid);
        }
        let mut writers = self.writers()?;
        let connection = writers.get_mut(session_id).ok_or(StoreError::NotFound)?;
        let transaction = connection.transaction()?;
        let mut last_sequence = 0_u64;
        let mut last_elapsed = 0_u64;
        for sample in samples {
            validate_sample(sample)?;
            if sample.sequence <= last_sequence || sample.monotonic_elapsed_millis < last_elapsed {
                return Err(StoreError::Invalid);
            }
            transaction.execute(
                "INSERT INTO sample(sequence, observed_at_ms, slow_observed_at_ms, monotonic_elapsed_ms, ride_state, package_state, package_mode, raw_pack_voltage_v, duty_cycle, speed_kph, extension_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![sample.sequence, sample.observed_at_millis, sample.slow_observed_at_millis, sample.monotonic_elapsed_millis, sample.ride_state, sample.package_state, sample.package_mode, sample.raw_pack_voltage_v, sample.duty_cycle, sample.speed_kph, sample.extension_json],
            )?;
            last_sequence = sample.sequence;
            last_elapsed = sample.monotonic_elapsed_millis;
        }
        let sample_count = i64::try_from(samples.len()).map_err(|_| StoreError::Invalid)?;
        transaction.execute(
            "UPDATE commit_marker SET committed_sample_sequence = ?1, recorded_sample_count = recorded_sample_count + ?2, stored_sample_count = stored_sample_count + ?2, committed_elapsed_ms = ?3, history_revision = history_revision + 1, committed_at_ms = ?4, last_speed_kph = ?5, last_ride_state = ?6 WHERE singleton = 1",
            params![last_sequence, sample_count, last_elapsed, committed_at_millis, samples.last().and_then(|sample| sample.speed_kph), samples.last().map(|sample| &sample.ride_state)],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn flush_catalog(&self, session_id: &str) -> Result<(), StoreError> {
        let writers = self.writers()?;
        let session = writers.get(session_id).ok_or(StoreError::NotFound)?;
        let marker: (i64, i64, i64, i64) = session.query_row(
            "SELECT committed_sample_sequence, stored_sample_count, committed_elapsed_ms, history_revision FROM commit_marker WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        drop(writers);
        self.catalog.writer.lock().map_err(|_| StoreError::Poisoned)?.execute(
            "UPDATE session_catalog SET committed_sequence = ?1, sample_count = ?2, stored_sample_count = ?2, committed_elapsed_ms = ?3, total_elapsed_ms = ?3, history_revision = ?4 WHERE session_id = ?5 AND state = 'active'",
            params![marker.0, marker.1, marker.2, marker.3, session_id],
        )?;
        Ok(())
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn close(
        &self,
        session_id: &str,
        ended_at_millis: i64,
        reason: &str,
    ) -> Result<(), StoreError> {
        if ended_at_millis < 0 || reason.is_empty() || reason.len() > 128 {
            return Err(StoreError::Invalid);
        }
        self.flush_catalog(session_id)?;
        let connection = self
            .writers()?
            .remove(session_id)
            .ok_or(StoreError::NotFound)?;
        connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        connection
            .close()
            .map_err(|(_, error)| StoreError::Sqlite(error))?;
        self.catalog.writer.lock().map_err(|_| StoreError::Poisoned)?.execute(
            "UPDATE session_catalog SET state = 'completed', integrity = 'complete', ended_at_ms = ?1, close_reason = ?2 WHERE session_id = ?3 AND state = 'active'",
            params![ended_at_millis, reason, session_id],
        )?;
        Ok(())
    }

    fn writers(&self) -> Result<MutexGuard<'_, HashMap<String, Connection>>, StoreError> {
        self.writers.lock().map_err(|_| StoreError::Poisoned)
    }
}

fn validate_session(value: &NewSession<'_>) -> Result<(), StoreError> {
    let valid_id = |id: &str| {
        !id.is_empty()
            && id.len() <= 128
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    };
    if !valid_id(value.session_id)
        || !valid_id(value.job_id)
        || value.device_history_id.is_empty()
        || value.started_at_millis < 0
        || value.effective_idle_millis == 0
        || value.effective_riding_millis == 0
        || value.device_snapshot_json.len() > crate::MAX_CATALOG_JSON_BYTES
        || value.profile_snapshot_json.len() > crate::MAX_CATALOG_JSON_BYTES
        || serde_json::from_str::<serde_json::Value>(value.device_snapshot_json).is_err()
        || serde_json::from_str::<serde_json::Value>(value.profile_snapshot_json).is_err()
    {
        Err(StoreError::Invalid)
    } else {
        Ok(())
    }
}

fn validate_sample(sample: &SessionSample) -> Result<(), StoreError> {
    if sample.sequence == 0
        || sample.observed_at_millis < 0
        || sample.ride_state.is_empty()
        || sample.ride_state.len() > 32
        || sample.extension_json.len() > 16_384
        || !matches!(
            serde_json::from_str::<serde_json::Value>(&sample.extension_json),
            Ok(serde_json::Value::Object(_))
        )
        || [
            sample.raw_pack_voltage_v,
            sample.duty_cycle,
            sample.speed_kph,
        ]
        .into_iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        Err(StoreError::Invalid)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> crate::TestRoot {
        crate::TestRoot::new("float-view-session-")
    }

    #[test]
    fn session_writer_commits_marker_catalog_and_clean_close() {
        let root = root();
        let store = SessionStore::new(CatalogStore::open(&root).expect("catalog"));
        store
            .create(&NewSession {
                session_id: "session",
                job_id: "job",
                device_history_id: "history",
                device_snapshot_json: "{}",
                profile_snapshot_json: "{}",
                profile_revision: 0,
                started_at_millis: 10,
                requested_idle_millis: 1_000,
                effective_idle_millis: 1_000,
                requested_riding_millis: 100,
                effective_riding_millis: 100,
            })
            .expect("session");
        store
            .append_samples(
                "session",
                &[SessionSample {
                    sequence: 1,
                    observed_at_millis: 20,
                    slow_observed_at_millis: None,
                    monotonic_elapsed_millis: 10,
                    ride_state: "riding".into(),
                    package_state: Some("running".into()),
                    package_mode: Some("normal".into()),
                    raw_pack_voltage_v: Some(75.0),
                    duty_cycle: Some(0.2),
                    speed_kph: Some(12.0),
                    extension_json: "{}".into(),
                }],
                20,
            )
            .expect("append");
        store.flush_catalog("session").expect("flush");
        store.close("session", 30, "explicit").expect("close");
        let row: (String, i64, i64) = store
            .catalog
            .writer
            .lock()
            .expect("writer")
            .query_row(
                "SELECT state, committed_sequence, sample_count FROM session_catalog WHERE session_id = 'session'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("row");
        assert_eq!(row, ("completed".into(), 1, 1));
        drop(store);
    }
}
