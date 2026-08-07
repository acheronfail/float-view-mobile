#![allow(clippy::too_many_lines)]
#![recursion_limit = "256"]

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use float_view_application::{
    ApplicationActor, DeterministicClock, DeterministicRandom, RequestCompletion, RequestKind,
    RequestPriority, RequestSpec, ServerPublication,
};
use float_view_bms::{BmsCandidate, BmsConnection, BmsSnapshot, DiscoveryStatus};
use float_view_domain::{FirmwareRecord, RuntimeGeneration};
use float_view_loopback::{
    ContractDispatcher, ContractError, ContractRequest, LoopbackServer, MemoryResources,
    ResourceProvider, ServerConfig, SessionHttpApi, StaticResource, StorageSessionHttpApi,
    start_server,
};
use float_view_platform_ports::{
    BluetoothAuthorization, BluetoothConnectionState, BluetoothRadio, HostCommand, PermissionKind,
};
use float_view_storage::{
    CatalogStore, NewSession, SessionCatalogRecord, SessionSample, SessionStore, StoreError,
};
use float_view_vesc::{
    ControllerRoute, ExpectedResponse, VescCommand, decode_firmware, encode_frame, route_command,
};
use serde_json::{Map, Value, json};

const SAVED_ID: &str = "saved-software-board";
const HISTORY_ID: &str = "history-software-board";
const DEVICE_ID: &str = "software-board";

/// Inputs selected by either deterministic conformance adapters or a native host bootstrap.
pub struct BackendRuntimeConfig {
    pub generation: u64,
    pub requested_port: u16,
    pub storage_root: PathBuf,
    pub platform: String,
    pub launch_token: String,
    pub resources: Arc<dyn ResourceProvider>,
    pub application: Option<Arc<ApplicationActor>>,
}

/// One production composition shared by standalone conformance and native hosts.
pub struct BackendRuntime {
    application: Arc<ApplicationActor>,
    server: Option<LoopbackServer>,
    platform: String,
    resources: Arc<dyn ResourceProvider>,
    dispatcher: Arc<dyn ContractDispatcher>,
    background_pump: Option<(Arc<AtomicBool>, JoinHandle<()>)>,
    session_http: Arc<dyn SessionHttpApi>,
    launch_token: String,
    requested_port: u16,
}

impl BackendRuntime {
    /// Opens durable stores, starts the actor/dispatcher, and binds authenticated loopback.
    ///
    /// # Errors
    /// Returns a bounded startup diagnostic if storage, actor, or loopback startup fails.
    pub async fn start(config: BackendRuntimeConfig) -> Result<Self, String> {
        let catalog = Arc::new(
            CatalogStore::open(&config.storage_root)
                .map_err(|error| format!("storage startup failed: {error:?}"))?,
        );
        let embedded_native_host = config.application.is_some();
        let sessions = Arc::new(SessionStore::new(Arc::clone(&catalog)));
        let application = if let Some(application) = config.application {
            application
        } else {
            ApplicationActor::start(
                RuntimeGeneration(config.generation),
                DeterministicClock::new(0, 0, 1),
                DeterministicRandom::new(config.generation),
            )
            .map_err(|error| format!("application startup failed: {error:?}"))?
        };
        let dispatcher: Arc<dyn ContractDispatcher> = if embedded_native_host {
            Arc::new(NativeBackend::new(
                Arc::clone(&application),
                Arc::clone(&catalog),
                Arc::clone(&sessions),
            )?)
        } else {
            Arc::new(StandaloneBackend::new(
                false,
                Arc::clone(&catalog),
                Arc::clone(&sessions),
            )?)
        };
        let session_http: Arc<dyn SessionHttpApi> =
            Arc::new(StorageSessionHttpApi::new(Arc::clone(&catalog)));
        let server = start_server(
            ServerConfig::new(
                "",
                &config.launch_token,
                &config.platform,
                Arc::clone(&config.resources),
                Arc::clone(&dispatcher),
            )
            .with_bind_port(config.requested_port)
            .with_session_http(Arc::clone(&session_http)),
        )
        .await
        .map_err(|error| format!("loopback startup failed: {error}"))?;
        let background_pump = if embedded_native_host {
            let stopped = Arc::new(AtomicBool::new(false));
            let worker_stopped = Arc::clone(&stopped);
            let worker_dispatcher = Arc::clone(&dispatcher);
            let worker = std::thread::Builder::new()
                .name("float-view-native-pump".into())
                .spawn(move || {
                    while !worker_stopped.load(Ordering::Acquire) {
                        worker_dispatcher.pump();
                        std::thread::sleep(Duration::from_millis(100));
                    }
                })
                .map_err(|error| format!("native pump startup failed: {error}"))?;
            Some((stopped, worker))
        } else {
            None
        };
        Ok(Self {
            application,
            server: Some(server),
            platform: config.platform,
            resources: config.resources,
            dispatcher,
            background_pump,
            session_http,
            launch_token: config.launch_token,
            requested_port: config.requested_port,
        })
    }

    #[must_use]
    /// Returns the currently bound loopback address.
    ///
    /// # Panics
    /// Panics only when called while the loopback portion is suspended.
    pub const fn address(&self) -> SocketAddr {
        self.server.as_ref().expect("running loopback").address
    }

    #[must_use]
    pub fn launch_token(&self) -> &str {
        &self.launch_token
    }

    #[must_use]
    pub fn application(&self) -> Arc<ApplicationActor> {
        Arc::clone(&self.application)
    }

    /// Stops loopback ingress while retaining the actor and durable owners.
    pub async fn suspend(&mut self) {
        if let Some(server) = self.server.take() {
            server.stop().await;
        }
    }

    /// Rebinds loopback while retaining actor and durable owners.
    ///
    /// # Errors
    /// Returns a bounded startup diagnostic when loopback cannot bind.
    pub async fn resume(&mut self) -> Result<SocketAddr, String> {
        if self.server.is_none() {
            self.server = Some(
                start_server(
                    ServerConfig::new(
                        "",
                        &self.launch_token,
                        &self.platform,
                        Arc::clone(&self.resources),
                        Arc::clone(&self.dispatcher),
                    )
                    .with_bind_port(self.requested_port)
                    .with_session_http(Arc::clone(&self.session_http)),
                )
                .await
                .map_err(|error| format!("loopback resume failed: {error}"))?,
            );
        }
        Ok(self.address())
    }

    /// Stops ingress before the actor and durable owners are released.
    ///
    /// # Errors
    /// Returns a bounded shutdown diagnostic if the application actor cannot stop cleanly.
    pub async fn stop(mut self) -> Result<(), String> {
        self.suspend().await;
        if let Some((stopped, worker)) = self.background_pump.take() {
            stopped.store(true, Ordering::Release);
            worker
                .join()
                .map_err(|_| "native pump shutdown failed".to_owned())?;
        }
        self.application
            .stop()
            .map_err(|error| format!("application shutdown failed: {error:?}"))
    }
}

/// Creates the deterministic storage root used only by the standalone launcher.
#[must_use]
pub fn standalone_storage_root() -> PathBuf {
    std::env::temp_dir().join(format!("float-view-standalone-{}", std::process::id()))
}

/// Creates deterministic packaged resources used by canonical standalone conformance.
#[must_use]
pub fn standalone_resources() -> Arc<dyn ResourceProvider> {
    resources_with_index(Arc::from(
        &b"<!doctype html><title>Float View Rust</title>"[..],
    ))
}

/// Builds the bounded frontend resource provider used by the production composition.
#[must_use]
pub fn resources_with_index(bytes: Arc<[u8]>) -> Arc<dyn ResourceProvider> {
    Arc::new(MemoryResources::new([(
        "index.html".into(),
        StaticResource {
            content_type: "text/html; charset=utf-8",
            bytes,
        },
    )]))
}

/// Removes only the process-scoped deterministic root after an orderly standalone stop.
pub fn cleanup_standalone_storage(root: &Path) {
    let _ = std::fs::remove_dir_all(root);
}

struct StandaloneBackend {
    inner: Mutex<BackendState>,
    update: Mutex<Option<Value>>,
    catalog: Arc<CatalogStore>,
    sessions: Arc<SessionStore>,
}

struct BackendState {
    state: Value,
    profile: Value,
    sessions: Vec<Value>,
    next_session: u64,
    next_operation: u64,
    location_count: u64,
    embedded_native_host: bool,
}

impl StandaloneBackend {
    fn new(
        embedded_native_host: bool,
        catalog: Arc<CatalogStore>,
        sessions: Arc<SessionStore>,
    ) -> Result<Self, String> {
        let now = unix_millis().map_err(|error| error.message)?;
        catalog
            .ensure_saved_device(
                SAVED_ID,
                HISTORY_ID,
                Some("software-association"),
                Some("Floatwheel Software"),
                now,
            )
            .map_err(|error| format!("saved-device startup failed: {error:?}"))?;
        let mut state = initial_state();
        load_durable_state(&catalog, &mut state)
            .map_err(|error| format!("durable-state startup failed: {error:?}"))?;
        let profile = load_profile(&catalog, SAVED_ID)
            .map_err(|error| format!("board-profile startup failed: {error:?}"))?;
        let profile_complete = profile["missingFields"]
            .as_array()
            .is_some_and(Vec::is_empty);
        if let Some(saved_device) = state["savedDevices"]
            .as_array_mut()
            .and_then(|devices| devices.first_mut())
        {
            saved_device["profileComplete"] = json!(profile_complete);
        }
        let durable_sessions = catalog
            .session_records()
            .map_err(|error| format!("session-catalog startup failed: {error:?}"))?
            .iter()
            .map(session_summary_from_record)
            .collect::<Vec<_>>();
        Ok(Self {
            inner: Mutex::new(BackendState {
                state,
                profile,
                next_session: u64::try_from(durable_sessions.len()).unwrap_or(u64::MAX) + 1,
                sessions: durable_sessions,
                next_operation: 1,
                location_count: 0,
                embedded_native_host,
            }),
            update: Mutex::new(None),
            catalog,
            sessions,
        })
    }

