#![recursion_limit = "256"]

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use app_coordinator::{
    ApplicationActor, ReconnectDecision, ReconnectModel, RequestCompletion, RequestKind,
    RequestPriority, RequestSpec, ServerPublication, TelemetryFamily, TelemetryPollingModel,
    TrackingMode,
};
use domain_types::{FirmwareDeviceUuid, FirmwareRecord, PlatformAssociation};
use host_ports::{
    BluetoothAuthorization, BluetoothConnectionState, BluetoothRadio, CompletionStatus,
    HostCommand, OperationCompletion, PermissionKind,
};
use infra_loopback_server::{ContractDispatcher, ContractError, ContractRequest};
use infra_sqlite_storage::{
    CatalogStore, MAX_IMPORT_BYTES, NewSession, SchemaCacheIdentity, SessionCatalogRecord,
    SessionSample, SessionStore, StoreError, TuneRecord,
};
use protocol_vesc::{
    ControllerRoute, ExpectedResponse, VescCommand, decode_firmware, encode_frame, route_command,
};
use protocol_vesc_bms::{BmsCandidate, BmsConnection, BmsSnapshot, DiscoveryStatus};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

mod board_profile;
mod settings;
#[cfg(feature = "software-host")]
mod software_host;

#[cfg(test)]
const SAVED_ID: &str = "saved-test-board";
#[cfg(test)]
const HISTORY_ID: &str = "history-test-board";

