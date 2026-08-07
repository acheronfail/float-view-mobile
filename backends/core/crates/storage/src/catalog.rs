#![allow(clippy::missing_errors_doc)]

use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{CATALOG_SQL, STORAGE_VERSION};

pub const MAX_CATALOG_JSON_BYTES: usize = 48 * 1024;
pub const MAX_SCHEMA_CACHE_BYTES: usize = 32 * 1024;

#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Invalid,
    NotFound,
    Conflict { current_revision: u64 },
    Poisoned,
}

impl From<rusqlite::Error> for StoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<std::io::Error> for StoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedDevice {
    pub saved_device_id: String,
    pub device_history_id: String,
    pub opaque_platform_id: Option<String>,
    pub advertised_name: Option<String>,
    pub custom_display_name: Option<String>,
    pub revision: u64,
    pub created_at_millis: i64,
    pub last_seen_at_millis: Option<i64>,
    pub last_connected_at_millis: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoardProfile {
    pub saved_device_id: String,
    pub revision: u64,
    pub json: String,
    pub updated_at_millis: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerRouteHint {
    pub saved_device_id: String,
    pub can_id: u8,
    pub controller_uuid: Option<[u8; 12]>,
    pub updated_at_millis: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settings {
    pub revision: u64,
    pub json: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuneRecord {
    pub configuration_id: String,
    pub revision: u64,
    pub name: String,
    pub description: Option<String>,
    pub package_version: String,
    pub schema_fingerprint: String,
    pub wire_signature: u32,
    pub values_json: String,
    pub created_at_millis: i64,
    pub updated_at_millis: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaCacheIdentity {
    pub firmware_version: String,
    pub hardware_name: String,
    pub package_version: String,
    pub custom_index: u8,
    pub hardware_configuration_crc: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionCatalogRecord {
    pub session_id: String,
    pub device_history_id: String,
    pub device_snapshot_json: String,
    pub started_at_millis: i64,
    pub ended_at_millis: Option<i64>,
    pub state: String,
    pub integrity: String,
    pub committed_sequence: u64,
    pub committed_event_sequence: u64,
    pub sample_count: u64,
    pub stored_sample_count: u64,
    pub event_count: u64,
    pub committed_elapsed_millis: u64,
    pub history_revision: u64,
    pub riding_elapsed_millis: u64,
    pub total_elapsed_millis: u64,
    pub distance_m: Option<f64>,
    pub maximum_speed_mps: Option<f64>,
    pub maximum_duty_cycle: Option<f64>,
    pub committed_location_sequence: u64,
    pub committed_location_elapsed_millis: u64,
    pub recorded_location_count: u64,
    pub stored_location_count: u64,
    pub location_status: String,
}

pub struct CatalogStore {
    pub(crate) root: PathBuf,
    pub(crate) writer: Mutex<Connection>,
    pub(crate) read_leases: Arc<Mutex<HashMap<String, usize>>>,
}

impl CatalogStore {
    pub fn open(root: &Path) -> Result<Self, StoreError> {
        prepare_storage_root(root)?;
        let connection = Connection::open(root.join("catalog.sqlite"))?;
        connection.execute_batch(CATALOG_SQL)?;
        let store = Self {
            root: root.to_path_buf(),
            writer: Mutex::new(connection),
            read_leases: Arc::new(Mutex::new(HashMap::new())),
        };
        store.recover_startup()?;
        store.cleanup_pending_deletes()?;
        Ok(store)
    }

    pub fn ensure_saved_device(
        &self,
        saved_device_id: &str,
        device_history_id: &str,
        opaque_platform_id: Option<&str>,
        advertised_name: Option<&str>,
        at_millis: i64,
    ) -> Result<SavedDevice, StoreError> {
        validate_id(saved_device_id)?;
        validate_id(device_history_id)?;
        validate_optional_text(opaque_platform_id, 512)?;
        validate_optional_text(advertised_name, 512)?;
        if at_millis < 0 {
            return Err(StoreError::Invalid);
        }
        let mut writer = self.writer()?;
        let transaction = writer.transaction()?;
        transaction.execute(
            "INSERT INTO saved_device(saved_device_id, device_history_id, opaque_platform_id, advertised_name, created_at_ms, last_seen_at_ms, last_connected_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?5) ON CONFLICT(saved_device_id) DO UPDATE SET advertised_name = COALESCE(excluded.advertised_name, advertised_name), last_seen_at_ms = excluded.last_seen_at_ms, last_connected_at_ms = excluded.last_connected_at_ms",
            params![saved_device_id, device_history_id, opaque_platform_id, advertised_name, at_millis],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO board_profile(saved_device_id, revision, json, updated_at_ms) VALUES (?1, 0, '{}', ?2)",
            params![saved_device_id, at_millis],
        )?;
        let value = saved_device(&transaction, saved_device_id)?;
        transaction.commit()?;
        Ok(value)
    }

    pub fn saved_devices(&self) -> Result<Vec<SavedDevice>, StoreError> {
        let writer = self.writer()?;
        let mut statement = writer.prepare(
            "SELECT saved_device_id, device_history_id, opaque_platform_id, advertised_name, custom_display_name, revision, created_at_ms, last_seen_at_ms, last_connected_at_ms FROM saved_device ORDER BY created_at_ms, saved_device_id",
        )?;
        Ok(statement
            .query_map([], map_saved_device)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn delete_saved_device(&self, saved_device_id: &str) -> Result<(), StoreError> {
        validate_id(saved_device_id)?;
        let changed = self.writer()?.execute(
            "DELETE FROM saved_device WHERE saved_device_id = ?1",
            [saved_device_id],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::NotFound)
        }
    }

    pub fn session_records(&self) -> Result<Vec<SessionCatalogRecord>, StoreError> {
        let writer = self.writer()?;
        let mut statement = writer.prepare(
            "SELECT session_id, device_history_id, device_snapshot_json, started_at_ms, ended_at_ms, state, integrity, committed_sequence, committed_event_sequence, sample_count, stored_sample_count, event_count, committed_elapsed_ms, history_revision, riding_elapsed_ms, total_elapsed_ms, distance_m, max_speed_mps, max_duty_cycle, committed_location_sequence, committed_location_elapsed_ms, recorded_location_count, stored_location_count, location_status FROM session_catalog WHERE pending_delete = 0 ORDER BY started_at_ms DESC, session_id DESC",
        )?;
        Ok(statement
            .query_map([], map_session_catalog_record)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn session_record(&self, session_id: &str) -> Result<SessionCatalogRecord, StoreError> {
        validate_id(session_id)?;
        self.writer()?
            .query_row(
                "SELECT session_id, device_history_id, device_snapshot_json, started_at_ms, ended_at_ms, state, integrity, committed_sequence, committed_event_sequence, sample_count, stored_sample_count, event_count, committed_elapsed_ms, history_revision, riding_elapsed_ms, total_elapsed_ms, distance_m, max_speed_mps, max_duty_cycle, committed_location_sequence, committed_location_elapsed_ms, recorded_location_count, stored_location_count, location_status FROM session_catalog WHERE session_id = ?1 AND pending_delete = 0",
                [session_id],
                map_session_catalog_record,
            )
            .optional()?
            .ok_or(StoreError::NotFound)
    }

    pub fn update_saved_device_name(
        &self,
        saved_device_id: &str,
        expected_revision: u64,
        custom_display_name: Option<&str>,
    ) -> Result<SavedDevice, StoreError> {
        validate_optional_text(custom_display_name, 512)?;
        if custom_display_name.is_some_and(|name| name.trim().is_empty()) {
            return Err(StoreError::Invalid);
        }
        let mut writer = self.writer()?;
        let transaction = writer.transaction()?;
        let changed = transaction.execute(
            "UPDATE saved_device SET custom_display_name = ?1, revision = revision + 1 WHERE saved_device_id = ?2 AND revision = ?3",
            params![custom_display_name, saved_device_id, expected_revision],
        )?;
        if changed != 1 {
            return Err(conflict_or_missing(
                &transaction,
                "saved_device",
                "saved_device_id",
                saved_device_id,
            )?);
        }
        let value = saved_device(&transaction, saved_device_id)?;
        transaction.commit()?;
        Ok(value)
    }

    pub fn board_profile(&self, saved_device_id: &str) -> Result<BoardProfile, StoreError> {
        let writer = self.writer()?;
        profile(&writer, saved_device_id)
    }

    pub fn update_board_profile(
        &self,
        saved_device_id: &str,
        expected_revision: u64,
        json: &str,
        at_millis: i64,
    ) -> Result<BoardProfile, StoreError> {
        validate_json(json, MAX_CATALOG_JSON_BYTES)?;
        let mut writer = self.writer()?;
        let transaction = writer.transaction()?;
        let changed = transaction.execute(
            "UPDATE board_profile SET revision = revision + 1, json = ?1, updated_at_ms = ?2 WHERE saved_device_id = ?3 AND revision = ?4",
            params![json, at_millis, saved_device_id, expected_revision],
        )?;
        if changed != 1 {
            return Err(conflict_or_missing(
                &transaction,
                "board_profile",
                "saved_device_id",
                saved_device_id,
            )?);
        }
        let value = profile(&transaction, saved_device_id)?;
        transaction.commit()?;
        Ok(value)
    }

    pub fn store_route_hint(&self, hint: &ControllerRouteHint) -> Result<(), StoreError> {
        if hint.can_id > 253 || hint.updated_at_millis < 0 {
            return Err(StoreError::Invalid);
        }
        let uuid = hint.controller_uuid.map(hex_uuid);
        self.writer()?.execute(
            "INSERT INTO controller_route_hint(saved_device_id, can_id, controller_uuid, updated_at_ms) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(saved_device_id) DO UPDATE SET can_id = excluded.can_id, controller_uuid = excluded.controller_uuid, updated_at_ms = excluded.updated_at_ms",
            params![hint.saved_device_id, hint.can_id, uuid, hint.updated_at_millis],
        )?;
        Ok(())
    }

    pub fn route_hint(
        &self,
        saved_device_id: &str,
    ) -> Result<Option<ControllerRouteHint>, StoreError> {
        self.writer()?
            .query_row(
                "SELECT saved_device_id, can_id, controller_uuid, updated_at_ms FROM controller_route_hint WHERE saved_device_id = ?1",
                [saved_device_id],
                |row| {
                    let encoded: Option<String> = row.get(2)?;
                    Ok(ControllerRouteHint {
                        saved_device_id: row.get(0)?,
                        can_id: row.get(1)?,
                        controller_uuid: encoded.as_deref().and_then(decode_uuid),
                        updated_at_millis: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn clear_route_hint(&self, saved_device_id: &str) -> Result<(), StoreError> {
        self.writer()?.execute(
            "DELETE FROM controller_route_hint WHERE saved_device_id = ?1",
            [saved_device_id],
        )?;
        Ok(())
    }

    pub fn settings(&self, default_json: &str) -> Result<Settings, StoreError> {
        validate_json(default_json, MAX_CATALOG_JSON_BYTES)?;
        let writer = self.writer()?;
        writer.execute(
            "INSERT OR IGNORE INTO settings(singleton, revision, json) VALUES (1, 0, ?1)",
            [default_json],
        )?;
        writer
            .query_row(
                "SELECT revision, json FROM settings WHERE singleton = 1",
                [],
                |row| {
                    Ok(Settings {
                        revision: row.get(0)?,
                        json: row.get(1)?,
                    })
                },
            )
            .map_err(StoreError::from)
    }

    pub fn update_settings(
        &self,
        expected_revision: u64,
        json: &str,
    ) -> Result<Settings, StoreError> {
        validate_json(json, MAX_CATALOG_JSON_BYTES)?;
        let writer = self.writer()?;
        let changed = writer.execute(
            "UPDATE settings SET revision = revision + 1, json = ?1 WHERE singleton = 1 AND revision = ?2",
            params![json, expected_revision],
        )?;
        if changed != 1 {
            let current = writer
                .query_row(
                    "SELECT revision FROM settings WHERE singleton = 1",
                    [],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(StoreError::NotFound)?;
            return Err(StoreError::Conflict {
                current_revision: current,
            });
        }
        writer
            .query_row(
                "SELECT revision, json FROM settings WHERE singleton = 1",
                [],
                |row| {
                    Ok(Settings {
                        revision: row.get(0)?,
                        json: row.get(1)?,
                    })
                },
            )
            .map_err(StoreError::from)
    }

    pub fn create_tune(&self, tune: &TuneRecord) -> Result<TuneRecord, StoreError> {
        validate_tune(tune)?;
        self.writer()?.execute(
            "INSERT INTO refloat_configuration(configuration_id, revision, name, description, package_name, package_version, schema_fingerprint, wire_signature, values_json, created_at_ms, updated_at_ms) VALUES (?1, 1, ?2, ?3, 'refloat', ?4, ?5, ?6, ?7, ?8, ?9)",
            params![tune.configuration_id, tune.name, tune.description, tune.package_version, tune.schema_fingerprint, tune.wire_signature, tune.values_json, tune.created_at_millis, tune.updated_at_millis],
        )?;
        self.tune(&tune.configuration_id)
    }

    pub fn tune(&self, configuration_id: &str) -> Result<TuneRecord, StoreError> {
        self.writer()?
            .query_row(
                "SELECT configuration_id, revision, name, description, package_version, schema_fingerprint, wire_signature, values_json, created_at_ms, updated_at_ms FROM refloat_configuration WHERE configuration_id = ?1",
                [configuration_id],
                map_tune,
            )
            .optional()?
            .ok_or(StoreError::NotFound)
    }

    pub fn update_tune(
        &self,
        tune: &TuneRecord,
        expected_revision: u64,
    ) -> Result<TuneRecord, StoreError> {
        validate_tune(tune)?;
        let writer = self.writer()?;
        let changed = writer.execute(
            "UPDATE refloat_configuration SET revision = revision + 1, name = ?1, description = ?2, values_json = ?3, updated_at_ms = ?4 WHERE configuration_id = ?5 AND revision = ?6",
            params![tune.name, tune.description, tune.values_json, tune.updated_at_millis, tune.configuration_id, expected_revision],
        )?;
        if changed != 1 {
            let current = writer
                .query_row(
                    "SELECT revision FROM refloat_configuration WHERE configuration_id = ?1",
                    [&tune.configuration_id],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(StoreError::NotFound)?;
            return Err(StoreError::Conflict {
                current_revision: current,
            });
        }
        drop(writer);
        self.tune(&tune.configuration_id)
    }

    pub fn store_schema_cache(
        &self,
        identity: &SchemaCacheIdentity,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        if bytes.is_empty() || bytes.len() > MAX_SCHEMA_CACHE_BYTES {
            return Err(StoreError::Invalid);
        }
        let directory = self.root.join("schema-cache");
        fs::create_dir_all(&directory)?;
        let digest = Sha256::digest(bytes);
        let mut envelope = Vec::with_capacity(32 + bytes.len());
        envelope.extend_from_slice(&digest);
        envelope.extend_from_slice(bytes);
        fs::write(directory.join(cache_key(identity)), envelope)?;
        Ok(())
    }

    pub fn schema_cache(
        &self,
        identity: &SchemaCacheIdentity,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let path = self.root.join("schema-cache").join(cache_key(identity));
        let envelope = match fs::read(&path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if envelope.len() <= 32 || envelope.len() > MAX_SCHEMA_CACHE_BYTES + 32 {
            let _ = fs::remove_file(path);
            return Ok(None);
        }
        let (expected, bytes) = envelope.split_at(32);
        if Sha256::digest(bytes).as_slice() != expected {
            let _ = fs::remove_file(path);
            return Ok(None);
        }
        Ok(Some(bytes.to_vec()))
    }

    fn recover_startup(&self) -> Result<(), StoreError> {
        let mut writer = self.writer()?;
        let transaction = writer.transaction()?;
        let sessions = {
            let mut statement = transaction.prepare(
                "SELECT session_id, file_name FROM session_catalog WHERE state = 'active'",
            )?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        for (session_id, file_name) in sessions {
            recover_session(&transaction, &self.root, &session_id, &file_name)?;
        }
        transaction.execute(
            "UPDATE session_maintenance_job SET state = 'pending' WHERE singleton = 1 AND state IN ('running', 'cancel_requested')",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn writer(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        self.writer.lock().map_err(|_| StoreError::Poisoned)
    }
}

fn prepare_storage_root(root: &Path) -> Result<(), StoreError> {
    if root.as_os_str().is_empty() || root.parent().is_none() {
        return Err(StoreError::Invalid);
    }
    let marker = root.join(".schema-version");
    let stored = fs::read_to_string(&marker).ok();
    if root.exists() && stored.as_deref().map(str::trim) != Some(STORAGE_VERSION) {
        fs::remove_dir_all(root)?;
    }
    fs::create_dir_all(root)?;
    fs::write(marker, STORAGE_VERSION)?;
    Ok(())
}

pub(crate) fn recover_session(
    catalog: &Transaction<'_>,
    root: &Path,
    session_id: &str,
    file_name: &str,
) -> Result<(), StoreError> {
    if file_name.contains('/') || file_name.contains('\\') || !file_name.ends_with(".sqlite") {
        return Err(StoreError::Invalid);
    }
    let Ok(session) = Connection::open(root.join(file_name)) else {
        catalog.execute("UPDATE session_catalog SET state = 'failed', integrity = 'failed', close_reason = 'recovery_open_failed' WHERE session_id = ?1", [session_id])?;
        return Ok(());
    };
    let quick_check: String = session.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if quick_check != "ok" {
        catalog.execute("UPDATE session_catalog SET state = 'failed', integrity = 'failed', close_reason = 'recovery_integrity_failed' WHERE session_id = ?1", [session_id])?;
        return Ok(());
    }
    let marker = session.query_row(
        "SELECT committed_sample_sequence, committed_event_sequence, recorded_sample_count, stored_sample_count, committed_event_count, committed_elapsed_ms, history_revision, riding_elapsed_ms, distance_m, maximum_speed_kph, maximum_speed_source_sequence, maximum_duty_cycle, maximum_duty_source_sequence, committed_location_sequence, committed_location_elapsed_ms, recorded_location_count, stored_location_count, last_location_status FROM commit_marker WHERE singleton = 1",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, i64>(5)?, row.get::<_, i64>(6)?, row.get::<_, i64>(7)?, row.get::<_, Option<f64>>(8)?, row.get::<_, Option<f64>>(9)?, row.get::<_, Option<i64>>(10)?, row.get::<_, Option<f64>>(11)?, row.get::<_, Option<i64>>(12)?, row.get::<_, i64>(13)?, row.get::<_, i64>(14)?, row.get::<_, i64>(15)?, row.get::<_, i64>(16)?, row.get::<_, String>(17)?)),
    ).optional()?;
    if let Some(value) = marker {
        catalog.execute(
            "UPDATE session_catalog SET state = 'interrupted', integrity = 'recovered', close_reason = 'process_recovery', committed_sequence = ?1, committed_event_sequence = ?2, sample_count = ?3, stored_sample_count = ?4, event_count = ?5, committed_elapsed_ms = ?6, history_revision = ?7, riding_elapsed_ms = ?8, total_elapsed_ms = ?6, distance_m = ?9, max_speed_mps = ?10 / 3.6, max_speed_source_sequence = ?11, max_duty_cycle = ?12, max_duty_source_sequence = ?13, committed_location_sequence = ?14, committed_location_elapsed_ms = ?15, location_count = ?16, recorded_location_count = ?16, stored_location_count = ?17, location_status = ?18 WHERE session_id = ?19",
            params![value.0, value.1, value.2, value.3, value.4, value.5, value.6, value.7, value.8, value.9, value.10, value.11, value.12, value.13, value.14, value.15, value.16, value.17, session_id],
        )?;
    } else {
        catalog.execute("UPDATE session_catalog SET state = 'interrupted', integrity = 'recovered', close_reason = 'process_recovery' WHERE session_id = ?1", [session_id])?;
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), StoreError> {
    if value.is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        Err(StoreError::Invalid)
    } else {
        Ok(())
    }
}

fn validate_optional_text(value: Option<&str>, maximum: usize) -> Result<(), StoreError> {
    if value.is_some_and(|text| text.len() > maximum || text.chars().any(char::is_control)) {
        Err(StoreError::Invalid)
    } else {
        Ok(())
    }
}

fn validate_json(value: &str, maximum: usize) -> Result<(), StoreError> {
    if value.len() > maximum
        || !matches!(serde_json::from_str::<Value>(value), Ok(Value::Object(_)))
    {
        Err(StoreError::Invalid)
    } else {
        Ok(())
    }
}

fn validate_tune(tune: &TuneRecord) -> Result<(), StoreError> {
    validate_id(&tune.configuration_id)?;
    if tune.name.trim().is_empty()
        || tune.name.len() > 128
        || tune
            .description
            .as_ref()
            .is_some_and(|value| value.len() > 1024)
        || tune.package_version.is_empty()
        || tune.package_version.len() > 128
        || tune.schema_fingerprint.len() != 64
        || !tune
            .schema_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || tune.updated_at_millis < tune.created_at_millis
    {
        return Err(StoreError::Invalid);
    }
    validate_json(&tune.values_json, MAX_CATALOG_JSON_BYTES)
}

fn saved_device(connection: &Connection, id: &str) -> Result<SavedDevice, StoreError> {
    connection
        .query_row(
            "SELECT saved_device_id, device_history_id, opaque_platform_id, advertised_name, custom_display_name, revision, created_at_ms, last_seen_at_ms, last_connected_at_ms FROM saved_device WHERE saved_device_id = ?1",
            [id], map_saved_device,
        )
        .optional()?
        .ok_or(StoreError::NotFound)
}

fn map_saved_device(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavedDevice> {
    Ok(SavedDevice {
        saved_device_id: row.get(0)?,
        device_history_id: row.get(1)?,
        opaque_platform_id: row.get(2)?,
        advertised_name: row.get(3)?,
        custom_display_name: row.get(4)?,
        revision: row.get(5)?,
        created_at_millis: row.get(6)?,
        last_seen_at_millis: row.get(7)?,
        last_connected_at_millis: row.get(8)?,
    })
}

fn map_session_catalog_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionCatalogRecord> {
    Ok(SessionCatalogRecord {
        session_id: row.get(0)?,
        device_history_id: row.get(1)?,
        device_snapshot_json: row.get(2)?,
        started_at_millis: row.get(3)?,
        ended_at_millis: row.get(4)?,
        state: row.get(5)?,
        integrity: row.get(6)?,
        committed_sequence: row.get(7)?,
        committed_event_sequence: row.get(8)?,
        sample_count: row.get(9)?,
        stored_sample_count: row.get(10)?,
        event_count: row.get(11)?,
        committed_elapsed_millis: row.get(12)?,
        history_revision: row.get(13)?,
        riding_elapsed_millis: row.get(14)?,
        total_elapsed_millis: row.get(15)?,
        distance_m: row.get(16)?,
        maximum_speed_mps: row.get(17)?,
        maximum_duty_cycle: row.get(18)?,
        committed_location_sequence: row.get(19)?,
        committed_location_elapsed_millis: row.get(20)?,
        recorded_location_count: row.get(21)?,
        stored_location_count: row.get(22)?,
        location_status: row.get(23)?,
    })
}

fn profile(connection: &Connection, id: &str) -> Result<BoardProfile, StoreError> {
    connection.query_row("SELECT saved_device_id, revision, json, updated_at_ms FROM board_profile WHERE saved_device_id = ?1", [id], |row| Ok(BoardProfile { saved_device_id: row.get(0)?, revision: row.get(1)?, json: row.get(2)?, updated_at_millis: row.get(3)? })).optional()?.ok_or(StoreError::NotFound)
}

fn conflict_or_missing(
    connection: &Connection,
    table: &str,
    key: &str,
    value: &str,
) -> Result<StoreError, StoreError> {
    if !matches!(
        (table, key),
        ("saved_device" | "board_profile", "saved_device_id")
    ) {
        return Err(StoreError::Invalid);
    }
    let sql = format!("SELECT revision FROM {table} WHERE {key} = ?1");
    Ok(connection
        .query_row(&sql, [value], |row| row.get(0))
        .optional()?
        .map_or(StoreError::NotFound, |current_revision| {
            StoreError::Conflict { current_revision }
        }))
}

fn map_tune(row: &rusqlite::Row<'_>) -> rusqlite::Result<TuneRecord> {
    Ok(TuneRecord {
        configuration_id: row.get(0)?,
        revision: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        package_version: row.get(4)?,
        schema_fingerprint: row.get(5)?,
        wire_signature: row.get(6)?,
        values_json: row.get(7)?,
        created_at_millis: row.get(8)?,
        updated_at_millis: row.get(9)?,
    })
}

fn hex_uuid(value: [u8; 12]) -> String {
    value
        .iter()
        .fold(String::with_capacity(24), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a String is infallible");
            output
        })
}

fn decode_uuid(value: &str) -> Option<[u8; 12]> {
    if value.len() != 24 {
        return None;
    }
    let mut output = [0; 12];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(output)
}

fn cache_key(identity: &SchemaCacheIdentity) -> String {
    let mut hasher = Sha256::new();
    hasher.update(identity.firmware_version.as_bytes());
    hasher.update([0]);
    hasher.update(identity.hardware_name.as_bytes());
    hasher.update([0]);
    hasher.update(identity.package_version.as_bytes());
    hasher.update([identity.custom_index]);
    hasher.update(identity.hardware_configuration_crc.to_be_bytes());
    format!("{:x}.cache", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::SESSION_SQL;

    fn temporary_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "float-view-storage-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn catalog_serializes_devices_profiles_settings_tunes_and_schema_cache() {
        let root = temporary_root("catalog");
        let store = CatalogStore::open(&root).expect("store");
        let saved = store
            .ensure_saved_device("saved", "history", Some("opaque"), Some("Board"), 10)
            .expect("saved");
        assert_eq!(saved.revision, 0);
        let saved = store
            .update_saved_device_name("saved", 0, Some("Custom"))
            .expect("rename");
        assert_eq!(saved.custom_display_name.as_deref(), Some("Custom"));
        assert!(matches!(
            store.update_saved_device_name("saved", 0, Some("Stale")),
            Err(StoreError::Conflict {
                current_revision: 1
            })
        ));
        let profile = store
            .update_board_profile("saved", 0, r#"{"battery":"18s"}"#, 20)
            .expect("profile");
        assert_eq!(profile.revision, 1);
        let hint = ControllerRouteHint {
            saved_device_id: "saved".into(),
            can_id: 42,
            controller_uuid: Some([0xab; 12]),
            updated_at_millis: 20,
        };
        store.store_route_hint(&hint).expect("hint");
        assert_eq!(store.route_hint("saved").expect("hint"), Some(hint));

        let settings = store.settings(r#"{"theme":"system"}"#).expect("settings");
        assert_eq!(settings.revision, 0);
        assert_eq!(
            store
                .update_settings(0, r#"{"theme":"dark"}"#)
                .expect("update")
                .revision,
            1
        );

        let tune = TuneRecord {
            configuration_id: "tune".into(),
            revision: 1,
            name: "Street".into(),
            description: None,
            package_version: "1.2.3".into(),
            schema_fingerprint: "fa40717969d1d86b429d7c78b59144b8387b4850be83a72fd3c7cf15ffecd01c"
                .into(),
            wire_signature: 2_427_955_642,
            values_json: r#"{"kp":1}"#.into(),
            created_at_millis: 30,
            updated_at_millis: 30,
        };
        assert_eq!(store.create_tune(&tune).expect("tune").revision, 1);

        let identity = SchemaCacheIdentity {
            firmware_version: "6.06".into(),
            hardware_name: "75_300_R2".into(),
            package_version: "1.2.3".into(),
            custom_index: 0,
            hardware_configuration_crc: 123,
        };
        store
            .store_schema_cache(&identity, b"compressed")
            .expect("cache");
        assert_eq!(
            store.schema_cache(&identity).expect("cache"),
            Some(b"compressed".to_vec())
        );
        drop(store);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn storage_version_recreates_and_startup_recovers_to_commit_marker() {
        let root = temporary_root("recovery");
        fs::create_dir_all(&root).expect("root");
        fs::write(root.join(".schema-version"), "old").expect("marker");
        fs::write(root.join("obsolete"), "remove").expect("obsolete");
        let store = CatalogStore::open(&root).expect("store");
        assert!(!root.join("obsolete").exists());

        let session_name = "session-recovery.sqlite";
        let session = Connection::open(root.join(session_name)).expect("session");
        session.execute_batch(SESSION_SQL).expect("schema");
        session
            .execute(
                "INSERT INTO commit_marker(singleton, committed_sample_sequence, committed_event_sequence, recorded_sample_count, stored_sample_count, committed_event_count, committed_elapsed_ms, history_revision, committed_at_ms, riding_elapsed_ms, committed_location_sequence, committed_location_elapsed_ms, recorded_location_count, stored_location_count, last_location_status) VALUES (1, 7, 2, 8, 7, 2, 900, 3, 1000, 500, 4, 800, 5, 4, 'recording')",
                [],
            )
            .expect("marker");
        drop(session);
        store
            .writer()
            .expect("writer")
            .execute(
                "INSERT INTO session_catalog(session_id, job_id, device_history_id, device_snapshot_json, profile_snapshot_json, profile_revision, file_name, started_at_ms, state, integrity, requested_idle_ms, effective_idle_ms, requested_riding_ms, effective_riding_ms) VALUES ('session', 'job', 'history', '{}', '{}', 0, ?1, 0, 'active', 'pending', 1000, 1000, 100, 100)",
                [session_name],
            )
            .expect("catalog session");
        drop(store);

        let recovered = CatalogStore::open(&root).expect("reopen");
        let row: (String, i64, i64, i64) = recovered
            .writer()
            .expect("writer")
            .query_row(
                "SELECT state, committed_sequence, stored_sample_count, committed_location_sequence FROM session_catalog WHERE session_id = 'session'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("recovered row");
        assert_eq!(row, ("interrupted".into(), 7, 7, 4));
        drop(recovered);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
