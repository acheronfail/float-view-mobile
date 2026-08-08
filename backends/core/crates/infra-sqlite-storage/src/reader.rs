use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, InterruptHandle, OpenFlags, OptionalExtension, params};

use crate::{
    CatalogStore, ElapsedRange, MAX_SERIES_FIELDS, PublishedBoundary, SeriesPoint, StoreError,
};

pub const MAX_SOURCE_ROWS: usize = 200_000;
pub const MAX_SAMPLE_PAGE_ROWS: usize = 2_000;
pub const MAX_EVENT_ROWS: usize = 500;
pub const MAX_LOCATION_ROWS: usize = 2_000;

pub struct SessionReadLease {
    pub session_id: String,
    pub path: PathBuf,
    pub boundary: PublishedBoundary,
    leases: Arc<Mutex<HashMap<String, usize>>>,
}

impl SessionReadLease {
    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn open(self) -> Result<SessionReader, StoreError> {
        let connection = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.execute_batch("PRAGMA query_only = ON; PRAGMA busy_timeout = 1000;")?;
        Ok(SessionReader {
            connection,
            lease: self,
        })
    }
}

impl Drop for SessionReadLease {
    fn drop(&mut self) {
        let Ok(mut leases) = self.leases.lock() else {
            return;
        };
        if let Some(count) = leases.get_mut(&self.session_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                leases.remove(&self.session_id);
            }
        }
    }
}

pub struct SessionReader {
    connection: Connection,
    lease: SessionReadLease,
}

impl SessionReader {
    #[must_use]
    pub fn interrupt_handle(&self) -> InterruptHandle {
        self.connection.get_interrupt_handle()
    }

