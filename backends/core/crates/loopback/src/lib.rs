//! Bounded, authenticated loopback HTTP and WebSocket transport.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{OriginalUri, Path, Request, State};
use axum::http::header::{
    ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_TYPE, ORIGIN,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use flate2::Compression;
use flate2::write::GzEncoder;
use float_view_storage::{
    CatalogStore, ExportFormat, MAX_EXPORT_BYTES, MAX_IMPORT_BYTES, SessionDataCancellation,
    SessionDataService, StoreError,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tower::limit::ConcurrencyLimitLayer;

pub const MAX_HEADERS: usize = 32;
pub const MAX_HEADER_BYTES: usize = 16 * 1024;
pub const MAX_BODY_BYTES: usize = 64 * 1024;
pub const MAX_SESSION_HTTP_REQUEST_BYTES: usize = 16 * 1024;
pub const MAX_SESSION_HTTP_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_MESSAGE_BYTES: usize = 65_536;
pub const MAX_RESOURCE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CONNECTIONS: usize = 16;
pub const MAX_WEBSOCKETS: usize = 4;
pub const MAX_PENDING_RESPONSES: usize = 8;
pub const MAX_WRITE_BUFFER: usize = 128 * 1024;
#[cfg(not(test))]
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
#[cfg(test)]
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(2);
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
pub const QUEUE_TIMEOUT: Duration = Duration::from_millis(250);

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug)]
pub struct StaticResource {
    pub content_type: &'static str,
    pub bytes: Arc<[u8]>,
}

/// Resolves validated logical resource names. Implementations must not interpret paths.
pub trait ResourceProvider: Send + Sync + 'static {
    fn get(&self, logical_name: &str) -> Option<StaticResource>;
}

#[derive(Default)]
pub struct MemoryResources {
    resources: HashMap<String, StaticResource>,
}

impl MemoryResources {
    #[must_use]
    pub fn new(resources: impl IntoIterator<Item = (String, StaticResource)>) -> Self {
        Self {
            resources: resources.into_iter().collect(),
        }
    }
}

impl ResourceProvider for MemoryResources {
    fn get(&self, logical_name: &str) -> Option<StaticResource> {
        self.resources.get(logical_name).cloned()
    }
}

#[derive(Clone, Debug)]
pub struct ContractRequest {
    pub id: u64,
    pub method: String,
    pub params: Value,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ContractError {
    #[must_use]
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }
}

/// Contract dispatch remains behind the application owner; transport only handles envelopes.
pub trait ContractDispatcher: Send + Sync + 'static {
    /// Dispatches one validated request envelope.
    ///
    /// # Errors
    /// Returns a bounded public contract error when the request cannot be completed.
    fn dispatch(&self, request: ContractRequest) -> Result<Value, ContractError>;

    /// Returns the latest complete state after dispatch, when the dispatcher changed it.
    fn take_state_update(&self) -> Option<Value> {
        None
    }

    /// Advances host-owned asynchronous work without consuming a state update.
    fn pump(&self) {}
}

pub struct ExportPayload {
    pub file_name: String,
    pub content_type: &'static str,
    pub path: PathBuf,
}

pub trait SessionHttpApi: Send + Sync + 'static {
    /// Produces one validated, bounded columnar session-data response.
    ///
    /// # Errors
    /// Returns a public contract error for invalid, stale, unavailable, or failed reads.
    fn session_data(
        &self,
        body: &[u8],
        cancellation: &SessionDataCancellation,
    ) -> Result<Vec<u8>, ContractError>;

    /// Creates a bounded portable archive or CSV export.
    ///
    /// # Errors
    /// Returns a public contract error when validation or export creation fails.
    fn export(&self, body: &[u8]) -> Result<ExportPayload, ContractError>;

    /// Validates and consumes one bounded staged portable archive.
    ///
    /// # Errors
    /// Returns a public contract error when validation or import fails.
    fn import(&self, body: &[u8]) -> Result<Value, ContractError>;
}

struct UnavailableSessionHttpApi;

impl SessionHttpApi for UnavailableSessionHttpApi {
    fn session_data(
        &self,
        _body: &[u8],
        _cancellation: &SessionDataCancellation,
    ) -> Result<Vec<u8>, ContractError> {
        Err(ContractError::new(
            "unavailable",
            "Session data is unavailable",
        ))
    }

    fn export(&self, _body: &[u8]) -> Result<ExportPayload, ContractError> {
        Err(ContractError::new(
            "unavailable",
            "Session export is unavailable",
        ))
    }

    fn import(&self, _body: &[u8]) -> Result<Value, ContractError> {
        Err(ContractError::new(
            "unavailable",
            "Session import is unavailable",
        ))
    }
}

pub struct StorageSessionHttpApi {
    catalog: Arc<CatalogStore>,
    session_data: SessionDataService,
    next_identifier: AtomicU64,
}