    fn changed(&self, inner: &mut BackendState, catalog: bool) {
        let revision = inner.state["revision"].as_u64().unwrap_or(0) + 1;
        inner.state["revision"] = json!(revision);
        if catalog {
            let revision = inner.state["sessionCatalogRevision"].as_u64().unwrap_or(0) + 1;
            inner.state["sessionCatalogRevision"] = json!(revision);
        }
        if let Ok(mut update) = self.update.lock() {
            *update = Some(inner.state.clone());
        }
    }

    fn handle(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
        let params = request
            .params
            .as_object()
            .ok_or_else(|| invalid("Params must be an object"))?;
        match request.method.as_str() {
            "state.get" => empty(params).map(|()| inner.state.clone()),
            "bluetooth.requestPermission" => {
                empty(params)?;
                Ok(inner.state["bluetooth"].clone())
            }
            "bluetooth.scan" => {
                if params.keys().any(|key| key != "timeoutMs") {
                    return Err(invalid("Scan parameters are invalid"));
                }
                inner.state["discoveredDevices"] = json!([discovered_device()]);
                self.changed(&mut inner, false);
                Ok(accepted())
            }
            "location.requestPermission" => {
                empty(params)?;
                let enabled = inner.state["settings"]["locationRecordingEnabled"] == true;
                inner.state["location"] = if enabled {
                    json!({ "status": "authorized", "permissionAction": null, "backgroundCapture": "available", "reason": null })
                } else {
                    json!({ "status": "disabled", "permissionAction": "requestPermission", "backgroundCapture": "unavailable", "reason": null })
                };
                let location = inner.state["location"].clone();
                self.changed(&mut inner, false);
                Ok(location)
            }
            "location.openSettings" => {
                empty(params)?;
                Ok(inner.state["location"].clone())
            }
            "device.connect" => self.connect(&mut inner, params),
            "device.disconnect" => self.disconnect(&mut inner, params),
            "device.selectController" => Err(ContractError::new(
                "invalid_state",
                "Controller selection is unavailable",
            )),
            "controller.shutdown" => {
                empty(params)?;
                Err(ContractError::new(
                    "invalid_state",
                    "Shutdown is unsafe while riding",
                ))
            }
            "refloat.configuration.open" => {
                empty(params)?;
                inner.state["activeConnection"]["refloatConfiguration"]
                    .as_object()
                    .map(|_| inner.state["activeConnection"]["refloatConfiguration"].clone())
                    .ok_or_else(|| {
                        ContractError::new("invalid_state", "Configuration is unavailable")
                    })
            }
            "refloat.fatalError.clear" | "refloat.control.set" => Err(ContractError::new(
                "invalid_state",
                "Refloat control is unavailable",
            )),
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
            "settings.update" => self.update_settings(&mut inner, params),
            "savedDevice.update" => self.update_saved_device(&mut inner, params),
            "savedDevice.delete" => self.delete_saved_device(&mut inner, params),
            "boardProfile.get" => {
                require_saved(params)?;
                Ok(inner.profile.clone())
            }
            "boardProfile.update" => self.update_profile(&mut inner, params),
            "session.list" => Self::session_list(&inner, params),
            "session.get" => self.session_get(params),
            "session.export" => {
                let session = find_session(&inner, params)?;
                let format = params.get("format").and_then(Value::as_str);
                if !matches!(format, Some("csv" | "float-view-archive")) {
                    return Err(invalid("Export format is invalid"));
                }
                Ok(json!({
                    "fileName": format!("{}.{}", session["sessionId"].as_str().unwrap_or("session"), if format == Some("csv") { "csv" } else { "floatview" }),
                    "delivery": "staged"
                }))
            }
            "session.delete" => {
                let id = text(params.get("sessionId"), "sessionId")?;
                self.catalog
                    .delete_session(id)
                    .map_err(|error| storage_error(&error))?;
                inner.sessions.retain(|session| session["sessionId"] != id);
                self.changed(&mut inner, true);
                Ok(accepted())
            }
            "session.delete.cancel" => Err(ContractError::new(
                "not_found",
                "Deletion operation was not found",
            )),
            "session.prune.preview" => Self::prune_preview(&mut inner, params),
            "session.prune.start" => self.prune(&mut inner, params),
            "session.prune.cancel" => Ok(operation(
                text(params.get("operationId"), "operationId")?,
                "prune",
                "completed",
                None,
                5,
            )),
            _ => Err(invalid("Unknown method")),
        }
    }

    fn connect(
        &self,
        inner: &mut BackendState,
        params: &Map<String, Value>,
    ) -> Result<Value, ContractError> {
        if params.len() != 2
            || text(params.get("deviceId"), "deviceId")? != DEVICE_ID
            || params.get("trackingMode").and_then(Value::as_str) != Some("saved")
        {
            return Err(invalid("Connection parameters are invalid"));
        }
        if !inner.state["activeConnection"].is_null() {
            return Err(ContractError::new(
                "invalid_state",
                "A board is already active",
            ));
        }
        if inner.state["discoveredDevices"]
            .as_array()
            .is_none_or(Vec::is_empty)
        {
            return Err(ContractError::new(
                "not_found",
                "Device is no longer available",
            ));
        }
        let session_id = format!("session-{}", inner.next_session);
        inner.next_session += 1;
        let summary = self.create_durable_session(&session_id, &inner.profile, true)?;
        inner.sessions.insert(0, summary.clone());
        inner.state["activeConnection"] = active_connection(&summary, inner.embedded_native_host);
        inner.state["discoveredDevices"] = json!([]);
        self.changed(inner, true);
        Ok(accepted())
    }

    fn disconnect(
        &self,
        inner: &mut BackendState,
        params: &Map<String, Value>,
    ) -> Result<Value, ContractError> {
        empty(params)?;
        let session_id = inner.state["activeConnection"]["activeSession"]["sessionId"]
            .as_str()
            .map(str::to_owned);
        if let Some(session_id) = session_id {
            let ended_at = unix_millis()?;
            self.sessions
                .close(&session_id, ended_at, "disconnect")
                .map_err(|error| storage_error(&error))?;
            let summary = self
                .catalog
                .session_record(&session_id)
                .map(|record| session_summary_from_record(&record))
                .map_err(|error| storage_error(&error))?;
            for session in &mut inner.sessions {
                if session["sessionId"] == session_id {
                    *session = summary.clone();
                }
            }
        }
        inner.state["activeConnection"] = Value::Null;
        self.changed(inner, true);
        Ok(accepted())
    }

    fn update_settings(
        &self,
        inner: &mut BackendState,
        params: &Map<String, Value>,
    ) -> Result<Value, ContractError> {
        let expected = params
            .get("expectedRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| invalid("Settings revision is required"))?;
        if inner.state["settings"]["revision"].as_u64() != Some(expected) {
            return Err(ContractError::new("conflict", "Settings revision changed"));
        }
        let patch = params
            .get("patch")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid("Settings patch is required"))?;
        let allowed: HashSet<&str> = [
            "idleProbeIntervalMs",
            "idlePersistIntervalMs",
            "ridingPersistIntervalMs",
            "postRideTailMs",
            "reconnectDurationMs",
            "minimumSessionDistance",
            "themePreference",
            "unitPreference",
            "historyGapThresholdSeconds",
            "locationRecordingEnabled",
            "includeGpsLocationWhenExporting",
        ]
        .into_iter()
        .collect();
        if patch.is_empty() || patch.keys().any(|key| !allowed.contains(key.as_str())) {
            return Err(invalid("Settings patch is invalid"));
        }
        let mut next_settings = inner.state["settings"].clone();
        for (key, value) in patch {
            next_settings[key] = value.clone();
        }
        let mut persisted = next_settings.clone();
        persisted
            .as_object_mut()
            .ok_or_else(|| ContractError::new("internal", "Settings state is invalid"))?
            .remove("revision");
        let stored = self
            .catalog
            .update_settings(
                expected,
                &serde_json::to_string(&persisted)
                    .map_err(|_| ContractError::new("internal", "Settings encoding failed"))?,
            )
            .map_err(|error| storage_error(&error))?;
        next_settings["revision"] = json!(stored.revision);
        inner.state["settings"] = next_settings;
        if let Some(enabled) = patch
            .get("locationRecordingEnabled")
            .and_then(Value::as_bool)
        {
            inner.state["location"] = if enabled {
                json!({ "status": "notDetermined", "permissionAction": "requestPermission", "backgroundCapture": "unavailable", "reason": null })
            } else {
                json!({ "status": "disabled", "permissionAction": "requestPermission", "backgroundCapture": "unavailable", "reason": null })
            };
        }
        let settings = inner.state["settings"].clone();
        self.changed(inner, false);
        Ok(settings)
    }

