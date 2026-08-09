use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params, types::ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{CatalogStore, StoreError, recover_session};

pub const MAX_IMPORT_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_EXPORT_BYTES: u64 = 512 * 1024 * 1024;
pub const ARCHIVE_FORMAT_VERSION: &str = "1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    FloatViewArchive,
    Csv,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportRecord {
    pub export_id: String,
    pub file_name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceSnapshot {
    display_name: Option<String>,
    name: Option<String>,
    #[serde(flatten)]
    extensions: Map<String, Value>,
}

impl DeviceSnapshot {
    fn display_name(&self) -> Option<&str> {
        self.display_name
            .as_deref()
            .or(self.name.as_deref())
            .filter(|value| !value.is_empty() && value.len() <= 128)
    }
}

pub(crate) fn valid_device_snapshot_json(value: &str) -> bool {
    serde_json::from_str::<DeviceSnapshot>(value).is_ok()
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoardProfileSnapshot {
    saved_device_id: Option<String>,
    revision: Option<u64>,
    values: Option<Map<String, Value>>,
    missing_fields: Option<Vec<String>>,
    #[serde(flatten)]
    extensions: Map<String, Value>,
}

pub(crate) fn valid_board_profile_snapshot_json(value: &str) -> bool {
    serde_json::from_str::<BoardProfileSnapshot>(value).is_ok()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveProvenance {
    format: String,
    format_version: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveManifestV1 {
    session_id: Option<String>,
    device_history_id: String,
    #[serde(default)]
    device_snapshot: DeviceSnapshot,
    #[serde(default)]
    board_profile_snapshot: BoardProfileSnapshot,
    board_profile_revision: Option<i64>,
    started_at_ms: Option<i64>,
    ended_at_ms: Option<i64>,
    state: Option<String>,
    integrity: Option<String>,
    committed_sequence: Option<i64>,
    sample_count: Option<i64>,
    event_count: Option<i64>,
    requested_idle_ms: Option<i64>,
    effective_idle_ms: Option<i64>,
    requested_riding_ms: Option<i64>,
    effective_riding_ms: Option<i64>,
    close_reason: Option<String>,
    riding_elapsed_ms: Option<i64>,
    total_elapsed_ms: Option<i64>,
    distance_m: Option<f64>,
    max_speed_mps: Option<f64>,
    max_duty_cycle: Option<f64>,
    provenance: Option<ArchiveProvenance>,
}

impl CatalogStore {
    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn staging_path(&self, token: &str) -> Result<PathBuf, StoreError> {
        validate_token(token)?;
        let staging = self.root.join("staging");
        fs::create_dir_all(&staging)?;
        Ok(staging.join(format!("import-{token}.floatview.tmp")))
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn export_session(
        &self,
        session_id: &str,
        export_id: &str,
        format: ExportFormat,
    ) -> Result<ExportRecord, StoreError> {
        self.export_session_with_policy(session_id, export_id, format, true)
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn export_session_with_policy(
        &self,
        session_id: &str,
        export_id: &str,
        format: ExportFormat,
        include_location: bool,
    ) -> Result<ExportRecord, StoreError> {
        validate_token(export_id)?;
        let writer = self.writer.lock().map_err(|_| StoreError::Poisoned)?;
        let row: Option<(String, String)> = writer
            .query_row(
                "SELECT file_name, state FROM session_catalog WHERE session_id = ?1 AND pending_delete = 0",
                [session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((source_name, state)) = row else {
            return Err(StoreError::NotFound);
        };
        if state == "active" {
            return Err(StoreError::Conflict {
                current_revision: 0,
            });
        }
        validate_file_name(&source_name)?;
        let source = self.root.join(source_name);
        let source_size = fs::metadata(&source)?.len();
        if source_size == 0 || source_size > MAX_EXPORT_BYTES {
            return Err(StoreError::Invalid);
        }
        let exports = self.root.join("exports");
        fs::create_dir_all(&exports)?;
        let extension = match format {
            ExportFormat::FloatViewArchive => "floatview",
            ExportFormat::Csv => "csv",
        };
        let file_name = format!("float-view-{export_id}.{extension}");
        let output = exports.join(&file_name);
        let result = match format {
            ExportFormat::FloatViewArchive => {
                let manifest = session_manifest(&writer, session_id)?;
                drop(writer);
                let source_connection = Connection::open(&source)?;
                source_connection.execute_batch("PRAGMA wal_checkpoint(FULL)")?;
                drop(source_connection);
                fs::copy(&source, &output)?;
                let archive = Connection::open(&output)?;
                if !include_location {
                    archive.execute("DELETE FROM location", [])?;
                    archive.execute(
                        "UPDATE commit_marker SET committed_location_sequence = 0, committed_location_elapsed_ms = 0, recorded_location_count = 0, stored_location_count = 0, last_location_status = 'excluded_from_export' WHERE singleton = 1",
                        [],
                    )?;
                }
                archive.execute_batch(
                    "CREATE TABLE IF NOT EXISTS float_view_archive_metadata(key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);",
                )?;
                archive.execute(
                    "INSERT OR REPLACE INTO float_view_archive_metadata(key, value) VALUES ('format_version', ?1)",
                    [ARCHIVE_FORMAT_VERSION],
                )?;
                archive.execute(
                    "INSERT OR REPLACE INTO float_view_archive_metadata(key, value) VALUES ('session', ?1)",
                    [serde_json::to_string(&manifest).map_err(|_| StoreError::Invalid)?],
                )?;
                archive.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
                Ok(())
            }
            ExportFormat::Csv => {
                drop(writer);
                write_csv(&source, &output)
            }
        };
        if let Err(error) = result {
            let _ = fs::remove_file(&output);
            return Err(error);
        }
        if fs::metadata(&output)?.len() > MAX_EXPORT_BYTES {
            let _ = fs::remove_file(&output);
            return Err(StoreError::Invalid);
        }
        Ok(ExportRecord {
            export_id: export_id.to_owned(),
            file_name,
            path: output,
        })
    }

    /// # Errors
    /// Returns `StoreError` when input validation, storage, serialization, or file I/O fails.
    pub fn import_staged_archive(
        &self,
        staging_path: &Path,
        session_id: &str,
        job_id: &str,
        imported_at_millis: i64,
    ) -> Result<String, StoreError> {
        validate_token(session_id)?;
        validate_token(job_id)?;
        if imported_at_millis < 0
            || staging_path.parent() != Some(self.root.join("staging").as_path())
        {
            return Err(StoreError::Invalid);
        }
        let size = fs::metadata(staging_path)?.len();
        if size == 0 || size > MAX_IMPORT_BYTES {
            let _ = fs::remove_file(staging_path);
            return Err(StoreError::Invalid);
        }
        let destination_name = format!("session-{session_id}.sqlite");
        let destination = self.root.join(&destination_name);
        let result = self.import_validated(
            staging_path,
            &destination,
            &destination_name,
            session_id,
            job_id,
            imported_at_millis,
        );
        if result.is_err() {
            remove_sqlite_files(staging_path);
            remove_sqlite_files(&destination);
        }
        result.map(|()| session_id.to_owned())
    }

    fn import_validated(
        &self,
        staging_path: &Path,
        destination: &Path,
        destination_name: &str,
        session_id: &str,
        job_id: &str,
        imported_at_millis: i64,
    ) -> Result<(), StoreError> {
        let archive = Connection::open(staging_path)?;
        let format_version: Option<String> = archive
            .query_row(
                "SELECT value FROM float_view_archive_metadata WHERE key = 'format_version'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let manifest_text: Option<String> = archive
            .query_row(
                "SELECT value FROM float_view_archive_metadata WHERE key = 'session'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let integrity: String =
            archive.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        let has_marker: bool = archive.query_row(
            "SELECT EXISTS(SELECT 1 FROM commit_marker WHERE singleton = 1)",
            [],
            |row| row.get(0),
        )?;
        let sample_count: u64 =
            archive.query_row("SELECT count(*) FROM sample", [], |row| row.get(0))?;
        if format_version.as_deref() != Some(ARCHIVE_FORMAT_VERSION)
            || integrity != "ok"
            || !has_marker
            || sample_count == 0
        {
            return Err(StoreError::Invalid);
        }
        let manifest: ArchiveManifestV1 =
            serde_json::from_str(manifest_text.as_deref().ok_or(StoreError::Invalid)?)
                .map_err(|_| StoreError::Invalid)?;
        let device_history_id = manifest.device_history_id.as_str();
        if device_history_id.is_empty() || device_history_id.len() > 128 {
            return Err(StoreError::Invalid);
        }
        let started_at = manifest.started_at_ms.unwrap_or(imported_at_millis);
        let ended_at = manifest.ended_at_ms;
        let requested_idle = positive(manifest.requested_idle_ms, 1_000);
        let effective_idle = positive(manifest.effective_idle_ms, requested_idle);
        let requested_riding = positive(manifest.requested_riding_ms, 1_000);
        let effective_riding = positive(manifest.effective_riding_ms, requested_riding);
        let profile_revision = manifest.board_profile_revision.unwrap_or(0).max(0);
        let close_reason = manifest.close_reason.as_deref();
        let display_name = manifest.device_snapshot.display_name().map(str::to_owned);
        let device_snapshot = manifest.device_snapshot;
        let profile_snapshot = manifest.board_profile_snapshot;
        archive.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        drop(archive);
        remove_sidecars(staging_path);
        fs::rename(staging_path, destination)?;

        let mut catalog = self.writer.lock().map_err(|_| StoreError::Poisoned)?;
        let transaction = catalog.transaction()?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT saved_device_id FROM saved_device WHERE device_history_id = ?1",
                [device_history_id],
                |row| row.get(0),
            )
            .optional()?;
        if existing.is_none() {
            let saved_device_id = format!("imported-{session_id}");
            transaction.execute(
                "INSERT INTO saved_device(saved_device_id, device_history_id, advertised_name, created_at_ms) VALUES (?1, ?2, ?3, ?4)",
                params![saved_device_id, device_history_id, display_name, started_at],
            )?;
            transaction.execute(
                "INSERT INTO board_profile(saved_device_id, revision, json, updated_at_ms) VALUES (?1, ?2, ?3, ?4)",
                params![saved_device_id, profile_revision, serde_json::to_string(&profile_snapshot).map_err(|_| StoreError::Invalid)?, imported_at_millis],
            )?;
        }
        transaction.execute(
            "INSERT INTO session_catalog(session_id, job_id, device_history_id, device_snapshot_json, profile_snapshot_json, profile_revision, file_name, started_at_ms, ended_at_ms, state, integrity, requested_idle_ms, effective_idle_ms, requested_riding_ms, effective_riding_ms, close_reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'active', 'pending', ?10, ?11, ?12, ?13, ?14)",
            params![session_id, job_id, device_history_id, serde_json::to_string(&device_snapshot).map_err(|_| StoreError::Invalid)?, serde_json::to_string(&profile_snapshot).map_err(|_| StoreError::Invalid)?, profile_revision, destination_name, started_at, ended_at, requested_idle, effective_idle, requested_riding, effective_riding, close_reason],
        )?;
        recover_session(&transaction, &self.root, session_id, destination_name)?;
        transaction.execute(
            "UPDATE session_catalog SET state = 'completed', integrity = 'complete', close_reason = COALESCE(?1, close_reason) WHERE session_id = ?2",
            params![close_reason, session_id],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn session_manifest(
    connection: &Connection,
    session_id: &str,
) -> Result<ArchiveManifestV1, StoreError> {
    connection
        .query_row(
            "SELECT device_history_id, device_snapshot_json, profile_snapshot_json, profile_revision, started_at_ms, ended_at_ms, state, integrity, committed_sequence, sample_count, event_count, requested_idle_ms, effective_idle_ms, requested_riding_ms, effective_riding_ms, close_reason, riding_elapsed_ms, total_elapsed_ms, distance_m, max_speed_mps, max_duty_cycle FROM session_catalog WHERE session_id = ?1 AND pending_delete = 0",
            [session_id],
            |row| {
                let device: String = row.get(1)?;
                let profile: String = row.get(2)?;
                Ok(ArchiveManifestV1 {
                    session_id: Some(session_id.to_owned()),
                    device_history_id: row.get(0)?,
                    device_snapshot: serde_json::from_str(&device).unwrap_or_default(),
                    board_profile_snapshot: serde_json::from_str(&profile).unwrap_or_default(),
                    board_profile_revision: Some(row.get(3)?),
                    started_at_ms: Some(row.get(4)?),
                    ended_at_ms: row.get(5)?,
                    state: Some(row.get(6)?),
                    integrity: Some(row.get(7)?),
                    committed_sequence: Some(row.get(8)?),
                    sample_count: Some(row.get(9)?),
                    event_count: Some(row.get(10)?),
                    requested_idle_ms: Some(row.get(11)?),
                    effective_idle_ms: Some(row.get(12)?),
                    requested_riding_ms: Some(row.get(13)?),
                    effective_riding_ms: Some(row.get(14)?),
                    close_reason: row.get(15)?,
                    riding_elapsed_ms: Some(row.get(16)?),
                    total_elapsed_ms: Some(row.get(17)?),
                    distance_m: row.get(18)?,
                    max_speed_mps: row.get(19)?,
                    max_duty_cycle: row.get(20)?,
                    provenance: Some(ArchiveProvenance {
                        format: "float-view-archive".into(),
                        format_version: 1,
                    }),
                })
            },
        )
        .map_err(StoreError::from)
}

fn write_csv(source: &Path, output: &Path) -> Result<(), StoreError> {
    let connection =
        Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let file = fs::File::create(output)?;
    let mut output = csv::Writer::from_writer(file);
    let columns = [
        "sequence",
        "observed_at_ms",
        "slow_observed_at_ms",
        "monotonic_elapsed_ms",
        "ride_state",
        "package_state",
        "package_mode",
        "footpad_state",
        "stop_condition",
        "setpoint_adjustment",
        "wheelslip",
        "darkride",
        "charging",
        "alert_reason",
        "firmware_fault",
        "raw_pack_voltage_v",
        "duty_cycle",
        "controller_temp_c",
        "motor_temp_c",
        "motor_current_a",
        "battery_current_a",
        "erpm",
        "speed_kph",
        "pitch_deg",
        "balance_pitch_deg",
        "roll_deg",
        "extension_json",
    ];
    output.write_record(columns).map_err(csv_error)?;
    let sql = format!("SELECT {} FROM sample ORDER BY sequence", columns.join(","));
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let values = (0..columns.len())
            .map(|index| csv_value(row.get_ref(index)))
            .collect::<Result<Vec<_>, _>>()?;
        output.write_record(values).map_err(csv_error)?;
    }
    output.flush().map_err(StoreError::Io)?;
    Ok(())
}

fn csv_value(value: rusqlite::Result<ValueRef<'_>>) -> Result<String, StoreError> {
    Ok(match value? {
        ValueRef::Null => String::new(),
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) if value.is_finite() => value.to_string(),
        ValueRef::Real(_) | ValueRef::Blob(_) => return Err(StoreError::Invalid),
        ValueRef::Text(value) => {
            let value = std::str::from_utf8(value).map_err(|_| StoreError::Invalid)?;
            value.to_owned()
        }
    })
}

fn csv_error(error: csv::Error) -> StoreError {
    match error.into_kind() {
        csv::ErrorKind::Io(error) => StoreError::Io(error),
        _ => StoreError::Invalid,
    }
}

fn validate_token(value: &str) -> Result<(), StoreError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Err(StoreError::Invalid)
    } else {
        Ok(())
    }
}

fn validate_file_name(value: &str) -> Result<(), StoreError> {
    if value.contains('/') || value.contains('\\') || !value.ends_with(".sqlite") {
        Err(StoreError::Invalid)
    } else {
        Ok(())
    }
}

fn positive(value: Option<i64>, fallback: i64) -> i64 {
    value.unwrap_or(fallback).max(1)
}

fn remove_sidecars(path: &Path) {
    let _ = fs::remove_file(format!("{}-wal", path.display()));
    let _ = fs::remove_file(format!("{}-shm", path.display()));
}

fn remove_sqlite_files(path: &Path) {
    remove_sidecars(path);
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use crate::{NewSession, SessionSample, SessionStore};

    use super::*;

    fn root() -> crate::TestRoot {
        crate::TestRoot::new("float-view-archive-")
    }

    fn completed_session(root: &Path) -> SessionStore {
        let sessions = SessionStore::new(CatalogStore::open(root).expect("catalog"));
        sessions
            .create(&NewSession {
                session_id: "original",
                job_id: "job-original",
                device_history_id: "0102030405060708090a0b0c",
                device_snapshot_json: r#"{"displayName":"Imported board","vescUuid":"0102030405060708090a0b0c"}"#,
                profile_snapshot_json: r#"{"battery":"18s"}"#,
                profile_revision: 2,
                started_at_millis: 100,
                requested_idle_millis: 1_000,
                effective_idle_millis: 1_000,
                requested_riding_millis: 100,
                effective_riding_millis: 100,
            })
            .expect("create");
        sessions
            .append_samples(
                "original",
                &[SessionSample {
                    sequence: 1,
                    observed_at_millis: 200,
                    slow_observed_at_millis: None,
                    monotonic_elapsed_millis: 100,
                    ride_state: "riding".into(),
                    package_state: Some("running".into()),
                    package_mode: Some("normal".into()),
                    raw_pack_voltage_v: Some(75.0),
                    duty_cycle: Some(0.2),
                    speed_kph: Some(12.0),
                    extension_json: r#"{"custom":1}"#.into(),
                }],
                200,
            )
            .expect("append");
        sessions.close("original", 300, "complete").expect("close");
        sessions
    }

    #[test]
    fn archive_and_csv_exports_are_portable_and_import_merges_identity() {
        let root = root();
        let sessions = completed_session(&root);
        let archive = sessions
            .catalog()
            .export_session("original", "archive", ExportFormat::FloatViewArchive)
            .expect("archive");
        let archive_db = Connection::open(&archive.path).expect("archive db");
        assert_eq!(
            archive_db
                .query_row(
                    "SELECT value FROM float_view_archive_metadata WHERE key = 'format_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("version"),
            ARCHIVE_FORMAT_VERSION
        );
        let manifest: ArchiveManifestV1 = serde_json::from_str(
            &archive_db
                .query_row(
                    "SELECT value FROM float_view_archive_metadata WHERE key = 'session'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("manifest"),
        )
        .expect("typed manifest");
        assert_eq!(manifest.device_history_id, "0102030405060708090a0b0c");
        assert_eq!(
            manifest.device_snapshot.extensions.get("vescUuid"),
            Some(&Value::String("0102030405060708090a0b0c".into()))
        );
        drop(archive_db);
        let csv = sessions
            .catalog()
            .export_session("original", "csv", ExportFormat::Csv)
            .expect("csv");
        let csv_text = fs::read_to_string(csv.path).expect("csv text");
        assert!(csv_text.starts_with("sequence,observed_at_ms"));
        assert!(csv_text.contains("\"{\"\"custom\"\":1}\""));

        let staging = sessions.catalog().staging_path("import").expect("staging");
        fs::copy(archive.path, &staging).expect("host copy");
        assert_eq!(
            sessions
                .catalog()
                .import_staged_archive(&staging, "imported", "job-imported", 400)
                .expect("import"),
            "imported"
        );
        assert!(!staging.exists());
        let writer = sessions.catalog().writer.lock().expect("writer");
        assert_eq!(
            writer
                .query_row(
                    "SELECT count(*) FROM session_catalog WHERE device_history_id = '0102030405060708090a0b0c'",
                    [],
                    |row| row.get::<_, u64>(0),
                )
                .expect("sessions"),
            2
        );
        assert_eq!(
            writer
                .query_row("SELECT count(*) FROM saved_device", [], |row| row
                    .get::<_, u64>(0))
                .expect("boards"),
            1
        );
        drop(writer);
        drop(sessions);
    }

    #[test]
    fn malformed_staging_is_bounded_consumed_and_cleaned() {
        let root = root();
        let catalog = CatalogStore::open(&root).expect("catalog");
        let staging = catalog.staging_path("bad").expect("staging");
        fs::write(&staging, b"not sqlite").expect("write");
        assert!(matches!(
            catalog.import_staged_archive(&staging, "new", "job", 0),
            Err(StoreError::Sqlite(_) | StoreError::Invalid)
        ));
        assert!(!staging.exists());
        drop(catalog);
    }
}