impl StorageSessionHttpApi {
    #[must_use]
    pub fn new(catalog: Arc<CatalogStore>) -> Self {
        Self {
            session_data: SessionDataService::new(Arc::clone(&catalog)),
            catalog,
            next_identifier: AtomicU64::new(1),
        }
    }

    fn identifier(&self, prefix: &str) -> String {
        format!(
            "{prefix}-{}",
            self.next_identifier.fetch_add(1, Ordering::Relaxed)
        )
    }
}

impl SessionHttpApi for StorageSessionHttpApi {
    fn session_data(
        &self,
        body: &[u8],
        cancellation: &SessionDataCancellation,
    ) -> Result<Vec<u8>, ContractError> {
        self.session_data
            .query_json(body, cancellation)
            .map_err(|error| storage_error(&error))
    }

    fn export(&self, body: &[u8]) -> Result<ExportPayload, ContractError> {
        let request: Value = serde_json::from_slice(body)
            .map_err(|_| ContractError::new("invalid_request", "Invalid export request"))?;
        let session_id = bounded_text(request.get("sessionId"), 128)?;
        let format = match request.get("format").and_then(Value::as_str) {
            Some("csv") => ExportFormat::Csv,
            Some("float-view-archive") => ExportFormat::FloatViewArchive,
            _ => {
                return Err(ContractError::new(
                    "invalid_request",
                    "Invalid export format",
                ));
            }
        };
        let record = self
            .catalog
            .export_session(session_id, &self.identifier("http-export"), format)
            .map_err(|error| storage_error(&error))?;
        Ok(ExportPayload {
            file_name: record.file_name,
            content_type: if format == ExportFormat::Csv {
                "text/csv"
            } else {
                "application/octet-stream"
            },
            path: record.path,
        })
    }

    fn import(&self, body: &[u8]) -> Result<Value, ContractError> {
        if body.is_empty() || u64::try_from(body.len()).unwrap_or(u64::MAX) > MAX_IMPORT_BYTES {
            return Err(ContractError::new("invalid_request", "Invalid import body"));
        }
        let staging_token = self.identifier("http-stage");
        let session_id = self.identifier("imported");
        let job_id = self.identifier("import-job");
        let staging = self
            .catalog
            .staging_path(&staging_token)
            .map_err(|error| storage_error(&error))?;
        fs::write(&staging, body)
            .map_err(|_| ContractError::new("internal", "Import staging failed"))?;
        let imported = self
            .catalog
            .import_staged_archive(&staging, &session_id, &job_id, unix_millis()?)
            .map_err(|error| storage_error(&error))?;
        Ok(json!({ "sessionId": imported }))
    }
}

fn bounded_text(value: Option<&Value>, maximum: usize) -> Result<&str, ContractError> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= maximum)
        .ok_or_else(|| ContractError::new("invalid_request", "Required text is invalid"))
}

fn unix_millis() -> Result<i64, ContractError> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ContractError::new("internal", "System clock is invalid"))?
        .as_millis();
    i64::try_from(millis).map_err(|_| ContractError::new("internal", "System clock is invalid"))
}

fn storage_error(error: &StoreError) -> ContractError {
    match error {
        StoreError::Invalid => ContractError::new("invalid_request", "Session request is invalid"),
        StoreError::NotFound => ContractError::new("not_found", "Session was not found"),
        StoreError::Conflict { current_revision } => ContractError {
            code: "conflict",
            message: "Session snapshot changed".into(),
            details: Some(json!({ "currentRevision": current_revision })),
        },
        StoreError::Sqlite(_) | StoreError::Io(_) | StoreError::Poisoned => {
            ContractError::new("internal", "Session operation failed")
        }
    }
}

struct UnavailableDispatcher;

impl ContractDispatcher for UnavailableDispatcher {
    fn dispatch(&self, _request: ContractRequest) -> Result<Value, ContractError> {
        Err(ContractError::new("unavailable", "Backend is unavailable"))
    }
}

#[derive(Clone)]
struct ServerState {
    origin: Arc<str>,
    token: Arc<str>,
    platform: Arc<str>,
    public_origin: Arc<str>,
    websocket_slots: Arc<Semaphore>,
    resources: Arc<dyn ResourceProvider>,
    dispatcher: Arc<dyn ContractDispatcher>,
    session_http: Arc<dyn SessionHttpApi>,
    state_updates: watch::Sender<Option<Arc<str>>>,
    published_revision: Arc<Mutex<Option<u64>>>,
    stopping: watch::Sender<bool>,
}

pub struct ServerConfig {
    pub bind_port: u16,
    pub expected_origin: String,
    pub launch_token: String,
    pub platform: String,
    pub resources: Arc<dyn ResourceProvider>,
    pub dispatcher: Arc<dyn ContractDispatcher>,
    pub session_http: Arc<dyn SessionHttpApi>,
}