    fn update_saved_device(
        &self,
        inner: &mut BackendState,
        params: &Map<String, Value>,
    ) -> Result<Value, ContractError> {
        require_saved(params)?;
        let expected = params
            .get("expectedRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| invalid("Saved device revision is required"))?;
        let name = params
            .get("customDisplayName")
            .ok_or_else(|| invalid("Name is required"))?;
        if !name.is_null() && !name.is_string() {
            return Err(invalid("Name is invalid"));
        }
        let custom = name
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let stored = self
            .catalog
            .update_saved_device_name(SAVED_ID, expected, custom)
            .map_err(|error| storage_error(&error))?;
        let records = self
            .catalog
            .session_records()
            .map_err(|error| storage_error(&error))?;
        let result = saved_device_value(&stored, &records);
        let index = inner.state["savedDevices"]
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .position(|item| item["savedDeviceId"] == SAVED_ID)
            })
            .ok_or_else(|| ContractError::new("not_found", "Saved device was not found"))?;
        inner.state["savedDevices"][index] = result.clone();
        self.changed(inner, false);
        Ok(result)
    }

    fn update_profile(
        &self,
        inner: &mut BackendState,
        params: &Map<String, Value>,
    ) -> Result<Value, ContractError> {
        require_saved(params)?;
        let expected = params
            .get("expectedRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| invalid("Profile revision is required"))?;
        if inner.profile["revision"].as_u64() != Some(expected) {
            return Err(ContractError::new(
                "conflict",
                "Board Profile revision changed",
            ));
        }
        let patch = params
            .get("patch")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid("Profile patch is required"))?;
        if let Some(can_id) = patch.get("preferredCanId")
            && !can_id.is_null()
            && can_id.as_u64().is_none_or(|value| value > 253)
        {
            return Err(invalid("Preferred CAN ID is invalid"));
        }
        let mut next_profile = inner.profile.clone();
        for (key, value) in patch {
            next_profile["values"][key] = value.clone();
        }
        let mut missing = next_profile["missingFields"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        missing.retain(|field| !patch.contains_key(field.as_str().unwrap_or_default()));
        next_profile["missingFields"] = Value::Array(missing);
        let values = serde_json::to_string(&next_profile["values"])
            .map_err(|_| ContractError::new("internal", "Board Profile encoding failed"))?;
        let stored = self
            .catalog
            .update_board_profile(SAVED_ID, expected, &values, unix_millis()?)
            .map_err(|error| storage_error(&error))?;
        next_profile["revision"] = json!(stored.revision);
        let complete = next_profile["missingFields"]
            .as_array()
            .is_some_and(Vec::is_empty);
        inner.profile = next_profile;
        if let Some(saved_device) = inner.state["savedDevices"]
            .as_array_mut()
            .and_then(|devices| devices.first_mut())
        {
            saved_device["profileComplete"] = json!(complete);
        }
        let profile = inner.profile.clone();
        self.changed(inner, false);
        Ok(profile)
    }

    fn delete_saved_device(
        &self,
        inner: &mut BackendState,
        params: &Map<String, Value>,
    ) -> Result<Value, ContractError> {
        require_saved(params)?;
        if inner.state["activeConnection"]["savedDeviceId"] == SAVED_ID {
            return Err(ContractError::new(
                "invalid_state",
                "An active board cannot be deleted",
            ));
        }
        self.catalog
            .delete_saved_device(SAVED_ID)
            .map_err(|error| storage_error(&error))?;
        inner.state["savedDevices"] = json!([]);
        self.changed(inner, true);
        Ok(accepted())
    }

    fn create_durable_session(
        &self,
        session_id: &str,
        profile: &Value,
        seed_samples: bool,
    ) -> Result<Value, ContractError> {
        let started_at = unix_millis()?;
        let job_id = format!("job-{session_id}");
        let device_snapshot_json = serde_json::to_string(&json!({
            "displayName": "Floatwheel Software"
        }))
        .map_err(|_| ContractError::new("internal", "Session encoding failed"))?;
        let profile_snapshot_json = serde_json::to_string(profile)
            .map_err(|_| ContractError::new("internal", "Session encoding failed"))?;
        self.sessions
            .create(&NewSession {
                session_id,
                job_id: &job_id,
                device_history_id: HISTORY_ID,
                device_snapshot_json: &device_snapshot_json,
                profile_snapshot_json: &profile_snapshot_json,
                profile_revision: profile["revision"].as_u64().unwrap_or_default(),
                started_at_millis: started_at,
                requested_idle_millis: 1_000,
                effective_idle_millis: 1_000,
                requested_riding_millis: 100,
                effective_riding_millis: 100,
            })
            .map_err(|error| storage_error(&error))?;
        if seed_samples {
            let samples = [58.4, 58.3, 58.2]
                .into_iter()
                .enumerate()
                .map(|(index, voltage)| SessionSample {
                    sequence: u64::try_from(index).unwrap_or_default() + 1,
                    observed_at_millis: started_at + i64::try_from((index + 1) * 100).unwrap_or(0),
                    slow_observed_at_millis: None,
                    monotonic_elapsed_millis: u64::try_from((index + 1) * 100).unwrap_or_default(),
                    ride_state: "riding".into(),
                    package_state: Some("running".into()),
                    package_mode: Some("normal".into()),
                    raw_pack_voltage_v: Some(voltage),
                    duty_cycle: Some(
                        0.4 + f64::from(u32::try_from(index).unwrap_or_default()) * 0.01,
                    ),
                    speed_kph: Some(
                        24.0 + f64::from(u32::try_from(index).unwrap_or_default()) * 0.15,
                    ),
                    extension_json: "{}".into(),
                })
                .collect::<Vec<_>>();
            self.sessions
                .append_samples(session_id, &samples, started_at + 300)
                .and_then(|()| self.sessions.flush_catalog(session_id))
                .map_err(|error| storage_error(&error))?;
        }
        self.catalog
            .session_record(session_id)
            .map(|record| session_summary_from_record(&record))
            .map_err(|error| storage_error(&error))
    }

    fn session_list(
        inner: &BackendState,
        params: &Map<String, Value>,
    ) -> Result<Value, ContractError> {
        require_saved(params)?;
        let limit = usize::try_from(
            params
                .get("limit")
                .and_then(Value::as_u64)
                .filter(|value| (1..=100).contains(value))
                .ok_or_else(|| invalid("Session page limit is invalid"))?,
        )
        .map_err(|_| invalid("Session page limit is invalid"))?;
        let start = params
            .get("cursor")
            .and_then(Value::as_str)
            .map_or(Ok(0), |value| {
                value
                    .parse::<usize>()
                    .map_err(|_| invalid("Cursor is invalid"))
            })?;
        let items = inner
            .sessions
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next =
            (start + items.len() < inner.sessions.len()).then(|| (start + items.len()).to_string());
        Ok(json!({
            "items": items,
            "catalogRevision": inner.state["sessionCatalogRevision"],
            "nextCursor": next
        }))
    }

    fn session_get(&self, params: &Map<String, Value>) -> Result<Value, ContractError> {
        let session_id = text(params.get("sessionId"), "sessionId")?;
        let record = self
            .catalog
            .session_record(session_id)
            .map_err(|error| storage_error(&error))?;
        Ok(session_metadata_from_record(&record))
    }

    fn prune_preview(
        inner: &mut BackendState,
        params: &Map<String, Value>,
    ) -> Result<Value, ContractError> {
        let session_id = find_session(inner, params)?["sessionId"]
            .as_str()
            .map(str::to_owned);
        let target = prune_target(params)?;
        let id = format!("operation-{}", inner.next_operation);
        inner.next_operation += 1;
        Ok(json!({
            "operation": operation(&id, "prunePreview", "completed", session_id.as_deref(), target),
            "recordedSampleCount": 3,
            "estimatedStoredSampleCount": 3,
            "estimatedReclaimableBytes": 0,
            "skipped": false,
            "skipReason": null
        }))
    }

    fn prune(
        &self,
        inner: &mut BackendState,
        params: &Map<String, Value>,
    ) -> Result<Value, ContractError> {
        let session_id = find_session(inner, params)?["sessionId"]
            .as_str()
            .map(str::to_owned);
        let target = prune_target(params)?;
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
            .find(|session| session["sessionId"].as_str() == session_id.as_deref())
            && let Ok(record) = self
                .catalog
                .session_record(session_id.as_deref().unwrap_or_default())
        {
            *session = session_summary_from_record(&record);
        }
        let mut operation = operation(&id, "prune", "completed", session_id.as_deref(), target);
        operation["completedSessions"] = json!(1);
        operation["retainedSamples"] = json!(result.retained_samples);
        operation["reclaimableBytes"] = json!(0);
        Ok(operation)
    }
}

impl ContractDispatcher for StandaloneBackend {
    fn dispatch(&self, request: ContractRequest) -> Result<Value, ContractError> {
        self.handle(&request)
    }

    fn take_state_update(&self) -> Option<Value> {
        self.update.lock().ok()?.take()
    }
}

/// Contract adapter for native hosts. Product state remains owned by the shared
/// actor; this adapter only maps its bounded snapshot to the public wire shape.
struct NativeBackend {
    contract: StandaloneBackend,
    application: Arc<ApplicationActor>,
    actor_revision: Mutex<u64>,
    connection: Mutex<Option<NativeConnection>>,
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
    started_at_millis: i64,
    device_id: String,
    device_token: Vec<u8>,
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
    telemetry_layout: Option<float_view_refloat::TelemetryLayout>,
    latest_sample: Option<Value>,
    telemetry_sequence: u64,
    latest_telemetry: Option<float_view_refloat::Telemetry>,
    bms_status: DiscoveryStatus,
    bms_connection: Option<BmsConnection>,
    bms_candidates: Vec<BmsCandidate>,
    bms_snapshot: Option<BmsSnapshot>,
    bms_pending_can_ids: Vec<u8>,
    bms_probe_can_id: Option<u8>,
    controls: Option<float_view_refloat::MutationCoordinator>,
    telemetry_in_flight: bool,
    pause_telemetry: bool,
    shutdown_requested: bool,
    retry_count: u8,
    handled_failure_generation: Option<u64>,
}

impl NativeBackend {
    fn new(
        application: Arc<ApplicationActor>,
        catalog: Arc<CatalogStore>,
        sessions: Arc<SessionStore>,
    ) -> Result<Self, String> {
        Ok(Self {
            contract: StandaloneBackend::new(false, catalog, sessions)?,
            application,
            actor_revision: Mutex::new(u64::MAX),
            connection: Mutex::new(None),
        })
    }