mod composition;
mod contract_mapping;
mod contract_state;
pub use composition::*;
use contract_mapping::{
    ConfigurationValueDto, TuneIdentityDto, configuration_field_value, configuration_schema_pages,
    configuration_summary_value, configuration_values_value, encode_configuration_values_json,
    parse_configuration_values, parse_tune_identity, tune_description, tune_identity_value,
    tune_name, tune_record_identity, tune_summary_value, tune_value,
};
use contract_state::{
    ActiveConnection, AppState, BluetoothState as ContractBluetoothState, DiscoveredDevice,
    DocumentError, DocumentOperationState, LiveLocation, LocationState, SavedDevice,
    SessionOperationState, SessionSummary,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyParams {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettingsUpdateParams {
    expected_revision: u64,
    patch: settings::SettingsPatch,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedDeviceUpdateParams {
    saved_device_id: String,
    expected_revision: u64,
    #[serde(deserialize_with = "deserialize_present")]
    custom_display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedDeviceIdParams {
    saved_device_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoardProfileUpdateParams {
    saved_device_id: String,
    expected_revision: u64,
    patch: board_profile::BoardProfilePatch,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionListParams {
    saved_device_id: String,
    limit: usize,
    cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionIdParams {
    session_id: String,
}

#[derive(Clone, Copy, Deserialize)]
enum ExportFormatParam {
    #[serde(rename = "float-view-archive")]
    FloatViewArchive,
    #[serde(rename = "csv")]
    Csv,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionExportParams {
    session_id: String,
    format: ExportFormatParam,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionDeviceSnapshot {
    display_name: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OperationIdParams {
    operation_id: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PruneScope {
    One,
    All,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PruneParams {
    scope: PruneScope,
    session_id: Option<String>,
    target_resolution_seconds: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ScanParams {
    #[serde(default, deserialize_with = "deserialize_present")]
    timeout_ms: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CursorParams {
    cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConfigurationReadParams {
    source: ConfigurationSource,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum ConfigurationSource {
    Current,
    Default,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConfigurationWriteParams {
    package_version: String,
    schema_fingerprint: String,
    wire_signature: u32,
    base_revision: u64,
    values: HashMap<String, ConfigurationValueDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TuneApplyParams {
    tune_id: String,
    expected_revision: u64,
    base_revision: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TuneListParams {
    identity: TuneIdentityDto,
    limit: usize,
    cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TuneIdParams {
    tune_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TuneCreateParams {
    identity: TuneIdentityDto,
    source_revision: u64,
    values: HashMap<String, ConfigurationValueDto>,
    name: String,
    #[serde(deserialize_with = "deserialize_present")]
    description: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TuneUpdateParams {
    tune_id: String,
    expected_revision: u64,
    values: HashMap<String, ConfigurationValueDto>,
    name: String,
    #[serde(deserialize_with = "deserialize_present")]
    description: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TuneDeleteParams {
    tune_id: String,
    expected_revision: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConnectParams {
    device_id: String,
    tracking_mode: TrackingModeParam,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum TrackingModeParam {
    Saved,
    Guest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectControllerParams {
    device_id: String,
    can_id: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetControlParams {
    control: ControlParam,
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ControlParam {
    Package,
    Flywheel,
    Leds,
    Headlights,
}
struct ContractBackend {
    inner: Mutex<BackendState>,
    update: Mutex<Option<Value>>,
    catalog: Arc<CatalogStore>,
    sessions: Arc<SessionStore>,
}

struct BackendState {
    state: AppState,
    sessions: Vec<SessionSummary>,
    next_operation: u64,
    location_count: u64,
}

impl ContractBackend {
    fn new(catalog: Arc<CatalogStore>, sessions: Arc<SessionStore>) -> Result<Self, String> {
        let mut state = initial_state();
        load_durable_state(&catalog, &mut state)
            .map_err(|error| format!("durable-state startup failed: {error:?}"))?;
        populate_profile_completeness(&catalog, &mut state)
            .map_err(|error| format!("board-profile startup failed: {error:?}"))?;
        let durable_sessions = catalog
            .session_records()
            .map_err(|error| format!("session-catalog startup failed: {error:?}"))?
            .iter()
            .map(session_summary_from_record)
            .collect::<Vec<_>>();
        Ok(Self {
            inner: Mutex::new(BackendState {
                state,
                sessions: durable_sessions,
                next_operation: 1,
                location_count: 0,
            }),
            update: Mutex::new(None),
            catalog,
            sessions,
        })
    }

    fn changed(&self, inner: &mut BackendState, catalog: bool) {
        inner.state.revision = inner.state.revision.saturating_add(1);
        if catalog {
            inner.state.session_catalog_revision =
                inner.state.session_catalog_revision.saturating_add(1);
        }
        if let Ok(mut update) = self.update.lock() {
            *update = serde_json::to_value(&inner.state).ok();
        }
    }

    fn take_state_update(&self) -> Option<Value> {
        self.update.lock().ok()?.take()
    }

    fn handle(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
        match request.method.as_str() {
            "state.get" => request_params::<EmptyParams>(request, "Method takes no parameters")
                .and_then(|_| {
                    serde_json::to_value(&inner.state)
                        .map_err(|_| ContractError::new("internal", "State encoding failed"))
                }),
            "refloat.configuration.open" => {
                request_params::<EmptyParams>(request, "Method takes no parameters")?;
                inner
                    .state
                    .active_connection
                    .as_ref()
                    .and_then(|connection| connection.refloat_configuration.as_ref())
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|_| ContractError::new("internal", "State encoding failed"))?
                    .ok_or_else(|| {
                        ContractError::new("invalid_state", "Configuration is unavailable")
                    })
            }
            "refloat.configuration.schema"
            | "refloat.configuration.read"
            | "refloat.configuration.write"
            | "refloat.tune.apply"
            | "refloat.tune.create" => Err(ContractError::new(
                "invalid_state",
                "Refloat configuration is unavailable",
            )),
            "refloat.tune.list" => Ok(json!({ "items": [], "nextCursor": null })),
            "refloat.tune.get" | "refloat.tune.update" | "refloat.tune.delete" => {
                Err(ContractError::new("not_found", "Tune was not found"))
            }
            "settings.update" => self.update_settings(&mut inner, request),
            "savedDevice.update" => self.update_saved_device(&mut inner, request),
            "savedDevice.delete" => self.delete_saved_device(&mut inner, request),
            "boardProfile.get" => {
                let params: SavedDeviceIdParams =
                    request_params(request, "Board Profile parameters are invalid")?;
                let saved_device_id = require_saved(&inner, &params.saved_device_id)?;
                let profile = load_profile(&self.catalog, saved_device_id)
                    .map_err(|error| storage_error(&error))?;
                serde_json::to_value(profile)
                    .map_err(|_| ContractError::new("internal", "Board Profile encoding failed"))
            }
            "boardProfile.update" => self.update_profile(&mut inner, request),
            "session.list" => Self::session_list(&inner, request),
            "session.get" => self.session_get(request),
            "session.delete" => {
                let params: SessionIdParams =
                    request_params(request, "Session parameters are invalid")?;
                let id = bounded_identifier(&params.session_id, "Session ID")?;
                self.catalog
                    .delete_session(id)
                    .map_err(|error| storage_error(&error))?;
                inner.sessions.retain(|session| session.session_id != id);
                self.changed(&mut inner, true);
                Ok(accepted())
            }
            "session.delete.cancel" => {
                request_params::<OperationIdParams>(request, "Operation parameters are invalid")?;
                Err(ContractError::new(
                    "not_found",
                    "Deletion operation was not found",
                ))
            }
            "session.prune.preview" => self.prune_preview(&mut inner, request),
            "session.prune.start" => self.prune(&mut inner, request),
            "session.prune.cancel" => Self::cancel_prune(&inner, request),
            _ => Err(invalid("Unknown method")),
        }
    }

    fn update_settings(
        &self,
        inner: &mut BackendState,
        request: &ContractRequest,
    ) -> Result<Value, ContractError> {
        let params: SettingsUpdateParams =
            request_params(request, "Settings parameters are invalid")?;
        let current = inner.state.settings.clone();
        if current.revision != params.expected_revision {
            return Err(ContractError::new("conflict", "Settings revision changed"));
        }
        let location_recording_enabled = params.patch.location_recording_enabled();
        let mut next_settings = current
            .apply_patch(params.patch)
            .ok_or_else(|| invalid("Settings patch is invalid"))?;
        let stored = self
            .catalog
            .update_settings(
                params.expected_revision,
                &next_settings
                    .stored_json()
                    .map_err(|_| ContractError::new("internal", "Settings encoding failed"))?,
            )
            .map_err(|error| storage_error(&error))?;
        next_settings.revision = stored.revision;
        inner.state.settings = next_settings.clone();
        if let Some(enabled) = location_recording_enabled {
            inner.state.location = if enabled {
                LocationState::recording_not_determined()
            } else {
                LocationState::disabled()
            };
        }
        let settings = serde_json::to_value(next_settings)
            .map_err(|_| ContractError::new("internal", "Settings encoding failed"))?;
        self.changed(inner, false);
        Ok(settings)
    }

    fn update_saved_device(
        &self,
        inner: &mut BackendState,
        request: &ContractRequest,
    ) -> Result<Value, ContractError> {
        let params: SavedDeviceUpdateParams =
            request_params(request, "Saved device parameters are invalid")?;
        let saved_device_id = require_saved(inner, &params.saved_device_id)?;
        let custom = params
            .custom_display_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if custom.is_some_and(|value| value.len() > 128) {
            return Err(invalid("Name is invalid"));
        }
        let stored = self
            .catalog
            .update_saved_device_name(saved_device_id, params.expected_revision, custom)
            .map_err(|error| storage_error(&error))?;
        let records = self
            .catalog
            .session_records()
            .map_err(|error| storage_error(&error))?;
        let result = saved_device_value(&stored, &records);
        let index = inner
            .state
            .saved_devices
            .iter()
            .position(|item| item.id == saved_device_id)
            .ok_or_else(|| ContractError::new("not_found", "Saved device was not found"))?;
        inner.state.saved_devices[index] = result.clone();
        self.changed(inner, false);
        serde_json::to_value(result)
            .map_err(|_| ContractError::new("internal", "Saved device encoding failed"))
    }

    fn update_profile(
        &self,
        inner: &mut BackendState,
        request: &ContractRequest,
    ) -> Result<Value, ContractError> {
        let params: BoardProfileUpdateParams =
            request_params(request, "Board Profile parameters are invalid")?;
        let saved_device_id = require_saved(inner, &params.saved_device_id)?;
        let mut profile =
            load_profile(&self.catalog, saved_device_id).map_err(|error| storage_error(&error))?;
        if profile.revision != params.expected_revision {
            return Err(ContractError::new(
                "conflict",
                "Board Profile revision changed",
            ));
        }
        if !profile.apply_patch(params.patch) {
            return Err(invalid("Board Profile patch is invalid"));
        }
        let values = profile
            .values_json()
            .map_err(|_| ContractError::new("internal", "Board Profile encoding failed"))?;
        let stored = self
            .catalog
            .update_board_profile(
                saved_device_id,
                params.expected_revision,
                &values,
                unix_millis()?,
            )
            .map_err(|error| storage_error(&error))?;
        profile.revision = stored.revision;
        let complete = profile.complete();
        if let Some(saved_device) = inner
            .state
            .saved_devices
            .iter_mut()
            .find(|device| device.id == saved_device_id)
        {
            saved_device.profile_complete = complete;
        }
        self.changed(inner, false);
        serde_json::to_value(profile)
            .map_err(|_| ContractError::new("internal", "Board Profile encoding failed"))
    }

    fn delete_saved_device(
        &self,
        inner: &mut BackendState,
        request: &ContractRequest,
    ) -> Result<Value, ContractError> {
        let params: SavedDeviceIdParams =
            request_params(request, "Saved device parameters are invalid")?;
        let saved_device_id = require_saved(inner, &params.saved_device_id)?;
        if inner
            .state
            .active_connection
            .as_ref()
            .and_then(|connection| connection.saved_device_id.as_deref())
            == Some(saved_device_id)
        {
            return Err(ContractError::new(
                "invalid_state",
                "An active board cannot be deleted",
            ));
        }
        self.catalog
            .delete_saved_device_history(saved_device_id)
            .map_err(|error| storage_error(&error))?;
        let history_id = inner
            .state
            .saved_devices
            .iter()
            .find(|device| device.id == saved_device_id)
            .map(|device| device.device_history_id.clone());
        inner
            .state
            .saved_devices
            .retain(|device| device.id != saved_device_id);
        if let Some(history_id) = history_id {
            inner
                .sessions
                .retain(|session| session.device_history_id.as_deref() != Some(&history_id));
        }
        self.changed(inner, true);
        Ok(accepted())
    }

    fn create_durable_session(
        &self,
        session_id: &str,
        device_history_id: &str,
        display_name: &str,
        profile: &board_profile::BoardProfile,
        started_at: i64,
        sampling: app_coordinator::SamplingSettings,
    ) -> Result<(), ContractError> {
        let job_id = format!("job-{session_id}");
        let device_snapshot_json = serde_json::to_string(&json!({
            "displayName": display_name
        }))
        .map_err(|_| ContractError::new("internal", "Session encoding failed"))?;
        let profile_snapshot_json = serde_json::to_string(profile)
            .map_err(|_| ContractError::new("internal", "Session encoding failed"))?;
        self.sessions
            .create(&NewSession {
                session_id,
                job_id: &job_id,
                device_history_id,
                device_snapshot_json: &device_snapshot_json,
                profile_snapshot_json: &profile_snapshot_json,
                profile_revision: profile.revision,
                started_at_millis: started_at,
                requested_idle_millis: sampling.idle_persist_millis,
                effective_idle_millis: sampling.idle_persist_millis,
                requested_riding_millis: sampling.riding_persist_millis,
                effective_riding_millis: sampling.riding_persist_millis,
            })
            .map_err(|error| storage_error(&error))
    }

    fn session_list(
        inner: &BackendState,
        request: &ContractRequest,
    ) -> Result<Value, ContractError> {
        let params: SessionListParams =
            request_params(request, "Session list parameters are invalid")?;
        let saved_device_id = require_saved(inner, &params.saved_device_id)?;
        let history_id = inner
            .state
            .saved_devices
            .iter()
            .find(|device| device.id == saved_device_id)
            .map(|device| device.device_history_id.as_str())
            .ok_or_else(|| ContractError::new("not_found", "Saved board was not found"))?;
        if !(1..=100).contains(&params.limit) {
            return Err(invalid("Session page limit is invalid"));
        }
        let limit = params.limit;
        let catalog_revision = inner.state.session_catalog_revision;
        let start = match params.cursor.as_deref() {
            None => 0,
            Some(value) => {
                let (revision, offset) = value
                    .split_once(':')
                    .ok_or_else(|| invalid("Cursor is invalid"))?;
                if revision.parse::<u64>().ok() != Some(catalog_revision) {
                    return Err(ContractError::new(
                        "conflict",
                        "Session catalog changed while paging",
                    ));
                }
                offset
                    .parse::<usize>()
                    .map_err(|_| invalid("Cursor is invalid"))?
            }
        };
        let items = inner
            .sessions
            .iter()
            .filter(|session| session.device_history_id.as_deref() == Some(history_id))
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let owned_count = inner
            .sessions
            .iter()
            .filter(|session| session.device_history_id.as_deref() == Some(history_id))
            .count();
        let next = (start + items.len() < owned_count)
            .then(|| format!("{catalog_revision}:{}", start + items.len()));
        Ok(json!({
            "items": items,
            "catalogRevision": inner.state.session_catalog_revision,
            "nextCursor": next
        }))
    }

    fn session_get(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: SessionIdParams = request_params(request, "Session parameters are invalid")?;
        let session_id = bounded_identifier(&params.session_id, "Session ID")?;
        let record = self
            .catalog
            .session_record(session_id)
            .map_err(|error| storage_error(&error))?;
        Ok(session_metadata_from_record(&record))
    }

    fn prune_preview(
        &self,
        inner: &mut BackendState,
        request: &ContractRequest,
    ) -> Result<Value, ContractError> {
        let params: PruneParams = request_params(request, "Prune parameters are invalid")?;
        let session_id = Some(find_session(inner, &params)?.session_id.clone());
        let target = prune_target(&params)?;
        let record = self
            .catalog
            .session_record(
                session_id
                    .as_deref()
                    .ok_or_else(|| invalid("Session ID is required"))?,
            )
            .map_err(|error| storage_error(&error))?;
        let id = format!("operation-{}", inner.next_operation);
        inner.next_operation += 1;
        let operation = operation(
            &id,
            "prunePreview",
            "completed",
            session_id.as_deref(),
            target,
        );
        Ok(json!({
            "operation": operation,
            "recordedSampleCount": record.sample_count,
            "estimatedStoredSampleCount": record.stored_sample_count,
            "estimatedReclaimableBytes": 0,
            "skipped": false,
            "skipReason": null
        }))
    }

    fn prune(
        &self,
        inner: &mut BackendState,
        request: &ContractRequest,
    ) -> Result<Value, ContractError> {
        let params: PruneParams = request_params(request, "Prune parameters are invalid")?;
        let session_id = Some(find_session(inner, &params)?.session_id.clone());
        let target = prune_target(&params)?;
        let id = format!("operation-{}", inner.next_operation);
        inner.next_operation += 1;
        let result = self
            .catalog
            .prune_session(
                session_id
                    .as_deref()
                    .ok_or_else(|| invalid("Session ID is required"))?,
                target,
                &["packVoltageV".into()],
                unix_millis()?,
            )
            .map_err(|error| storage_error(&error))?;
        if let Some(session) = inner
            .sessions
            .iter_mut()
            .find(|session| Some(session.session_id.as_str()) == session_id.as_deref())
            && let Ok(record) = self
                .catalog
                .session_record(session_id.as_deref().unwrap_or_default())
        {
            *session = session_summary_from_record(&record);
        }
        let mut operation = operation(&id, "prune", "completed", session_id.as_deref(), target);
        operation.completed_sessions = 1;
        operation.retained_samples = Some(result.retained_samples);
        operation.reclaimable_bytes = Some(0);
        inner.state.session_operation = Some(operation.clone());
        self.changed(inner, true);
        serde_json::to_value(operation)
            .map_err(|_| ContractError::new("internal", "Operation encoding failed"))
    }

    fn cancel_prune(
        inner: &BackendState,
        request: &ContractRequest,
    ) -> Result<Value, ContractError> {
        let params: OperationIdParams =
            request_params(request, "Operation parameters are invalid")?;
        let operation_id = bounded_identifier(&params.operation_id, "Operation ID")?;
        let operation = inner.state.session_operation.as_ref();
        if operation.is_some_and(|operation| {
            operation.operation_id == operation_id && operation.kind == "prune"
        }) {
            serde_json::to_value(operation)
                .map_err(|_| ContractError::new("internal", "Operation encoding failed"))
        } else {
            Err(ContractError::new(
                "not_found",
                "Prune operation was not found",
            ))
        }
    }
}

/// Contract adapter for native hosts. Product state remains owned by the shared
/// actor; this adapter only maps its bounded snapshot to the public wire shape.
struct NativeBackend {
    contract: ContractBackend,
    application: Arc<ApplicationActor>,
    actor_revision: Mutex<u64>,
    connection: Mutex<Option<NativeConnection>>,
    configuration_signal: Mutex<u64>,
    configuration_changed: Condvar,
    documents: Mutex<DocumentOperations>,
    document_start: Mutex<()>,
}

const DOCUMENT_HISTORY_LIMIT: usize = 16;
const DOCUMENT_OPERATION_EXPIRY_MILLIS: i64 = 10 * 60 * 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DocumentKind {
    Import,
    Export,
}

struct PendingDocument {
    operation_id: String,
    kind: DocumentKind,
    staging_path: PathBuf,
    display_name: Option<String>,
    generation: u64,
    created_at_millis: i64,
}

#[derive(Default)]
struct DocumentOperations {
    pending: HashMap<u64, PendingDocument>,
    history: VecDeque<DocumentOperationState>,
    unclaimed_completions: VecDeque<OperationCompletion>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IdentificationStage {
    WaitingBluetooth,
    TransportFirmware,
    CanPing,
    EnumeratingControllers,
    ControllerFirmware,
    RefloatInfo,
    RefloatLayout,
    BmsCanPing,
    EnumeratingBms,
    BmsValues,
    Telemetry,
    Selecting,
    ReadyBms,
    Reconnecting,
}

struct NativeConnection {
    session_id: String,
    ownership: ConnectionOwnership,
    started_at_millis: i64,
    device_id: String,
    association: PlatformAssociation,
    identity_nonce: [u8; 16],
    saved_firmware_uuid: Option<FirmwareDeviceUuid>,
    name: Option<String>,
    stage: IdentificationStage,
    connect_after_generation: u64,
    vesc_generation: Option<u64>,
    controller_can_id: Option<u8>,
    pending_can_ids: Vec<u8>,
    controller_inventory: Vec<FirmwareRecord>,
    transport_firmware: Option<FirmwareRecord>,
    firmware: Option<FirmwareRecord>,
    package_version: Option<String>,
    telemetry_layout: Option<protocol_refloat::TelemetryLayout>,
    latest_sample: Option<Value>,
    telemetry_sequence: u64,
    last_telemetry_elapsed_millis: Option<u64>,
    riding_elapsed_millis: u64,
    latest_telemetry: Option<protocol_refloat::Telemetry>,
    bms_status: DiscoveryStatus,
    bms_connection: Option<BmsConnection>,
    bms_candidates: Vec<BmsCandidate>,
    bms_snapshot: Option<BmsSnapshot>,
    bms_pending_can_ids: Vec<u8>,
    bms_probe_can_id: Option<u8>,
    controls: Option<protocol_refloat::MutationCoordinator>,
    telemetry_in_flight: bool,
    telemetry_polling: Option<TelemetryPollingModel>,
    next_telemetry_due_millis: Option<u64>,
    pause_telemetry: bool,
    shutdown_requested: bool,
    reconnect: ReconnectModel,
    reconnect_due_millis: Option<u64>,
    handled_failure_generation: Option<u64>,
    configuration_operation: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ConnectionOwnership {
    Guest,
    PendingSaved,
    Saved {
        saved_device_id: String,
        device_history_id: String,
    },
}

impl ConnectionOwnership {
    const fn tracking_mode(&self) -> TrackingMode {
        match self {
            Self::Guest => TrackingMode::Guest,
            Self::PendingSaved | Self::Saved { .. } => TrackingMode::Saved,
        }
    }

    fn saved(&self) -> Option<(&str, &str)> {
        match self {
            Self::Guest | Self::PendingSaved => None,
            Self::Saved {
                saved_device_id,
                device_history_id,
            } => Some((saved_device_id, device_history_id)),
        }
    }
}

impl NativeBackend {
    fn new(
        application: Arc<ApplicationActor>,
        catalog: Arc<CatalogStore>,
        sessions: Arc<SessionStore>,
    ) -> Result<Self, String> {
        let contract = ContractBackend::new(catalog, sessions)?;
        let sampling = {
            let inner = contract
                .inner
                .lock()
                .map_err(|_| "backend state failed".to_owned())?;
            inner.state.settings.sampling()
        };
        application
            .configure_sampling(sampling)
            .map_err(|_| "sampling settings startup failed".to_owned())?;
        Ok(Self {
            contract,
            application,
            actor_revision: Mutex::new(u64::MAX),
            connection: Mutex::new(None),
            configuration_signal: Mutex::new(0),
            configuration_changed: Condvar::new(),
            documents: Mutex::new(DocumentOperations::default()),
            document_start: Mutex::new(()),
        })
    }

    fn refresh(&self) -> Result<(), ContractError> {
        self.expire_document_operation()?;
        let connection_changed = self.process_publications()?;
        let snapshot = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?;
        let telemetry_due = self.schedule_due_telemetry(snapshot.monotonic_millis)?;
        self.reconnect_if_needed(&snapshot)?;
        let mut observed = self
            .actor_revision
            .lock()
            .map_err(|_| ContractError::new("internal", "Application state failed"))?;
        if *observed == snapshot.revision && !connection_changed && !telemetry_due {
            return Ok(());
        }
        {
            let mut connection = self
                .connection
                .lock()
                .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
            if connection.as_ref().is_some_and(|connection| {
                connection.shutdown_requested
                    && snapshot.bluetooth_connection == Some(BluetoothConnectionState::Disconnected)
            }) {
                *connection = None;
            }
        }
        self.begin_identification_if_ready(&snapshot)?;
        let mut active_connection = self.native_connection_value(snapshot.wall_millis)?;
        if let (Some(active), Some((latitude, longitude, accuracy))) =
            (active_connection.as_mut(), snapshot.latest_location)
        {
            active.latest_location = Some(LiveLocation {
                observed_at: iso8601_or_empty(snapshot.wall_millis),
                latitude: f64::from_bits(latitude),
                longitude: f64::from_bits(longitude),
                accuracy_m: accuracy.map(f64::from_bits),
            });
        }
        let native_summary = self.native_session_summary()?;
        let mut inner = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
        update_platform_state(&self.contract.catalog, &mut inner, &snapshot)
            .map_err(|error| storage_error(&error))?;
        if let Some(summary) = native_summary {
            self.merge_native_session(&mut inner, summary)?;
        }
        inner.state.active_connection = active_connection;
        self.contract.changed(&mut inner, false);
        *observed = snapshot.revision;
        Ok(())
    }

    fn merge_native_session(
        &self,
        inner: &mut BackendState,
        summary: SessionSummary,
    ) -> Result<(), ContractError> {
        if let Some(existing) = inner
            .sessions
            .iter_mut()
            .find(|session| session.session_id == summary.session_id)
        {
            *existing = summary;
            return Ok(());
        }
        let session_id = summary.session_id.as_str();
        if matches!(
            self.contract.catalog.session_record(session_id),
            Err(StoreError::NotFound)
        ) {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
            let connection = connection
                .as_ref()
                .ok_or_else(|| ContractError::new("internal", "Connection state is unavailable"))?;
            let (saved_device_id, device_history_id) = connection
                .ownership
                .saved()
                .ok_or_else(|| ContractError::new("internal", "Saved board is unavailable"))?;
            let profile = load_profile(&self.contract.catalog, saved_device_id)
                .map_err(|error| storage_error(&error))?;
            self.contract.create_durable_session(
                session_id,
                device_history_id,
                connection.name.as_deref().unwrap_or("VESC"),
                &profile,
                connection.started_at_millis,
                inner.state.settings.sampling(),
            )?;
        }
        inner.sessions.insert(0, summary);
        Ok(())
    }

    fn request_host_operation(
        &self,
        command: HostCommand,
    ) -> Result<domain_types::OperationId, ContractError> {
        self.application
            .request_host(command)
            .map_err(|_| ContractError::new("unavailable", "Platform request is unavailable"))
    }

    fn request_host(&self, command: HostCommand) -> Result<(), ContractError> {
        self.request_host_operation(command).map(|_| ())
    }

    fn schedule_due_telemetry(&self, now_millis: u64) -> Result<bool, ContractError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let Some(connection) = connection.as_mut() else {
            return Ok(false);
        };
        if connection.pause_telemetry
            || connection.telemetry_in_flight
            || connection.stage != IdentificationStage::Telemetry
            || connection
                .next_telemetry_due_millis
                .is_none_or(|due| due > now_millis)
        {
            return Ok(false);
        }
        connection.next_telemetry_due_millis = None;
        self.schedule_telemetry(connection)?;
        Ok(true)
    }

    fn current_sampling_settings(
        &self,
    ) -> Result<app_coordinator::SamplingSettings, ContractError> {
        let inner = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
        Ok(inner.state.settings.sampling())
    }

    fn configure_connection_sampling(
        &self,
        settings: app_coordinator::SamplingSettings,
        now_millis: u64,
    ) -> Result<(), ContractError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let Some(connection) = connection.as_mut() else {
            return Ok(());
        };
        let family = if connection.package_version.as_deref() == Some("1.0") {
            TelemetryFamily::Legacy
        } else {
            TelemetryFamily::Dynamic
        };
        connection.telemetry_polling = Some(TelemetryPollingModel::new(
            family,
            settings.idle_probe_millis,
            settings.post_ride_tail_millis,
        ));
        connection.reconnect = ReconnectModel::new(settings.reconnect_duration_millis);
        connection.reconnect_due_millis = None;
        connection.next_telemetry_due_millis = Some(now_millis);
        Ok(())
    }

    fn process_publications(&self) -> Result<bool, ContractError> {
        self.application
            .tick_vesc_now()
            .map_err(|_| ContractError::new("internal", "Application timer failed"))?;
        let mut changed = false;
        while let Some(publication) = self
            .application
            .try_publication()
            .map_err(|_| ContractError::new("internal", "Application publication failed"))?
        {
            match publication {
                ServerPublication::VescCompleted(completion) => {
                    self.accept_vesc_completion(completion)?;
                    changed = true;
                }
                ServerPublication::HostCompleted(completion) => {
                    if self.accept_document_completion(completion)? {
                        changed = true;
                    }
                }
                ServerPublication::StateChanged(_) | ServerPublication::StorageCompleted(_) => {}
            }
        }
        Ok(changed)
    }

    fn begin_document_import(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        request_params::<EmptyParams>(request, "Method takes no parameters")?;
        let _start = self
            .document_start
            .lock()
            .map_err(|_| ContractError::new("internal", "Document state failed"))?;
        self.ensure_no_pending_document()?;
        let token = format!("document-{}", unix_millis()?.max(0));
        let staging_path = self
            .contract
            .catalog
            .staging_path(&token)
            .map_err(|error| storage_error(&error))?;
        let path = document_path(&staging_path)?;
        let operation = self.request_host_operation(HostCommand::PresentImport {
            staging_path: path,
            allowed_content_types: vec!["application/octet-stream".into()],
            maximum_bytes: MAX_IMPORT_BYTES,
        })?;
        let generation = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?
            .generation
            .0;
        let created_at_millis = unix_millis()?;
        let operation_id = format!("document-{}", operation.0);
        let state = document_operation_value(
            &operation_id,
            DocumentKind::Import,
            "presenting",
            None,
            None,
            None,
        );
        self.documents
            .lock()
            .map_err(|_| ContractError::new("internal", "Document state failed"))?
            .pending
            .insert(
                operation.0,
                PendingDocument {
                    operation_id,
                    kind: DocumentKind::Import,
                    staging_path,
                    display_name: None,
                    generation,
                    created_at_millis,
                },
            );
        let early = self.take_unclaimed_document_completion(operation.0)?;
        self.publish_document_state(state.clone(), false)?;
        if let Some(completion) = early {
            self.accept_document_completion(completion)?;
            return self.current_document_state();
        }
        serde_json::to_value(state)
            .map_err(|_| ContractError::new("internal", "Document state encoding failed"))
    }

    fn begin_document_export(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let _start = self
            .document_start
            .lock()
            .map_err(|_| ContractError::new("internal", "Document state failed"))?;
        self.ensure_no_pending_document()?;
        let params: SessionExportParams = request_params(request, "Export parameters are invalid")?;
        let (session_id, format) = {
            let inner = self
                .contract
                .inner
                .lock()
                .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
            let session_id = bounded_identifier(&params.session_id, "Session ID")?;
            let session = find_session_by_id(&inner, session_id)?;
            let session_id = session.session_id.clone();
            let format = match params.format {
                ExportFormatParam::Csv => infra_sqlite_storage::ExportFormat::Csv,
                ExportFormatParam::FloatViewArchive => {
                    infra_sqlite_storage::ExportFormat::FloatViewArchive
                }
            };
            (session_id, format)
        };
        let export_id = format!("document-{}", unix_millis()?.max(0));
        let include_location = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?
            .state
            .settings
            .include_gps_location_when_exporting();
        let exported = self
            .contract
            .catalog
            .export_session_with_policy(&session_id, &export_id, format, include_location)
            .map_err(|error| storage_error(&error))?;
        let content_type = if matches!(format, infra_sqlite_storage::ExportFormat::Csv) {
            "text/csv"
        } else {
            "application/octet-stream"
        };
        let operation = match self.request_host_operation(HostCommand::PresentExport {
            staging_path: document_path(&exported.path)?,
            display_name: exported.file_name.clone(),
            content_type: content_type.into(),
        }) {
            Ok(operation) => operation,
            Err(error) => {
                let _ = std::fs::remove_file(&exported.path);
                return Err(error);
            }
        };
        let generation = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?
            .generation
            .0;
        let created_at_millis = unix_millis()?;
        let operation_id = format!("document-{}", operation.0);
        let state = document_operation_value(
            &operation_id,
            DocumentKind::Export,
            "presenting",
            None,
            Some(&exported.file_name),
            None,
        );
        self.documents
            .lock()
            .map_err(|_| ContractError::new("internal", "Document state failed"))?
            .pending
            .insert(
                operation.0,
                PendingDocument {
                    operation_id,
                    kind: DocumentKind::Export,
                    staging_path: exported.path,
                    display_name: Some(exported.file_name),
                    generation,
                    created_at_millis,
                },
            );
        let early = self.take_unclaimed_document_completion(operation.0)?;
        self.publish_document_state(state.clone(), false)?;
        if let Some(completion) = early {
            self.accept_document_completion(completion)?;
            return self.current_document_state();
        }
        serde_json::to_value(state)
            .map_err(|_| ContractError::new("internal", "Document state encoding failed"))
    }

    fn ensure_no_pending_document(&self) -> Result<(), ContractError> {
        if self
            .documents
            .lock()
            .map_err(|_| ContractError::new("internal", "Document state failed"))?
            .pending
            .is_empty()
        {
            Ok(())
        } else {
            Err(ContractError::new(
                "conflict",
                "Another document operation is presenting",
            ))
        }
    }

    fn expire_document_operation(&self) -> Result<(), ContractError> {
        let now = unix_millis()?;
        let expired = {
            let mut documents = self
                .documents
                .lock()
                .map_err(|_| ContractError::new("internal", "Document state failed"))?;
            let operation = documents.pending.iter().find_map(|(operation, pending)| {
                (now.saturating_sub(pending.created_at_millis) >= DOCUMENT_OPERATION_EXPIRY_MILLIS)
                    .then_some(*operation)
            });
            operation.and_then(|operation| {
                documents
                    .pending
                    .remove(&operation)
                    .map(|pending| (operation, pending))
            })
        };
        let Some((operation, pending)) = expired else {
            return Ok(());
        };
        let _ = std::fs::remove_file(&pending.staging_path);
        let _ = self.application.complete(OperationCompletion {
            generation: domain_types::RuntimeGeneration(pending.generation),
            operation: domain_types::OperationId(operation),
            status: CompletionStatus::Cancelled,
            payload: Vec::new(),
        });
        let state = document_operation_value(
            &pending.operation_id,
            pending.kind,
            "failed",
            Some(document_error("unavailable", "Document operation expired")),
            pending.display_name.as_deref(),
            None,
        );
        self.record_document_history(state.clone())?;
        self.publish_document_state(state, false)
    }

    fn accept_document_completion(
        &self,
        completion: OperationCompletion,
    ) -> Result<bool, ContractError> {
        let pending = self
            .documents
            .lock()
            .map_err(|_| ContractError::new("internal", "Document state failed"))?
            .pending
            .remove(&completion.operation.0);
        let Some(pending) = pending else {
            let mut documents = self
                .documents
                .lock()
                .map_err(|_| ContractError::new("internal", "Document state failed"))?;
            if documents.unclaimed_completions.len() == DOCUMENT_HISTORY_LIMIT {
                documents.unclaimed_completions.pop_front();
            }
            documents.unclaimed_completions.push_back(completion);
            return Ok(false);
        };
        let mut session_id = None;
        let (status, error) = match completion.status {
            CompletionStatus::Success if pending.kind == DocumentKind::Import => {
                let imported_id = format!("imported-{}", completion.operation.0);
                let result = self.contract.catalog.import_staged_archive(
                    &pending.staging_path,
                    &imported_id,
                    &pending.operation_id,
                    unix_millis()?,
                );
                match result {
                    Ok(value) => {
                        session_id = Some(value);
                        ("completed", None)
                    }
                    Err(error) => ("failed", Some(storage_error_value(&error))),
                }
            }
            CompletionStatus::Success => ("completed", None),
            CompletionStatus::Cancelled => ("cancelled", None),
            CompletionStatus::PermissionDenied => (
                "failed",
                Some(document_error(
                    "permission_required",
                    "Document access was denied",
                )),
            ),
            CompletionStatus::Unavailable => (
                "failed",
                Some(document_error(
                    "unavailable",
                    "Document presentation is unavailable",
                )),
            ),
            CompletionStatus::Failed => (
                "failed",
                Some(document_error("internal", "Document operation failed")),
            ),
        };
        let _ = std::fs::remove_file(&pending.staging_path);
        if let Some(session_id) = session_id.as_deref() {
            let record = self
                .contract
                .catalog
                .session_record(session_id)
                .map_err(|error| storage_error(&error))?;
            let mut inner = self
                .contract
                .inner
                .lock()
                .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
            inner
                .sessions
                .insert(0, session_summary_from_record(&record));
        }
        let state = document_operation_value(
            &pending.operation_id,
            pending.kind,
            status,
            error,
            pending.display_name.as_deref(),
            session_id.as_deref(),
        );
        self.record_document_history(state.clone())?;
        self.publish_document_state(state, session_id.is_some())?;
        Ok(true)
    }

    fn record_document_history(&self, state: DocumentOperationState) -> Result<(), ContractError> {
        let mut documents = self
            .documents
            .lock()
            .map_err(|_| ContractError::new("internal", "Document state failed"))?;
        if documents.history.len() == DOCUMENT_HISTORY_LIMIT {
            documents.history.pop_front();
        }
        documents.history.push_back(state);
        Ok(())
    }

    fn take_unclaimed_document_completion(
        &self,
        operation: u64,
    ) -> Result<Option<OperationCompletion>, ContractError> {
        let mut documents = self
            .documents
            .lock()
            .map_err(|_| ContractError::new("internal", "Document state failed"))?;
        let position = documents
            .unclaimed_completions
            .iter()
            .position(|completion| completion.operation.0 == operation);
        Ok(position.and_then(|position| documents.unclaimed_completions.remove(position)))
    }

    fn current_document_state(&self) -> Result<Value, ContractError> {
        self.contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))
            .and_then(|inner| {
                serde_json::to_value(&inner.state.document_operation)
                    .map_err(|_| ContractError::new("internal", "Document state encoding failed"))
            })
    }

    fn publish_document_state(
        &self,
        state: DocumentOperationState,
        catalog: bool,
    ) -> Result<(), ContractError> {
        let mut inner = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
        inner.state.document_operation = Some(state);
        self.contract.changed(&mut inner, catalog);
        Ok(())
    }

    fn begin_identification_if_ready(
        &self,
        snapshot: &app_coordinator::ApplicationSnapshot,
    ) -> Result<(), ContractError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let Some(connection) = connection.as_mut() else {
            return Ok(());
        };
        if !ready_generation_is_fresh(
            snapshot.bluetooth_ready,
            snapshot.connection_generation,
            connection.connect_after_generation,
        ) || !matches!(
            connection.stage,
            IdentificationStage::WaitingBluetooth | IdentificationStage::Reconnecting
        ) {
            return Ok(());
        }
        reset_identification(connection);
        conformance_trace(&format!(
            "begin_identification connection_generation={}",
            snapshot.connection_generation
        ));
        let generation = self
            .application
            .begin_vesc_generation()
            .map_err(|_| ContractError::new("unavailable", "VESC generation could not start"))?;
        connection.vesc_generation = Some(generation);
        self.schedule(
            connection,
            IdentificationStage::TransportFirmware,
            &[VescCommand::GetFirmwareVersion as u8],
            ExpectedResponse::new(VescCommand::GetFirmwareVersion, None),
            RequestKind::ControllerFirmware,
        )
    }

    fn reconnect_if_needed(
        &self,
        snapshot: &app_coordinator::ApplicationSnapshot,
    ) -> Result<(), ContractError> {
        if !matches!(
            snapshot.bluetooth_connection,
            Some(BluetoothConnectionState::Disconnected | BluetoothConnectionState::Failed)
        ) {
            return Ok(());
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let Some(connection) = connection.as_mut() else {
            return Ok(());
        };
        if connection.shutdown_requested {
            return Ok(());
        }
        if connection.stage != IdentificationStage::Reconnecting {
            connection.stage = IdentificationStage::Reconnecting;
            connection.connect_after_generation = snapshot.connection_generation;
            connection.telemetry_in_flight = false;
            connection.reconnect_due_millis = reconnect_due(
                connection.reconnect.begin(snapshot.monotonic_millis),
                snapshot.monotonic_millis,
            );
        } else if connection.reconnect_due_millis.is_none()
            && connection.handled_failure_generation != Some(snapshot.connection_generation)
        {
            connection.reconnect_due_millis = reconnect_due(
                connection.reconnect.schedule(snapshot.monotonic_millis),
                snapshot.monotonic_millis,
            );
        }
        if connection
            .reconnect_due_millis
            .is_some_and(|due| due <= snapshot.monotonic_millis)
        {
            connection.reconnect_due_millis = None;
            match connection.reconnect.timer_fired(
                snapshot.monotonic_millis,
                snapshot.bluetooth_radio == BluetoothRadio::PoweredOn,
            ) {
                ReconnectDecision::ConnectNow => {
                    connection.handled_failure_generation = Some(snapshot.connection_generation);
                    connection.connect_after_generation = snapshot.connection_generation;
                    self.request_host(HostCommand::BluetoothConnect {
                        connection_generation: snapshot.connection_generation.saturating_add(1),
                        association: connection.association.clone(),
                    })?;
                }
                decision => {
                    connection.reconnect_due_millis =
                        reconnect_due(decision, snapshot.monotonic_millis);
                }
            }
        }
        Ok(())
    }

    fn accept_transport_firmware(
        &self,
        connection: &mut NativeConnection,
        payload: &[u8],
    ) -> Result<(), ContractError> {
        let firmware = decode_firmware(payload)
            .map_err(|_| ContractError::new("invalid_state", "Firmware identity is invalid"))?;
        connection.transport_firmware = Some(firmware.clone());
        if protocol_vesc_bms::supports_firmware(&firmware) {
            self.establish_saved_identity(connection, &firmware)?;
            self.schedule_bms_values(connection, BmsConnection::Direct)
        } else if firmware.hardware_type == 0 {
            self.establish_saved_identity(connection, &firmware)?;
            connection.firmware = Some(firmware);
            self.schedule_refloat_info(connection, ControllerRoute::Direct)
        } else if let Some(can_id) = self.preferred_can_id(connection)? {
            connection.controller_can_id = Some(can_id);
            let route =
                ControllerRoute::Can(domain_types::CanId::new(can_id).map_err(|_| {
                    ContractError::new("invalid_state", "Preferred CAN ID is invalid")
                })?);
            self.schedule(
                connection,
                IdentificationStage::ControllerFirmware,
                &route_command(&[VescCommand::GetFirmwareVersion as u8], route),
                ExpectedResponse::new(VescCommand::GetFirmwareVersion, None),
                RequestKind::ControllerFirmware,
            )
        } else {
            self.schedule(
                connection,
                IdentificationStage::CanPing,
                &[VescCommand::PingCan as u8],
                ExpectedResponse::new(VescCommand::PingCan, None),
                RequestKind::CanPing,
            )
        }
    }

    fn establish_saved_identity(
        &self,
        connection: &mut NativeConnection,
        firmware: &FirmwareRecord,
    ) -> Result<(), ContractError> {
        let firmware_uuid = firmware
            .controller_uuid
            .and_then(|value| FirmwareDeviceUuid::new(value).ok());
        match &connection.ownership {
            ConnectionOwnership::Guest => return Ok(()),
            ConnectionOwnership::Saved { .. } => {
                if connection.saved_firmware_uuid.is_some()
                    && connection.saved_firmware_uuid != firmware_uuid
                {
                    return Err(ContractError::new(
                        "conflict",
                        "VESC firmware identity changed during reconnect",
                    ));
                }
                return Ok(());
            }
            ConnectionOwnership::PendingSaved => {}
        }

        let (saved_device_id, device_history_id) = firmware_uuid.map_or_else(
            || fallback_board_ids(connection.identity_nonce, &connection.association),
            firmware_board_ids,
        );
        let platform_id = encode_platform_association(&connection.association);
        if let Ok(existing) = self.contract.catalog.saved_device(&saved_device_id)
            && existing.opaque_platform_id.as_deref() != Some(platform_id.as_str())
            && existing.opaque_platform_id.as_deref().is_some_and(|old| {
                self.application.snapshot().is_ok_and(|snapshot| {
                    snapshot
                        .discovered_devices
                        .iter()
                        .any(|device| encode_platform_association(&device.association) == old)
                })
            })
        {
            return Err(ContractError::new(
                "conflict",
                "VESC firmware identity is already associated with another visible transport",
            ));
        }
        self.contract
            .catalog
            .ensure_saved_device(
                &saved_device_id,
                &device_history_id,
                firmware_uuid.map(FirmwareDeviceUuid::bytes),
                Some(&platform_id),
                connection.name.as_deref(),
                connection.started_at_millis,
            )
            .map_err(|error| storage_error(&error))?;
        connection.ownership = ConnectionOwnership::Saved {
            saved_device_id,
            device_history_id,
        };
        connection.saved_firmware_uuid = firmware_uuid;

        let mut inner = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
        load_durable_state(&self.contract.catalog, &mut inner.state)
            .and_then(|()| populate_profile_completeness(&self.contract.catalog, &mut inner.state))
            .map_err(|error| storage_error(&error))?;
        self.contract.changed(&mut inner, false);
        Ok(())
    }

    fn accept_can_ping(
        &self,
        connection: &mut NativeConnection,
        payload: &[u8],
    ) -> Result<(), ContractError> {
        connection.pending_can_ids = payload
            .get(1..)
            .unwrap_or_default()
            .iter()
            .copied()
            .filter(|id| *id <= 253)
            .collect();
        connection.pending_can_ids.sort_unstable();
        connection.pending_can_ids.dedup();
        connection.controller_inventory.clear();
        self.schedule_next_inventory_probe(connection)
    }

    fn accept_controller_firmware(
        &self,
        connection: &mut NativeConnection,
        payload: &[u8],
        inventory: bool,
    ) -> Result<(), ContractError> {
        let mut firmware = decode_firmware(payload)
            .map_err(|_| ContractError::new("invalid_state", "Firmware identity is invalid"))?;
        let can_id = connection
            .controller_can_id
            .and_then(|value| domain_types::CanId::new(value).ok())
            .ok_or_else(|| ContractError::new("invalid_state", "CAN route is unavailable"))?;
        firmware.can_id = Some(can_id);
        if inventory {
            connection.controller_inventory.push(firmware);
            self.schedule_next_inventory_probe(connection)
        } else {
            self.establish_saved_identity(connection, &firmware)?;
            connection.firmware = Some(firmware);
            self.schedule_refloat_info(connection, ControllerRoute::Can(can_id))
        }
    }

    fn accept_refloat_info(
        &self,
        connection: &mut NativeConnection,
        payload: &[u8],
    ) -> Result<(), ContractError> {
        let version = decode_refloat_version(payload)?;
        let legacy = version == "1.0";
        connection.package_version = Some(version);
        let sampling = self.current_sampling_settings()?;
        connection.telemetry_polling = Some(TelemetryPollingModel::new(
            if legacy {
                TelemetryFamily::Legacy
            } else {
                TelemetryFamily::Dynamic
            },
            sampling.idle_probe_millis,
            sampling.post_ride_tail_millis,
        ));
        connection.next_telemetry_due_millis = None;
        connection.controls = Some(protocol_refloat::MutationCoordinator::new(
            connection.vesc_generation.unwrap_or_default(),
            connection
                .firmware
                .as_ref()
                .is_some_and(|firmware| firmware.test_version == 0),
        ));
        self.begin_configuration(connection)?;
        if legacy {
            self.begin_bms_discovery(connection)
        } else {
            let route = connection_route(connection)?;
            self.schedule(
                connection,
                IdentificationStage::RefloatLayout,
                &route_command(
                    &protocol_refloat::request(
                        protocol_refloat::RefloatCommand::RealtimeDataInternalIds,
                        &[],
                    ),
                    route,
                ),
                ExpectedResponse::new(
                    VescCommand::CustomAppData,
                    Some(protocol_refloat::RefloatCommand::RealtimeDataInternalIds as u8),
                ),
                RequestKind::RefloatLayout,
            )
        }
    }

    fn accept_refloat_layout(
        &self,
        connection: &mut NativeConnection,
        payload: &[u8],
    ) -> Result<(), ContractError> {
        connection.telemetry_layout = Some(
            protocol_refloat::decode_layout(payload)
                .map_err(|_| ContractError::new("invalid_state", "Refloat layout is invalid"))?,
        );
        let now_millis = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?
            .monotonic_millis;
        if let Some(polling) = connection.telemetry_polling.as_mut() {
            polling.layout_established(now_millis);
        }
        self.begin_bms_discovery(connection)
    }

    fn accept_bms_ping(
        &self,
        connection: &mut NativeConnection,
        payload: &[u8],
    ) -> Result<(), ContractError> {
        conformance_trace(&format!("bms_ping_response bytes={}", payload.len()));
        connection.bms_pending_can_ids = payload
            .get(1..)
            .unwrap_or_default()
            .iter()
            .copied()
            .filter(|id| *id <= 253)
            .collect();
        connection.bms_pending_can_ids.sort_unstable();
        connection.bms_pending_can_ids.dedup();
        self.schedule_next_bms_probe(connection)
    }

    fn accept_bms_identity(
        &self,
        connection: &mut NativeConnection,
        payload: &[u8],
    ) -> Result<(), ContractError> {
        let mut firmware = decode_firmware(payload)
            .map_err(|_| ContractError::new("invalid_state", "BMS identity is invalid"))?;
        conformance_trace(&format!(
            "bms_identity_response supported={}",
            protocol_vesc_bms::supports_firmware(&firmware)
        ));
        let can_id = connection
            .bms_probe_can_id
            .and_then(|value| domain_types::CanId::new(value).ok())
            .ok_or_else(|| ContractError::new("invalid_state", "BMS route is unavailable"))?;
        firmware.can_id = Some(can_id);
        if protocol_vesc_bms::supports_firmware(&firmware) {
            connection.controller_inventory.push(firmware);
        }
        self.schedule_next_bms_probe(connection)
    }

    fn accept_bms_values(
        &self,
        connection: &mut NativeConnection,
        payload: &[u8],
    ) -> Result<(), ContractError> {
        let selected = connection
            .bms_connection
            .ok_or_else(|| ContractError::new("invalid_state", "BMS connection is unavailable"))?;
        let expected_can_id = match selected {
            BmsConnection::Direct => None,
            BmsConnection::Can(can_id) => Some(can_id),
        };
        let snapshot = protocol_vesc_bms::decode_values(payload, expected_can_id)
            .map_err(|_| ContractError::new("invalid_state", "BMS values are invalid"))?;
        let selected_current = self
            .application
            .select_bms(selected)
            .map_err(|_| ContractError::new("invalid_state", "BMS selection is stale"))?;
        if !selected_current {
            return Err(ContractError::new(
                "invalid_state",
                "BMS selection is stale",
            ));
        }
        let values_current = self
            .application
            .accept_bms(snapshot.clone())
            .map_err(|_| ContractError::new("invalid_state", "BMS values are stale"))?;
        if !values_current {
            return Err(ContractError::new("invalid_state", "BMS values are stale"));
        }
        connection.bms_snapshot = Some(snapshot);
        connection.bms_status = DiscoveryStatus::Attached;
        if connection.firmware.is_some() {
            self.schedule_telemetry(connection)
        } else {
            connection.stage = IdentificationStage::ReadyBms;
            Ok(())
        }
    }

    fn accept_control_response(
        connection: &mut NativeConnection,
        payload: &[u8],
    ) -> Result<(), ContractError> {
        let version = connection
            .package_version
            .as_deref()
            .ok_or_else(|| ContractError::new("invalid_state", "Refloat version is unavailable"))?;
        let profile = protocol_refloat::compatibility(version)
            .map(protocol_refloat::CompatibilityEntry::control_profile)
            .ok_or_else(|| {
                ContractError::new("invalid_state", "Refloat controls are unsupported")
            })?;
        let lighting = protocol_refloat::decode_lighting(payload, profile).map_err(|_| {
            ContractError::new("invalid_state", "Refloat control response is invalid")
        })?;
        observe_controls(connection, Some(lighting));
        Ok(())
    }

    fn accept_telemetry_response(
        &self,
        connection: &mut NativeConnection,
        payload: &[u8],
    ) -> Result<(), ContractError> {
        connection.telemetry_in_flight = false;
        let telemetry = if let Some(layout) = connection.telemetry_layout.as_ref() {
            protocol_refloat::decode_dynamic(payload, layout)
        } else {
            protocol_refloat::decode_legacy(payload)
        }
        .map_err(|_| ContractError::new("invalid_state", "Telemetry is invalid"))?;
        connection.telemetry_sequence = connection.telemetry_sequence.saturating_add(1);
        let actor_snapshot = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?;
        let elapsed_millis =
            wall_elapsed_millis(connection.started_at_millis, actor_snapshot.wall_millis);
        let elapsed_delta = connection
            .last_telemetry_elapsed_millis
            .map_or(0, |previous| elapsed_millis.saturating_sub(previous));
        connection.last_telemetry_elapsed_millis = Some(elapsed_millis);
        if telemetry.ride_state == domain_types::RideState::Riding {
            connection.riding_elapsed_millis = connection
                .riding_elapsed_millis
                .saturating_add(elapsed_delta);
        }
        let persist = self
            .application
            .observe_telemetry(telemetry.clone(), elapsed_millis)
            .map_err(|_| ContractError::new("internal", "Telemetry could not be observed"))?;
        let sample = session_sample(
            &telemetry,
            connection.telemetry_sequence,
            actor_snapshot.wall_millis,
            elapsed_millis,
        )?;
        if persist && connection.ownership.saved().is_some() {
            self.contract
                .sessions
                .append_samples(
                    &connection.session_id,
                    &[sample],
                    actor_snapshot.wall_millis,
                )
                .map_err(|error| storage_error(&error))?;
        }
        connection.latest_sample = Some(telemetry_value(
            &telemetry,
            connection.telemetry_sequence,
            actor_snapshot.wall_millis,
            elapsed_millis,
        ));
        connection.latest_telemetry = Some(telemetry);
        let lighting = connection.controls.as_ref().and_then(|controls| {
            controls
                .state
                .lighting
                .or(Some(protocol_refloat::LightingState {
                    leds_enabled: true,
                    headlights_enabled: true,
                }))
        });
        observe_controls(connection, lighting);
        if !connection.pause_telemetry {
            let ride_state = connection
                .latest_telemetry
                .as_ref()
                .map_or(domain_types::RideState::Unknown, |value| value.ride_state);
            let interval = connection
                .telemetry_polling
                .as_mut()
                .map_or(app_coordinator::RIDING_POLL_MILLIS, |polling| {
                    polling.accept_ride_state(ride_state, actor_snapshot.monotonic_millis)
                });
            connection.next_telemetry_due_millis =
                Some(actor_snapshot.monotonic_millis.saturating_add(interval));
        }
        Ok(())
    }

    fn accept_configuration_response(
        &self,
        connection: &mut NativeConnection,
        generation: u64,
        payload: Vec<u8>,
    ) -> Result<(), ContractError> {
        let operation = connection.configuration_operation.ok_or_else(|| {
            ContractError::new("invalid_state", "Configuration operation is unavailable")
        })?;
        let result = self
            .application
            .accept_configuration_response(generation, operation, payload);
        if let Ok(mut signal) = self.configuration_signal.lock() {
            *signal = signal.saturating_add(1);
            self.configuration_changed.notify_all();
        }
        let effect = result.map_err(|_| {
            ContractError::new("invalid_state", "Configuration response is invalid")
        })?;
        self.cache_configuration_schema(connection)?;
        self.schedule_configuration_effect(connection, effect)
    }

    fn schedule(
        &self,
        connection: &mut NativeConnection,
        stage: IdentificationStage,
        payload: &[u8],
        expected: ExpectedResponse,
        kind: RequestKind,
    ) -> Result<(), ContractError> {
        let generation = connection
            .vesc_generation
            .ok_or_else(|| ContractError::new("invalid_state", "VESC generation is unavailable"))?;
        let framed = encode_frame(payload)
            .map_err(|_| ContractError::new("internal", "VESC request is invalid"))?;
        let completions = self
            .application
            .schedule_vesc(RequestSpec {
                payload: framed,
                expected,
                priority: RequestPriority::ControllerIdentification,
                kind,
                timeout_millis: 2_000,
                generation,
                expects_response: true,
            })
            .map_err(|_| ContractError::new("unavailable", "VESC request is unavailable"))?;
        if !completions.is_empty() {
            return Err(ContractError::new(
                "unavailable",
                "VESC request was rejected",
            ));
        }
        connection.stage = stage;
        Ok(())
    }

    fn accept_vesc_completion(&self, completion: RequestCompletion) -> Result<(), ContractError> {
        let (request, payload) = match completion {
            RequestCompletion::Response { request, payload } => (request, payload),
            RequestCompletion::TimedOut(request) => {
                let mut connection = self
                    .connection
                    .lock()
                    .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
                if let Some(connection) = connection.as_mut()
                    && connection.vesc_generation == Some(request.generation)
                {
                    connection.telemetry_in_flight = false;
                    connection.stage = IdentificationStage::Reconnecting;
                    let connection_generation = self
                        .application
                        .snapshot()
                        .map_err(|_| {
                            ContractError::new("internal", "Application state is unavailable")
                        })?
                        .connection_generation;
                    self.request_host(HostCommand::BluetoothDisconnect {
                        connection_generation,
                    })?;
                }
                return Ok(());
            }
            _ => return Ok(()),
        };
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let Some(connection) = connection.as_mut() else {
            return Ok(());
        };
        if connection.vesc_generation != Some(request.generation) {
            return Ok(());
        }
        conformance_trace(&format!(
            "vesc_response stage={:?} kind={:?}",
            connection.stage, request.kind
        ));
        match (connection.stage, request.kind) {
            (IdentificationStage::TransportFirmware, RequestKind::ControllerFirmware) => {
                self.accept_transport_firmware(connection, &payload)?;
            }
            (IdentificationStage::CanPing, RequestKind::CanPing) => {
                self.accept_can_ping(connection, &payload)?;
            }
            (IdentificationStage::EnumeratingControllers, RequestKind::ControllerFirmware) => {
                self.accept_controller_firmware(connection, &payload, true)?;
            }
            (IdentificationStage::ControllerFirmware, RequestKind::ControllerFirmware) => {
                self.accept_controller_firmware(connection, &payload, false)?;
            }
            (IdentificationStage::RefloatInfo, RequestKind::RefloatInfo) => {
                self.accept_refloat_info(connection, &payload)?;
            }
            (IdentificationStage::RefloatLayout, RequestKind::RefloatLayout) => {
                self.accept_refloat_layout(connection, &payload)?;
            }
            (IdentificationStage::BmsCanPing, RequestKind::CanPing) => {
                self.accept_bms_ping(connection, &payload)?;
            }
            (IdentificationStage::EnumeratingBms, RequestKind::ControllerFirmware) => {
                self.accept_bms_identity(connection, &payload)?;
            }
            (IdentificationStage::BmsValues, RequestKind::BmsValues) => {
                self.accept_bms_values(connection, &payload)?;
            }
            (IdentificationStage::Telemetry, RequestKind::RefloatControl) => {
                Self::accept_control_response(connection, &payload)?;
            }
            (IdentificationStage::Telemetry, RequestKind::RefloatTelemetry) => {
                self.accept_telemetry_response(connection, &payload)?;
            }
            (_, RequestKind::RefloatConfiguration) => {
                self.accept_configuration_response(connection, request.generation, payload)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn schedule_refloat_info(
        &self,
        connection: &mut NativeConnection,
        route: ControllerRoute,
    ) -> Result<(), ContractError> {
        self.schedule(
            connection,
            IdentificationStage::RefloatInfo,
            &route_command(
                &protocol_refloat::request(protocol_refloat::RefloatCommand::Info, &[2, 0]),
                route,
            ),
            ExpectedResponse::new(
                VescCommand::CustomAppData,
                Some(protocol_refloat::RefloatCommand::Info as u8),
            ),
            RequestKind::RefloatInfo,
        )
    }

    fn schedule_next_inventory_probe(
        &self,
        connection: &mut NativeConnection,
    ) -> Result<(), ContractError> {
        if let Some(can_id) = connection.pending_can_ids.first().copied() {
            connection.pending_can_ids.remove(0);
            connection.controller_can_id = Some(can_id);
            let route = ControllerRoute::Can(
                domain_types::CanId::new(can_id)
                    .map_err(|_| ContractError::new("invalid_state", "CAN ID is invalid"))?,
            );
            return self.schedule(
                connection,
                IdentificationStage::EnumeratingControllers,
                &route_command(&[VescCommand::GetFirmwareVersion as u8], route),
                ExpectedResponse::new(VescCommand::GetFirmwareVersion, None),
                RequestKind::ControllerFirmware,
            );
        }
        let controllers: Vec<_> = connection
            .controller_inventory
            .iter()
            .filter(|firmware| firmware.hardware_type == 0)
            .cloned()
            .collect();
        match controllers.as_slice() {
            [firmware] => {
                self.establish_saved_identity(connection, firmware)?;
                connection.controller_can_id = firmware.can_id.map(domain_types::CanId::get);
                connection.firmware = Some(firmware.clone());
                self.schedule_refloat_info(connection, connection_route(connection)?)
            }
            [] => Err(ContractError::new("not_found", "No controller was found")),
            _ => {
                connection.controller_can_id = None;
                connection.firmware = None;
                connection.stage = IdentificationStage::Selecting;
                Ok(())
            }
        }
    }

    fn preferred_can_id(&self, connection: &NativeConnection) -> Result<Option<u8>, ContractError> {
        let Some((saved_device_id, _)) = connection.ownership.saved() else {
            return Ok(None);
        };
        let profile = load_profile(&self.contract.catalog, saved_device_id)
            .map_err(|error| storage_error(&error))?;
        Ok(profile.values.preferred_can_id())
    }

    fn begin_bms_discovery(&self, connection: &mut NativeConnection) -> Result<(), ContractError> {
        connection.bms_status = DiscoveryStatus::Discovering;
        let candidates: Vec<_> = connection
            .controller_inventory
            .iter()
            .filter(|firmware| protocol_vesc_bms::supports_firmware(firmware))
            .cloned()
            .collect();
        if !candidates.is_empty() {
            return self.finish_bms_discovery(connection, &candidates);
        }
        if connection
            .transport_firmware
            .as_ref()
            .is_some_and(|firmware| firmware.hardware_type == 0)
        {
            return self.schedule(
                connection,
                IdentificationStage::BmsCanPing,
                &[VescCommand::PingCan as u8],
                ExpectedResponse::new(VescCommand::PingCan, None),
                RequestKind::CanPing,
            );
        }
        connection.bms_status = DiscoveryStatus::NotFound;
        self.schedule_telemetry(connection)
    }

    fn schedule_next_bms_probe(
        &self,
        connection: &mut NativeConnection,
    ) -> Result<(), ContractError> {
        if let Some(can_id) = connection.bms_pending_can_ids.first().copied() {
            connection.bms_pending_can_ids.remove(0);
            connection.bms_probe_can_id = Some(can_id);
            let route = ControllerRoute::Can(
                domain_types::CanId::new(can_id)
                    .map_err(|_| ContractError::new("invalid_state", "BMS CAN ID is invalid"))?,
            );
            return self.schedule(
                connection,
                IdentificationStage::EnumeratingBms,
                &route_command(&[VescCommand::GetFirmwareVersion as u8], route),
                ExpectedResponse::new(VescCommand::GetFirmwareVersion, None),
                RequestKind::ControllerFirmware,
            );
        }
        connection.bms_probe_can_id = None;
        let candidates: Vec<_> = connection
            .controller_inventory
            .iter()
            .filter(|firmware| protocol_vesc_bms::supports_firmware(firmware))
            .cloned()
            .collect();
        self.finish_bms_discovery(connection, &candidates)
    }

    fn finish_bms_discovery(
        &self,
        connection: &mut NativeConnection,
        firmware: &[FirmwareRecord],
    ) -> Result<(), ContractError> {
        connection.bms_candidates = firmware
            .iter()
            .filter_map(|identity| {
                let can_id = identity.can_id?;
                Some(BmsCandidate::from_firmware(
                    BmsConnection::Can(can_id),
                    identity,
                ))
            })
            .take(protocol_vesc_bms::MAX_CANDIDATES)
            .collect();
        conformance_trace(&format!(
            "bms_discovery_complete candidates={}",
            connection.bms_candidates.len()
        ));
        match connection.bms_candidates.as_slice() {
            [candidate] => self.schedule_bms_values(connection, candidate.connection),
            [] => {
                connection.bms_status = DiscoveryStatus::NotFound;
                self.schedule_telemetry(connection)
            }
            _ => {
                connection.bms_status = DiscoveryStatus::Ambiguous;
                self.schedule_telemetry(connection)
            }
        }
    }

    fn schedule_bms_values(
        &self,
        connection: &mut NativeConnection,
        selected: BmsConnection,
    ) -> Result<(), ContractError> {
        connection.bms_connection = Some(selected);
        connection.bms_candidates.clear();
        let generation = connection
            .vesc_generation
            .ok_or_else(|| ContractError::new("invalid_state", "VESC generation is unavailable"))?;
        let payload = encode_frame(&protocol_vesc_bms::values_request(selected))
            .map_err(|_| ContractError::new("internal", "BMS request is invalid"))?;
        let completions = self
            .application
            .schedule_vesc(RequestSpec {
                payload,
                expected: ExpectedResponse::new(VescCommand::BmsGetValues, None),
                priority: RequestPriority::Auxiliary,
                kind: RequestKind::BmsValues,
                timeout_millis: 2_000,
                generation,
                expects_response: true,
            })
            .map_err(|_| ContractError::new("unavailable", "BMS request is unavailable"))?;
        if !completions.is_empty() {
            return Err(ContractError::new(
                "unavailable",
                "BMS request was rejected",
            ));
        }
        connection.stage = IdentificationStage::BmsValues;
        Ok(())
    }

    fn schedule_telemetry(&self, connection: &mut NativeConnection) -> Result<(), ContractError> {
        let route = connection_route(connection)?;
        let legacy = connection.package_version.as_deref() == Some("1.0");
        let (command, payload) = if legacy {
            (
                protocol_refloat::RefloatCommand::GetAllData,
                protocol_refloat::request(protocol_refloat::RefloatCommand::GetAllData, &[4]),
            )
        } else {
            (
                protocol_refloat::RefloatCommand::RealtimeDataInternal,
                protocol_refloat::request(
                    protocol_refloat::RefloatCommand::RealtimeDataInternal,
                    &[],
                ),
            )
        };
        let result = self.schedule(
            connection,
            IdentificationStage::Telemetry,
            &route_command(&payload, route),
            ExpectedResponse::new(VescCommand::CustomAppData, Some(command as u8)),
            RequestKind::RefloatTelemetry,
        );
        if result.is_ok() {
            connection.telemetry_in_flight = true;
        }
        result
    }

    fn begin_configuration(&self, connection: &mut NativeConnection) -> Result<(), ContractError> {
        let firmware = connection.firmware.clone().ok_or_else(|| {
            ContractError::new("invalid_state", "Controller firmware is unavailable")
        })?;
        let package_version = connection
            .package_version
            .clone()
            .ok_or_else(|| ContractError::new("invalid_state", "Refloat version is unavailable"))?;
        let cached_schema = configuration_cache_identity(&firmware, &package_version)
            .map(|identity| self.contract.catalog.schema_cache(&identity))
            .transpose()
            .map_err(|error| storage_error(&error))?
            .flatten();
        let writable = firmware.test_version == 0;
        let effect = self
            .application
            .begin_configuration(app_coordinator::ConfigurationContext {
                route: connection_route(connection)?,
                generation: connection.vesc_generation.unwrap_or_default(),
                firmware,
                reported_version: package_version,
                writable,
                flywheel: false,
                cached_schema,
            })
            .map_err(|_| ContractError::new("invalid_state", "Configuration could not start"))?;
        self.schedule_configuration_effect(connection, effect)
    }

    fn schedule_configuration_effect(
        &self,
        connection: &mut NativeConnection,
        effect: app_coordinator::ConfigurationEffect,
    ) -> Result<(), ContractError> {
        let app_coordinator::ConfigurationEffect::Request {
            route,
            payload,
            operation,
        } = effect
        else {
            connection.configuration_operation = None;
            return Ok(());
        };
        let command = payload
            .first()
            .copied()
            .and_then(|value| VescCommand::try_from(value).ok())
            .ok_or_else(|| ContractError::new("internal", "Configuration request is invalid"))?;
        let generation = connection
            .vesc_generation
            .ok_or_else(|| ContractError::new("invalid_state", "VESC generation is unavailable"))?;
        let completions = self
            .application
            .schedule_vesc(RequestSpec {
                payload: encode_frame(&route_command(&payload, route)).map_err(|_| {
                    ContractError::new("internal", "Configuration request is invalid")
                })?,
                expected: ExpectedResponse::new(command, None),
                priority: RequestPriority::ControllerWork,
                kind: RequestKind::RefloatConfiguration,
                timeout_millis: app_coordinator::CONFIGURATION_REQUEST_TIMEOUT_MILLIS,
                generation,
                expects_response: true,
            })
            .map_err(|_| {
                ContractError::new("unavailable", "Configuration request is unavailable")
            })?;
        if !completions.is_empty() {
            return Err(ContractError::new(
                "unavailable",
                "Configuration request was rejected",
            ));
        }
        connection.configuration_operation = Some(operation);
        Ok(())
    }

    fn cache_configuration_schema(
        &self,
        connection: &NativeConnection,
    ) -> Result<(), ContractError> {
        let Some(identity) = connection.firmware.as_ref().and_then(|firmware| {
            configuration_cache_identity(firmware, connection.package_version.as_deref()?)
        }) else {
            return Ok(());
        };
        let Some(bytes) = self
            .application
            .configuration_schema_bytes()
            .map_err(|_| ContractError::new("internal", "Configuration schema is unavailable"))?
        else {
            return Ok(());
        };
        self.contract
            .catalog
            .store_schema_cache(&identity, &bytes)
            .map_err(|error| storage_error(&error))
    }

    fn native_connection_value(
        &self,
        observed_at_millis: i64,
    ) -> Result<Option<ActiveConnection>, ContractError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let Some(connection) = connection.as_ref() else {
            return Ok(None);
        };
        let summary = connection
            .ownership
            .saved()
            .is_some()
            .then(|| native_session_summary(connection));
        let mut value = active_connection(summary.as_ref(), &connection.ownership);
        value["lastSeenAt"] = json!(iso8601_or_empty(observed_at_millis));
        value["deviceId"] = json!(connection.device_id);
        value["name"] = json!(connection.name);
        value["displayName"] = json!(connection.name);
        value["state"] = json!(if matches!(
            connection.stage,
            IdentificationStage::Telemetry | IdentificationStage::ReadyBms
        ) {
            "ready"
        } else if connection.stage == IdentificationStage::Reconnecting {
            "reconnecting"
        } else if connection.stage == IdentificationStage::Selecting {
            "selecting_controller"
        } else if connection.stage == IdentificationStage::WaitingBluetooth {
            "connecting"
        } else {
            "identifying"
        });
        value["controllerConnection"] = match connection.controller_can_id {
            Some(can_id) => json!({ "kind": "can", "canId": can_id }),
            None if connection.firmware.is_some() => json!({ "kind": "direct" }),
            None => Value::Null,
        };
        value["controllerCandidates"] = if connection.stage == IdentificationStage::Selecting {
            Value::Array(
                connection
                    .controller_inventory
                    .iter()
                    .filter(|firmware| firmware.hardware_type == 0)
                    .filter_map(|firmware| {
                        Some(json!({
                            "canId": firmware.can_id?.get(),
                            "uuid": canonical_firmware_uuid(firmware.controller_uuid),
                            "hardwareName": firmware.hardware_name,
                            "firmwareName": firmware.firmware_name,
                            "vescVersion": format!("{}.{:02}", firmware.major, firmware.minor),
                            "vescSupported": protocol_vesc::firmware_access(firmware).is_some()
                        }))
                    })
                    .collect(),
            )
        } else {
            json!([])
        };
        value["firmware"] = match (&connection.firmware, &connection.package_version) {
            (Some(firmware), Some(package_version))
                if connection.stage == IdentificationStage::Telemetry =>
            {
                json!({
                    "uuid": canonical_firmware_uuid(firmware.controller_uuid),
                    "vescVersion": format!("{}.{:02}", firmware.major, firmware.minor),
                    "packageName": "refloat",
                    "packageVersion": package_version,
                    "hardwareName": firmware.hardware_name,
                    "accessMode": "readWrite",
                    "supported": true
                })
            }
            _ => Value::Null,
        };
        value["bms"] = bms_state_value(connection, observed_at_millis);
        value["latestSample"] = connection.latest_sample.clone().unwrap_or(Value::Null);
        let shutdown_available = connection
            .firmware
            .as_ref()
            .is_some_and(protocol_vesc::supports_shutdown)
            && connection
                .latest_telemetry
                .as_ref()
                .is_some_and(|telemetry| {
                    telemetry.ride_state == domain_types::RideState::Idle
                        && telemetry.package_mode == protocol_refloat::PackageMode::Normal
                });
        value["controllerActions"] = json!({
            "shutdownAvailable": shutdown_available,
            "pendingAction": null
        });
        decorate_bms_connection(&mut value, connection);
        self.decorate_refloat_capabilities(&mut value, connection)?;
        serde_json::from_value(value)
            .map(Some)
            .map_err(|_| ContractError::new("internal", "Active connection mapping is invalid"))
    }

    fn decorate_refloat_capabilities(
        &self,
        value: &mut Value,
        connection: &NativeConnection,
    ) -> Result<(), ContractError> {
        if let (Some(controls), Some(version)) = (&connection.controls, &connection.package_version)
        {
            let profile = protocol_refloat::compatibility(version)
                .map(protocol_refloat::CompatibilityEntry::control_profile);
            value["refloatControls"] = json!({
                "status": "ready",
                "writable": controls.state.writable,
                "packageEnabled": controls.state.package_enabled,
                "flywheelEnabled": controls.state.flywheel_enabled,
                "lightingAvailable": controls.state.lighting.is_some(),
                "ledsEnabled": controls.state.lighting.map(|lighting| lighting.leds_enabled),
                "headlightsEnabled": controls.state.lighting.map(|lighting| lighting.headlights_enabled),
                "fatalErrorClearAvailable": profile.is_some_and(|profile| profile.fatal_clear_available),
                "fatalErrorLatched": controls.state.fatal_latched,
                "fatalErrorActive": controls.state.fatal_active,
                "fatalErrorClearable": profile.is_some_and(|profile| profile.fatal_clear_available)
                    && controls.state.fatal_latched == Some(true)
                    && controls.state.fatal_active == Some(false),
                "pendingAction": controls.state.pending.map(pending_action_name)
            });
        }
        if connection.firmware.is_some() {
            value["refloatConfiguration"] =
                configuration_summary_value(&self.application.configuration_summary().map_err(
                    |_| ContractError::new("internal", "Configuration state is unavailable"),
                )?);
        }
        Ok(())
    }

    fn native_session_summary(&self) -> Result<Option<SessionSummary>, ContractError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        Ok(connection
            .as_ref()
            .filter(|connection| {
                connection.firmware.is_some() && connection.ownership.saved().is_some()
            })
            .map(native_session_summary))
    }

    fn configuration_open(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        request_params::<EmptyParams>(request, "Method takes no parameters")?;
        self.application
            .configuration_summary()
            .map(|summary| configuration_summary_value(&summary))
            .map_err(|_| ContractError::new("internal", "Configuration state is unavailable"))
    }

    fn configuration_schema(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: CursorParams =
            request_params(request, "Configuration schema parameters are invalid")?;
        let cursor = parse_cursor(params.cursor.as_deref())?;
        let schema = self
            .application
            .configuration_schema()
            .map_err(|_| ContractError::new("internal", "Configuration schema is unavailable"))?
            .ok_or_else(|| ContractError::new("invalid_state", "Configuration is unavailable"))?;
        let summary = self
            .application
            .configuration_summary()
            .map_err(|_| ContractError::new("internal", "Configuration state is unavailable"))?;
        let identity = summary
            .identity
            .as_ref()
            .ok_or_else(|| ContractError::new("invalid_state", "Configuration is unavailable"))?;
        let pages = configuration_schema_pages(&schema);
        let (group, subgroup, fields) = pages
            .get(cursor)
            .ok_or_else(|| invalid("Configuration schema cursor is invalid"))?;
        Ok(json!({
            "identity": tune_identity_value(identity),
            "group": group,
            "subgroup": subgroup,
            "fields": fields
                .iter()
                .map(|field| configuration_field_value(field))
                .collect::<Vec<_>>(),
            "nextCursor": (cursor + 1 < pages.len()).then(|| (cursor + 1).to_string()),
        }))
    }

    fn configuration_read(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: ConfigurationReadParams =
            request_params(request, "Configuration read parameters are invalid")?;
        let defaults = matches!(params.source, ConfigurationSource::Default);
        let summary = self
            .application
            .configuration_summary()
            .map_err(|_| ContractError::new("internal", "Configuration state is unavailable"))?;
        let identity = summary
            .identity
            .as_ref()
            .ok_or_else(|| ContractError::new("invalid_state", "Configuration is unavailable"))?;
        let values = self
            .application
            .configuration_values(defaults)
            .map_err(|_| ContractError::new("internal", "Configuration values are unavailable"))?
            .ok_or_else(|| {
                ContractError::new("invalid_state", "Configuration values are not loaded")
            })?;
        Ok(configuration_values_value(
            identity,
            summary.revision,
            defaults,
            &values,
        ))
    }

    fn configuration_write(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: ConfigurationWriteParams =
            request_params(request, "Configuration write parameters are invalid")?;
        if params.package_version.is_empty() || params.schema_fingerprint.is_empty() {
            return Err(invalid("Configuration identity is invalid"));
        }
        let identity = protocol_refloat::TuneIdentity {
            package_version: params.package_version,
            schema_fingerprint: params.schema_fingerprint,
            wire_signature: params.wire_signature,
        };
        let values = parse_configuration_values(&params.values)?;
        self.write_configuration(&identity, params.base_revision, values)
    }

    fn tune_apply(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: TuneApplyParams = request_params(request, "Tune apply parameters are invalid")?;
        if params.tune_id.is_empty() {
            return Err(invalid("Tune ID is required"));
        }
        let tune = self
            .contract
            .catalog
            .tune(&params.tune_id)
            .map_err(|error| storage_error(&error))?;
        if tune.revision != params.expected_revision {
            return Err(ContractError {
                code: "conflict",
                message: "Stored revision changed".into(),
                details: Some(json!({ "currentRevision": tune.revision })),
            });
        }
        let encoded: HashMap<String, ConfigurationValueDto> =
            serde_json::from_str(&tune.values_json)
                .map_err(|_| ContractError::new("internal", "Stored Tune is invalid"))?;
        let values = parse_configuration_values(&encoded)?;
        self.write_configuration(&tune_record_identity(&tune), params.base_revision, values)
    }

    fn write_configuration(
        &self,
        identity: &protocol_refloat::TuneIdentity,
        base_revision: u64,
        values: std::collections::HashMap<String, protocol_refloat::ConfigurationValue>,
    ) -> Result<Value, ContractError> {
        self.require_current_configuration(identity, base_revision)?;
        self.validate_configuration_values(identity, &values)?;
        {
            let mut connection = self
                .connection
                .lock()
                .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
            let connection = connection
                .as_mut()
                .ok_or_else(|| ContractError::new("invalid_state", "No board is active"))?;
            let safe = connection
                .latest_telemetry
                .as_ref()
                .is_some_and(|telemetry| {
                    telemetry.ride_state == domain_types::RideState::Idle
                        && telemetry.package_mode == protocol_refloat::PackageMode::Normal
                });
            if !safe || connection.pause_telemetry {
                return Err(ContractError::new(
                    "invalid_state",
                    "Configuration write is unsafe",
                ));
            }
            connection.pause_telemetry = true;
            let effect = self
                .application
                .request_configuration_write(identity.clone(), base_revision, values)
                .map_err(|_| {
                    connection.pause_telemetry = false;
                    ContractError::new("invalid_state", "Configuration write is unavailable")
                })?;
            self.schedule_configuration_effect(connection, effect)
                .inspect_err(|_| connection.pause_telemetry = false)?;
        }
        let result = self.await_configuration_write(identity, base_revision);
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        if let Some(connection) = connection.as_mut() {
            connection.pause_telemetry = false;
            if !connection.telemetry_in_flight && connection.stage == IdentificationStage::Telemetry
            {
                self.schedule_telemetry(connection)?;
            }
        }
        result
    }

    fn await_configuration_write(
        &self,
        identity: &protocol_refloat::TuneIdentity,
        base_revision: u64,
    ) -> Result<Value, ContractError> {
        let deadline = Instant::now()
            + Duration::from_millis(
                app_coordinator::CONFIGURATION_REQUEST_TIMEOUT_MILLIS * 2 + 1_000,
            );
        let mut signal = self
            .configuration_signal
            .lock()
            .map_err(|_| ContractError::new("internal", "Configuration state failed"))?;
        loop {
            let summary = self
                .application
                .configuration_summary()
                .map_err(|_| ContractError::new("internal", "Configuration state failed"))?;
            if summary.pending.is_none() {
                if summary.identity.as_ref() != Some(identity) || summary.revision <= base_revision
                {
                    return Err(ContractError::new(
                        "conflict",
                        "Configuration write was not verified",
                    ));
                }
                let values = self
                    .application
                    .configuration_values(false)
                    .map_err(|_| ContractError::new("internal", "Configuration state failed"))?
                    .ok_or_else(|| {
                        ContractError::new("conflict", "Configuration readback is unavailable")
                    })?;
                return Ok(configuration_values_value(
                    identity,
                    summary.revision,
                    false,
                    &values,
                ));
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(ContractError::new(
                    "unavailable",
                    "Configuration verification timed out",
                ));
            }
            let (next, timeout) = self
                .configuration_changed
                .wait_timeout(signal, deadline.saturating_duration_since(now))
                .map_err(|_| ContractError::new("internal", "Configuration state failed"))?;
            signal = next;
            if timeout.timed_out() {
                return Err(ContractError::new(
                    "unavailable",
                    "Configuration verification timed out",
                ));
            }
        }
    }

    fn tune_list(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: TuneListParams = request_params(request, "Tune list parameters are invalid")?;
        let identity = parse_tune_identity(params.identity)?;
        if !(1..=100).contains(&params.limit) {
            return Err(invalid("Tune page limit is invalid"));
        }
        let limit = params.limit;
        let offset = parse_cursor(params.cursor.as_deref())?;
        let records = self
            .contract
            .catalog
            .tunes(
                &identity.package_version,
                &identity.schema_fingerprint,
                identity.wire_signature,
                limit.saturating_add(1),
                offset,
            )
            .map_err(|error| storage_error(&error))?;
        let has_next = records.len() > limit;
        let items = records
            .iter()
            .take(limit)
            .map(tune_summary_value)
            .collect::<Vec<_>>();
        Ok(json!({
            "items": items,
            "nextCursor": has_next.then(|| offset.saturating_add(limit).to_string()),
        }))
    }

    fn tune_get(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: TuneIdParams = request_params(request, "Tune parameters are invalid")?;
        if params.tune_id.is_empty() {
            return Err(invalid("Tune ID is required"));
        }
        let record = self
            .contract
            .catalog
            .tune(&params.tune_id)
            .map_err(|error| storage_error(&error))?;
        tune_value(&record)
    }

    fn tune_create(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: TuneCreateParams =
            request_params(request, "Tune create parameters are invalid")?;
        let identity = parse_tune_identity(params.identity)?;
        self.require_current_configuration(&identity, params.source_revision)?;
        let values = parse_configuration_values(&params.values)?;
        self.validate_configuration_values(&identity, &values)?;
        let now = unix_millis()?;
        let tune_id = self.next_tune_id(now)?;
        let record = TuneRecord {
            configuration_id: tune_id,
            revision: 0,
            name: tune_name(&params.name)?,
            description: tune_description(params.description)?,
            package_version: identity.package_version,
            schema_fingerprint: identity.schema_fingerprint,
            wire_signature: identity.wire_signature,
            values_json: encode_configuration_values_json(&values)?,
            created_at_millis: now,
            updated_at_millis: now,
        };
        let stored = self
            .contract
            .catalog
            .create_tune(&record)
            .map_err(|error| storage_error(&error))?;
        tune_value(&stored)
    }

    fn tune_update(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: TuneUpdateParams =
            request_params(request, "Tune update parameters are invalid")?;
        if params.tune_id.is_empty() {
            return Err(invalid("Tune ID is required"));
        }
        let current = self
            .contract
            .catalog
            .tune(&params.tune_id)
            .map_err(|error| storage_error(&error))?;
        let identity = tune_record_identity(&current);
        let values = parse_configuration_values(&params.values)?;
        self.validate_configuration_values(&identity, &values)?;
        let record = TuneRecord {
            configuration_id: params.tune_id,
            revision: current.revision,
            name: tune_name(&params.name)?,
            description: tune_description(params.description)?,
            package_version: current.package_version,
            schema_fingerprint: current.schema_fingerprint,
            wire_signature: current.wire_signature,
            values_json: encode_configuration_values_json(&values)?,
            created_at_millis: current.created_at_millis,
            updated_at_millis: unix_millis()?,
        };
        let stored = self
            .contract
            .catalog
            .update_tune(&record, params.expected_revision)
            .map_err(|error| storage_error(&error))?;
        tune_value(&stored)
    }

    fn tune_delete(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: TuneDeleteParams =
            request_params(request, "Tune delete parameters are invalid")?;
        if params.tune_id.is_empty() {
            return Err(invalid("Tune ID is required"));
        }
        self.contract
            .catalog
            .delete_tune(&params.tune_id, params.expected_revision)
            .map_err(|error| storage_error(&error))?;
        Ok(accepted())
    }

    fn require_current_configuration(
        &self,
        identity: &protocol_refloat::TuneIdentity,
        revision: u64,
    ) -> Result<(), ContractError> {
        let summary = self
            .application
            .configuration_summary()
            .map_err(|_| ContractError::new("internal", "Configuration state is unavailable"))?;
        if summary.identity.as_ref() == Some(identity) && summary.revision == revision {
            Ok(())
        } else {
            Err(ContractError::new(
                "conflict",
                "Configuration identity or revision changed",
            ))
        }
    }

    fn validate_configuration_values(
        &self,
        identity: &protocol_refloat::TuneIdentity,
        values: &std::collections::HashMap<String, protocol_refloat::ConfigurationValue>,
    ) -> Result<(), ContractError> {
        let schema = self
            .application
            .configuration_schema()
            .map_err(|_| ContractError::new("internal", "Configuration schema is unavailable"))?
            .ok_or_else(|| ContractError::new("invalid_state", "Configuration is unavailable"))?;
        identity
            .validate(&identity.package_version, &schema)
            .and_then(|()| {
                protocol_refloat::encode_configuration_values(values, &schema).map(|_| ())
            })
            .map_err(|_| invalid("Configuration values are invalid"))
    }

    fn next_tune_id(&self, now: i64) -> Result<String, ContractError> {
        let mut inner = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
        let sequence = inner.next_operation;
        inner.next_operation = inner.next_operation.saturating_add(1);
        Ok(format!("tune-{}-{sequence}", now.max(0)))
    }

    fn connect(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: ConnectParams = request_params(request, "Connection parameters are invalid")?;
        if params.device_id.is_empty() {
            return Err(invalid("Device ID is required"));
        }
        let tracking_mode = match params.tracking_mode {
            TrackingModeParam::Saved => TrackingMode::Saved,
            TrackingModeParam::Guest => TrackingMode::Guest,
        };
        if self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?
            .is_some()
        {
            return Err(ContractError::new(
                "invalid_state",
                "A device is already active",
            ));
        }
        let id = params.device_id;
        let snapshot = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?;
        let association = snapshot
            .discovered_devices
            .iter()
            .find(|device| public_discovery_id(snapshot.instance_nonce, &device.association) == id)
            .map(|device| device.association.clone())
            .ok_or_else(|| ContractError::new("not_found", "Device is no longer available"))?;
        let sampling = self.current_sampling_settings()?;
        let (ownership, saved_firmware_uuid) = if tracking_mode == TrackingMode::Guest {
            (ConnectionOwnership::Guest, None)
        } else {
            let private_association = encode_platform_association(&association);
            let existing = self
                .contract
                .catalog
                .saved_devices()
                .map_err(|error| storage_error(&error))?
                .into_iter()
                .find(|saved| {
                    saved.opaque_platform_id.as_deref() == Some(private_association.as_str())
                });
            existing.map_or((ConnectionOwnership::PendingSaved, None), |saved| {
                (
                    ConnectionOwnership::Saved {
                        saved_device_id: saved.saved_device_id,
                        device_history_id: saved.device_history_id,
                    },
                    saved
                        .firmware_device_uuid
                        .and_then(|uuid| FirmwareDeviceUuid::new(uuid).ok()),
                )
            })
        };
        self.application
            .start_tracking(ownership.tracking_mode())
            .map_err(|_| ContractError::new("invalid_state", "Tracking could not start"))?;
        self.request_host(HostCommand::BluetoothStopScan)?;
        self.request_host(HostCommand::BluetoothConnect {
            connection_generation: snapshot.connection_generation.saturating_add(1),
            association: association.clone(),
        })?;
        let connection = new_native_connection(
            &snapshot,
            id,
            association,
            ownership,
            saved_firmware_uuid,
            sampling.reconnect_duration_millis,
        );
        *self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))? =
            Some(connection);
        Ok(accepted())
    }

    fn disconnect(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        request_params::<EmptyParams>(request, "Method takes no parameters")?;
        let snapshot = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?;
        self.request_host(HostCommand::BluetoothDisconnect {
            connection_generation: snapshot.connection_generation,
        })?;
        let session_id = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?
            .take()
            .and_then(|connection| {
                connection
                    .ownership
                    .saved()
                    .is_some()
                    .then_some(connection.session_id)
            });
        let mut inner = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
        if let Some(session_id) = session_id
            && self.contract.catalog.session_record(&session_id).is_ok()
        {
            self.contract
                .sessions
                .close(&session_id, snapshot.wall_millis, "disconnect")
                .map_err(|error| storage_error(&error))?;
            let summary = self
                .contract
                .catalog
                .session_record(&session_id)
                .map(|record| session_summary_from_record(&record))
                .map_err(|error| storage_error(&error))?;
            if let Some(session) = inner
                .sessions
                .iter_mut()
                .find(|session| session.session_id == session_id)
            {
                *session = summary;
            }
        }
        inner.state.active_connection = None;
        self.contract.changed(&mut inner, true);
        Ok(accepted())
    }

    fn select_controller(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: SelectControllerParams =
            request_params(request, "Controller selection parameters are invalid")?;
        if params.device_id.is_empty() || params.can_id > 253 {
            return Err(invalid("Controller selection parameters are invalid"));
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let connection = connection
            .as_mut()
            .filter(|connection| {
                connection.device_id == params.device_id
                    && connection.stage == IdentificationStage::Selecting
            })
            .ok_or_else(|| {
                ContractError::new("invalid_state", "Controller selection is unavailable")
            })?;
        let firmware = connection
            .controller_inventory
            .iter()
            .find(|firmware| {
                firmware.can_id.map(domain_types::CanId::get) == Some(params.can_id)
                    && firmware.hardware_type == 0
                    && protocol_vesc::firmware_access(firmware).is_some()
            })
            .cloned()
            .ok_or_else(|| ContractError::new("not_found", "Controller was not found"))?;
        self.establish_saved_identity(connection, &firmware)?;
        connection.controller_can_id = Some(params.can_id);
        connection.firmware = Some(firmware);
        let route = ControllerRoute::Can(
            domain_types::CanId::new(params.can_id)
                .map_err(|_| ContractError::new("invalid_state", "CAN ID is invalid"))?,
        );
        self.schedule_refloat_info(connection, route)?;
        Ok(accepted())
    }

    fn set_control(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: SetControlParams = request_params(request, "Control parameters are invalid")?;
        let enabled = params.enabled;
        let control = match params.control {
            ControlParam::Package => protocol_refloat::BooleanControl::Package,
            ControlParam::Flywheel => protocol_refloat::BooleanControl::Flywheel,
            ControlParam::Leds => protocol_refloat::BooleanControl::Leds,
            ControlParam::Headlights => protocol_refloat::BooleanControl::Headlights,
        };
        let deadline =
            self.pause_telemetry_and_wait("Telemetry did not yield for the control request")?;
        let Some(one_way) = self.schedule_control(control, enabled)? else {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
            return control_state_value(
                connection
                    .as_ref()
                    .ok_or_else(|| ContractError::new("invalid_state", "No board is active"))?,
            );
        };
        if !one_way {
            loop {
                self.process_publications()?;
                let completed = self
                    .connection
                    .lock()
                    .map_err(|_| ContractError::new("internal", "Connection state failed"))?
                    .as_ref()
                    .and_then(|connection| connection.controls.as_ref())
                    .is_some_and(|controls| controls.state.pending.is_none());
                if completed {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err(ContractError::new(
                        "unavailable",
                        "Control confirmation timed out",
                    ));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let connection = connection
            .as_mut()
            .ok_or_else(|| ContractError::new("invalid_state", "No board is active"))?;
        self.resume_telemetry(connection)?;
        control_state_value(connection)
    }

    fn schedule_control(
        &self,
        control: protocol_refloat::BooleanControl,
        enabled: bool,
    ) -> Result<Option<bool>, ContractError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let connection = connection
            .as_mut()
            .ok_or_else(|| ContractError::new("invalid_state", "No board is active"))?;
        let telemetry = connection
            .latest_telemetry
            .as_ref()
            .ok_or_else(|| ContractError::new("invalid_state", "Telemetry is unavailable"))?;
        let version = connection
            .package_version
            .as_deref()
            .ok_or_else(|| ContractError::new("invalid_state", "Refloat version is unavailable"))?;
        let profile = protocol_refloat::compatibility(version)
            .map(protocol_refloat::CompatibilityEntry::control_profile)
            .ok_or_else(|| {
                ContractError::new("invalid_state", "Refloat controls are unsupported")
            })?;
        let context = protocol_refloat::MutationContext {
            generation: connection.vesc_generation.unwrap_or_default(),
            profile,
            connection_ready: true,
            writable: connection
                .firmware
                .as_ref()
                .is_some_and(|firmware| firmware.test_version == 0),
            ride_state: telemetry.ride_state,
            package_state: telemetry.package_state,
            package_mode: telemetry.package_mode,
        };
        let authorization = connection
            .controls
            .as_mut()
            .ok_or_else(|| ContractError::new("invalid_state", "Controls are unavailable"))?
            .authorize_set(control, enabled, context)
            .map_err(|_| ContractError::new("invalid_state", "Control is unsafe"))?;
        let protocol_refloat::Authorization::Mutation {
            payload, one_way, ..
        } = authorization
        else {
            self.resume_telemetry(connection)?;
            return Ok(None);
        };
        let route = connection_route(connection)?;
        let command = payload
            .get(2)
            .copied()
            .ok_or_else(|| ContractError::new("internal", "Control request is invalid"))?;
        let generation = connection.vesc_generation.unwrap_or_default();
        let completions = self
            .application
            .schedule_vesc(RequestSpec {
                payload: encode_frame(&route_command(&payload, route))
                    .map_err(|_| ContractError::new("internal", "Control request is invalid"))?,
                expected: ExpectedResponse::new(VescCommand::CustomAppData, Some(command)),
                priority: RequestPriority::ControllerIdentification,
                kind: RequestKind::RefloatControl,
                timeout_millis: 2_000,
                generation,
                expects_response: !one_way,
            })
            .map_err(|_| ContractError::new("unavailable", "Control request failed"))?;
        if one_way {
            if !one_way_request_accepted(&completions) {
                return Err(ContractError::new("unavailable", "Control was not sent"));
            }
            apply_one_way_control(connection, control, enabled);
            std::thread::sleep(Duration::from_millis(30));
        } else if !completions.is_empty() {
            return Err(ContractError::new("unavailable", "Control was rejected"));
        }
        Ok(Some(one_way))
    }

    fn dispatch_platform_request(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        match request.method.as_str() {
            "bluetooth.requestPermission" => {
                request_params::<EmptyParams>(request, "Method takes no parameters")?;
                self.request_host(HostCommand::PresentPermission {
                    permission: PermissionKind::Bluetooth,
                })?;
                self.refresh()?;
                let inner = self
                    .contract
                    .inner
                    .lock()
                    .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
                serde_json::to_value(&inner.state.bluetooth)
                    .map_err(|_| ContractError::new("internal", "Bluetooth state encoding failed"))
            }
            "bluetooth.scan" => {
                let params: ScanParams = request_params(request, "Scan parameters are invalid")?;
                if params
                    .timeout_ms
                    .is_some_and(|timeout| !(1..=60_000).contains(&timeout))
                {
                    return Err(invalid("Scan timeout is invalid"));
                }
                self.request_host(HostCommand::BluetoothStartScan)?;
                Ok(accepted())
            }
            "location.requestPermission" => {
                request_params::<EmptyParams>(request, "Method takes no parameters")?;
                self.request_host(HostCommand::PresentPermission {
                    permission: PermissionKind::Location,
                })?;
                self.request_host(HostCommand::LocationStart { background: true })?;
                self.request_host(HostCommand::SetBackgroundLogging { enabled: true })?;
                self.location_state_after_refresh()
            }
            "location.openSettings" => {
                request_params::<EmptyParams>(request, "Method takes no parameters")?;
                self.request_host(HostCommand::PresentSettings)?;
                self.request_host(HostCommand::LocationStart { background: true })?;
                self.location_state_after_refresh()
            }
            "settings.update" => self.update_settings(request),
            _ => unreachable!("only platform requests are routed here"),
        }
    }

    fn location_state_after_refresh(&self) -> Result<Value, ContractError> {
        self.refresh()?;
        let inner = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
        serde_json::to_value(&inner.state.location)
            .map_err(|_| ContractError::new("internal", "Location state encoding failed"))
    }

    fn update_settings(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params: SettingsUpdateParams =
            request_params(request, "Settings parameters are invalid")?;
        let location_enabled = params.patch.location_recording_enabled();
        let result = self.contract.handle(request)?;
        let sampling = serde_json::from_value::<settings::Settings>(result.clone())
            .map_err(|_| ContractError::new("internal", "Settings result is invalid"))?
            .sampling();
        self.application.configure_sampling(sampling).map_err(|_| {
            ContractError::new("internal", "Sampling settings could not be applied")
        })?;
        let snapshot = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?;
        self.configure_connection_sampling(sampling, snapshot.monotonic_millis)?;
        if let Some(enabled) = location_enabled {
            self.request_host(if enabled {
                HostCommand::LocationStart { background: true }
            } else {
                HostCommand::LocationStop
            })?;
            self.request_host(HostCommand::SetBackgroundLogging { enabled })?;
        }
        Ok(result)
    }

    fn clear_fatal(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        request_params::<EmptyParams>(request, "Method takes no parameters")?;
        self.pause_telemetry_and_wait("Telemetry did not yield for the control request")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let connection = connection
            .as_mut()
            .ok_or_else(|| ContractError::new("invalid_state", "No board is active"))?;
        let telemetry = connection
            .latest_telemetry
            .as_ref()
            .ok_or_else(|| ContractError::new("invalid_state", "Telemetry is unavailable"))?;
        let version = connection
            .package_version
            .as_deref()
            .ok_or_else(|| ContractError::new("invalid_state", "Refloat version is unavailable"))?;
        let profile = protocol_refloat::compatibility(version)
            .map(protocol_refloat::CompatibilityEntry::control_profile)
            .ok_or_else(|| {
                ContractError::new("invalid_state", "Refloat controls are unsupported")
            })?;
        let context = protocol_refloat::MutationContext {
            generation: connection.vesc_generation.unwrap_or_default(),
            profile,
            connection_ready: true,
            writable: connection
                .firmware
                .as_ref()
                .is_some_and(|firmware| firmware.test_version == 0),
            ride_state: telemetry.ride_state,
            package_state: telemetry.package_state,
            package_mode: telemetry.package_mode,
        };
        let payload = connection
            .controls
            .as_mut()
            .ok_or_else(|| ContractError::new("invalid_state", "Controls are unavailable"))?
            .authorize_fatal_clear(context)
            .map_err(|_| ContractError::new("invalid_state", "Fatal error cannot be cleared"))?;
        let route = connection_route(connection)?;
        let completions = self
            .application
            .schedule_vesc(RequestSpec {
                payload: encode_frame(&route_command(&payload, route))
                    .map_err(|_| ContractError::new("internal", "Control request is invalid"))?,
                expected: ExpectedResponse::new(
                    VescCommand::CustomAppData,
                    Some(protocol_refloat::RefloatCommand::AlertsControl as u8),
                ),
                priority: RequestPriority::ControllerIdentification,
                kind: RequestKind::RefloatControl,
                timeout_millis: 2_000,
                generation: connection.vesc_generation.unwrap_or_default(),
                expects_response: false,
            })
            .map_err(|_| ContractError::new("unavailable", "Control request failed"))?;
        if !one_way_request_accepted(&completions) {
            return Err(ContractError::new("unavailable", "Control was not sent"));
        }
        if let Some(telemetry) = connection.latest_telemetry.as_mut() {
            telemetry.fatal_error_latched = Some(false);
            telemetry.firmware_fault = None;
        }
        let lighting = connection
            .controls
            .as_ref()
            .and_then(|controls| controls.state.lighting);
        observe_controls(connection, lighting);
        std::thread::sleep(Duration::from_millis(30));
        self.resume_telemetry(connection)?;
        control_state_value(connection)
    }

    fn shutdown_controller(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        request_params::<EmptyParams>(request, "Method takes no parameters")?;
        self.pause_telemetry_and_wait("Telemetry did not yield for shutdown")?;
        let starting_sequence = self.send_shutdown()?;
        self.wait_for_shutdown_probe(starting_sequence)?;
        Ok(accepted())
    }

    fn pause_telemetry_and_wait(
        &self,
        timeout_message: &'static str,
    ) -> Result<Instant, ContractError> {
        {
            let mut connection = self
                .connection
                .lock()
                .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
            connection
                .as_mut()
                .ok_or_else(|| ContractError::new("invalid_state", "No board is active"))?
                .pause_telemetry = true;
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            self.process_publications()?;
            let in_flight = self
                .connection
                .lock()
                .map_err(|_| ContractError::new("internal", "Connection state failed"))?
                .as_ref()
                .is_some_and(|connection| connection.telemetry_in_flight);
            if !in_flight {
                break;
            }
            if Instant::now() >= deadline {
                return Err(ContractError::new("unavailable", timeout_message));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(deadline)
    }

    fn resume_telemetry(&self, connection: &mut NativeConnection) -> Result<(), ContractError> {
        connection.pause_telemetry = false;
        if !connection.telemetry_in_flight {
            self.schedule_telemetry(connection)?;
        }
        Ok(())
    }

    fn send_shutdown(&self) -> Result<u64, ContractError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let connection = connection
            .as_mut()
            .ok_or_else(|| ContractError::new("invalid_state", "No board is active"))?;
        let firmware = connection
            .firmware
            .as_ref()
            .ok_or_else(|| ContractError::new("invalid_state", "Firmware is unavailable"))?;
        let telemetry = connection
            .latest_telemetry
            .as_ref()
            .ok_or_else(|| ContractError::new("invalid_state", "Telemetry is unavailable"))?;
        if !protocol_vesc::supports_shutdown(firmware)
            || telemetry.ride_state != domain_types::RideState::Idle
            || telemetry.package_mode != protocol_refloat::PackageMode::Normal
        {
            return Err(ContractError::new("invalid_state", "Shutdown is unsafe"));
        }
        let route = connection_route(connection)?;
        let completions = self
            .application
            .schedule_vesc(RequestSpec {
                payload: encode_frame(&protocol_vesc::shutdown_command(route))
                    .map_err(|_| ContractError::new("internal", "Shutdown request is invalid"))?,
                expected: ExpectedResponse::new(VescCommand::Shutdown, None),
                priority: RequestPriority::ControllerIdentification,
                kind: RequestKind::ControllerShutdown,
                timeout_millis: 2_000,
                generation: connection.vesc_generation.unwrap_or_default(),
                expects_response: false,
            })
            .map_err(|_| ContractError::new("unavailable", "Shutdown request failed"))?;
        if !one_way_request_accepted(&completions) {
            return Err(ContractError::new("unavailable", "Shutdown was not sent"));
        }
        connection.shutdown_requested = true;
        let sequence = connection.telemetry_sequence;
        self.schedule_telemetry(connection)?;
        Ok(sequence)
    }

    fn wait_for_shutdown_probe(&self, starting_sequence: u64) -> Result<(), ContractError> {
        let probe_deadline = Instant::now() + Duration::from_secs(2);
        loop {
            self.process_publications()?;
            let probe = self
                .connection
                .lock()
                .map_err(|_| ContractError::new("internal", "Connection state failed"))?
                .as_ref()
                .map(|connection| connection.telemetry_sequence > starting_sequence);
            match probe {
                None | Some(true) => break,
                Some(false) if Instant::now() < probe_deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Some(false) => {
                    *self
                        .connection
                        .lock()
                        .map_err(|_| ContractError::new("internal", "Connection state failed"))? =
                        None;
                    let mut inner = self
                        .contract
                        .inner
                        .lock()
                        .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
                    inner.state.active_connection = None;
                    self.contract.changed(&mut inner, false);
                    break;
                }
            }
        }
        Ok(())
    }
}

fn new_native_connection(
    snapshot: &app_coordinator::ApplicationSnapshot,
    device_id: String,
    association: PlatformAssociation,
    ownership: ConnectionOwnership,
    saved_firmware_uuid: Option<FirmwareDeviceUuid>,
    reconnect_duration_millis: u64,
) -> NativeConnection {
    let name = snapshot
        .discovered_devices
        .iter()
        .find(|device| {
            public_discovery_id(snapshot.instance_nonce, &device.association) == device_id
        })
        .and_then(|device| device.name.clone());
    NativeConnection {
        session_id: format!(
            "native-{}-{}",
            snapshot.wall_millis.max(0),
            snapshot.connection_generation.saturating_add(1)
        ),
        ownership,
        started_at_millis: snapshot.wall_millis,
        device_id,
        association,
        identity_nonce: snapshot.instance_nonce,
        saved_firmware_uuid,
        name,
        stage: IdentificationStage::WaitingBluetooth,
        connect_after_generation: snapshot.connection_generation,
        vesc_generation: None,
        controller_can_id: None,
        pending_can_ids: Vec::new(),
        controller_inventory: Vec::new(),
        transport_firmware: None,
        firmware: None,
        package_version: None,
        telemetry_layout: None,
        latest_sample: None,
        telemetry_sequence: 0,
        last_telemetry_elapsed_millis: None,
        riding_elapsed_millis: 0,
        latest_telemetry: None,
        bms_status: DiscoveryStatus::Discovering,
        bms_connection: None,
        bms_candidates: Vec::new(),
        bms_snapshot: None,
        bms_pending_can_ids: Vec::new(),
        bms_probe_can_id: None,
        controls: None,
        telemetry_in_flight: false,
        telemetry_polling: None,
        next_telemetry_due_millis: None,
        pause_telemetry: false,
        shutdown_requested: false,
        reconnect: ReconnectModel::new(reconnect_duration_millis),
        reconnect_due_millis: None,
        handled_failure_generation: None,
        configuration_operation: None,
    }
}

fn decorate_bms_connection(value: &mut Value, connection: &NativeConnection) {
    if connection.stage != IdentificationStage::ReadyBms {
        return;
    }
    value["deviceKind"] = json!("bms");
    value["controllerConnection"] = Value::Null;
    value["controllerCandidates"] = json!([]);
    value["activeSession"] = Value::Null;
    value["latestSample"] = Value::Null;
    value["latestLocation"] = Value::Null;
    value["currentLimits"] = Value::Null;
    value["refloatControls"] = Value::Null;
    value["refloatConfiguration"] = Value::Null;
    value["controllerActions"] = json!({
        "shutdownAvailable": false,
        "pendingAction": null
    });
}

fn update_platform_state(
    catalog: &CatalogStore,
    inner: &mut BackendState,
    snapshot: &app_coordinator::ApplicationSnapshot,
) -> Result<(), StoreError> {
    let radio = match snapshot.bluetooth_radio {
        BluetoothRadio::PoweredOn => "powered_on",
        BluetoothRadio::PoweredOff => "powered_off",
        BluetoothRadio::Unknown => "unknown",
    };
    let permission = match snapshot.bluetooth_authorization {
        BluetoothAuthorization::Granted => "granted",
        BluetoothAuthorization::Denied => "denied",
        BluetoothAuthorization::Unknown => "not_determined",
    };
    inner.state.bluetooth = ContractBluetoothState {
        radio: radio.into(),
        scan_permission: permission.into(),
        connect_permission: permission.into(),
        scanning: false,
    };
    let location_enabled = inner.state.settings.location_recording_enabled();
    inner.state.location = if location_enabled {
        match snapshot.location_authorization {
            host_ports::LocationAuthorization::Background => LocationState {
                status: "authorized".into(),
                permission_action: None,
                background_capture: "available".into(),
                reason: None,
            },
            host_ports::LocationAuthorization::Foreground => LocationState {
                status: "foregroundOnly".into(),
                permission_action: Some("openSettings".into()),
                background_capture: "foregroundOnly".into(),
                reason: None,
            },
            host_ports::LocationAuthorization::Denied => LocationState {
                status: "denied".into(),
                permission_action: Some("openSettings".into()),
                background_capture: "unavailable".into(),
                reason: None,
            },
            host_ports::LocationAuthorization::Unknown => LocationState::recording_not_determined(),
        }
    } else {
        LocationState::disabled()
    };
    let saved_devices = catalog.saved_devices()?;
    inner.state.discovered_devices = snapshot
        .discovered_devices
        .iter()
        .map(|device| {
            let device_id = public_discovery_id(snapshot.instance_nonce, &device.association);
            let private_association = encode_platform_association(&device.association);
            let saved_device_id = saved_devices
                .iter()
                .find(|saved| {
                    saved.opaque_platform_id.as_deref() == Some(private_association.as_str())
                })
                .map(|saved| saved.saved_device_id.clone());
            DiscoveredDevice {
                device_id,
                name: device.name.clone(),
                state: "discovered".into(),
                last_seen_at: Some(iso8601_or_empty(snapshot.wall_millis)),
                rssi: Some(device.rssi),
                saved_device_id,
            }
        })
        .collect();
    inner.location_count = snapshot.location_sequence;
    Ok(())
}

const fn reconnect_due(decision: ReconnectDecision, now_millis: u64) -> Option<u64> {
    match decision {
        ReconnectDecision::ScheduleAfter(delay) => Some(now_millis.saturating_add(delay)),
        ReconnectDecision::ConnectNow => Some(now_millis),
        ReconnectDecision::TimedOut | ReconnectDecision::Disabled => None,
    }
}

fn conformance_trace(message: &str) {
    if std::env::var("FLOAT_VIEW_CONFORMANCE_TRACE").as_deref() == Ok("1") {
        eprintln!("[FloatViewCore] {message}");
    }
}

impl ContractDispatcher for NativeBackend {
    fn dispatch(&self, request: ContractRequest) -> Result<Value, ContractError> {
        self.refresh()?;
        match request.method.as_str() {
            "bluetooth.requestPermission"
            | "bluetooth.scan"
            | "location.requestPermission"
            | "location.openSettings"
            | "settings.update" => self.dispatch_platform_request(&request),
            "device.connect" => self.connect(&request),
            "device.selectController" => self.select_controller(&request),
            "device.disconnect" => self.disconnect(&request),
            "refloat.configuration.open" => self.configuration_open(&request),
            "refloat.configuration.schema" => self.configuration_schema(&request),
            "refloat.configuration.read" => self.configuration_read(&request),
            "refloat.configuration.write" => self.configuration_write(&request),
            "refloat.tune.list" => self.tune_list(&request),
            "refloat.tune.get" => self.tune_get(&request),
            "refloat.tune.create" => self.tune_create(&request),
            "refloat.tune.update" => self.tune_update(&request),
            "refloat.tune.delete" => self.tune_delete(&request),
            "refloat.tune.apply" => self.tune_apply(&request),
            "refloat.control.set" => self.set_control(&request),
            "refloat.fatalError.clear" => self.clear_fatal(&request),
            "controller.shutdown" => self.shutdown_controller(&request),
            "session.import" => self.begin_document_import(&request),
            "session.export" => self.begin_document_export(&request),
            _ => self.contract.handle(&request),
        }
    }

    fn take_state_update(&self) -> Option<Value> {
        self.refresh().ok()?;
        self.contract.take_state_update()
    }

    fn pump(&self) {
        if let Err(error) = self.refresh() {
            conformance_trace(&format!("native_pump_error code={}", error.code));
        }
    }

    fn next_pump_wait(&self) -> Duration {
        let Ok(snapshot) = self.application.snapshot() else {
            return Duration::from_millis(25);
        };
        let Ok(connection) = self.connection.lock() else {
            return Duration::from_millis(25);
        };
        let Some(connection) = connection.as_ref() else {
            return Duration::from_millis(100);
        };
        let due = [
            connection.next_telemetry_due_millis,
            connection.reconnect_due_millis,
        ]
        .into_iter()
        .flatten()
        .min();
        due.map_or(Duration::from_millis(25), |due| {
            Duration::from_millis(due.saturating_sub(snapshot.monotonic_millis).clamp(1, 100))
        })
    }
}

impl Drop for NativeBackend {
    fn drop(&mut self) {
        if let Ok(documents) = self.documents.get_mut() {
            for pending in documents.pending.values() {
                let _ = std::fs::remove_file(&pending.staging_path);
            }
            documents.pending.clear();
        }
    }
}

fn encode_platform_association(association: &PlatformAssociation) -> String {
    let mut value = String::with_capacity(association.as_bytes().len() * 2);
    for byte in association.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(&mut value, "{byte:02x}");
    }
    value
}

fn public_discovery_id(instance_nonce: [u8; 16], association: &PlatformAssociation) -> String {
    let mut identity = Vec::with_capacity(instance_nonce.len() + association.as_bytes().len());
    identity.extend_from_slice(&instance_nonce);
    identity.extend_from_slice(association.as_bytes());
    private_identifier(b"discovered", b"runtime-platform-association", &identity)
}

fn ready_generation_is_fresh(ready: bool, observed: u64, requested_after: u64) -> bool {
    ready && observed > requested_after
}

fn reset_identification(connection: &mut NativeConnection) {
    connection.controller_can_id = None;
    connection.pending_can_ids.clear();
    connection.controller_inventory.clear();
    connection.transport_firmware = None;
    connection.firmware = None;
    connection.package_version = None;
    connection.telemetry_layout = None;
    connection.latest_telemetry = None;
    connection.last_telemetry_elapsed_millis = None;
    connection.riding_elapsed_millis = 0;
    connection.controls = None;
    connection.telemetry_in_flight = false;
    connection.telemetry_polling = None;
    connection.next_telemetry_due_millis = None;
    connection.reconnect.cancel();
    connection.reconnect_due_millis = None;
    connection.pause_telemetry = false;
    connection.bms_status = DiscoveryStatus::Discovering;
    connection.bms_connection = None;
    connection.bms_candidates.clear();
    connection.bms_snapshot = None;
    connection.bms_pending_can_ids.clear();
    connection.bms_probe_can_id = None;
    connection.configuration_operation = None;
}

fn native_session_summary(connection: &NativeConnection) -> SessionSummary {
    let distance = connection
        .latest_sample
        .as_ref()
        .and_then(|sample| sample["distanceM"].as_f64());
    SessionSummary {
        session_id: connection.session_id.clone(),
        device_history_id: connection
            .ownership
            .saved()
            .map(|(_, device_history_id)| device_history_id.to_owned()),
        display_name: connection.name.clone().unwrap_or_else(|| "VESC".into()),
        started_at: iso8601_or_empty(connection.started_at_millis),
        ended_at: None,
        state: "active".into(),
        integrity: "complete".into(),
        sample_count: connection.telemetry_sequence,
        event_count: 0,
        riding_elapsed_ms: connection.riding_elapsed_millis,
        total_elapsed_ms: connection.last_telemetry_elapsed_millis.unwrap_or_default(),
        distance_m: distance,
        max_speed_mps: None,
        max_duty_cycle: None,
    }
}

fn bms_state_value(connection: &NativeConnection, observed_at_millis: i64) -> Value {
    let status = match connection.bms_status {
        DiscoveryStatus::Discovering => "discovering",
        DiscoveryStatus::NotFound => "not_found",
        DiscoveryStatus::Attached => "attached",
        DiscoveryStatus::Unavailable => "unavailable",
        DiscoveryStatus::Ambiguous => "ambiguous",
    };
    let selected_uuid = connection
        .bms_connection
        .and_then(|selected| match selected {
            BmsConnection::Direct => connection
                .transport_firmware
                .as_ref()
                .and_then(|firmware| canonical_firmware_uuid(firmware.controller_uuid)),
            BmsConnection::Can(can_id) => connection
                .controller_inventory
                .iter()
                .find(|firmware| firmware.can_id == Some(can_id))
                .and_then(|firmware| canonical_firmware_uuid(firmware.controller_uuid)),
        });
    json!({
        "status": status,
        "uuid": selected_uuid,
        "connection": connection.bms_connection.map(bms_connection_value),
        "candidates": connection.bms_candidates.iter().map(|candidate| json!({
            "connection": bms_connection_value(candidate.connection),
            "uuid": candidate.uuid,
            "hardwareName": candidate.hardware_name,
            "vescVersion": candidate.vesc_version
        })).collect::<Vec<_>>(),
        "latestSnapshot": connection.bms_snapshot.as_ref().map(|snapshot| bms_snapshot_value(snapshot, observed_at_millis))
    })
}

fn bms_connection_value(connection: BmsConnection) -> Value {
    match connection {
        BmsConnection::Direct => json!({ "kind": "direct" }),
        BmsConnection::Can(can_id) => json!({ "kind": "can", "canId": can_id.get() }),
    }
}

fn bms_snapshot_value(snapshot: &BmsSnapshot, observed_at_millis: i64) -> Value {
    json!({
        "observedAt": iso8601_or_empty(observed_at_millis),
        "totalVoltageV": snapshot.total_voltage_v,
        "chargeVoltageV": snapshot.charge_voltage_v,
        "inputCurrentA": snapshot.input_current_a,
        "inputIcCurrentA": snapshot.input_ic_current_a,
        "sessionAmpHoursAh": snapshot.session_amp_hours_ah,
        "sessionWattHoursWh": snapshot.session_watt_hours_wh,
        "cellVoltagesV": snapshot.cell_voltages_v,
        "balancing": snapshot.balancing,
        "temperaturesC": snapshot.temperatures_c,
        "icTemperatureC": snapshot.ic_temperature_c,
        "humiditySensorTemperatureC": snapshot.humidity_sensor_temperature_c,
        "humidityPercent": snapshot.humidity_percent,
        "highestCellTemperatureC": snapshot.highest_cell_temperature_c,
        "stateOfCharge": snapshot.state_of_charge,
        "stateOfHealth": snapshot.state_of_health,
        "reportedCanId": snapshot.reported_can_id.get(),
        "lifetimeChargeAmpHoursAh": snapshot.lifetime_charge_amp_hours_ah,
        "lifetimeChargeWattHoursWh": snapshot.lifetime_charge_watt_hours_wh,
        "lifetimeDischargeAmpHoursAh": snapshot.lifetime_discharge_amp_hours_ah,
        "lifetimeDischargeWattHoursWh": snapshot.lifetime_discharge_watt_hours_wh,
        "pressurePa": snapshot.pressure_pa,
        "dataVersion": snapshot.data_version,
        "status": snapshot.status
    })
}

fn connection_route(connection: &NativeConnection) -> Result<ControllerRoute, ContractError> {
    match connection.controller_can_id {
        Some(can_id) => domain_types::CanId::new(can_id)
            .map(ControllerRoute::Can)
            .map_err(|_| ContractError::new("invalid_state", "CAN route is invalid")),
        None => Ok(ControllerRoute::Direct),
    }
}

fn decode_refloat_version(payload: &[u8]) -> Result<String, ContractError> {
    if payload.get(..3)
        != Some(&[
            VescCommand::CustomAppData as u8,
            protocol_refloat::PACKAGE_ID,
            protocol_refloat::RefloatCommand::Info as u8,
        ])
    {
        return Err(ContractError::new(
            "invalid_state",
            "Refloat identity is invalid",
        ));
    }
    let response_version = payload
        .get(3)
        .copied()
        .ok_or_else(|| ContractError::new("invalid_state", "Refloat identity is truncated"))?;
    if response_version == 10 {
        return Ok("1.0".into());
    }
    if response_version > 10 {
        return Ok(format!(
            "{}.{}",
            response_version / 10,
            response_version % 10
        ));
    }
    if response_version != 2 || payload.len() < 61 {
        return Err(ContractError::new(
            "invalid_state",
            "Refloat identity is truncated",
        ));
    }
    Ok(format!("{}.{}.{}", payload[25], payload[26], payload[27]))
}

fn session_sample(
    telemetry: &protocol_refloat::Telemetry,
    sequence: u64,
    observed_at_millis: i64,
    elapsed_millis: u64,
) -> Result<SessionSample, ContractError> {
    Ok(SessionSample {
        sequence,
        observed_at_millis,
        slow_observed_at_millis: None,
        monotonic_elapsed_millis: elapsed_millis,
        ride_state: ride_state_name(telemetry.ride_state).into(),
        package_state: Some(package_state_name(telemetry.package_state).into()),
        package_mode: Some(package_mode_name(telemetry.package_mode).into()),
        raw_pack_voltage_v: telemetry.numeric.get("packVoltageV").copied(),
        duty_cycle: telemetry.numeric.get("dutyCycle").copied(),
        speed_kph: telemetry.numeric.get("speedKph").copied(),
        extension_json: serde_json::to_string(&telemetry.extensions)
            .map_err(|_| ContractError::new("internal", "Telemetry encoding failed"))?,
    })
}

fn telemetry_value(
    value: &protocol_refloat::Telemetry,
    sequence: u64,
    observed_at_millis: i64,
    elapsed_millis: u64,
) -> Value {
    let mut sample = telemetry_template();
    for field in protocol_refloat::NORMALIZED_NUMERIC_FIELDS {
        sample[*field] = Value::Null;
    }
    sample["sequence"] = json!(sequence);
    sample["observedAt"] =
        json!(infra_sqlite_storage::format_iso8601(observed_at_millis).unwrap_or_default());
    sample["elapsedMs"] = json!(elapsed_millis);
    sample["rideState"] = json!(ride_state_name(value.ride_state));
    sample["packageState"] = json!(package_state_name(value.package_state));
    sample["packageMode"] = json!(package_mode_name(value.package_mode));
    sample["footpadState"] = json!(match value.footpad_state {
        protocol_refloat::FootpadState::None => "none",
        protocol_refloat::FootpadState::Left => "left",
        protocol_refloat::FootpadState::Right => "right",
        protocol_refloat::FootpadState::Both => "both",
        protocol_refloat::FootpadState::Unknown => "unknown",
    });
    sample["stopCondition"] = json!(match value.stop_condition {
        protocol_refloat::StopCondition::None => "none",
        protocol_refloat::StopCondition::Pitch => "pitch",
        protocol_refloat::StopCondition::Roll => "roll",
        protocol_refloat::StopCondition::SwitchHalf => "switchHalf",
        protocol_refloat::StopCondition::SwitchFull => "switchFull",
        protocol_refloat::StopCondition::ReverseStop => "reverseStop",
        protocol_refloat::StopCondition::QuickStop => "quickStop",
        protocol_refloat::StopCondition::Unknown => "unknown",
    });
    sample["setpointAdjustment"] = json!(match value.setpoint_adjustment {
        protocol_refloat::SetpointAdjustment::None => "none",
        protocol_refloat::SetpointAdjustment::Centering => "centering",
        protocol_refloat::SetpointAdjustment::ReverseStop => "reverseStop",
        protocol_refloat::SetpointAdjustment::Speed => "speed",
        protocol_refloat::SetpointAdjustment::Duty => "duty",
        protocol_refloat::SetpointAdjustment::Error => "error",
        protocol_refloat::SetpointAdjustment::HighVoltage => "highVoltage",
        protocol_refloat::SetpointAdjustment::LowVoltage => "lowVoltage",
        protocol_refloat::SetpointAdjustment::Temperature => "temperature",
        protocol_refloat::SetpointAdjustment::Unknown => "unknown",
    });
    sample["alertReason"] = json!(match value.alert_reason {
        protocol_refloat::AlertReason::None => "none",
        protocol_refloat::AlertReason::LowVoltage => "lowVoltage",
        protocol_refloat::AlertReason::HighVoltage => "highVoltage",
        protocol_refloat::AlertReason::ControllerTemperature => "controllerTemperature",
        protocol_refloat::AlertReason::MotorTemperature => "motorTemperature",
        protocol_refloat::AlertReason::Current => "current",
        protocol_refloat::AlertReason::Duty => "duty",
        protocol_refloat::AlertReason::Sensors => "sensors",
        protocol_refloat::AlertReason::LowBattery => "lowBattery",
        protocol_refloat::AlertReason::Idle => "idle",
        protocol_refloat::AlertReason::Error => "error",
        protocol_refloat::AlertReason::Speed => "speed",
        protocol_refloat::AlertReason::CellUnderTemperature => "cellUnderTemperature",
        protocol_refloat::AlertReason::CellOverTemperature => "cellOverTemperature",
        protocol_refloat::AlertReason::CellLowVoltage => "cellLowVoltage",
        protocol_refloat::AlertReason::CellHighVoltage => "cellHighVoltage",
        protocol_refloat::AlertReason::CellImbalance => "cellImbalance",
        protocol_refloat::AlertReason::BmsConnection => "bmsConnection",
        protocol_refloat::AlertReason::BmsOverTemperature => "bmsOverTemperature",
        protocol_refloat::AlertReason::FirmwareFault => "firmwareFault",
        protocol_refloat::AlertReason::Unknown => "unknown",
    });
    sample["wheelslip"] = json!(value.wheelslip);
    sample["darkride"] = json!(value.darkride);
    sample["charging"] = json!(value.charging);
    sample["firmwareFault"] = value.firmware_fault.map_or(Value::Null, |code| json!(code));
    for (name, number) in &value.numeric {
        sample[*name] = json!(number);
    }
    sample["extensions"] = Value::Object(
        value
            .extensions
            .iter()
            .map(|(name, number)| (name.clone(), json!(number)))
            .collect(),
    );
    sample
}

fn ride_state_name(value: domain_types::RideState) -> &'static str {
    match value {
        domain_types::RideState::Idle => "idle",
        domain_types::RideState::Riding => "riding",
        domain_types::RideState::Unknown => "unknown",
    }
}

fn package_state_name(value: protocol_refloat::PackageState) -> &'static str {
    match value {
        protocol_refloat::PackageState::Disabled => "disabled",
        protocol_refloat::PackageState::Startup => "startup",
        protocol_refloat::PackageState::Ready => "ready",
        protocol_refloat::PackageState::Running => "running",
        protocol_refloat::PackageState::Unknown => "unknown",
    }
}

fn package_mode_name(value: protocol_refloat::PackageMode) -> &'static str {
    match value {
        protocol_refloat::PackageMode::Normal => "normal",
        protocol_refloat::PackageMode::HandTest => "handTest",
        protocol_refloat::PackageMode::Flywheel => "flywheel",
        protocol_refloat::PackageMode::Unknown => "unknown",
    }
}

fn observe_controls(
    connection: &mut NativeConnection,
    lighting: Option<protocol_refloat::LightingState>,
) {
    let Some(telemetry) = connection.latest_telemetry.as_ref() else {
        return;
    };
    if let Some(controls) = connection.controls.as_mut() {
        controls.observe(
            telemetry.package_state,
            telemetry.package_mode,
            lighting,
            telemetry.fatal_error_latched,
            Some(telemetry.firmware_fault.is_some()),
        );
    }
}

fn apply_one_way_control(
    connection: &mut NativeConnection,
    control: protocol_refloat::BooleanControl,
    enabled: bool,
) {
    if let Some(telemetry) = connection.latest_telemetry.as_mut() {
        match control {
            protocol_refloat::BooleanControl::Package => {
                telemetry.package_state = if enabled {
                    protocol_refloat::PackageState::Ready
                } else {
                    protocol_refloat::PackageState::Disabled
                };
            }
            protocol_refloat::BooleanControl::Flywheel => {
                telemetry.package_state = if enabled {
                    protocol_refloat::PackageState::Running
                } else {
                    protocol_refloat::PackageState::Ready
                };
                telemetry.package_mode = if enabled {
                    protocol_refloat::PackageMode::Flywheel
                } else {
                    protocol_refloat::PackageMode::Normal
                };
            }
            protocol_refloat::BooleanControl::Leds
            | protocol_refloat::BooleanControl::Headlights => {}
        }
    }
    let lighting = connection
        .controls
        .as_ref()
        .and_then(|controls| controls.state.lighting);
    observe_controls(connection, lighting);
}

fn control_state_value(connection: &NativeConnection) -> Result<Value, ContractError> {
    let controls = connection
        .controls
        .as_ref()
        .ok_or_else(|| ContractError::new("invalid_state", "Controls are unavailable"))?;
    let version = connection
        .package_version
        .as_deref()
        .ok_or_else(|| ContractError::new("invalid_state", "Refloat version is unavailable"))?;
    let profile = protocol_refloat::compatibility(version)
        .map(protocol_refloat::CompatibilityEntry::control_profile)
        .ok_or_else(|| ContractError::new("invalid_state", "Refloat controls are unsupported"))?;
    Ok(json!({
        "status": "ready",
        "writable": controls.state.writable,
        "packageEnabled": controls.state.package_enabled,
        "flywheelEnabled": controls.state.flywheel_enabled,
        "lightingAvailable": controls.state.lighting.is_some(),
        "ledsEnabled": controls.state.lighting.map(|lighting| lighting.leds_enabled),
        "headlightsEnabled": controls.state.lighting.map(|lighting| lighting.headlights_enabled),
        "fatalErrorClearAvailable": profile.fatal_clear_available,
        "fatalErrorLatched": controls.state.fatal_latched,
        "fatalErrorActive": controls.state.fatal_active,
        "fatalErrorClearable": profile.fatal_clear_available
            && controls.state.fatal_latched == Some(true)
            && controls.state.fatal_active == Some(false),
        "pendingAction": controls.state.pending.map(pending_action_name)
    }))
}

fn pending_action_name(action: protocol_refloat::PendingAction) -> &'static str {
    match action {
        protocol_refloat::PendingAction::Package => "package",
        protocol_refloat::PendingAction::Flywheel => "flywheel",
        protocol_refloat::PendingAction::Leds => "leds",
        protocol_refloat::PendingAction::Headlights => "headlights",
        protocol_refloat::PendingAction::ClearFatal => "clearFatalError",
        protocol_refloat::PendingAction::Shutdown => "shutdown",
        protocol_refloat::PendingAction::Configuration => "configuration",
    }
}

fn initial_state() -> AppState {
    AppState::default()
}

fn document_path(path: &std::path::Path) -> Result<String, ContractError> {
    let canonical_parent = path
        .parent()
        .and_then(|parent| parent.canonicalize().ok())
        .ok_or_else(|| ContractError::new("internal", "Document staging root is unavailable"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| ContractError::new("internal", "Document staging path is invalid"))?;
    canonical_parent
        .join(file_name)
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| ContractError::new("internal", "Document staging path is invalid"))
}

fn document_operation_value(
    operation_id: &str,
    kind: DocumentKind,
    status: &str,
    error: Option<DocumentError>,
    export_display_name: Option<&str>,
    session_id: Option<&str>,
) -> DocumentOperationState {
    DocumentOperationState {
        operation_id: operation_id.into(),
        kind: match kind {
            DocumentKind::Import => "import",
            DocumentKind::Export => "export",
        }
        .into(),
        status: status.into(),
        error,
        export_display_name: export_display_name.map(str::to_owned),
        session_id: session_id.map(str::to_owned),
    }
}

fn document_error(code: &str, message: &str) -> DocumentError {
    DocumentError {
        code: code.into(),
        message: message.into(),
    }
}

fn storage_error_value(error: &StoreError) -> DocumentError {
    let mapped = storage_error(error);
    document_error(mapped.code, &mapped.message)
}

fn unix_millis() -> Result<i64, ContractError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .ok_or_else(|| ContractError::new("internal", "System clock is invalid"))
}

fn wall_elapsed_millis(started_at_millis: i64, observed_at_millis: i64) -> u64 {
    u64::try_from(observed_at_millis.saturating_sub(started_at_millis)).unwrap_or_default()
}

fn iso8601_or_empty(millis: i64) -> String {
    infra_sqlite_storage::format_iso8601(millis).unwrap_or_default()
}

fn storage_error(error: &StoreError) -> ContractError {
    match error {
        StoreError::Invalid => invalid("Stored data is invalid"),
        StoreError::NotFound => ContractError::new("not_found", "Stored data was not found"),
        StoreError::Conflict { current_revision } => ContractError {
            code: "conflict",
            message: "Stored revision changed".into(),
            details: Some(json!({ "currentRevision": current_revision })),
        },
        StoreError::Sqlite(_) | StoreError::Io(_) | StoreError::Poisoned => {
            ContractError::new("internal", "Storage operation failed")
        }
    }
}

fn load_durable_state(catalog: &CatalogStore, state: &mut AppState) -> Result<(), StoreError> {
    let defaults = settings::Settings::default();
    let settings = catalog.settings(&defaults.stored_json().map_err(|_| StoreError::Invalid)?)?;
    let settings = settings::Settings::from_stored(settings.revision, &settings.json)
        .ok_or(StoreError::Invalid)?;
    state.settings = settings;

    let records = catalog.session_records()?;
    state.saved_devices = catalog
        .saved_devices()?
        .iter()
        .map(|device| saved_device_value(device, &records))
        .collect();
    state.session_catalog_revision = u64::try_from(records.len()).unwrap_or(u64::MAX);
    Ok(())
}

fn populate_profile_completeness(
    catalog: &CatalogStore,
    state: &mut AppState,
) -> Result<(), StoreError> {
    for device in &mut state.saved_devices {
        let profile = load_profile(catalog, &device.id)?;
        device.profile_complete = profile.complete();
    }
    Ok(())
}

fn firmware_board_ids(uuid: FirmwareDeviceUuid) -> (String, String) {
    let canonical = uuid.canonical();
    (canonical.clone(), canonical)
}

fn canonical_firmware_uuid(uuid: Option<[u8; 12]>) -> Option<String> {
    uuid.and_then(|value| FirmwareDeviceUuid::new(value).ok())
        .map(FirmwareDeviceUuid::canonical)
}

fn fallback_board_ids(
    identity_nonce: [u8; 16],
    association: &PlatformAssociation,
) -> (String, String) {
    let mut identity = Vec::with_capacity(identity_nonce.len() + association.as_bytes().len());
    identity.extend_from_slice(&identity_nonce);
    identity.extend_from_slice(association.as_bytes());
    private_board_ids(b"vesc-unavailable-uuid", &identity)
}

fn private_board_ids(domain: &[u8], identity: &[u8]) -> (String, String) {
    (
        private_identifier(b"saved", domain, identity),
        private_identifier(b"history", domain, identity),
    )
}

fn private_identifier(kind: &[u8], domain: &[u8], identity: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"float-view-private-board-id-v1");
    hasher.update([0]);
    hasher.update(kind);
    hasher.update([0]);
    hasher.update(domain);
    hasher.update([0]);
    hasher.update(identity);
    format!(
        "{kind}-{:x}",
        hasher.finalize(),
        kind = String::from_utf8_lossy(kind)
    )
}

fn configuration_cache_identity(
    firmware: &FirmwareRecord,
    package_version: &str,
) -> Option<SchemaCacheIdentity> {
    Some(SchemaCacheIdentity {
        firmware_version: format!("{}.{:02}", firmware.major, firmware.minor),
        hardware_name: firmware.hardware_name.clone()?,
        package_version: package_version.to_owned(),
        custom_index: app_coordinator::CONFIGURATION_INDEX,
        hardware_configuration_crc: firmware.hardware_configuration_crc?,
    })
}

fn saved_device_value(
    device: &infra_sqlite_storage::SavedDevice,
    sessions: &[SessionCatalogRecord],
) -> SavedDevice {
    let owned = sessions
        .iter()
        .filter(|session| session.device_history_id == device.device_history_id)
        .collect::<Vec<_>>();
    let advertised = device.advertised_name.as_deref();
    let identifier = device
        .opaque_platform_id
        .as_ref()
        .map(|_| "Local VESC".to_owned());
    let display = device
        .custom_display_name
        .as_deref()
        .or(advertised)
        .or(identifier.as_deref())
        .unwrap_or("VESC");
    SavedDevice {
        id: device.saved_device_id.clone(),
        device_history_id: device.device_history_id.clone(),
        vesc_uuid: canonical_firmware_uuid(device.firmware_device_uuid),
        revision: device.revision,
        display_name: display.into(),
        custom_display_name: device.custom_display_name.clone(),
        advertised_name: device.advertised_name.clone(),
        bluetooth_identifier: identifier,
        last_seen_at: device
            .last_seen_at_millis
            .and_then(|value| infra_sqlite_storage::format_iso8601(value).ok()),
        last_connected_at: device
            .last_connected_at_millis
            .and_then(|value| infra_sqlite_storage::format_iso8601(value).ok()),
        profile_complete: false,
        summary: contract_state::BoardHistorySummary {
            session_count: owned.len(),
            most_recent_session_at: owned.first().and_then(|session| {
                infra_sqlite_storage::format_iso8601(session.started_at_millis).ok()
            }),
            total_riding_elapsed_ms: owned
                .iter()
                .map(|session| session.riding_elapsed_millis)
                .sum(),
            total_distance_m: owned
                .iter()
                .filter_map(|session| session.distance_m)
                .reduce(|left, right| left + right),
        },
    }
}

fn load_profile(
    catalog: &CatalogStore,
    saved_device_id: &str,
) -> Result<board_profile::BoardProfile, StoreError> {
    let profile = catalog.board_profile(saved_device_id)?;
    board_profile::BoardProfile::from_stored(profile).ok_or(StoreError::Invalid)
}

fn session_summary_from_record(record: &SessionCatalogRecord) -> SessionSummary {
    let device: SessionDeviceSnapshot = serde_json::from_str(&record.device_snapshot_json)
        .unwrap_or(SessionDeviceSnapshot {
            display_name: None,
            name: None,
        });
    SessionSummary {
        session_id: record.session_id.clone(),
        device_history_id: Some(record.device_history_id.clone()),
        display_name: device
            .display_name
            .or(device.name)
            .unwrap_or_else(|| "VESC".into()),
        started_at: infra_sqlite_storage::format_iso8601(record.started_at_millis)
            .unwrap_or_default(),
        ended_at: record
            .ended_at_millis
            .and_then(|value| infra_sqlite_storage::format_iso8601(value).ok()),
        state: record.state.clone(),
        integrity: match record.integrity.as_str() {
            "complete" => "complete",
            "recovered" => "recovered",
            "failed" => "corrupt",
            _ => "gaps",
        }
        .into(),
        sample_count: record.sample_count,
        event_count: record.event_count,
        riding_elapsed_ms: record.riding_elapsed_millis,
        total_elapsed_ms: record.total_elapsed_millis,
        distance_m: record.distance_m,
        max_speed_mps: record.maximum_speed_mps,
        max_duty_cycle: record.maximum_duty_cycle,
    }
}

fn active_connection(summary: Option<&SessionSummary>, ownership: &ConnectionOwnership) -> Value {
    let saved_device_id = ownership
        .saved()
        .map(|(saved_device_id, _)| saved_device_id);
    json!({
        "deviceId": null, "name": null, "displayName": null,
        "state": "ready", "lastSeenAt": null, "deviceKind": "board",
        "trackingMode": if ownership.tracking_mode() == TrackingMode::Saved { "saved" } else { "guest" },
        "savedDeviceId": saved_device_id,
        "controllerConnection": { "kind": "direct" }, "controllerCandidates": [], "connectionIssue": null,
        "bms": { "status": "not_found", "uuid": null, "connection": null, "candidates": [], "latestSnapshot": null },
        "refloatControls": {
            "status": "unavailable", "writable": false, "packageEnabled": null, "flywheelEnabled": null,
            "lightingAvailable": false, "ledsEnabled": null, "headlightsEnabled": null,
            "fatalErrorClearAvailable": false, "fatalErrorLatched": null, "fatalErrorActive": null,
            "fatalErrorClearable": false, "pendingAction": null
        },
        "refloatConfiguration": {
            "availability": "unavailable", "writable": false, "packageVersion": null,
            "schemaFingerprint": null, "wireSignature": null, "currentLoaded": false,
            "defaultLoaded": false, "pendingOperation": null
        },
        "controllerActions": { "shutdownAvailable": false, "pendingAction": null },
        "activeSession": summary, "latestSample": null, "latestLocation": null,
        "currentLimits": null,
        "firmware": { "uuid": null, "vescVersion": "6.06", "packageName": "refloat", "packageVersion": "1.2.3", "hardwareName": "Express", "accessMode": "readWrite", "supported": true }
    })
}

fn telemetry_template() -> Value {
    json!({
        "sequence": 0, "observedAt": "", "slowObservedAt": null,
        "elapsedMs": 300, "rideState": "riding", "packageState": "running", "packageMode": "normal",
        "footpadState": "both", "stopCondition": "none", "setpointAdjustment": "none",
        "wheelslip": false, "darkride": false, "charging": false, "alertReason": "none", "firmwareFault": null,
        "packVoltageV": 58.2, "dutyCycle": 0.42, "controllerTempC": 38.2, "motorTempC": 41.7,
        "motorCurrentA": 18.6, "batteryCurrentA": 8.4, "erpm": 3240, "speedKph": 24.3,
        "pitchDeg": 1.4, "balancePitchDeg": 1.1, "rollDeg": 0.6,
        "directionalMotorCurrentA": 17.9, "filteredMotorCurrentA": 17.4,
        "requestedBalanceCurrentA": 20.2, "boosterCurrentA": 1.2, "fieldWeakeningCurrentA": 0,
        "footpadAdc1V": 2.8, "footpadAdc2V": 2.75, "remoteInput": 0, "setpointDeg": 1,
        "atrSetpointDeg": 0.4, "brakeTiltSetpointDeg": 0, "torqueTiltSetpointDeg": 0.6,
        "turnTiltSetpointDeg": 0, "remoteSetpointDeg": 0,
        "atrAccelerationDifferenceErpmPerSecond": 180, "atrSpeedBoost": 0.05,
        "distanceM": 2.0, "odometerM": 12347.0, "ampHoursDrawnAh": 18.4,
        "ampHoursChargedAh": 2.7, "wattHoursDrawnWh": 812, "wattHoursChargedWh": 119,
        "batteryLevel": 0.68, "chargingCurrentA": 0, "chargingVoltageV": 0, "extensions": {}
    })
}

#[cfg(test)]
fn session_summary(id: &str, state: &str) -> SessionSummary {
    SessionSummary {
        session_id: id.into(),
        device_history_id: None,
        display_name: "VESC".into(),
        started_at: String::new(),
        ended_at: None,
        state: state.into(),
        integrity: "complete".into(),
        sample_count: 3,
        event_count: 0,
        riding_elapsed_ms: 300,
        total_elapsed_ms: 300,
        distance_m: Some(2.0),
        max_speed_mps: Some(6.75),
        max_duty_cycle: Some(0.42),
    }
}

fn session_metadata_from_record(record: &SessionCatalogRecord) -> Value {
    let summary = session_summary_from_record(record);
    json!({
        "summary": summary,
        "fields": [{ "name": "packVoltageV", "source": "stable" }],
        "eventCount": record.event_count,
        "recordedSampleCount": record.sample_count,
        "storedSampleCount": record.stored_sample_count,
        "prune": { "targetResolutionSeconds": null, "recordedSampleCount": record.sample_count, "storedSampleCount": record.stored_sample_count, "policy": null },
        "location": {
            "available": record.recorded_location_count > 0,
            "count": record.recorded_location_count,
            "lastSequence": record.committed_location_sequence,
            "lastElapsedMs": record.committed_location_elapsed_millis,
            "status": record.location_status,
        },
        "snapshot": {
            "historyRevision": record.history_revision,
            "committedSampleSequence": record.committed_sequence,
            "committedEventSequence": record.committed_event_sequence,
            "committedElapsedMs": record.committed_elapsed_millis,
            "committedLocationSequence": record.committed_location_sequence,
            "committedLocationElapsedMs": record.committed_location_elapsed_millis,
        }
    })
}

fn operation(
    id: &str,
    kind: &str,
    status: &str,
    session_id: Option<&str>,
    target: u64,
) -> SessionOperationState {
    SessionOperationState {
        operation_id: id.into(),
        kind: kind.into(),
        status: status.into(),
        session_id: session_id.map(str::to_owned),
        scope: if session_id.is_some() { "one" } else { "all" }.into(),
        target_resolution_seconds: Some(target),
        completed_sessions: 1,
        eligible_sessions: Some(1),
        error: None,
        retained_samples: None,
        reclaimable_bytes: None,
    }
}

fn accepted() -> Value {
    json!({ "accepted": true })
}

fn request_params<T: DeserializeOwned>(
    request: &ContractRequest,
    message: &str,
) -> Result<T, ContractError> {
    T::deserialize(&request.params).map_err(|_| invalid(message))
}

fn deserialize_present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn parse_cursor(cursor: Option<&str>) -> Result<usize, ContractError> {
    match cursor {
        None => Ok(0),
        Some(value) if !value.is_empty() && value.len() <= 32 => value
            .parse::<usize>()
            .map_err(|_| invalid("Cursor is invalid")),
        Some(_) => Err(invalid("Cursor is invalid")),
    }
}

fn require_saved<'a>(
    inner: &BackendState,
    saved_device_id: &'a str,
) -> Result<&'a str, ContractError> {
    let saved_device_id = bounded_identifier(saved_device_id, "Saved device ID")?;
    inner
        .state
        .saved_devices
        .iter()
        .any(|device| device.id == saved_device_id)
        .then_some(saved_device_id)
        .ok_or_else(|| ContractError::new("not_found", "Saved board was not found"))
}

fn find_session<'a>(
    inner: &'a BackendState,
    params: &PruneParams,
) -> Result<&'a SessionSummary, ContractError> {
    if !matches!(params.scope, PruneScope::One) {
        return Err(invalid("All-session pruning is unavailable"));
    }
    let id = params
        .session_id
        .as_deref()
        .ok_or_else(|| invalid("Session ID is required"))?;
    let id = bounded_identifier(id, "Session ID")?;
    find_session_by_id(inner, id)
}

fn find_session_by_id<'a>(
    inner: &'a BackendState,
    session_id: &str,
) -> Result<&'a SessionSummary, ContractError> {
    inner
        .sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .ok_or_else(|| ContractError::new("not_found", "Session was not found"))
}

fn prune_target(params: &PruneParams) -> Result<u64, ContractError> {
    (1..=300)
        .contains(&params.target_resolution_seconds)
        .then_some(params.target_resolution_seconds)
        .ok_or_else(|| invalid("Prune resolution is invalid"))
}

fn bounded_identifier<'a>(value: &'a str, name: &str) -> Result<&'a str, ContractError> {
    (!value.is_empty() && value.len() <= 128)
        .then_some(value)
        .ok_or_else(|| invalid(&format!("{name} is invalid")))
}

fn invalid(message: &str) -> ContractError {
    ContractError::new("invalid_request", message)
}

fn one_way_request_accepted(completions: &[RequestCompletion]) -> bool {
    // The scheduler returns no immediate completion when a request is valid but
    // queued behind an in-flight response. It publishes `Sent` when that queue
    // advances. A non-empty result here is therefore accepted only when it is
    // the immediate `Sent` completion; every other variant is a rejection.
    completions.is_empty()
        || completions
            .iter()
            .any(|completion| matches!(completion, RequestCompletion::Sent(_)))
}

#[cfg(test)]
mod composition_tests {
    use super::*;
    use app_coordinator::{DeterministicClock, DeterministicRandom, RuntimeGeneration};

    fn test_root(name: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("float-view-{name}-"))
            .tempdir()
            .expect("temporary directory")
    }

    #[test]
    fn required_nullable_fields_accept_explicit_null_but_reject_absence() {
        #[derive(Deserialize)]
        struct RequiredNullable {
            #[serde(deserialize_with = "deserialize_present")]
            value: Option<String>,
        }

        assert_eq!(
            serde_json::from_value::<RequiredNullable>(json!({ "value": null }))
                .expect("explicit null")
                .value,
            None
        );
        assert_eq!(
            serde_json::from_value::<RequiredNullable>(json!({ "value": "present" }))
                .expect("present string")
                .value
                .as_deref(),
            Some("present")
        );
        assert!(serde_json::from_value::<RequiredNullable>(json!({})).is_err());
    }

    #[test]
    fn reconnect_deadlines_are_monotonic_and_disable_cleanly() {
        assert_eq!(
            reconnect_due(ReconnectDecision::ScheduleAfter(500), 1_000),
            Some(1_500)
        );
        assert_eq!(
            reconnect_due(ReconnectDecision::ConnectNow, 1_000),
            Some(1_000)
        );
        assert_eq!(reconnect_due(ReconnectDecision::Disabled, 1_000), None);
        assert_eq!(reconnect_due(ReconnectDecision::TimedOut, 1_000), None);
    }

    #[test]
    fn one_way_requests_accept_scheduler_queueing_but_reject_failures() {
        assert!(one_way_request_accepted(&[]));
        assert!(!one_way_request_accepted(&[RequestCompletion::QueueFull]));
        assert!(!one_way_request_accepted(&[
            RequestCompletion::StaleGeneration
        ]));
    }

    #[test]
    fn connection_ownership_cannot_attach_durable_identity_to_guest_mode() {
        let guest = ConnectionOwnership::Guest;
        assert_eq!(guest.tracking_mode(), TrackingMode::Guest);
        assert_eq!(guest.saved(), None);

        let pending = ConnectionOwnership::PendingSaved;
        assert_eq!(pending.tracking_mode(), TrackingMode::Saved);
        assert_eq!(pending.saved(), None);

        let first = firmware_board_ids(FirmwareDeviceUuid::new([1; 12]).unwrap());
        let second = firmware_board_ids(FirmwareDeviceUuid::new([2; 12]).unwrap());
        assert_ne!(first, second);
        let saved = ConnectionOwnership::Saved {
            saved_device_id: first.0.clone(),
            device_history_id: first.1.clone(),
        };
        assert_eq!(saved.tracking_mode(), TrackingMode::Saved);
        assert_eq!(saved.saved(), Some((first.0.as_str(), first.1.as_str())));
    }

    #[test]
    fn firmware_identity_is_stable_distinct_and_canonical() {
        let uuid = FirmwareDeviceUuid::new([0xab; 12]).unwrap();
        let repeated = firmware_board_ids(uuid);
        assert_eq!(repeated, firmware_board_ids(uuid));
        assert_ne!(
            repeated,
            firmware_board_ids(FirmwareDeviceUuid::new([0xac; 12]).unwrap())
        );

        let raw_uuid = "ab".repeat(12);
        assert_eq!(repeated.0, raw_uuid);
        assert_eq!(repeated.1, raw_uuid);

        let association = PlatformAssociation::new(b"AA:BB:CC:DD:EE:FF".to_vec()).unwrap();
        let discovery = public_discovery_id([7; 16], &association);
        assert_eq!(discovery, public_discovery_id([7; 16], &association));
        assert_ne!(discovery, public_discovery_id([8; 16], &association));
        assert!(!discovery.contains("AA"));

        let fallback = fallback_board_ids([9; 16], &association);
        assert_ne!(fallback, fallback_board_ids([10; 16], &association));
        assert!(!fallback.0.contains("41413a"));
    }

    #[test]
    fn a_new_connection_does_not_reuse_a_stale_ready_generation() {
        let connection = NativeConnection {
            session_id: "test-session".into(),
            ownership: ConnectionOwnership::Saved {
                saved_device_id: SAVED_ID.into(),
                device_history_id: HISTORY_ID.into(),
            },
            started_at_millis: 0,
            device_id: String::new(),
            association: PlatformAssociation::new(vec![1]).unwrap(),
            identity_nonce: [1; 16],
            saved_firmware_uuid: Some(FirmwareDeviceUuid::new([1; 12]).unwrap()),
            name: None,
            stage: IdentificationStage::WaitingBluetooth,
            connect_after_generation: 4,
            vesc_generation: None,
            controller_can_id: None,
            pending_can_ids: Vec::new(),
            controller_inventory: Vec::new(),
            transport_firmware: None,
            firmware: None,
            package_version: None,
            telemetry_layout: None,
            latest_sample: None,
            telemetry_sequence: 0,
            last_telemetry_elapsed_millis: None,
            riding_elapsed_millis: 0,
            latest_telemetry: None,
            bms_status: DiscoveryStatus::Discovering,
            bms_connection: None,
            bms_candidates: Vec::new(),
            bms_snapshot: None,
            bms_pending_can_ids: Vec::new(),
            bms_probe_can_id: None,
            controls: None,
            telemetry_in_flight: false,
            telemetry_polling: None,
            next_telemetry_due_millis: None,
            pause_telemetry: false,
            shutdown_requested: false,
            reconnect: ReconnectModel::default(),
            reconnect_due_millis: None,
            handled_failure_generation: None,
            configuration_operation: None,
        };

        assert!(!ready_generation_is_fresh(
            true,
            4,
            connection.connect_after_generation
        ));
        assert!(ready_generation_is_fresh(
            true,
            5,
            connection.connect_after_generation
        ));
        assert!(!ready_generation_is_fresh(
            false,
            5,
            connection.connect_after_generation
        ));
    }

    #[test]
    fn startup_does_not_fabricate_a_saved_board() {
        let root = test_root("empty-startup");
        let catalog = Arc::new(CatalogStore::open(root.path()).expect("catalog"));
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        let backend = ContractBackend::new(catalog, sessions).expect("backend");
        let state = backend
            .handle(&ContractRequest {
                id: 1,
                method: "state.get".into(),
                params: json!({}),
            })
            .expect("state");
        assert_eq!(state["savedDevices"], json!([]));
        drop(backend);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn native_document_import_is_bounded_monotonic_and_cancelled_exactly_once() {
        let root = test_root("document-import");
        let catalog = Arc::new(CatalogStore::open(root.path()).expect("catalog"));
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        let application = ApplicationActor::start(
            RuntimeGeneration(9),
            DeterministicClock::new(0, 100, 1),
            DeterministicRandom::new(7),
        )
        .expect("actor");
        let backend = NativeBackend::new(Arc::clone(&application), Arc::clone(&catalog), sessions)
            .expect("backend");
        let presenting = backend
            .dispatch(ContractRequest {
                id: 1,
                method: "session.import".into(),
                params: json!({}),
            })
            .expect("present import");
        assert_eq!(presenting["kind"], "import");
        assert_eq!(presenting["status"], "presenting");
        let command = application
            .try_host_command()
            .expect("host channel")
            .expect("document command");
        let staging_path = match &command.command {
            HostCommand::PresentImport {
                staging_path,
                allowed_content_types,
                maximum_bytes,
            } => {
                assert_eq!(allowed_content_types, &["application/octet-stream"]);
                assert_eq!(*maximum_bytes, MAX_IMPORT_BYTES);
                PathBuf::from(staging_path)
            }
            other => panic!("unexpected command: {other:?}"),
        };
        assert!(staging_path.starts_with(root.path().canonicalize().expect("canonical root")));
        let conflict = backend.dispatch(ContractRequest {
            id: 2,
            method: "session.import".into(),
            params: json!({}),
        });
        assert!(matches!(
            conflict,
            Err(ContractError {
                code: "conflict",
                ..
            })
        ));
        application
            .complete(OperationCompletion {
                generation: command.generation,
                operation: command.operation,
                status: CompletionStatus::Cancelled,
                payload: Vec::new(),
            })
            .expect("cancel");
        let state = backend
            .dispatch(ContractRequest {
                id: 3,
                method: "state.get".into(),
                params: json!({}),
            })
            .expect("state");
        assert_eq!(state["documentOperation"]["status"], "cancelled");
        assert_eq!(
            state["documentOperation"]["operationId"],
            presenting["operationId"]
        );
        assert!(!staging_path.exists());

        backend
            .dispatch(ContractRequest {
                id: 4,
                method: "session.import".into(),
                params: json!({}),
            })
            .expect("second import");
        let expiring = application
            .try_host_command()
            .expect("host channel")
            .expect("expiring command");
        let expiring_path = match &expiring.command {
            HostCommand::PresentImport { staging_path, .. } => PathBuf::from(staging_path),
            other => panic!("unexpected command: {other:?}"),
        };
        std::fs::write(&expiring_path, [1, 2, 3]).expect("partial staging file");
        backend
            .documents
            .lock()
            .expect("documents")
            .pending
            .get_mut(&expiring.operation.0)
            .expect("pending")
            .created_at_millis = 0;
        let expired = backend
            .dispatch(ContractRequest {
                id: 5,
                method: "state.get".into(),
                params: json!({}),
            })
            .expect("expired state");
        assert_eq!(expired["documentOperation"]["status"], "failed");
        assert_eq!(expired["documentOperation"]["error"]["code"], "unavailable");
        assert!(!expiring_path.exists());
        assert_eq!(
            application.complete(OperationCompletion {
                generation: expiring.generation,
                operation: expiring.operation,
                status: CompletionStatus::Success,
                payload: Vec::new(),
            }),
            Err(app_coordinator::ActorError::DuplicateOperation)
        );
        application.stop().expect("stop");
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn native_document_round_trip_succeeds_and_cleans_staging() {
        let root = test_root("document-round-trip");
        let catalog = Arc::new(CatalogStore::open(root.path()).expect("catalog"));
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        sessions
            .create(&NewSession {
                session_id: "original",
                job_id: "job-original",
                device_history_id: "0102030405060708090a0b0c",
                device_snapshot_json: r#"{"displayName":"Test board","vescUuid":"0102030405060708090a0b0c"}"#,
                profile_snapshot_json: r#"{"battery":"18s"}"#,
                profile_revision: 1,
                started_at_millis: 100,
                requested_idle_millis: 1_000,
                effective_idle_millis: 1_000,
                requested_riding_millis: 100,
                effective_riding_millis: 100,
            })
            .expect("session");
        sessions
            .append_samples(
                "original",
                &[SessionSample {
                    sequence: 1,
                    observed_at_millis: 150,
                    slow_observed_at_millis: None,
                    monotonic_elapsed_millis: 50,
                    ride_state: "riding".into(),
                    package_state: Some("running".into()),
                    package_mode: Some("normal".into()),
                    raw_pack_voltage_v: Some(75.0),
                    duty_cycle: Some(0.2),
                    speed_kph: Some(12.0),
                    extension_json: "{}".into(),
                }],
                150,
            )
            .expect("sample");
        sessions.close("original", 200, "complete").expect("close");
        let archive = catalog
            .export_session(
                "original",
                "round-trip-source",
                infra_sqlite_storage::ExportFormat::FloatViewArchive,
            )
            .expect("source archive");
        let application = ApplicationActor::start(
            RuntimeGeneration(10),
            DeterministicClock::new(0, 100, 1),
            DeterministicRandom::new(8),
        )
        .expect("actor");
        let backend = NativeBackend::new(Arc::clone(&application), Arc::clone(&catalog), sessions)
            .expect("backend");

        backend
            .dispatch(ContractRequest {
                id: 1,
                method: "session.import".into(),
                params: json!({}),
            })
            .expect("present import");
        let import_command = application
            .try_host_command()
            .expect("host channel")
            .expect("import command");
        let import_path = match &import_command.command {
            HostCommand::PresentImport { staging_path, .. } => PathBuf::from(staging_path),
            other => panic!("unexpected command: {other:?}"),
        };
        std::fs::copy(&archive.path, &import_path).expect("bounded host copy");
        application
            .complete(OperationCompletion {
                generation: import_command.generation,
                operation: import_command.operation,
                status: CompletionStatus::Success,
                payload: Vec::new(),
            })
            .expect("complete import");
        let imported = backend
            .dispatch(ContractRequest {
                id: 2,
                method: "state.get".into(),
                params: json!({}),
            })
            .expect("imported state");
        assert_eq!(
            imported["documentOperation"]["status"], "completed",
            "{imported}"
        );
        assert!(imported["documentOperation"]["sessionId"].is_string());
        assert!(!import_path.exists());

        backend
            .dispatch(ContractRequest {
                id: 3,
                method: "session.export".into(),
                params: json!({ "sessionId": "original", "format": "csv" }),
            })
            .expect("present export");
        let export_command = application
            .try_host_command()
            .expect("host channel")
            .expect("export command");
        let (export_path, export_name) = match &export_command.command {
            HostCommand::PresentExport {
                staging_path,
                display_name,
                content_type,
            } => {
                assert!(display_name.starts_with("float-view-document-"));
                assert!(display_name.ends_with(".csv"));
                assert_eq!(content_type, "text/csv");
                (PathBuf::from(staging_path), display_name.clone())
            }
            other => panic!("unexpected command: {other:?}"),
        };
        assert!(export_path.is_file());
        application
            .complete(OperationCompletion {
                generation: export_command.generation,
                operation: export_command.operation,
                status: CompletionStatus::Success,
                payload: Vec::new(),
            })
            .expect("complete export");
        let exported = backend
            .dispatch(ContractRequest {
                id: 4,
                method: "state.get".into(),
                params: json!({}),
            })
            .expect("exported state");
        assert_eq!(exported["documentOperation"]["status"], "completed");
        assert_eq!(
            exported["documentOperation"]["exportDisplayName"],
            export_name
        );
        assert!(!export_path.exists());
        application.stop().expect("stop");
    }

    #[test]
    fn deleting_one_saved_board_survives_restart_without_touching_another() {
        let root = test_root("multiple-boards");
        let catalog = Arc::new(CatalogStore::open(root.path()).expect("catalog"));
        for (saved, history) in [("saved-a", "history-a"), ("saved-b", "history-b")] {
            catalog
                .ensure_saved_device(saved, history, None, Some(saved), Some(saved), 1)
                .expect("saved device");
        }
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        let backend = ContractBackend::new(Arc::clone(&catalog), sessions).expect("backend");
        backend
            .handle(&ContractRequest {
                id: 1,
                method: "savedDevice.delete".into(),
                params: json!({ "savedDeviceId": "saved-a" }),
            })
            .expect("delete");
        drop(backend);
        drop(catalog);

        let catalog = Arc::new(CatalogStore::open(root.path()).expect("reopened catalog"));
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        let reopened = ContractBackend::new(catalog, sessions).expect("reopened backend");
        let state = reopened
            .handle(&ContractRequest {
                id: 2,
                method: "state.get".into(),
                params: json!({}),
            })
            .expect("state");
        let devices = state["savedDevices"].as_array().expect("saved devices");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0]["savedDeviceId"], "saved-b");
        drop(reopened);
    }

    #[test]
    fn session_pages_are_board_scoped_and_snapshot_bound() {
        let root = test_root("session-cursor");
        let catalog = Arc::new(CatalogStore::open(root.path()).expect("catalog"));
        for (saved, history) in [("saved-a", "history-a"), ("saved-b", "history-b")] {
            catalog
                .ensure_saved_device(saved, history, None, Some(saved), Some(saved), 1)
                .expect("saved device");
        }
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        let backend = ContractBackend::new(catalog, sessions).expect("backend");
        let mut inner = backend.inner.lock().expect("state");
        inner.sessions = [
            ("a-1", "history-a"),
            ("b-1", "history-b"),
            ("a-2", "history-a"),
        ]
        .into_iter()
        .map(|(session_id, history_id)| {
            let mut summary = session_summary(session_id, "completed");
            summary.device_history_id = Some(history_id.into());
            summary
        })
        .collect();
        inner.state.session_catalog_revision = 7;
        let first = ContractBackend::session_list(
            &inner,
            &ContractRequest {
                id: 1,
                method: "session.list".into(),
                params: json!({ "savedDeviceId": "saved-a", "limit": 1 }),
            },
        )
        .expect("first page");
        assert_eq!(first["items"][0]["sessionId"], "a-1");
        assert_eq!(first["nextCursor"], "7:1");
        inner.state.session_catalog_revision = 8;
        let stale = ContractBackend::session_list(
            &inner,
            &ContractRequest {
                id: 2,
                method: "session.list".into(),
                params: json!({
                    "savedDeviceId": "saved-a",
                    "limit": 1,
                    "cursor": first["nextCursor"]
                }),
            },
        );
        assert!(matches!(
            stale,
            Err(ContractError {
                code: "conflict",
                ..
            })
        ));
        drop(inner);
        drop(backend);
    }

    #[test]
    fn contract_state_and_session_catalog_survive_reopen() {
        let root = test_root("contract-reopen");
        let catalog = Arc::new(CatalogStore::open(root.path()).expect("catalog"));
        catalog
            .ensure_saved_device(
                SAVED_ID,
                HISTORY_ID,
                Some([1; 12]),
                Some("test-device"),
                Some("Test Board"),
                1,
            )
            .expect("saved device");
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        let backend = ContractBackend::new(Arc::clone(&catalog), sessions).expect("backend");
        backend
            .handle(&ContractRequest {
                id: 1,
                method: "settings.update".into(),
                params: json!({ "expectedRevision": 0, "patch": { "themePreference": "dark" } }),
            })
            .expect("settings update");
        backend
            .handle(&ContractRequest {
                id: 2,
                method: "boardProfile.update".into(),
                params: json!({
                    "savedDeviceId": SAVED_ID,
                    "expectedRevision": 0,
                    "patch": { "motor": "superflux" }
                }),
            })
            .expect("profile update");
        backend
            .create_durable_session(
                "restart-session",
                HISTORY_ID,
                "Test Board",
                &load_profile(&catalog, SAVED_ID).expect("profile"),
                1,
                app_coordinator::SamplingSettings::default(),
            )
            .expect("session");
        drop(backend);
        drop(catalog);

        let catalog = Arc::new(CatalogStore::open(root.path()).expect("reopened catalog"));
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        let reopened = ContractBackend::new(catalog, sessions).expect("reopened backend");
        let state = reopened
            .handle(&ContractRequest {
                id: 3,
                method: "state.get".into(),
                params: json!({}),
            })
            .expect("state");
        assert_eq!(state["settings"]["themePreference"], "dark");
        let profile = reopened
            .handle(&ContractRequest {
                id: 4,
                method: "boardProfile.get".into(),
                params: json!({ "savedDeviceId": SAVED_ID }),
            })
            .expect("profile");
        assert_eq!(profile["values"]["motor"], "superflux");
        let session = reopened
            .handle(&ContractRequest {
                id: 5,
                method: "session.get".into(),
                params: json!({ "sessionId": "restart-session" }),
            })
            .expect("session");
        assert_eq!(session["summary"]["sessionId"], "restart-session");
        drop(reopened);
    }

    #[cfg(feature = "software-host")]
    #[tokio::test]
    async fn production_composition_reopens_durable_state_after_ordered_stop() {
        let temporary = tempfile::Builder::new()
            .prefix("float-view-composition-test-")
            .tempdir()
            .expect("temporary directory");
        let root = temporary.path().join("storage");
        for generation in 1..=2 {
            let runtime = BackendRuntime::start(BackendRuntimeConfig {
                generation,
                requested_port: 0,
                storage_root: root.clone(),
                platform: "macos".into(),
                launch_token: format!("composition-token-{generation}"),
                resources: standalone_resources(),
                application: None,
                transport: TransportMode::Software,
                platform_commands: None,
            })
            .await
            .unwrap();
            assert!(runtime.address().ip().is_loopback());
            assert!(root.join("catalog.sqlite").is_file());
            runtime.stop().await.unwrap();
        }
    }
}