impl ServerConfig {
    #[must_use]
    pub fn new(
        expected_origin: impl Into<String>,
        launch_token: impl Into<String>,
        platform: impl Into<String>,
        resources: Arc<dyn ResourceProvider>,
        dispatcher: Arc<dyn ContractDispatcher>,
    ) -> Self {
        Self {
            bind_port: 0,
            expected_origin: expected_origin.into(),
            launch_token: launch_token.into(),
            platform: platform.into(),
            resources,
            dispatcher,
            session_http: Arc::new(UnavailableSessionHttpApi),
        }
    }

    #[must_use]
    pub const fn with_bind_port(mut self, bind_port: u16) -> Self {
        self.bind_port = bind_port;
        self
    }

    #[must_use]
    pub fn with_session_http(mut self, session_http: Arc<dyn SessionHttpApi>) -> Self {
        self.session_http = session_http;
        self
    }
}

pub struct LoopbackServer {
    pub address: SocketAddr,
    state: ServerState,
    shutdown: Option<oneshot::Sender<()>>,
    worker: JoinHandle<()>,
}

pub type SmokeServer = LoopbackServer;

impl LoopbackServer {
    /// Publishes a complete authoritative state if its revision is strictly newer.
    ///
    /// # Errors
    /// Returns an invalid-state error for incomplete, invalid, stale, or oversized snapshots.
    pub fn publish_state(&self, state: &Value) -> Result<(), ContractError> {
        publish_state(&self.state, state)
    }

    /// Stops the listener, closes active connection tasks, and waits for the server task.
    ///
    /// # Panics
    /// Panics only if the server task itself panicked.
    pub async fn stop(mut self) {
        self.state.stopping.send_replace(true);
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.worker.await.expect("loopback server task");
    }
}

fn publish_state(server: &ServerState, state: &Value) -> Result<(), ContractError> {
    let revision = complete_state_revision(state)
        .ok_or_else(|| ContractError::new("invalid_state", "State snapshot is incomplete"))?;
    let envelope = serde_json::to_string(&json!({ "event": "state.changed", "data": state }))
        .map_err(|_| ContractError::new("internal", "State serialization failed"))?;
    if envelope.len() > MAX_MESSAGE_BYTES {
        return Err(ContractError::new(
            "invalid_state",
            "State exceeds the message limit",
        ));
    }
    let mut published_revision = server
        .published_revision
        .lock()
        .map_err(|_| ContractError::new("internal", "State publication failed"))?;
    if published_revision.is_some_and(|current| revision <= current) {
        return Err(ContractError::new(
            "conflict",
            "State revision is not monotonic",
        ));
    }
    *published_revision = Some(revision);
    server.state_updates.send_replace(Some(Arc::from(envelope)));
    Ok(())
}

fn complete_state_revision(state: &Value) -> Option<u64> {
    let object = state.as_object()?;
    let revision = object
        .get("revision")?
        .as_u64()
        .filter(|revision| *revision <= MAX_SAFE_INTEGER)?;
    object.get("bluetooth")?.as_object()?;
    object.get("discoveredDevices")?.as_array()?;
    object.get("savedDevices")?.as_array()?;
    if !object
        .get("activeConnection")
        .is_some_and(|value| value.is_null() || value.is_object())
    {
        return None;
    }
    object
        .get("sessionCatalogRevision")?
        .as_u64()
        .filter(|value| *value <= MAX_SAFE_INTEGER)?;
    object.get("settings")?.as_object()?;
    object.get("location")?.as_object()?;
    Some(revision)
}

/// Starts the production loopback router on the requested IPv4 loopback port.
/// Port zero selects an ephemeral port.
///
/// # Errors
/// Returns a bind error.
pub async fn start_server(config: ServerConfig) -> std::io::Result<LoopbackServer> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.bind_port)).await?;
    let address = listener.local_addr()?;
    let public_origin: Arc<str> = Arc::from(format!("http://{address}"));
    let (state_updates, _) = watch::channel(None);
    let (stopping, _) = watch::channel(false);
    let state = ServerState {
        origin: if config.expected_origin.is_empty() {
            Arc::clone(&public_origin)
        } else {
            Arc::from(config.expected_origin)
        },
        token: Arc::from(config.launch_token),
        platform: Arc::from(config.platform),
        public_origin,
        websocket_slots: Arc::new(Semaphore::new(MAX_WEBSOCKETS)),
        resources: config.resources,
        dispatcher: config.dispatcher,
        session_http: config.session_http,
        state_updates,
        published_revision: Arc::new(Mutex::new(None)),
        stopping,
    };
    let app = Router::new()
        .route("/", get(index))
        .route("/api/bootstrap", get(bootstrap))
        .route("/api/ws", get(websocket_upgrade))
        .route("/api/session-data", post(session_data))
        .route("/api/session-export", post(session_export))
        .route("/api/session-import", post(session_import))
        .route("/{*path}", get(resource))
        .layer(ConcurrencyLimitLayer::new(MAX_CONNECTIONS))
        .layer(middleware::from_fn(validate_headers))
        .with_state(state.clone());
    let (shutdown, stopped) = oneshot::channel();
    let worker = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await;
    });
    Ok(LoopbackServer {
        address,
        state,
        shutdown: Some(shutdown),
        worker,
    })
}