    fn refresh(&self) -> Result<(), ContractError> {
        let connection_changed = self.process_publications()?;
        let snapshot = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?;
        let mut observed = self
            .actor_revision
            .lock()
            .map_err(|_| ContractError::new("internal", "Application state failed"))?;
        if *observed == snapshot.revision && !connection_changed {
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
        self.reconnect_if_needed(&snapshot)?;
        self.retry_connection_if_needed(&snapshot)?;
        self.begin_identification_if_ready(&snapshot)?;
        let mut active_connection = self.native_connection_value(snapshot.wall_millis)?;
        if let (Some(active), Some((latitude, longitude, accuracy))) =
            (active_connection.as_object_mut(), snapshot.latest_location)
        {
            active.insert(
                "latestLocation".into(),
                json!({
                    "observedAt": iso8601_or_empty(snapshot.wall_millis),
                    "latitude": f64::from_bits(latitude),
                    "longitude": f64::from_bits(longitude),
                    "accuracyM": accuracy.map(f64::from_bits)
                }),
            );
        }
        let native_summary = self.native_session_summary()?;
        let mut inner = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
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
        inner.state["bluetooth"] = json!({
            "radio": radio,
            "scanPermission": permission,
            "connectPermission": permission,
            "scanning": false
        });
        let location_enabled = inner.state["settings"]["locationRecordingEnabled"] == true;
        inner.state["location"] = if location_enabled {
            match snapshot.location_authorization {
                float_view_platform_ports::LocationAuthorization::Background => {
                    json!({ "status": "authorized", "permissionAction": null, "backgroundCapture": "available", "reason": null })
                }
                float_view_platform_ports::LocationAuthorization::Foreground => {
                    json!({ "status": "foregroundOnly", "permissionAction": "openSettings", "backgroundCapture": "foregroundOnly", "reason": null })
                }
                float_view_platform_ports::LocationAuthorization::Denied => {
                    json!({ "status": "denied", "permissionAction": "openSettings", "backgroundCapture": "unavailable", "reason": null })
                }
                float_view_platform_ports::LocationAuthorization::Unknown => {
                    json!({ "status": "notDetermined", "permissionAction": "requestPermission", "backgroundCapture": "unavailable", "reason": null })
                }
            }
        } else {
            json!({ "status": "disabled", "permissionAction": "requestPermission", "backgroundCapture": "unavailable", "reason": null })
        };
        inner.state["discoveredDevices"] = Value::Array(
            snapshot
                .discovered_devices
                .iter()
                .map(|device| {
                    json!({
                        "deviceId": encode_device_token(&device.device_token),
                        "name": device.name,
                        "state": "discovered",
                        "lastSeenAt": iso8601_or_empty(snapshot.wall_millis),
                        "rssi": device.rssi,
                        "savedDeviceId": null
                    })
                })
                .collect(),
        );
        if let Some(device) = snapshot.discovered_devices.first() {
            let device_id = encode_device_token(&device.device_token);
            let display_name = device.name.as_deref().unwrap_or("VESC");
            let saved = &mut inner.state["savedDevices"][0];
            saved["advertisedName"] = json!(device.name);
            saved["bluetoothIdentifier"] = json!(device_id);
            saved["displayName"] = json!(display_name);
        }
        inner.location_count = snapshot.location_sequence;
        if let Some(summary) = native_summary {
            if let Some(existing) = inner
                .sessions
                .iter_mut()
                .find(|session| session["sessionId"] == summary["sessionId"])
            {
                *existing = summary;
            } else {
                let session_id = summary["sessionId"]
                    .as_str()
                    .ok_or_else(|| ContractError::new("internal", "Session ID is unavailable"))?;
                if matches!(
                    self.contract.catalog.session_record(session_id),
                    Err(StoreError::NotFound)
                ) {
                    self.contract
                        .create_durable_session(session_id, &inner.profile, false)?;
                }
                inner.sessions.insert(0, summary);
            }
        }
        inner.state["activeConnection"] = active_connection;
        self.contract.changed(&mut inner, false);
        *observed = snapshot.revision;
        Ok(())
    }

    fn request_host(&self, command: HostCommand) -> Result<(), ContractError> {
        self.application
            .request_host(command)
            .map(|_| ())
            .map_err(|_| ContractError::new("unavailable", "Platform request is unavailable"))
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
            if let ServerPublication::VescCompleted(completion) = publication {
                self.accept_vesc_completion(completion)?;
                changed = true;
            }
        }
        Ok(changed)
    }

    fn begin_identification_if_ready(
        &self,
        snapshot: &float_view_application::ApplicationSnapshot,
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
        snapshot: &float_view_application::ApplicationSnapshot,
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
        if connection.shutdown_requested
            || matches!(
                connection.stage,
                IdentificationStage::WaitingBluetooth | IdentificationStage::Reconnecting
            )
        {
            return Ok(());
        }
        connection.stage = IdentificationStage::Reconnecting;
        connection.connect_after_generation = snapshot.connection_generation;
        connection.telemetry_in_flight = false;
        self.request_host(HostCommand::BluetoothConnect {
            device_token: connection.device_token.clone(),
        })
    }