    #[must_use]
    pub const fn boundary(&self) -> PublishedBoundary {
        self.lease.boundary
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn query_series(
        &self,
        range: ElapsedRange,
        fields: &[String],
        limit: usize,
    ) -> Result<Vec<SeriesPoint>, StoreError> {
        if range.start_elapsed_millis > range.end_elapsed_millis
            || fields.is_empty()
            || fields.len() > MAX_SERIES_FIELDS
            || !(1..=MAX_SOURCE_ROWS).contains(&limit)
        {
            return Err(StoreError::Invalid);
        }
        let expressions = fields
            .iter()
            .map(|field| self.field_expression(field))
            .collect::<Result<Vec<_>, _>>()?;
        let selected = expressions
            .iter()
            .enumerate()
            .map(|(index, expression)| format!("{expression} AS value_{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT sequence, monotonic_elapsed_ms, ride_state, {selected} FROM sample WHERE monotonic_elapsed_ms BETWEEN ?1 AND ?2 AND sequence <= ?3 ORDER BY monotonic_elapsed_ms, sequence LIMIT ?4"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(
            params![
                range.start_elapsed_millis,
                range.end_elapsed_millis,
                self.lease.boundary.sample_sequence,
                limit + 1
            ],
            |row| {
                let mut values = HashMap::with_capacity(fields.len());
                for (index, field) in fields.iter().enumerate() {
                    let value: Option<f64> = row.get(index + 3)?;
                    values.insert(field.clone(), value.filter(|value| value.is_finite()));
                }
                Ok(SeriesPoint {
                    sequence: row.get(0)?,
                    elapsed_millis: row.get(1)?,
                    ride_state: row.get(2)?,
                    values,
                })
            },
        )?;
        let rows = rows.collect::<Result<Vec<_>, _>>()?;
        if rows.len() > limit {
            return Err(StoreError::Invalid);
        }
        Ok(rows)
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn events(&self, range: ElapsedRange) -> Result<Vec<SessionEvent>, StoreError> {
        if range.start_elapsed_millis > range.end_elapsed_millis {
            return Err(StoreError::Invalid);
        }
        let mut statement = self.connection.prepare(
            "SELECT sequence, source_sample_sequence, monotonic_elapsed_ms, kind, details_json FROM session_event WHERE monotonic_elapsed_ms BETWEEN ?1 AND ?2 AND sequence <= ?3 AND kind IN ('packageState', 'packageMode', 'footpadState', 'stopCondition', 'setpointAdjustment', 'wheelslip', 'darkride', 'charging', 'alertReason', 'firmwareFault') ORDER BY monotonic_elapsed_ms, sequence LIMIT ?4",
        )?;
        let rows = statement
            .query_map(
                params![
                    range.start_elapsed_millis,
                    range.end_elapsed_millis,
                    self.lease.boundary.event_sequence,
                    MAX_EVENT_ROWS + 1
                ],
                |row| {
                    Ok(SessionEvent {
                        sequence: row.get(0)?,
                        source_sample_sequence: row.get(1)?,
                        elapsed_millis: row.get(2)?,
                        kind: row.get(3)?,
                        details_json: row.get(4)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        if rows.len() > MAX_EVENT_ROWS
            || rows
                .iter()
                .any(|event| event.kind.len() > 64 || event.details_json.len() > 16_384)
        {
            return Err(StoreError::Invalid);
        }
        Ok(rows)
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn locations(
        &self,
        range: ElapsedRange,
        after_sequence: u64,
        limit: usize,
    ) -> Result<Vec<SessionLocation>, StoreError> {
        if range.start_elapsed_millis > range.end_elapsed_millis
            || !(1..=MAX_LOCATION_ROWS).contains(&limit)
        {
            return Err(StoreError::Invalid);
        }
        let mut statement = self.connection.prepare(
            "SELECT location_sequence, observed_at_ms, monotonic_elapsed_ms, latitude, longitude, accuracy_m FROM location WHERE monotonic_elapsed_ms BETWEEN ?1 AND ?2 AND location_sequence <= ?3 AND location_sequence > ?4 ORDER BY location_sequence LIMIT ?5",
        )?;
        Ok(statement
            .query_map(
                params![
                    range.start_elapsed_millis,
                    range.end_elapsed_millis,
                    self.lease.boundary.location_sequence,
                    after_sequence,
                    limit
                ],
                |row| {
                    Ok(SessionLocation {
                        sequence: row.get(0)?,
                        observed_at_millis: row.get(1)?,
                        elapsed_millis: row.get(2)?,
                        latitude: row.get(3)?,
                        longitude: row.get(4)?,
                        accuracy_m: row.get(5)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn field_source(&self, field: &str) -> Result<&'static str, StoreError> {
        if stable_column(field).is_some() {
            return Ok("stable");
        }
        self.field_expression(field)?;
        Ok("extension")
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn history_configuration(&self) -> Result<HistoryConfiguration, StoreError> {
        self.connection
            .query_row(
                "SELECT expected_riding_persistence_ms, expected_idle_persistence_ms, COALESCE(retained_resolution_seconds, 0) FROM session_metadata WHERE singleton = 1",
                [],
                |row| {
                    Ok(HistoryConfiguration {
                        riding_persistence_millis: row.get(0)?,
                        idle_persistence_millis: row.get(1)?,
                        retained_resolution_seconds: row.get(2)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn events_page(
        &self,
        range: ElapsedRange,
        after: Option<(u64, u64)>,
        limit: usize,
    ) -> Result<Vec<SessionEvent>, StoreError> {
        if range.start_elapsed_millis > range.end_elapsed_millis
            || !(1..=MAX_EVENT_ROWS).contains(&limit)
        {
            return Err(StoreError::Invalid);
        }
        let (after_elapsed, after_sequence) = after.unwrap_or((0, 0));
        let mut statement = self.connection.prepare(
            "SELECT sequence, source_sample_sequence, monotonic_elapsed_ms, kind, details_json FROM session_event WHERE monotonic_elapsed_ms BETWEEN ?1 AND ?2 AND sequence <= ?3 AND (monotonic_elapsed_ms > ?4 OR (monotonic_elapsed_ms = ?4 AND sequence > ?5)) AND kind IN ('packageState', 'packageMode', 'footpadState', 'stopCondition', 'setpointAdjustment', 'wheelslip', 'darkride', 'charging', 'alertReason', 'firmwareFault') ORDER BY monotonic_elapsed_ms, sequence LIMIT ?6",
        )?;
        let rows = statement
            .query_map(
                params![
                    range.start_elapsed_millis,
                    range.end_elapsed_millis,
                    self.lease.boundary.event_sequence,
                    after_elapsed,
                    after_sequence,
                    limit + 1
                ],
                |row| {
                    Ok(SessionEvent {
                        sequence: row.get(0)?,
                        source_sample_sequence: row.get(1)?,
                        elapsed_millis: row.get(2)?,
                        kind: row.get(3)?,
                        details_json: row.get(4)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        if rows
            .iter()
            .any(|event| event.kind.len() > 64 || event.details_json.len() > 16_384)
        {
            return Err(StoreError::Invalid);
        }
        Ok(rows)
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn locations_page(
        &self,
        range: ElapsedRange,
        after_sequence: u64,
        limit: usize,
    ) -> Result<Vec<SessionLocation>, StoreError> {
        if !(1..=MAX_LOCATION_ROWS).contains(&limit) {
            return Err(StoreError::Invalid);
        }
        let fetch = limit + 1;
        let mut statement = self.connection.prepare(
            "SELECT location_sequence, observed_at_ms, monotonic_elapsed_ms, latitude, longitude, accuracy_m FROM location WHERE monotonic_elapsed_ms BETWEEN ?1 AND ?2 AND location_sequence <= ?3 AND location_sequence > ?4 ORDER BY location_sequence LIMIT ?5",
        )?;
        Ok(statement
            .query_map(
                params![
                    range.start_elapsed_millis,
                    range.end_elapsed_millis,
                    self.lease.boundary.location_sequence,
                    after_sequence,
                    fetch
                ],
                |row| {
                    Ok(SessionLocation {
                        sequence: row.get(0)?,
                        observed_at_millis: row.get(1)?,
                        elapsed_millis: row.get(2)?,
                        latitude: row.get(3)?,
                        longitude: row.get(4)?,
                        accuracy_m: row.get(5)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn field_expression(&self, field: &str) -> Result<String, StoreError> {
        if let Some(column) = stable_column(field) {
            return Ok(column.into());
        }
        if field.is_empty()
            || field.len() > 64
            || !field
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        {
            return Err(StoreError::Invalid);
        }
        let exists = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM session_numeric_field WHERE field_name = ?1 AND source = 'extension')",
            [field],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(StoreError::NotFound);
        }
        Ok(format!("json_extract(extension_json, '$.\"{field}\"')"))
    }
}

impl CatalogStore {
    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn acquire_read_lease(&self, session_id: &str) -> Result<SessionReadLease, StoreError> {
        let writer = self.writer.lock().map_err(|_| StoreError::Poisoned)?;
        let row: Option<(String, u64, u64, u64, u64, u64, u64)> = writer
            .query_row(
                "SELECT file_name, committed_sequence, committed_event_sequence, committed_location_sequence, history_revision, committed_elapsed_ms, committed_location_elapsed_ms FROM session_catalog WHERE session_id = ?1 AND pending_delete = 0",
                [session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
            )
            .optional()?;
        let Some((
            file_name,
            sample_sequence,
            event_sequence,
            location_sequence,
            history_revision,
            sample_elapsed_millis,
            location_elapsed_millis,
        )) = row
        else {
            return Err(StoreError::NotFound);
        };
        if file_name.contains('/') || file_name.contains('\\') || !file_name.ends_with(".sqlite") {
            return Err(StoreError::Invalid);
        }
        let mut leases = self.read_leases.lock().map_err(|_| StoreError::Poisoned)?;
        *leases.entry(session_id.to_owned()).or_default() += 1;
        Ok(SessionReadLease {
            session_id: session_id.to_owned(),
            path: self.root.join(file_name),
            boundary: PublishedBoundary {
                sample_sequence,
                event_sequence,
                location_sequence,
                history_revision,
                sample_elapsed_millis,
                location_elapsed_millis,
            },
            leases: Arc::clone(&self.read_leases),
        })
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn delete_session(&self, session_id: &str) -> Result<(), StoreError> {
        let mut writer = self.writer.lock().map_err(|_| StoreError::Poisoned)?;
        let transaction = writer.transaction()?;
        let file_name: String = transaction
            .query_row(
                "SELECT file_name FROM session_catalog WHERE session_id = ?1 AND pending_delete = 0 AND state != 'active'",
                [session_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound)?;
        let leased = self
            .read_leases
            .lock()
            .map_err(|_| StoreError::Poisoned)?
            .get(session_id)
            .copied()
            .unwrap_or(0);
        if leased != 0 {
            return Err(StoreError::Conflict {
                current_revision: 0,
            });
        }
        transaction.execute(
            "UPDATE session_catalog SET pending_delete = 1 WHERE session_id = ?1",
            [session_id],
        )?;
        transaction.commit()?;
        drop(writer);
        remove_session_files(&self.root, &file_name)?;
        self.writer
            .lock()
            .map_err(|_| StoreError::Poisoned)?
            .execute(
                "DELETE FROM session_catalog WHERE session_id = ?1 AND pending_delete = 1",
                [session_id],
            )?;
        Ok(())
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn delete_saved_device_history(&self, saved_device_id: &str) -> Result<usize, StoreError> {
        if saved_device_id.is_empty() || saved_device_id.len() > 512 {
            return Err(StoreError::Invalid);
        }
        let mut writer = self.writer.lock().map_err(|_| StoreError::Poisoned)?;
        let transaction = writer.transaction()?;
        let device_history_id: String = transaction
            .query_row(
                "SELECT device_history_id FROM saved_device WHERE saved_device_id = ?1",
                [saved_device_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound)?;
        let sessions = {
            let mut statement = transaction.prepare(
                "SELECT session_id, state FROM session_catalog WHERE device_history_id = ?1 AND pending_delete = 0",
            )?;
            statement
                .query_map([&device_history_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        if sessions.iter().any(|(_, state)| state == "active") {
            return Err(StoreError::Conflict {
                current_revision: 0,
            });
        }
        let leases = self.read_leases.lock().map_err(|_| StoreError::Poisoned)?;
        if sessions
            .iter()
            .any(|(session_id, _)| leases.get(session_id).copied().unwrap_or(0) != 0)
        {
            return Err(StoreError::Conflict {
                current_revision: 0,
            });
        }
        drop(leases);
        transaction.execute(
            "UPDATE session_catalog SET pending_delete = 1 WHERE device_history_id = ?1 AND pending_delete = 0",
            [&device_history_id],
        )?;
        transaction.execute(
            "DELETE FROM saved_device WHERE saved_device_id = ?1",
            [saved_device_id],
        )?;
        transaction.commit()?;
        drop(writer);
        self.cleanup_pending_deletes()?;
        Ok(sessions.len())
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn cleanup_pending_deletes(&self) -> Result<usize, StoreError> {
        let rows = {
            let writer = self.writer.lock().map_err(|_| StoreError::Poisoned)?;
            let mut statement = writer.prepare(
                "SELECT session_id, file_name FROM session_catalog WHERE pending_delete = 1",
            )?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        for (session_id, file_name) in &rows {
            remove_session_files(&self.root, file_name)?;
            self.writer
                .lock()
                .map_err(|_| StoreError::Poisoned)?
                .execute(
                    "DELETE FROM session_catalog WHERE session_id = ?1 AND pending_delete = 1",
                    [session_id],
                )?;
        }
        Ok(rows.len())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionEvent {
    pub sequence: u64,
    pub source_sample_sequence: Option<u64>,
    pub elapsed_millis: u64,
    pub kind: String,
    pub details_json: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionLocation {
    pub sequence: u64,
    pub observed_at_millis: i64,
    pub elapsed_millis: u64,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryConfiguration {
    pub riding_persistence_millis: u64,
    pub idle_persistence_millis: u64,
    pub retained_resolution_seconds: u64,
}

pub(crate) fn stable_column(field: &str) -> Option<&'static str> {
    Some(match field {
        "packVoltageV" => "raw_pack_voltage_v",
        "dutyCycle" => "duty_cycle",
        "controllerTempC" => "controller_temp_c",
        "motorTempC" => "motor_temp_c",
        "motorCurrentA" => "motor_current_a",
        "batteryCurrentA" => "battery_current_a",
        "erpm" => "erpm",
        "speedKph" => "speed_kph",
        "pitchDeg" => "pitch_deg",
        "balancePitchDeg" => "balance_pitch_deg",
        "rollDeg" => "roll_deg",
        "distanceM" => "distance_m",
        "odometerM" => "odometer_m",
        "batteryLevel" => "battery_level",
        "chargingCurrentA" => "charging_current_a",
        "chargingVoltageV" => "charging_voltage_v",
        _ => return None,
    })
}

fn remove_session_files(root: &std::path::Path, file_name: &str) -> Result<(), StoreError> {
    if file_name.contains('/') || file_name.contains('\\') || !file_name.ends_with(".sqlite") {
        return Err(StoreError::Invalid);
    }
    let path = root.join(file_name);
    for candidate in [
        path.clone(),
        PathBuf::from(format!("{}-wal", path.display())),
        PathBuf::from(format!("{}-shm", path.display())),
    ] {
        match fs::remove_file(candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{NewSession, SessionSample, SessionStore};

    use super::*;

    fn root() -> crate::TestRoot {
        crate::TestRoot::new("float-view-reader-")
    }

    fn sample(sequence: u64) -> SessionSample {
        let numeric = f64::from(u32::try_from(sequence).expect("test sequence"));
        SessionSample {
            sequence,
            observed_at_millis: i64::try_from(sequence * 100).expect("time"),
            slow_observed_at_millis: None,
            monotonic_elapsed_millis: sequence * 100,
            ride_state: "riding".into(),
            package_state: Some("running".into()),
            package_mode: Some("normal".into()),
            raw_pack_voltage_v: Some(80.0 - numeric),
            duty_cycle: Some(0.1 * numeric),
            speed_kph: Some(10.0 * numeric),
            extension_json: "{}".into(),
        }
    }

    #[test]
    fn wal_reader_holds_published_boundary_and_deletion_lease() {
        let root = root();
        let sessions = SessionStore::new(CatalogStore::open(&root).expect("catalog"));
        sessions
            .create(&NewSession {
                session_id: "session",
                job_id: "job",
                device_history_id: "history",
                device_snapshot_json: "{}",
                profile_snapshot_json: "{}",
                profile_revision: 0,
                started_at_millis: 0,
                requested_idle_millis: 1_000,
                effective_idle_millis: 1_000,
                requested_riding_millis: 100,
                effective_riding_millis: 100,
            })
            .expect("create");
        sessions
            .append_samples("session", &[sample(1)], 100)
            .expect("append");
        sessions.flush_catalog("session").expect("publish");
        sessions
            .append_samples("session", &[sample(2)], 200)
            .expect("unpublished append");
        let reader = sessions
            .catalog()
            .acquire_read_lease("session")
            .expect("lease")
            .open()
            .expect("reader");
        assert_eq!(reader.boundary().sample_sequence, 1);
        let rows = reader
            .query_series(
                ElapsedRange {
                    start_elapsed_millis: 0,
                    end_elapsed_millis: 1_000,
                },
                &["packVoltageV".into(), "speedKph".into()],
                10,
            )
            .expect("query");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sequence, 1);
        sessions.close("session", 300, "done").expect("close");
        assert!(matches!(
            sessions.catalog().delete_session("session"),
            Err(StoreError::Conflict { .. })
        ));
        drop(reader);
        sessions
            .catalog()
            .delete_session("session")
            .expect("delete");
        assert!(!root.join("session-session.sqlite").exists());
        drop(sessions);
    }

    #[test]
    fn saved_device_deletion_consumes_owned_history_and_files() {
        let root = root();
        let sessions = SessionStore::new(CatalogStore::open(&root).expect("catalog"));
        sessions
            .catalog()
            .ensure_saved_device("saved", "history", Some("opaque"), Some("Board"), 0)
            .expect("saved device");
        sessions
            .create(&NewSession {
                session_id: "owned",
                job_id: "owned-job",
                device_history_id: "history",
                device_snapshot_json: "{}",
                profile_snapshot_json: "{}",
                profile_revision: 0,
                started_at_millis: 0,
                requested_idle_millis: 1_000,
                effective_idle_millis: 1_000,
                requested_riding_millis: 100,
                effective_riding_millis: 100,
            })
            .expect("create");
        sessions.close("owned", 1, "done").expect("close");
        assert!(root.join("session-owned.sqlite").exists());

        assert_eq!(
            sessions
                .catalog()
                .delete_saved_device_history("saved")
                .expect("delete board"),
            1
        );
        assert!(
            sessions
                .catalog()
                .saved_devices()
                .expect("devices")
                .is_empty()
        );
        assert!(
            sessions
                .catalog()
                .session_records()
                .expect("sessions")
                .is_empty()
        );
        assert!(!root.join("session-owned.sqlite").exists());
        drop(sessions);
    }

    #[test]
    fn dedicated_reader_is_interruptible() {
        let root = root();
        let catalog = CatalogStore::open(&root).expect("catalog");
        let path = root.join("session-interrupt.sqlite");
        let connection = Connection::open(&path).expect("session");
        connection
            .execute_batch(crate::SESSION_SQL)
            .expect("schema");
        drop(connection);
        catalog
            .writer
            .lock()
            .expect("writer")
            .execute(
                "INSERT INTO session_catalog(session_id, job_id, device_history_id, device_snapshot_json, profile_snapshot_json, profile_revision, file_name, started_at_ms, state, integrity, requested_idle_ms, effective_idle_ms, requested_riding_ms, effective_riding_ms) VALUES ('interrupt', 'job', 'history', '{}', '{}', 0, 'session-interrupt.sqlite', 0, 'completed', 'complete', 1000, 1000, 100, 100)",
                [],
            )
            .expect("row");
        let reader = catalog
            .acquire_read_lease("interrupt")
            .expect("lease")
            .open()
            .expect("reader");
        let interrupt = reader.interrupt_handle();
        let worker = std::thread::spawn(move || {
            reader.connection.query_row(
                "WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x + 1 FROM n WHERE x < 100000000) SELECT sum(x) FROM n",
                [],
                |row| row.get::<_, i64>(0),
            )
        });
        std::thread::sleep(Duration::from_millis(5));
        interrupt.interrupt();
        assert!(worker.join().expect("worker").is_err());
        drop(catalog);
    }
}