/// Compatibility entry point used by target smoke binaries.
///
/// # Errors
/// Returns a bind error.
pub async fn start(origin: &str, token: &str) -> std::io::Result<SmokeServer> {
    let resources = MemoryResources::new([(
        "index.html".to_owned(),
        StaticResource {
            content_type: "text/html; charset=utf-8",
            bytes: Arc::from(&b"<!doctype html><title>Float View</title>"[..]),
        },
    )]);
    start_server(ServerConfig::new(
        origin,
        token,
        "web",
        Arc::new(resources),
        Arc::new(UnavailableDispatcher),
    ))
    .await
}

async fn validate_headers(request: Request, next: Next) -> Response {
    if !headers_within_bounds(request.headers()) {
        return StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE.into_response();
    }
    next.run(request).await
}

fn headers_within_bounds(headers: &HeaderMap) -> bool {
    headers.len() <= MAX_HEADERS
        && headers
            .iter()
            .try_fold(0_usize, |total, (name, value)| {
                total.checked_add(name.as_str().len() + value.as_bytes().len())
            })
            .is_some_and(|bytes| bytes <= MAX_HEADER_BYTES)
}

async fn bootstrap(State(state): State<ServerState>) -> Response {
    let websocket_origin = state.public_origin.replacen("http://", "ws://", 1);
    json_response(&json!({
        "webSocketUrl": format!("{websocket_origin}/api/ws"),
        "sessionDataUrl": format!("{}/api/session-data", state.public_origin),
        "sessionExportUrl": format!("{}/api/session-export", state.public_origin),
        "sessionImportUrl": format!("{}/api/session-import", state.public_origin),
        "sessionToken": state.token.as_ref(),
        "platform": state.platform.as_ref(),
    }))
}

async fn index(State(state): State<ServerState>) -> Response {
    static_resource(&state, "index.html")
}

async fn resource(State(state): State<ServerState>, Path(path): Path<String>) -> Response {
    if !valid_logical_name(&path) || path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    static_resource(&state, &path)
}