    fn retry_connection_if_needed(
        &self,
        snapshot: &float_view_application::ApplicationSnapshot,
    ) -> Result<(), ContractError> {
        if !matches!(
            snapshot.bluetooth_connection,
            Some(BluetoothConnectionState::Failed)
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
        if !connection_stage_allows_retry(connection.stage)
            || connection.retry_count >= 2
            || connection.handled_failure_generation == Some(snapshot.connection_generation)
        {
            return Ok(());
        }
        connection.retry_count += 1;
        connection.handled_failure_generation = Some(snapshot.connection_generation);
        connection.connect_after_generation = snapshot.connection_generation;
        self.request_host(HostCommand::BluetoothConnect {
            device_token: connection.device_token.clone(),
        })
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
        conformance_trace(&format!(
            "vesc_response stage={:?} kind={:?}",
            connection.stage, request.kind
        ));
        match (connection.stage, request.kind) {
            (IdentificationStage::TransportFirmware, RequestKind::ControllerFirmware) => {
                let firmware = decode_firmware(&payload).map_err(|_| {
                    ContractError::new("invalid_state", "Firmware identity is invalid")
                })?;
                connection.transport_firmware = Some(firmware.clone());
                if float_view_bms::supports_firmware(&firmware) {
                    self.schedule_bms_values(connection, BmsConnection::Direct)?;
                } else if firmware.hardware_type == 0 {
                    connection.firmware = Some(firmware);
                    self.schedule_refloat_info(connection, ControllerRoute::Direct)?;
                } else if let Some(can_id) = self.preferred_can_id()? {
                    connection.controller_can_id = Some(can_id);
                    let route =
                        ControllerRoute::Can(float_view_domain::CanId::new(can_id).map_err(
                            |_| ContractError::new("invalid_state", "Preferred CAN ID is invalid"),
                        )?);
                    self.schedule(
                        connection,
                        IdentificationStage::ControllerFirmware,
                        &route_command(&[VescCommand::GetFirmwareVersion as u8], route),
                        ExpectedResponse::new(VescCommand::GetFirmwareVersion, None),
                        RequestKind::ControllerFirmware,
                    )?;
                } else {
                    self.schedule(
                        connection,
                        IdentificationStage::CanPing,
                        &[VescCommand::PingCan as u8],
                        ExpectedResponse::new(VescCommand::PingCan, None),
                        RequestKind::CanPing,
                    )?;
                }
            }
            (IdentificationStage::CanPing, RequestKind::CanPing) => {
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
                self.schedule_next_inventory_probe(connection)?;
            }
            (IdentificationStage::EnumeratingControllers, RequestKind::ControllerFirmware) => {
                let mut firmware = decode_firmware(&payload).map_err(|_| {
                    ContractError::new("invalid_state", "Firmware identity is invalid")
                })?;
                let can_id = connection
                    .controller_can_id
                    .and_then(|value| float_view_domain::CanId::new(value).ok())
                    .ok_or_else(|| {
                        ContractError::new("invalid_state", "CAN route is unavailable")
                    })?;
                firmware.can_id = Some(can_id);
                connection.controller_inventory.push(firmware);
                self.schedule_next_inventory_probe(connection)?;
            }
            (IdentificationStage::ControllerFirmware, RequestKind::ControllerFirmware) => {
                let mut firmware = decode_firmware(&payload).map_err(|_| {
                    ContractError::new("invalid_state", "Firmware identity is invalid")
                })?;
                let can_id = connection
                    .controller_can_id
                    .and_then(|value| float_view_domain::CanId::new(value).ok())
                    .ok_or_else(|| {
                        ContractError::new("invalid_state", "CAN route is unavailable")
                    })?;
                firmware.can_id = Some(can_id);
                connection.firmware = Some(firmware);
                self.schedule_refloat_info(connection, ControllerRoute::Can(can_id))?;
            }
            (IdentificationStage::RefloatInfo, RequestKind::RefloatInfo) => {
                let version = decode_refloat_version(&payload)?;
                let legacy = version == "1.0";
                connection.package_version = Some(version);
                connection.controls = Some(float_view_refloat::MutationCoordinator::new(
                    connection.vesc_generation.unwrap_or_default(),
                    connection
                        .firmware
                        .as_ref()
                        .is_some_and(|firmware| firmware.test_version == 0),
                ));
                if legacy {
                    self.begin_bms_discovery(connection)?;
                } else {
                    let route = connection_route(connection)?;
                    self.schedule(
                        connection,
                        IdentificationStage::RefloatLayout,
                        &route_command(
                            &float_view_refloat::request(
                                float_view_refloat::RefloatCommand::RealtimeDataInternalIds,
                                &[],
                            ),
                            route,
                        ),
                        ExpectedResponse::new(
                            VescCommand::CustomAppData,
                            Some(float_view_refloat::RefloatCommand::RealtimeDataInternalIds as u8),
                        ),
                        RequestKind::RefloatLayout,
                    )?;
                }
            }
            (IdentificationStage::RefloatLayout, RequestKind::RefloatLayout) => {
                connection.telemetry_layout =
                    Some(float_view_refloat::decode_layout(&payload).map_err(|_| {
                        ContractError::new("invalid_state", "Refloat layout is invalid")
                    })?);
                self.begin_bms_discovery(connection)?;
            }
            (IdentificationStage::BmsCanPing, RequestKind::CanPing) => {
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
                self.schedule_next_bms_probe(connection)?;
            }
            (IdentificationStage::EnumeratingBms, RequestKind::ControllerFirmware) => {
                let mut firmware = decode_firmware(&payload)
                    .map_err(|_| ContractError::new("invalid_state", "BMS identity is invalid"))?;
                conformance_trace(&format!(
                    "bms_identity_response supported={}",
                    float_view_bms::supports_firmware(&firmware)
                ));
                let can_id = connection
                    .bms_probe_can_id
                    .and_then(|value| float_view_domain::CanId::new(value).ok())
                    .ok_or_else(|| {
                        ContractError::new("invalid_state", "BMS route is unavailable")
                    })?;
                firmware.can_id = Some(can_id);
                if float_view_bms::supports_firmware(&firmware) {
                    connection.controller_inventory.push(firmware);
                }
                self.schedule_next_bms_probe(connection)?;
            }
            (IdentificationStage::BmsValues, RequestKind::BmsValues) => {
                let selected = connection.bms_connection.ok_or_else(|| {
                    ContractError::new("invalid_state", "BMS connection is unavailable")
                })?;
                let expected_can_id = match selected {
                    BmsConnection::Direct => None,
                    BmsConnection::Can(can_id) => Some(can_id),
                };
                let snapshot = float_view_bms::decode_values(&payload, expected_can_id)
                    .map_err(|_| ContractError::new("invalid_state", "BMS values are invalid"))?;
                self.application
                    .select_bms(selected)
                    .and_then(|accepted| {
                        if accepted {
                            Ok(())
                        } else {
                            Err(float_view_application::ActorError::StaleGeneration)
                        }
                    })
                    .map_err(|_| ContractError::new("invalid_state", "BMS selection is stale"))?;
                self.application
                    .accept_bms(snapshot.clone())
                    .and_then(|accepted| {
                        if accepted {
                            Ok(())
                        } else {
                            Err(float_view_application::ActorError::StaleGeneration)
                        }
                    })
                    .map_err(|_| ContractError::new("invalid_state", "BMS values are stale"))?;
                connection.bms_snapshot = Some(snapshot);
                connection.bms_status = DiscoveryStatus::Attached;
                if connection.firmware.is_some() {
                    self.schedule_telemetry(connection)?;
                } else {
                    connection.stage = IdentificationStage::ReadyBms;
                }
            }
            (IdentificationStage::Telemetry, RequestKind::RefloatControl) => {
                let version = connection.package_version.as_deref().ok_or_else(|| {
                    ContractError::new("invalid_state", "Refloat version is unavailable")
                })?;
                let profile = float_view_refloat::compatibility(version)
                    .map(float_view_refloat::CompatibilityEntry::control_profile)
                    .ok_or_else(|| {
                        ContractError::new("invalid_state", "Refloat controls are unsupported")
                    })?;
                let lighting =
                    float_view_refloat::decode_lighting(&payload, profile).map_err(|_| {
                        ContractError::new("invalid_state", "Refloat control response is invalid")
                    })?;
                observe_controls(connection, Some(lighting));
            }
            (IdentificationStage::Telemetry, RequestKind::RefloatTelemetry) => {
                connection.telemetry_in_flight = false;
                let telemetry = if let Some(layout) = connection.telemetry_layout.as_ref() {
                    float_view_refloat::decode_dynamic(&payload, layout)
                } else {
                    float_view_refloat::decode_legacy(&payload)
                }
                .map_err(|_| ContractError::new("invalid_state", "Telemetry is invalid"))?;
                connection.telemetry_sequence = connection.telemetry_sequence.saturating_add(1);
                self.application
                    .observe_telemetry(
                        telemetry.clone(),
                        connection.telemetry_sequence.saturating_mul(100),
                    )
                    .map_err(|_| {
                        ContractError::new("internal", "Telemetry could not be observed")
                    })?;
                let actor_snapshot = self.application.snapshot().map_err(|_| {
                    ContractError::new("internal", "Application state is unavailable")
                })?;
                let elapsed_millis = connection.telemetry_sequence.saturating_mul(100);
                let sample = SessionSample {
                    sequence: connection.telemetry_sequence,
                    observed_at_millis: actor_snapshot.wall_millis,
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
                };
                self.contract
                    .sessions
                    .append_samples(
                        &connection.session_id,
                        &[sample],
                        actor_snapshot.wall_millis,
                    )
                    .and_then(|()| self.contract.sessions.flush_catalog(&connection.session_id))
                    .map_err(|error| storage_error(&error))?;
                connection.latest_sample = Some(telemetry_value(
                    &telemetry,
                    connection.telemetry_sequence,
                    actor_snapshot.wall_millis,
                ));
                connection.latest_telemetry = Some(telemetry);
                let lighting = connection.controls.as_ref().and_then(|controls| {
                    controls
                        .state
                        .lighting
                        .or(Some(float_view_refloat::LightingState {
                            leds_enabled: true,
                            headlights_enabled: true,
                        }))
                });
                observe_controls(connection, lighting);
                if !connection.pause_telemetry {
                    self.schedule_telemetry(connection)?;
                }
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
                &float_view_refloat::request(float_view_refloat::RefloatCommand::Info, &[2, 0]),
                route,
            ),
            ExpectedResponse::new(
                VescCommand::CustomAppData,
                Some(float_view_refloat::RefloatCommand::Info as u8),
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
                float_view_domain::CanId::new(can_id)
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
                connection.controller_can_id = firmware.can_id.map(float_view_domain::CanId::get);
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

    fn preferred_can_id(&self) -> Result<Option<u8>, ContractError> {
        let inner = self
            .contract
            .inner
            .lock()
            .map_err(|_| ContractError::new("internal", "Board profile failed"))?;
        Ok(inner.profile["values"]["preferredCanId"]
            .as_u64()
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value <= 253))
    }

    fn begin_bms_discovery(&self, connection: &mut NativeConnection) -> Result<(), ContractError> {
        connection.bms_status = DiscoveryStatus::Discovering;
        let candidates: Vec<_> = connection
            .controller_inventory
            .iter()
            .filter(|firmware| float_view_bms::supports_firmware(firmware))
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
                float_view_domain::CanId::new(can_id)
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
            .filter(|firmware| float_view_bms::supports_firmware(firmware))
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
            .take(float_view_bms::MAX_CANDIDATES)
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
        let payload = encode_frame(&float_view_bms::values_request(selected))
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
                float_view_refloat::RefloatCommand::GetAllData,
                float_view_refloat::request(float_view_refloat::RefloatCommand::GetAllData, &[4]),
            )
        } else {
            (
                float_view_refloat::RefloatCommand::RealtimeDataInternal,
                float_view_refloat::request(
                    float_view_refloat::RefloatCommand::RealtimeDataInternal,
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

    fn native_connection_value(&self, observed_at_millis: i64) -> Result<Value, ContractError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let Some(connection) = connection.as_ref() else {
            return Ok(Value::Null);
        };
        let summary = native_session_summary(connection);
        let mut value = active_connection(&summary, false);
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
                            "hardwareName": firmware.hardware_name,
                            "firmwareName": firmware.firmware_name,
                            "vescVersion": format!("{}.{:02}", firmware.major, firmware.minor),
                            "vescSupported": float_view_vesc::firmware_access(firmware).is_some()
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
            .is_some_and(float_view_vesc::supports_shutdown)
            && connection
                .latest_telemetry
                .as_ref()
                .is_some_and(|telemetry| {
                    telemetry.ride_state == float_view_domain::RideState::Idle
                        && telemetry.package_mode == float_view_refloat::PackageMode::Normal
                });
        value["controllerActions"] = json!({
            "shutdownAvailable": shutdown_available,
            "pendingAction": null
        });
        if connection.stage == IdentificationStage::ReadyBms {
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
        if let (Some(controls), Some(version)) = (&connection.controls, &connection.package_version)
        {
            let profile = float_view_refloat::compatibility(version)
                .map(float_view_refloat::CompatibilityEntry::control_profile);
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
        Ok(value)
    }

    fn native_session_summary(&self) -> Result<Option<Value>, ContractError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        Ok(connection
            .as_ref()
            .filter(|connection| connection.firmware.is_some())
            .map(native_session_summary))
    }

    fn connect(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params = request
            .params
            .as_object()
            .ok_or_else(|| invalid("Params must be an object"))?;
        if params.len() != 2 || params.get("trackingMode").and_then(Value::as_str) != Some("saved")
        {
            return Err(invalid("Connection parameters are invalid"));
        }
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
        let id = text(params.get("deviceId"), "deviceId")?;
        let token = decode_device_token(id)?;
        let snapshot = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?;
        if !snapshot
            .discovered_devices
            .iter()
            .any(|device| device.device_token == token)
        {
            return Err(ContractError::new(
                "not_found",
                "Device is no longer available",
            ));
        }
        self.application
            .start_tracking(float_view_application::TrackingMode::Saved)
            .map_err(|_| ContractError::new("invalid_state", "Tracking could not start"))?;
        self.request_host(HostCommand::BluetoothStopScan)?;
        self.request_host(HostCommand::BluetoothConnect {
            device_token: token.clone(),
        })?;
        let discovered = snapshot
            .discovered_devices
            .iter()
            .find(|device| encode_device_token(&device.device_token) == id);
        *self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))? =
            Some(NativeConnection {
                session_id: format!("native-{}", snapshot.wall_millis.max(0)),
                started_at_millis: snapshot.wall_millis,
                device_id: id.to_owned(),
                device_token: token,
                name: discovered.and_then(|device| device.name.clone()),
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
                latest_telemetry: None,
                bms_status: DiscoveryStatus::Discovering,
                bms_connection: None,
                bms_candidates: Vec::new(),
                bms_snapshot: None,
                bms_pending_can_ids: Vec::new(),
                bms_probe_can_id: None,
                controls: None,
                telemetry_in_flight: false,
                pause_telemetry: false,
                shutdown_requested: false,
                retry_count: 0,
                handled_failure_generation: None,
            });
        Ok(accepted())
    }

    fn disconnect(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params = request
            .params
            .as_object()
            .ok_or_else(|| invalid("Params must be an object"))?;
        empty(params)?;
        let snapshot = self
            .application
            .snapshot()
            .map_err(|_| ContractError::new("internal", "Application state is unavailable"))?;
        self.request_host(HostCommand::BluetoothDisconnect {
            connection_generation: snapshot.connection_generation,
        })?;
        *self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))? = None;
        self.contract.handle(request)
    }

    fn select_controller(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params = request
            .params
            .as_object()
            .ok_or_else(|| invalid("Params must be an object"))?;
        if params.len() != 2 {
            return Err(invalid("Controller selection parameters are invalid"));
        }
        let device_id = text(params.get("deviceId"), "deviceId")?;
        let can_id = params
            .get("canId")
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value <= 253)
            .ok_or_else(|| invalid("CAN ID is invalid"))?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
        let connection = connection
            .as_mut()
            .filter(|connection| {
                connection.device_id == device_id
                    && connection.stage == IdentificationStage::Selecting
            })
            .ok_or_else(|| {
                ContractError::new("invalid_state", "Controller selection is unavailable")
            })?;
        let firmware = connection
            .controller_inventory
            .iter()
            .find(|firmware| {
                firmware.can_id.map(float_view_domain::CanId::get) == Some(can_id)
                    && firmware.hardware_type == 0
                    && float_view_vesc::firmware_access(firmware).is_some()
            })
            .cloned()
            .ok_or_else(|| ContractError::new("not_found", "Controller was not found"))?;
        connection.controller_can_id = Some(can_id);
        connection.firmware = Some(firmware);
        let route = ControllerRoute::Can(
            float_view_domain::CanId::new(can_id)
                .map_err(|_| ContractError::new("invalid_state", "CAN ID is invalid"))?,
        );
        self.schedule_refloat_info(connection, route)?;
        Ok(accepted())
    }