fn valid_logical_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 512
        && !name.contains('\\')
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn static_resource(state: &ServerState, logical_name: &str) -> Response {
    let Some(resource) = state.resources.get(logical_name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if resource.bytes.len() > MAX_RESOURCE_BYTES {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    let mut response = Response::new(Body::from(resource.bytes.to_vec()));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(value) = HeaderValue::from_str(resource.content_type) {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }
    response
}

fn json_response(value: &Value) -> Response {
    let Ok(body) = serde_json::to_vec(&value) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = Response::new(Body::from(body));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn session_data(
    State(state): State<ServerState>,
    headers: HeaderMap,
    request: Request,
) -> Response {
    if !authenticated_http(&state, &headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let accepts_gzip = headers
        .get(ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|item| item.trim().eq_ignore_ascii_case("gzip"))
        });
    let Ok(body) = to_bytes(request.into_body(), MAX_SESSION_HTTP_REQUEST_BYTES).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let cancellation = Arc::new(SessionDataCancellation::default());
    let cancel_on_drop = CancelOnDrop(Arc::clone(&cancellation));
    let api = Arc::clone(&state.session_http);
    let result = tokio::task::spawn_blocking(move || api.session_data(&body, &cancellation)).await;
    drop(cancel_on_drop);
    let encoded = match result {
        Ok(Ok(encoded)) if encoded.len() <= MAX_SESSION_HTTP_RESPONSE_BYTES => encoded,
        Ok(Ok(_)) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        Ok(Err(error)) => return contract_error_response(&error),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let encoded = if accepts_gzip {
        match gzip(&encoded) {
            Ok(encoded) => encoded,
            Err(()) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    } else {
        encoded
    };
    let mut response = Response::new(Body::from(encoded));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if accepts_gzip {
        response
            .headers_mut()
            .insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    }
    response
}

async fn session_export(
    State(state): State<ServerState>,
    headers: HeaderMap,
    request: Request,
) -> Response {
    if !authenticated_http(&state, &headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(body) = to_bytes(request.into_body(), MAX_SESSION_HTTP_REQUEST_BYTES).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let api = Arc::clone(&state.session_http);
    let result = tokio::task::spawn_blocking(move || {
        let export = api.export(&body)?;
        if !valid_file_name(&export.file_name) {
            return Err(ContractError::new(
                "internal",
                "Export file name is invalid",
            ));
        }
        let metadata = fs::metadata(&export.path)
            .map_err(|_| ContractError::new("internal", "Export file is unavailable"))?;
        if metadata.len() == 0 || metadata.len() > MAX_EXPORT_BYTES {
            return Err(ContractError::new(
                "internal",
                "Export exceeds the file limit",
            ));
        }
        let bytes = fs::read(&export.path)
            .map_err(|_| ContractError::new("internal", "Export file is unavailable"))?;
        let _ = fs::remove_file(&export.path);
        Ok((export, bytes))
    })
    .await;
    let (export, bytes) = match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return contract_error_response(&error),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let Ok(content_type) = HeaderValue::from_str(export.content_type) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Ok(disposition) =
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", export.file_name))
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(CONTENT_TYPE, content_type);
    response
        .headers_mut()
        .insert(CONTENT_DISPOSITION, disposition);
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn session_import(
    State(state): State<ServerState>,
    headers: HeaderMap,
    request: Request,
) -> Response {
    if !authenticated_http(&state, &headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let import_limit = usize::try_from(MAX_IMPORT_BYTES).unwrap_or(usize::MAX);
    let Ok(body) = to_bytes(request.into_body(), import_limit).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let api = Arc::clone(&state.session_http);
    match tokio::task::spawn_blocking(move || api.import(&body)).await {
        Ok(Ok(value)) => json_response(&value),
        Ok(Err(error)) => contract_error_response(&error),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

struct CancelOnDrop(Arc<SessionDataCancellation>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

fn authenticated_http(state: &ServerState, headers: &HeaderMap) -> bool {
    headers.get(ORIGIN).and_then(|value| value.to_str().ok()) == Some(state.origin.as_ref())
        && headers
            .get("x-float-view-token")
            .and_then(|value| value.to_str().ok())
            == Some(state.token.as_ref())
}

fn gzip(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).map_err(|_| ())?;
    encoder.finish().map_err(|_| ())
}

fn contract_error_response(error: &ContractError) -> Response {
    let status = match error.code {
        "invalid_request" | "invalid_state" => StatusCode::BAD_REQUEST,
        "not_found" => StatusCode::NOT_FOUND,
        "conflict" => StatusCode::CONFLICT,
        "unavailable" | "permission_required" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut response = json_response(&json!({ "error": error }));
    *response.status_mut() = status;
    response
}

fn valid_file_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.contains(['/', '\\', '"', '\r', '\n'])
        && value != "."
        && value != ".."
}

async fn websocket_upgrade(
    State(state): State<ServerState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !authenticated_websocket(&state, &headers, &uri) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(permit) = Arc::clone(&state.websocket_slots).try_acquire_owned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    upgrade
        .max_message_size(MAX_MESSAGE_BYTES)
        .max_frame_size(MAX_MESSAGE_BYTES)
        .write_buffer_size(16 * 1024)
        .max_write_buffer_size(MAX_WRITE_BUFFER)
        .on_upgrade(move |socket| websocket(socket, state, permit))
}

fn authenticated_websocket(
    state: &ServerState,
    headers: &HeaderMap,
    uri: &axum::http::Uri,
) -> bool {
    let origin = headers.get(ORIGIN).and_then(|value| value.to_str().ok());
    let mut tokens = uri
        .query()
        .unwrap_or_default()
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .filter(|(name, _)| *name == "token")
        .map(|(_, value)| value);
    origin == Some(state.origin.as_ref())
        && tokens.next() == Some(state.token.as_ref())
        && tokens.next().is_none()
}

async fn websocket(socket: WebSocket, state: ServerState, _permit: OwnedSemaphorePermit) {
    let (mut sink, mut source) = socket.split();
    let (responses_tx, mut responses_rx) = mpsc::channel::<Message>(MAX_PENDING_RESPONSES);
    let mut state_updates = state.state_updates.subscribe();
    let mut sender_stopping = state.stopping.subscribe();
    let sender = tokio::spawn(async move {
        loop {
            let message = tokio::select! {
                biased;
                changed = sender_stopping.changed() => {
                    if changed.is_err() || *sender_stopping.borrow() { return; }
                    continue;
                }
                response = responses_rx.recv() => response,
                changed = state_updates.changed() => {
                    if changed.is_err() { return; }
                    state_updates.borrow_and_update().clone()
                        .map(|value| Message::Text(value.to_string().into()))
                }
            };
            let Some(message) = message else { return };
            let written = tokio::time::timeout(WRITE_TIMEOUT, sink.send(message)).await;
            if !matches!(written, Ok(Ok(()))) {
                return;
            }
        }
    });

    let mut receiver_stopping = state.stopping.subscribe();
    let mut dispatcher_poll = tokio::time::interval(Duration::from_millis(100));
    dispatcher_poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut idle_deadline = tokio::time::Instant::now() + IDLE_TIMEOUT;
    loop {
        let incoming = tokio::select! {
            changed = receiver_stopping.changed() => {
                if changed.is_err() || *receiver_stopping.borrow() { break; }
                continue;
            }
            _ = dispatcher_poll.tick() => {
                if let Some(update) = state.dispatcher.take_state_update() {
                    let _ = publish_state(&state, &update);
                }
                continue;
            }
            incoming = tokio::time::timeout_at(idle_deadline, source.next()) => incoming,
        };
        match incoming {
            Ok(Some(Ok(Message::Text(text)))) => {
                idle_deadline = tokio::time::Instant::now() + IDLE_TIMEOUT;
                let response = dispatch_message(&state, text.as_str());
                let queued = tokio::time::timeout(
                    QUEUE_TIMEOUT,
                    responses_tx.send(Message::Text(response.into())),
                )
                .await;
                if !matches!(queued, Ok(Ok(()))) {
                    break;
                }
            }
            Ok(Some(Ok(Message::Close(frame)))) => {
                let _ =
                    tokio::time::timeout(QUEUE_TIMEOUT, responses_tx.send(Message::Close(frame)))
                        .await;
                break;
            }
            Ok(Some(Err(_)) | None) | Err(_) => break,
            Ok(Some(Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_)))) => {
                idle_deadline = tokio::time::Instant::now() + IDLE_TIMEOUT;
            }
        }
    }
    drop(responses_tx);
    let mut sender = sender;
    if tokio::time::timeout(WRITE_TIMEOUT, &mut sender)
        .await
        .is_err()
    {
        sender.abort();
        let _ = sender.await;
    }
}

fn dispatch_message(state: &ServerState, text: &str) -> String {
    let parsed = serde_json::from_str::<Value>(text);
    let request = parsed.as_ref().ok().and_then(parse_request);
    let (id, response) = match request {
        Some(request) => {
            let id = request.id;
            let mut response = state.dispatcher.dispatch(request);
            if let Some(update) = state.dispatcher.take_state_update()
                && let Err(error) = publish_state(state, &update)
            {
                response = Err(error);
            }
            (id, response)
        }
        None => (
            parsed
                .ok()
                .and_then(|value| value.get("id").and_then(Value::as_u64))
                .filter(|id| *id <= MAX_SAFE_INTEGER)
                .unwrap_or(0),
            Err(ContractError::new(
                "invalid_request",
                "Request must be a valid contract envelope",
            )),
        ),
    };
    let envelope = match response {
        Ok(result) => json!({ "id": id, "result": result }),
        Err(error) => json!({ "id": id, "error": error }),
    };
    let encoded = serde_json::to_string(&envelope).unwrap_or_else(|_| {
        r#"{"id":0,"error":{"code":"internal","message":"Response serialization failed"}}"#
            .to_owned()
    });
    if encoded.len() <= MAX_MESSAGE_BYTES {
        encoded
    } else {
        format!(
            r#"{{"id":{id},"error":{{"code":"internal","message":"Response exceeds the message limit"}}}}"#
        )
    }
}

fn parse_request(value: &Value) -> Option<ContractRequest> {
    let object = value.as_object()?;
    let id = object
        .get("id")?
        .as_u64()
        .filter(|id| *id <= MAX_SAFE_INTEGER)?;
    let method = object.get("method")?.as_str()?;
    let params = object.get("params")?.clone();
    if id == 0 || method.is_empty() || method.len() > 128 || !params.is_object() {
        return None;
    }
    Some(ContractRequest {
        id,
        method: method.to_owned(),
        params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::SinkExt;
    use std::fmt::Write as _;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message as ClientMessage;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    struct TestDispatcher;

    impl ContractDispatcher for TestDispatcher {
        fn dispatch(&self, request: ContractRequest) -> Result<Value, ContractError> {
            if request.method == "echo" {
                Ok(request.params)
            } else if request.method == "oversized" {
                Ok(json!({ "value": "x".repeat(MAX_MESSAGE_BYTES) }))
            } else {
                Err(ContractError::new("invalid_request", "Unknown method"))
            }
        }
    }

    struct TestSessionHttp;

    impl SessionHttpApi for TestSessionHttp {
        fn session_data(
            &self,
            body: &[u8],
            cancellation: &SessionDataCancellation,
        ) -> Result<Vec<u8>, ContractError> {
            if body == b"cancel" {
                cancellation.cancel();
                return Err(ContractError::new("invalid_request", "cancelled"));
            }
            Ok(br#"{"kind":"samples","rows":1}"#.to_vec())
        }

        fn export(&self, _body: &[u8]) -> Result<ExportPayload, ContractError> {
            let path = std::env::temp_dir().join(format!(
                "float-view-http-export-{}-{}.csv",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::write(&path, b"sequence,value\n1,2\n").unwrap();
            Ok(ExportPayload {
                file_name: "session.csv".into(),
                content_type: "text/csv",
                path,
            })
        }

        fn import(&self, body: &[u8]) -> Result<Value, ContractError> {
            Ok(json!({ "sessionId": format!("imported-{}", body.len()) }))
        }
    }

    fn config() -> ServerConfig {
        ServerConfig::new(
            "app://float-view",
            "secret",
            "ios",
            Arc::new(MemoryResources::new([
                (
                    "index.html".to_owned(),
                    StaticResource {
                        content_type: "text/html",
                        bytes: Arc::from(&b"index"[..]),
                    },
                ),
                (
                    "assets/app.js".to_owned(),
                    StaticResource {
                        content_type: "text/javascript",
                        bytes: Arc::from(&b"app"[..]),
                    },
                ),
            ])),
            Arc::new(TestDispatcher),
        )
        .with_session_http(Arc::new(TestSessionHttp))
    }

    fn state(revision: u64) -> Value {
        json!({
            "revision": revision,
            "bluetooth": {},
            "discoveredDevices": [],
            "savedDevices": [],
            "activeConnection": null,
            "sessionCatalogRevision": 0,
            "settings": {},
            "location": {}
        })
    }

    async fn request(address: SocketAddr, path: &str, extra: &str) -> String {
        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: {address}\r\n{extra}Connection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        response
    }

    async fn post_request(address: SocketAddr, path: &str, body: &[u8], extra: &str) -> Vec<u8> {
        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        let headers = format!(
            "POST {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(headers.as_bytes()).await.unwrap();
        stream.write_all(body).await.unwrap();
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response).await;
        response
    }

    async fn socket(
        address: SocketAddr,
        token: &str,
        origin: &str,
    ) -> Result<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Error,
    > {
        let mut request = format!("ws://{address}/api/ws?token={token}")
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("origin", origin.parse().unwrap());
        connect_async(request).await.map(|(socket, _)| socket)
    }

    #[tokio::test]
    async fn serves_public_bootstrap_and_validated_static_resources() {
        let server = start_server(config()).await.unwrap();
        let bootstrap = request(server.address, "/api/bootstrap", "").await;
        assert!(bootstrap.starts_with("HTTP/1.1 200"));
        assert!(bootstrap.contains("\"platform\":\"ios\""));
        assert!(bootstrap.contains("\"sessionToken\":\"secret\""));
        assert!(
            request(server.address, "/assets/app.js", "")
                .await
                .ends_with("app")
        );
        assert!(
            request(server.address, "/api/missing", "")
                .await
                .starts_with("HTTP/1.1 404")
        );
        server.stop().await;
    }

    #[tokio::test]
    async fn authenticates_dispatches_and_stops_cleanly() {
        let first = start_server(config()).await.unwrap();
        let bad_origin = socket(first.address, "secret", "wrong").await.unwrap_err();
        assert!(bad_origin.to_string().contains("403"));
        let bad_token = socket(first.address, "wrong", "app://float-view")
            .await
            .unwrap_err();
        assert!(bad_token.to_string().contains("403"));
        let mut client = socket(first.address, "secret", "app://float-view")
            .await
            .unwrap();
        client
            .send(ClientMessage::Text(
                r#"{"id":1,"method":"echo","params":{"ok":true}}"#.into(),
            ))
            .await
            .unwrap();
        assert_eq!(
            client.next().await.unwrap().unwrap().to_text().unwrap(),
            r#"{"id":1,"result":{"ok":true}}"#
        );
        first.stop().await;
        let second = start_server(config()).await.unwrap();
        assert!(
            request(second.address, "/", "")
                .await
                .starts_with("HTTP/1.1 200")
        );
        second.stop().await;
    }

    #[tokio::test]
    async fn rejects_excess_headers_and_invalid_envelopes() {
        let server = start_server(config()).await.unwrap();
        let headers = (0..MAX_HEADERS).fold(String::new(), |mut headers, index| {
            write!(headers, "X-{index}: x\r\n").unwrap();
            headers
        });
        assert!(
            request(server.address, "/api/bootstrap", &headers)
                .await
                .starts_with("HTTP/1.1 431")
        );
        let mut client = socket(server.address, "secret", "app://float-view")
            .await
            .unwrap();
        client
            .send(ClientMessage::Text("not json".into()))
            .await
            .unwrap();
        assert!(
            client
                .next()
                .await
                .unwrap()
                .unwrap()
                .to_text()
                .unwrap()
                .contains("invalid_request")
        );
        server.stop().await;
    }

    #[tokio::test]
    async fn publishes_only_complete_strictly_monotonic_conflated_state() {
        let server = start_server(config()).await.unwrap();
        let mut client = socket(server.address, "secret", "app://float-view")
            .await
            .unwrap();
        server.publish_state(&state(0)).unwrap();
        let first = client.next().await.unwrap().unwrap();
        assert!(first.to_text().unwrap().contains("\"revision\":0"));
        assert!(server.publish_state(&state(0)).is_err());
        assert!(server.publish_state(&json!({"bluetooth": {}})).is_err());
        server.publish_state(&state(1)).unwrap();
        let second = client.next().await.unwrap().unwrap();
        assert!(second.to_text().unwrap().contains("\"revision\":1"));
        server.stop().await;
    }

    #[tokio::test]
    async fn fragmented_allowed_message_dispatches_and_oversize_is_rejected() {
        use tokio_tungstenite::tungstenite::protocol::frame::Frame;
        use tokio_tungstenite::tungstenite::protocol::frame::coding::{Data as FrameData, OpCode};

        let server = start_server(config()).await.unwrap();
        let mut client = socket(server.address, "secret", "app://float-view")
            .await
            .unwrap();
        client
            .send(ClientMessage::Frame(Frame::message(
                "{\"id\":2,\"method\":\"echo\",\"params\":{\"value\":\"",
                OpCode::Data(FrameData::Text),
                false,
            )))
            .await
            .unwrap();
        client
            .send(ClientMessage::Frame(Frame::message(
                r#"fragmented"}}"#,
                OpCode::Data(FrameData::Continue),
                true,
            )))
            .await
            .unwrap();
        assert!(
            client
                .next()
                .await
                .unwrap()
                .unwrap()
                .to_text()
                .unwrap()
                .contains("fragmented")
        );
        client
            .send(ClientMessage::Binary(vec![0; MAX_MESSAGE_BYTES + 1].into()))
            .await
            .unwrap();
        let rejected = tokio::time::timeout(Duration::from_secs(1), client.next()).await;
        assert!(!matches!(rejected, Ok(Some(Ok(ClientMessage::Binary(_))))));
        server.stop().await;
    }

    #[tokio::test]
    async fn websocket_clients_responses_and_idle_lifetime_are_bounded() {
        let server = start_server(config()).await.unwrap();
        let mut clients = Vec::new();
        for _ in 0..MAX_WEBSOCKETS {
            clients.push(
                socket(server.address, "secret", "app://float-view")
                    .await
                    .unwrap(),
            );
        }
        let excess = socket(server.address, "secret", "app://float-view")
            .await
            .unwrap_err();
        assert!(excess.to_string().contains("503"));

        clients[0]
            .send(ClientMessage::Text(
                r#"{"id":3,"method":"oversized","params":{}}"#.into(),
            ))
            .await
            .unwrap();
        let response = clients[0].next().await.unwrap().unwrap();
        assert!(response.to_text().unwrap().contains("Response exceeds"));
        assert!(response.len() <= MAX_MESSAGE_BYTES);

        let idle_closed =
            tokio::time::timeout(IDLE_TIMEOUT + Duration::from_secs(1), clients[1].next()).await;
        assert!(idle_closed.is_ok());
        server.stop().await;
    }

    #[tokio::test]
    async fn session_http_requires_exact_auth_gzips_and_bounds_bodies() {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let server = start_server(config()).await.unwrap();
        let denied = post_request(server.address, "/api/session-data", b"{}", "").await;
        assert!(denied.starts_with(b"HTTP/1.1 403"));
        let auth =
            "Origin: app://float-view\r\nX-Float-View-Token: secret\r\nAccept-Encoding: gzip\r\n";
        let response = post_request(server.address, "/api/session-data", b"{}", auth).await;
        assert!(response.starts_with(b"HTTP/1.1 200"));
        assert!(String::from_utf8_lossy(&response).contains("content-encoding: gzip"));
        let body_offset = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap()
            + 4;
        let mut decoded = String::new();
        GzDecoder::new(&response[body_offset..])
            .read_to_string(&mut decoded)
            .unwrap();
        assert_eq!(decoded, r#"{"kind":"samples","rows":1}"#);

        let oversized = post_request(
            server.address,
            "/api/session-data",
            &vec![b'x'; MAX_SESSION_HTTP_REQUEST_BYTES + 1],
            auth,
        )
        .await;
        assert!(oversized.starts_with(b"HTTP/1.1 413"));
        let imported = post_request(server.address, "/api/session-import", b"archive", auth).await;
        assert!(imported.starts_with(b"HTTP/1.1 200"));
        assert!(String::from_utf8_lossy(&imported).contains("imported-7"));
        let exported = post_request(server.address, "/api/session-export", b"{}", auth).await;
        assert!(exported.starts_with(b"HTTP/1.1 200"));
        let exported_text = String::from_utf8_lossy(&exported);
        assert!(exported_text.contains("content-type: text/csv"));
        assert!(exported_text.ends_with("sequence,value\n1,2\n"));
        server.stop().await;
    }
}