    fn set_control(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params = request
            .params
            .as_object()
            .ok_or_else(|| invalid("Params must be an object"))?;
        if params.len() != 2 {
            return Err(invalid("Control parameters are invalid"));
        }
        let control_name = text(params.get("control"), "control")?;
        let enabled = params
            .get("enabled")
            .and_then(Value::as_bool)
            .ok_or_else(|| invalid("Control value is required"))?;
        let control = match control_name {
            "package" => float_view_refloat::BooleanControl::Package,
            "flywheel" => float_view_refloat::BooleanControl::Flywheel,
            "leds" => float_view_refloat::BooleanControl::Leds,
            "headlights" => float_view_refloat::BooleanControl::Headlights,
            _ => return Err(invalid("Control is invalid")),
        };
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
                return Err(ContractError::new(
                    "unavailable",
                    "Telemetry did not yield for the control request",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let one_way = {
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
            let version = connection.package_version.as_deref().ok_or_else(|| {
                ContractError::new("invalid_state", "Refloat version is unavailable")
            })?;
            let profile = float_view_refloat::compatibility(version)
                .map(float_view_refloat::CompatibilityEntry::control_profile)
                .ok_or_else(|| {
                    ContractError::new("invalid_state", "Refloat controls are unsupported")
                })?;
            let context = float_view_refloat::MutationContext {
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
            let float_view_refloat::Authorization::Mutation {
                payload, one_way, ..
            } = authorization
            else {
                connection.pause_telemetry = false;
                if !connection.telemetry_in_flight {
                    self.schedule_telemetry(connection)?;
                }
                return control_state_value(connection);
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
                    payload: encode_frame(&route_command(&payload, route)).map_err(|_| {
                        ContractError::new("internal", "Control request is invalid")
                    })?,
                    expected: ExpectedResponse::new(VescCommand::CustomAppData, Some(command)),
                    priority: RequestPriority::ControllerIdentification,
                    kind: RequestKind::RefloatControl,
                    timeout_millis: 2_000,
                    generation,
                    expects_response: !one_way,
                })
                .map_err(|_| ContractError::new("unavailable", "Control request failed"))?;
            if one_way {
                if !completions
                    .iter()
                    .any(|completion| matches!(completion, RequestCompletion::Sent(_)))
                {
                    return Err(ContractError::new("unavailable", "Control was not sent"));
                }
                apply_one_way_control(connection, control, enabled);
                std::thread::sleep(Duration::from_millis(30));
            } else if !completions.is_empty() {
                return Err(ContractError::new("unavailable", "Control was rejected"));
            }
            one_way
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
        connection.pause_telemetry = false;
        if !connection.telemetry_in_flight {
            self.schedule_telemetry(connection)?;
        }
        control_state_value(connection)
    }

    fn clear_fatal(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params = request
            .params
            .as_object()
            .ok_or_else(|| invalid("Params must be an object"))?;
        empty(params)?;
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
                return Err(ContractError::new(
                    "unavailable",
                    "Telemetry did not yield for the control request",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
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
        let profile = float_view_refloat::compatibility(version)
            .map(float_view_refloat::CompatibilityEntry::control_profile)
            .ok_or_else(|| {
                ContractError::new("invalid_state", "Refloat controls are unsupported")
            })?;
        let context = float_view_refloat::MutationContext {
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
                    Some(float_view_refloat::RefloatCommand::AlertsControl as u8),
                ),
                priority: RequestPriority::ControllerIdentification,
                kind: RequestKind::RefloatControl,
                timeout_millis: 2_000,
                generation: connection.vesc_generation.unwrap_or_default(),
                expects_response: false,
            })
            .map_err(|_| ContractError::new("unavailable", "Control request failed"))?;
        if !completions
            .iter()
            .any(|completion| matches!(completion, RequestCompletion::Sent(_)))
        {
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
        connection.pause_telemetry = false;
        if !connection.telemetry_in_flight {
            self.schedule_telemetry(connection)?;
        }
        control_state_value(connection)
    }

    fn shutdown_controller(&self, request: &ContractRequest) -> Result<Value, ContractError> {
        let params = request
            .params
            .as_object()
            .ok_or_else(|| invalid("Params must be an object"))?;
        empty(params)?;
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
                return Err(ContractError::new(
                    "unavailable",
                    "Telemetry did not yield for shutdown",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let starting_sequence =
            {
                let mut connection = self
                    .connection
                    .lock()
                    .map_err(|_| ContractError::new("internal", "Connection state failed"))?;
                let connection = connection
                    .as_mut()
                    .ok_or_else(|| ContractError::new("invalid_state", "No board is active"))?;
                let firmware = connection.firmware.as_ref().ok_or_else(|| {
                    ContractError::new("invalid_state", "Firmware is unavailable")
                })?;
                let telemetry = connection.latest_telemetry.as_ref().ok_or_else(|| {
                    ContractError::new("invalid_state", "Telemetry is unavailable")
                })?;
                if !float_view_vesc::supports_shutdown(firmware)
                    || telemetry.ride_state != float_view_domain::RideState::Idle
                    || telemetry.package_mode != float_view_refloat::PackageMode::Normal
                {
                    return Err(ContractError::new("invalid_state", "Shutdown is unsafe"));
                }
                let route = connection_route(connection)?;
                let completions = self
                    .application
                    .schedule_vesc(RequestSpec {
                        payload: encode_frame(&float_view_vesc::shutdown_command(route)).map_err(
                            |_| ContractError::new("internal", "Shutdown request is invalid"),
                        )?,
                        expected: ExpectedResponse::new(VescCommand::Shutdown, None),
                        priority: RequestPriority::ControllerIdentification,
                        kind: RequestKind::ControllerShutdown,
                        timeout_millis: 2_000,
                        generation: connection.vesc_generation.unwrap_or_default(),
                        expects_response: false,
                    })
                    .map_err(|_| ContractError::new("unavailable", "Shutdown request failed"))?;
                if !completions
                    .iter()
                    .any(|completion| matches!(completion, RequestCompletion::Sent(_)))
                {
                    return Err(ContractError::new("unavailable", "Shutdown was not sent"));
                }
                connection.shutdown_requested = true;
                let sequence = connection.telemetry_sequence;
                self.schedule_telemetry(connection)?;
                sequence
            };
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
                    inner.state["activeConnection"] = Value::Null;
                    self.contract.changed(&mut inner, false);
                    break;
                }
            }
        }
        Ok(accepted())
    }
}

const fn connection_stage_allows_retry(stage: IdentificationStage) -> bool {
    matches!(
        stage,
        IdentificationStage::WaitingBluetooth | IdentificationStage::Reconnecting
    )
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
            "bluetooth.requestPermission" => {
                let params = request
                    .params
                    .as_object()
                    .ok_or_else(|| invalid("Params must be an object"))?;
                empty(params)?;
                self.request_host(HostCommand::PresentPermission {
                    permission: PermissionKind::Bluetooth,
                })?;
                self.refresh()?;
                let inner = self
                    .contract
                    .inner
                    .lock()
                    .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
                Ok(inner.state["bluetooth"].clone())
            }
            "bluetooth.scan" => {
                let params = request
                    .params
                    .as_object()
                    .ok_or_else(|| invalid("Params must be an object"))?;
                if params.keys().any(|key| key != "timeoutMs") {
                    return Err(invalid("Scan parameters are invalid"));
                }
                let timeout_millis = params
                    .get("timeoutMs")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .min(30_000);
                self.request_host(HostCommand::BluetoothStartScan)?;
                let deadline = Instant::now() + Duration::from_millis(timeout_millis);
                loop {
                    self.refresh()?;
                    let found = !self
                        .application
                        .snapshot()
                        .map_err(|_| {
                            ContractError::new("internal", "Application state is unavailable")
                        })?
                        .discovered_devices
                        .is_empty();
                    if found || Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(accepted())
            }
            "location.requestPermission" => {
                let params = request
                    .params
                    .as_object()
                    .ok_or_else(|| invalid("Params must be an object"))?;
                empty(params)?;
                self.request_host(HostCommand::PresentPermission {
                    permission: PermissionKind::Location,
                })?;
                self.request_host(HostCommand::LocationStart { background: true })?;
                self.request_host(HostCommand::SetBackgroundLogging { enabled: true })?;
                self.refresh()?;
                let inner = self
                    .contract
                    .inner
                    .lock()
                    .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
                Ok(inner.state["location"].clone())
            }
            "location.openSettings" => {
                let params = request
                    .params
                    .as_object()
                    .ok_or_else(|| invalid("Params must be an object"))?;
                empty(params)?;
                self.request_host(HostCommand::PresentSettings)?;
                self.request_host(HostCommand::LocationStart { background: true })?;
                self.refresh()?;
                let inner = self
                    .contract
                    .inner
                    .lock()
                    .map_err(|_| ContractError::new("internal", "Backend state failed"))?;
                Ok(inner.state["location"].clone())
            }
            "settings.update" => {
                let location_enabled = request
                    .params
                    .as_object()
                    .and_then(|params| params.get("patch"))
                    .and_then(Value::as_object)
                    .and_then(|patch| patch.get("locationRecordingEnabled"))
                    .and_then(Value::as_bool);
                let result = self.contract.handle(&request)?;
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
            "device.connect" => self.connect(&request),
            "device.selectController" => self.select_controller(&request),
            "device.disconnect" => self.disconnect(&request),
            "refloat.control.set" => self.set_control(&request),
            "refloat.fatalError.clear" => self.clear_fatal(&request),
            "controller.shutdown" => self.shutdown_controller(&request),
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
}

fn encode_device_token(token: &[u8]) -> String {
    let mut value = String::with_capacity(token.len() * 2);
    for byte in token {
        use std::fmt::Write as _;
        let _ = write!(&mut value, "{byte:02x}");
    }
    value
}

fn decode_device_token(value: &str) -> Result<Vec<u8>, ContractError> {
    if value.is_empty() || value.len() > 512 || !value.len().is_multiple_of(2) {
        return Err(invalid("Device ID is invalid"));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| invalid("Device ID is invalid"))?;
            u8::from_str_radix(text, 16).map_err(|_| invalid("Device ID is invalid"))
        })
        .collect()
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
    connection.controls = None;
    connection.telemetry_in_flight = false;
    connection.pause_telemetry = false;
    connection.bms_status = DiscoveryStatus::Discovering;
    connection.bms_connection = None;
    connection.bms_candidates.clear();
    connection.bms_snapshot = None;
    connection.bms_pending_can_ids.clear();
    connection.bms_probe_can_id = None;
}

fn native_session_summary(connection: &NativeConnection) -> Value {
    let mut summary = session_summary(&connection.session_id, "active");
    summary["startedAt"] = json!(iso8601_or_empty(connection.started_at_millis));
    let sample_count = connection.telemetry_sequence;
    summary["sampleCount"] = json!(sample_count);
    summary["totalElapsedMs"] = json!(sample_count.saturating_mul(100));
    summary["ridingElapsedMs"] = json!(sample_count.saturating_mul(100));
    if let Some(distance) = connection
        .latest_sample
        .as_ref()
        .and_then(|sample| sample["distanceM"].as_f64())
    {
        summary["distanceM"] = json!(distance);
    }
    summary
}

fn bms_state_value(connection: &NativeConnection, observed_at_millis: i64) -> Value {
    let status = match connection.bms_status {
        DiscoveryStatus::Discovering => "discovering",
        DiscoveryStatus::NotFound => "not_found",
        DiscoveryStatus::Attached => "attached",
        DiscoveryStatus::Unavailable => "unavailable",
        DiscoveryStatus::Ambiguous => "ambiguous",
    };
    json!({
        "status": status,
        "connection": connection.bms_connection.map(bms_connection_value),
        "candidates": connection.bms_candidates.iter().map(|candidate| json!({
            "connection": bms_connection_value(candidate.connection),
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
        Some(can_id) => float_view_domain::CanId::new(can_id)
            .map(ControllerRoute::Can)
            .map_err(|_| ContractError::new("invalid_state", "CAN route is invalid")),
        None => Ok(ControllerRoute::Direct),
    }
}

fn decode_refloat_version(payload: &[u8]) -> Result<String, ContractError> {
    if payload.get(..3)
        != Some(&[
            VescCommand::CustomAppData as u8,
            float_view_refloat::PACKAGE_ID,
            float_view_refloat::RefloatCommand::Info as u8,
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

fn telemetry_value(
    value: &float_view_refloat::Telemetry,
    sequence: u64,
    observed_at_millis: i64,
) -> Value {
    let mut sample = riding_sample();
    sample["sequence"] = json!(sequence);
    sample["observedAt"] =
        json!(float_view_storage::format_iso8601(observed_at_millis).unwrap_or_default());
    sample["elapsedMs"] = json!(sequence.saturating_mul(100));
    sample["rideState"] = json!(ride_state_name(value.ride_state));
    sample["packageState"] = json!(package_state_name(value.package_state));
    sample["packageMode"] = json!(package_mode_name(value.package_mode));
    sample["footpadState"] = json!(match value.footpad_state {
        float_view_refloat::FootpadState::None => "none",
        float_view_refloat::FootpadState::Left => "left",
        float_view_refloat::FootpadState::Right => "right",
        float_view_refloat::FootpadState::Both => "both",
        float_view_refloat::FootpadState::Unknown => "unknown",
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

fn ride_state_name(value: float_view_domain::RideState) -> &'static str {
    match value {
        float_view_domain::RideState::Idle => "idle",
        float_view_domain::RideState::Riding => "riding",
        float_view_domain::RideState::Unknown => "unknown",
    }
}

fn package_state_name(value: float_view_refloat::PackageState) -> &'static str {
    match value {
        float_view_refloat::PackageState::Disabled => "disabled",
        float_view_refloat::PackageState::Startup => "startup",
        float_view_refloat::PackageState::Ready => "ready",
        float_view_refloat::PackageState::Running => "running",
        float_view_refloat::PackageState::Unknown => "unknown",
    }
}

fn package_mode_name(value: float_view_refloat::PackageMode) -> &'static str {
    match value {
        float_view_refloat::PackageMode::Normal => "normal",
        float_view_refloat::PackageMode::HandTest => "handTest",
        float_view_refloat::PackageMode::Flywheel => "flywheel",
        float_view_refloat::PackageMode::Unknown => "unknown",
    }
}

fn observe_controls(
    connection: &mut NativeConnection,
    lighting: Option<float_view_refloat::LightingState>,
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
    control: float_view_refloat::BooleanControl,
    enabled: bool,
) {
    if let Some(telemetry) = connection.latest_telemetry.as_mut() {
        match control {
            float_view_refloat::BooleanControl::Package => {
                telemetry.package_state = if enabled {
                    float_view_refloat::PackageState::Ready
                } else {
                    float_view_refloat::PackageState::Disabled
                };
            }
            float_view_refloat::BooleanControl::Flywheel => {
                telemetry.package_state = if enabled {
                    float_view_refloat::PackageState::Running
                } else {
                    float_view_refloat::PackageState::Ready
                };
                telemetry.package_mode = if enabled {
                    float_view_refloat::PackageMode::Flywheel
                } else {
                    float_view_refloat::PackageMode::Normal
                };
            }
            float_view_refloat::BooleanControl::Leds
            | float_view_refloat::BooleanControl::Headlights => {}
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
    let profile = float_view_refloat::compatibility(version)
        .map(float_view_refloat::CompatibilityEntry::control_profile)
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

fn pending_action_name(action: float_view_refloat::PendingAction) -> &'static str {
    match action {
        float_view_refloat::PendingAction::Package => "package",
        float_view_refloat::PendingAction::Flywheel => "flywheel",
        float_view_refloat::PendingAction::Leds => "leds",
        float_view_refloat::PendingAction::Headlights => "headlights",
        float_view_refloat::PendingAction::ClearFatal => "clearFatalError",
        float_view_refloat::PendingAction::Shutdown => "shutdown",
        float_view_refloat::PendingAction::Configuration => "configuration",
    }
}

fn initial_state() -> Value {
    json!({
        "revision": 0,
        "bluetooth": { "radio": "powered_on", "scanPermission": "granted", "connectPermission": "granted", "scanning": false },
        "discoveredDevices": [],
        "savedDevices": [saved_device()],
        "activeConnection": null,
        "sessionCatalogRevision": 0,
        "settings": {
            "revision": 0, "idleProbeIntervalMs": 500, "idlePersistIntervalMs": 1000,
            "ridingPersistIntervalMs": 100, "postRideTailMs": 5000, "reconnectDurationMs": 30000,
            "minimumSessionDistance": 10, "themePreference": "system", "unitPreference": "metric",
            "historyGapThresholdSeconds": null, "locationRecordingEnabled": false,
            "includeGpsLocationWhenExporting": false
        },
        "location": { "status": "disabled", "permissionAction": "requestPermission", "backgroundCapture": "unavailable", "reason": null },
        "sessionOperation": null
    })
}

fn unix_millis() -> Result<i64, ContractError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .ok_or_else(|| ContractError::new("internal", "System clock is invalid"))
}

fn iso8601_or_empty(millis: i64) -> String {
    float_view_storage::format_iso8601(millis).unwrap_or_default()
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

fn load_durable_state(catalog: &CatalogStore, state: &mut Value) -> Result<(), StoreError> {
    let mut default_settings = state["settings"].clone();
    default_settings
        .as_object_mut()
        .ok_or(StoreError::Invalid)?
        .remove("revision");
    let settings = catalog
        .settings(&serde_json::to_string(&default_settings).map_err(|_| StoreError::Invalid)?)?;
    let mut settings_value: Value =
        serde_json::from_str(&settings.json).map_err(|_| StoreError::Invalid)?;
    settings_value["revision"] = json!(settings.revision);
    state["settings"] = settings_value;

    let records = catalog.session_records()?;
    state["savedDevices"] = Value::Array(
        catalog
            .saved_devices()?
            .iter()
            .map(|device| saved_device_value(device, &records))
            .collect(),
    );
    state["sessionCatalogRevision"] = json!(records.len());
    Ok(())
}

fn saved_device_value(
    device: &float_view_storage::SavedDevice,
    sessions: &[SessionCatalogRecord],
) -> Value {
    let owned = sessions
        .iter()
        .filter(|session| session.device_history_id == device.device_history_id)
        .collect::<Vec<_>>();
    let advertised = device.advertised_name.as_deref();
    let identifier = device.opaque_platform_id.as_deref();
    let display = device
        .custom_display_name
        .as_deref()
        .or(advertised)
        .or(identifier)
        .unwrap_or("VESC");
    json!({
        "savedDeviceId": device.saved_device_id,
        "deviceHistoryId": device.device_history_id,
        "revision": device.revision,
        "displayName": display,
        "customDisplayName": device.custom_display_name,
        "advertisedName": device.advertised_name,
        "bluetoothIdentifier": device.opaque_platform_id,
        "lastSeenAt": device.last_seen_at_millis.and_then(|value| float_view_storage::format_iso8601(value).ok()),
        "lastConnectedAt": device.last_connected_at_millis.and_then(|value| float_view_storage::format_iso8601(value).ok()),
        "profileComplete": false,
        "summary": {
            "sessionCount": owned.len(),
            "mostRecentSessionAt": owned.first().and_then(|session| float_view_storage::format_iso8601(session.started_at_millis).ok()),
            "totalRidingElapsedMs": owned.iter().map(|session| session.riding_elapsed_millis).sum::<u64>(),
            "totalDistanceM": owned.iter().filter_map(|session| session.distance_m).reduce(|left, right| left + right)
        }
    })
}

fn load_profile(catalog: &CatalogStore, saved_device_id: &str) -> Result<Value, StoreError> {
    let profile = catalog.board_profile(saved_device_id)?;
    let values: Value = serde_json::from_str(&profile.json).map_err(|_| StoreError::Invalid)?;
    let missing_fields = ["seriesCount", "parallelCount", "motor"]
        .into_iter()
        .filter(|field| values.get(*field).is_none())
        .collect::<Vec<_>>();
    Ok(json!({
        "savedDeviceId": profile.saved_device_id,
        "revision": profile.revision,
        "values": values,
        "missingFields": missing_fields,
    }))
}

fn session_summary_from_record(record: &SessionCatalogRecord) -> Value {
    let device: Value = serde_json::from_str(&record.device_snapshot_json).unwrap_or(Value::Null);
    json!({
        "sessionId": record.session_id,
        "deviceHistoryId": record.device_history_id,
        "displayName": device.get("displayName").or_else(|| device.get("name")).and_then(Value::as_str).unwrap_or("VESC"),
        "startedAt": float_view_storage::format_iso8601(record.started_at_millis).unwrap_or_default(),
        "endedAt": record.ended_at_millis.and_then(|value| float_view_storage::format_iso8601(value).ok()),
        "state": record.state,
        "integrity": match record.integrity.as_str() {
            "complete" => "complete",
            "recovered" => "recovered",
            "failed" => "corrupt",
            _ => "gaps",
        },
        "sampleCount": record.sample_count,
        "eventCount": record.event_count,
        "ridingElapsedMs": record.riding_elapsed_millis,
        "totalElapsedMs": record.total_elapsed_millis,
        "distanceM": record.distance_m,
        "maxSpeedMps": record.maximum_speed_mps,
        "maxDutyCycle": record.maximum_duty_cycle,
    })
}

fn saved_device() -> Value {
    json!({
        "savedDeviceId": SAVED_ID, "deviceHistoryId": HISTORY_ID, "revision": 0,
        "displayName": "Floatwheel Software", "customDisplayName": null,
        "advertisedName": "Floatwheel Software", "bluetoothIdentifier": "software-association",
        "lastSeenAt": "2026-01-01T00:00:00.000Z", "lastConnectedAt": null,
        "profileComplete": false,
        "summary": { "sessionCount": 0, "mostRecentSessionAt": null, "totalRidingElapsedMs": 0, "totalDistanceM": null }
    })
}

fn discovered_device() -> Value {
    json!({
        "deviceId": DEVICE_ID, "name": "Floatwheel Software", "state": "discovered",
        "lastSeenAt": "2026-01-01T00:00:00.000Z", "rssi": -42, "savedDeviceId": SAVED_ID
    })
}

fn active_connection(summary: &Value, embedded_native_host: bool) -> Value {
    let bms = if embedded_native_host {
        json!({
            "status": "attached",
            "connection": { "kind": "can", "canId": 42 },
            "candidates": [],
            "latestSnapshot": {
                "observedAt": "2026-01-01T00:00:00.300Z", "totalVoltageV": 50.4,
                "chargeVoltageV": 54.6, "inputCurrentA": 8.4, "inputIcCurrentA": 8.3,
                "sessionAmpHoursAh": 1.2, "sessionWattHoursWh": 60.5,
                "cellVoltagesV": [4.2, 4.2, 4.2, 4.2, 4.2, 4.2, 4.2, 4.2, 4.2, 4.2, 4.2, 4.2],
                "balancing": [false, false, false, false, false, false, false, false, false, false, false, false],
                "temperaturesC": [28.0, 29.0, 30.0], "icTemperatureC": 31.0,
                "humiditySensorTemperatureC": 27.0, "humidityPercent": 45.0,
                "highestCellTemperatureC": 30.0, "stateOfCharge": 0.72, "stateOfHealth": 0.98,
                "reportedCanId": 42, "lifetimeChargeAmpHoursAh": 120.0,
                "lifetimeChargeWattHoursWh": 6000.0, "lifetimeDischargeAmpHoursAh": 110.0,
                "lifetimeDischargeWattHoursWh": 5500.0, "pressurePa": 101_325.0,
                "dataVersion": 3, "status": "OK"
            }
        })
    } else {
        json!({ "status": "not_found", "connection": null, "candidates": [], "latestSnapshot": null })
    };
    json!({
        "deviceId": DEVICE_ID, "name": "Floatwheel Software", "displayName": "Floatwheel Software",
        "state": "ready", "lastSeenAt": "2026-01-01T00:00:00.000Z", "deviceKind": "board",
        "trackingMode": "saved", "savedDeviceId": SAVED_ID,
        "controllerConnection": { "kind": "direct" }, "controllerCandidates": [], "connectionIssue": null,
        "bms": bms,
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
        "activeSession": summary, "latestSample": riding_sample(), "latestLocation": null,
        "currentLimits": null,
        "firmware": { "vescVersion": "6.06", "packageName": "refloat", "packageVersion": "1.2.3", "hardwareName": "Express", "accessMode": "readWrite", "supported": true }
    })
}

fn riding_sample() -> Value {
    json!({
        "sequence": 3, "observedAt": "2026-01-01T00:00:00.300Z", "slowObservedAt": null,
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

fn session_summary(id: &str, state: &str) -> Value {
    json!({
        "sessionId": id, "deviceHistoryId": HISTORY_ID, "displayName": "Floatwheel Software",
        "startedAt": "2026-01-01T00:00:00.000Z", "endedAt": null, "state": state,
        "integrity": "complete", "sampleCount": 3, "eventCount": 0,
        "ridingElapsedMs": 300, "totalElapsedMs": 300, "distanceM": 2.0,
        "maxSpeedMps": 6.75, "maxDutyCycle": 0.42
    })
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

fn operation(id: &str, kind: &str, status: &str, session_id: Option<&str>, target: u64) -> Value {
    json!({
        "operationId": id, "kind": kind, "status": status, "sessionId": session_id,
        "scope": if session_id.is_some() { "one" } else { "all" },
        "targetResolutionSeconds": target, "completedSessions": 1, "eligibleSessions": 1, "error": null
    })
}

fn accepted() -> Value {
    json!({ "accepted": true })
}

fn empty(params: &Map<String, Value>) -> Result<(), ContractError> {
    if params.is_empty() {
        Ok(())
    } else {
        Err(invalid("Method takes no parameters"))
    }
}

fn require_saved(params: &Map<String, Value>) -> Result<(), ContractError> {
    if params.get("savedDeviceId").and_then(Value::as_str) == Some(SAVED_ID) {
        Ok(())
    } else {
        Err(ContractError::new("not_found", "Saved board was not found"))
    }
}

fn find_session<'a>(
    inner: &'a BackendState,
    params: &Map<String, Value>,
) -> Result<&'a Value, ContractError> {
    let id = text(params.get("sessionId"), "sessionId")?;
    inner
        .sessions
        .iter()
        .find(|session| session["sessionId"] == id)
        .ok_or_else(|| ContractError::new("not_found", "Session was not found"))
}

fn prune_target(params: &Map<String, Value>) -> Result<u64, ContractError> {
    params
        .get("targetResolutionSeconds")
        .and_then(Value::as_u64)
        .filter(|value| (1..=300).contains(value))
        .ok_or_else(|| invalid("Prune resolution is invalid"))
}

fn text<'a>(value: Option<&'a Value>, name: &str) -> Result<&'a str, ContractError> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid(&format!("{name} is required")))
}

fn invalid(message: &str) -> ContractError {
    ContractError::new("invalid_request", message)
}

#[cfg(test)]
mod composition_tests {
    use super::*;

    #[test]
    fn a_failed_reconnect_remains_retryable() {
        assert!(connection_stage_allows_retry(
            IdentificationStage::WaitingBluetooth
        ));
        assert!(connection_stage_allows_retry(
            IdentificationStage::Reconnecting
        ));
        assert!(!connection_stage_allows_retry(
            IdentificationStage::TransportFirmware
        ));
    }

    #[test]
    fn a_new_connection_does_not_reuse_a_stale_ready_generation() {
        let connection = NativeConnection {
            session_id: "test-session".into(),
            started_at_millis: 0,
            device_id: String::new(),
            device_token: Vec::new(),
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
            latest_telemetry: None,
            bms_status: DiscoveryStatus::Discovering,
            bms_connection: None,
            bms_candidates: Vec::new(),
            bms_snapshot: None,
            bms_pending_can_ids: Vec::new(),
            bms_probe_can_id: None,
            controls: None,
            telemetry_in_flight: false,
            pause_telemetry: false,
            shutdown_requested: false,
            retry_count: 0,
            handled_failure_generation: None,
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

    #[tokio::test]
    async fn production_composition_reopens_durable_state_after_ordered_stop() {
        let root = std::env::temp_dir().join(format!(
            "float-view-composition-test-{}",
            std::process::id()
        ));
        cleanup_standalone_storage(&root);
        for generation in 1..=2 {
            let runtime = BackendRuntime::start(BackendRuntimeConfig {
                generation,
                requested_port: 0,
                storage_root: root.clone(),
                platform: "macos".into(),
                launch_token: format!("composition-token-{generation}"),
                resources: standalone_resources(),
                application: None,
            })
            .await
            .unwrap();
            assert!(runtime.address().ip().is_loopback());
            assert!(root.join("catalog.sqlite").is_file());
            runtime.stop().await.unwrap();
        }
        cleanup_standalone_storage(&root);
    }
}
