//! One loopback executor, bounded socket admission and authenticated routes.
use crate::{backend::Backend, limits::*, protocol::Platform};
use axum::{
    Router,
    body::to_bytes,
    extract::{
        Request, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::{
    future::Future,
    io,
    net::{Ipv4Addr, SocketAddr, TcpListener as StdListener},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    task::{Context, Poll},
    thread,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::{OwnedSemaphorePermit, Semaphore, oneshot},
    time::{Instant, Sleep},
};

static CLIENT: AtomicU64 = AtomicU64::new(1);
pub(crate) struct Client {
    pub backend: Arc<Backend>,
    pub id: u64,
    pub generation: u64,
}
impl Client {
    pub fn new(backend: &Arc<Backend>) -> Result<Self, StatusCode> {
        let id = CLIENT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok(Self {
            backend: backend.clone(),
            id,
            generation: backend.generation.load(Ordering::Acquire),
        })
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.backend.disconnect_service_client(self.id);
        let _ = self
            .backend
            .exchange
            .detach_client(self.generation, self.id);
        self.backend.changed.notify_waiters();
    }
}
pub(crate) struct App {
    pub backend: Arc<Backend>,
    pub token: String,
    pub html: Arc<[u8]>,
    pub platform: Platform,
    pub port: AtomicU64,
}
struct Running {
    stop: Option<oneshot::Sender<()>>,
    finished: mpsc::Receiver<()>,
    thread: thread::JoinHandle<()>,
}
pub struct Server {
    pub backend: Arc<Backend>,
    app: Arc<App>,
    running: Mutex<Option<Running>>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerError {
    InvalidAsset,
    Entropy,
    Bind,
    Executor,
    AlreadyRunning,
    Shutdown,
    Poisoned,
}
impl Server {
    pub fn new(platform: Platform, html: &[u8]) -> Result<Self, ServerError> {
        Self::with_backend(platform, html, Arc::new(Backend::default()))
    }
    pub fn with_backend(
        platform: Platform,
        html: &[u8],
        backend: Arc<Backend>,
    ) -> Result<Self, ServerError> {
        if html.is_empty() || html.len() > 8 * 1024 * 1024 {
            return Err(ServerError::InvalidAsset);
        }
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| ServerError::Entropy)?;
        let token = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Self {
            backend: backend.clone(),
            app: Arc::new(App {
                backend,
                token,
                html: Arc::from(html),
                platform,
                port: AtomicU64::new(0),
            }),
            running: Mutex::new(None),
        })
    }
    pub fn start(&self, port: u16) -> Result<u16, ServerError> {
        let mut running = self.running.lock().map_err(|_| ServerError::Poisoned)?;
        if running.is_some() {
            return Err(ServerError::AlreadyRunning);
        }
        let listener =
            StdListener::bind((Ipv4Addr::LOCALHOST, port)).map_err(|_| ServerError::Bind)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| ServerError::Bind)?;
        let port = listener.local_addr().map_err(|_| ServerError::Bind)?.port();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(MAX_BLOCKING_WORKERS)
            .build()
            .map_err(|_| ServerError::Executor)?;
        self.backend.start().map_err(|_| ServerError::Executor)?;
        self.backend
            .sync_service()
            .map_err(|_| ServerError::Executor)?;
        self.app.port.store(port as u64, Ordering::Release);
        let (ready, started) = mpsc::sync_channel(1);
        let (stop, stopped) = oneshot::channel();
        let (finished, done) = mpsc::sync_channel(1);
        let app = self.app.clone();
        let thread = match thread::Builder::new()
            .name("float-view-loopback".into())
            .spawn(move || {
                runtime.block_on(async move {
                    app.backend.watch_service();
                    let listener = match TcpListener::from_std(listener) {
                        Ok(listener) => listener,
                        Err(_) => {
                            let _ = ready.send(false);
                            return;
                        }
                    };
                    let listener = BoundedListener {
                        listener,
                        permits: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
                    };
                    let protected = Router::new()
                        .route("/api/ws", get(websocket))
                        .route("/api/session-data", post(session_data))
                        .route("/api/session-import", post(crate::archives::import))
                        .route("/api/session-export", post(crate::archives::export))
                        .route_layer(middleware::from_fn_with_state(app.clone(), authenticate));
                    let router = Router::new()
                        .route("/", get(root))
                        .route("/api/bootstrap", get(bootstrap))
                        .merge(protected)
                        .with_state(app);
                    let _ = ready.send(true);
                    tokio::select! {_=stopped=>{},_=axum::serve(listener,router)=>{}}
                });
                // Dropping the current-thread executor cancels all socket futures;
                // there are no native callbacks or native-owned blocking closures.
                drop(runtime);
                let _ = finished.send(());
            }) {
            Ok(thread) => thread,
            Err(_) => {
                self.backend.stop();
                return Err(ServerError::Executor);
            }
        };
        if started.recv_timeout(SHUTDOWN_TIMEOUT) != Ok(true) {
            self.backend.stop();
            let _ = stop.send(());
            if done.recv_timeout(SHUTDOWN_TIMEOUT).is_ok() {
                let _ = thread.join();
            }
            return Err(ServerError::Executor);
        }
        *running = Some(Running {
            stop: Some(stop),
            finished: done,
            thread,
        });
        Ok(port)
    }
    pub fn stop(&self) -> Result<Vec<u64>, ServerError> {
        let mut running = self.running.lock().map_err(|_| ServerError::Poisoned)?;
        self.backend.stop();
        if let Some(worker) = running.as_mut() {
            if let Some(stop) = worker.stop.take() {
                let _ = stop.send(());
            }
            worker
                .finished
                .recv_timeout(SHUTDOWN_TIMEOUT)
                .map_err(|_| ServerError::Shutdown)?;
            if let Some(worker) = running.take() {
                worker.thread.join().map_err(|_| ServerError::Executor)?;
            }
        }
        let mut ids = self
            .backend
            .transfers
            .lock()
            .map_err(|_| ServerError::Poisoned)?
            .clear();
        ids.extend(
            self.backend
                .cleanup
                .lock()
                .map_err(|_| ServerError::Poisoned)?
                .drain(..),
        );
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }
    pub fn platform(&self) -> &Platform {
        &self.app.platform
    }
    pub fn token(&self) -> &str {
        &self.app.token
    }
    pub fn port(&self) -> u16 {
        self.app.port.load(Ordering::Acquire) as u16
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
impl App {
    fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port.load(Ordering::Acquire))
    }
}
async fn root(State(app): State<Arc<App>>) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        app.html.to_vec(),
    )
        .into_response()
}
async fn bootstrap(State(app): State<Arc<App>>) -> Response {
    // Binding the socket does not mean the process owner has published its
    // initial state. Clients must never receive a ready endpoint before that.
    if app.backend.snapshots.borrow().is_none() {
        return status(StatusCode::SERVICE_UNAVAILABLE);
    }
    let origin = app.origin();
    json_response(
        json!({"webSocketUrl":format!("ws://127.0.0.1:{}/api/ws",app.port.load(Ordering::Acquire)),
        "sessionDataUrl":format!("{origin}/api/session-data"),"sessionExportUrl":format!("{origin}/api/session-export"),
        "sessionImportUrl":format!("{origin}/api/session-import"),"sessionToken":app.token,"platform":app.platform}),
    )
}
pub(crate) fn json_response(value: Value) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        value.to_string(),
    )
        .into_response()
}
pub(crate) fn status(code: StatusCode) -> Response {
    (code, [(header::CACHE_CONTROL, "no-store")]).into_response()
}
fn single<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        None
    } else {
        Some(value)
    }
}
async fn authenticate(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    let token = if request.uri().path() == "/api/ws" {
        request
            .uri()
            .query()
            .and_then(|q| q.strip_prefix("token="))
            .filter(|v| !v.contains('&'))
    } else {
        single(request.headers(), "X-Float-View-Token")
    };
    if single(request.headers(), "Origin") != Some(app.origin().as_str())
        || !token.is_some_and(|t| constant_equal(t.as_bytes(), app.token.as_bytes()))
    {
        return status(StatusCode::FORBIDDEN);
    }
    match tokio::time::timeout(IO_TIMEOUT, next.run(request)).await {
        Ok(response) => response,
        Err(_) => status(StatusCode::REQUEST_TIMEOUT),
    }
}
fn constant_equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0, |diff, (a, b)| diff | (a ^ b))
            == 0
}
async fn websocket(State(app): State<Arc<App>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .max_frame_size(MAX_MESSAGE_BYTES)
        .max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| serve_websocket(app, socket))
}
async fn serve_websocket(app: Arc<App>, socket: WebSocket) {
    let Ok(client) = Client::new(&app.backend) else {
        return;
    };
    let client = Arc::new(client);
    let (mut sink, mut stream) = socket.split();
    let (responses, mut response_rx) = {
        let (tx, rx) = tokio::sync::mpsc::channel::<Value>(MAX_RESPONSE_QUEUE);
        (tx, rx)
    };
    let mut snapshots = app.backend.snapshots.subscribe();
    snapshots.mark_changed();
    let mut tasks = tokio::task::JoinSet::new();
    let permits = Arc::new(Semaphore::new(MAX_CLIENT_OPERATIONS));
    let reader = {
        let app = app.clone();
        let client = client.clone();
        async move {
            while let Some(Ok(message)) = stream.next().await {
                let Message::Text(text) = message else {
                    continue;
                };
                if text.len() > MAX_MESSAGE_BYTES {
                    break;
                }
                let Ok(request) = serde_json::from_str::<Value>(&text) else {
                    break;
                };
                let Ok(permit) = permits.clone().try_acquire_owned() else {
                    break;
                };
                let app = app.clone();
                let client = client.clone();
                let sender = responses.clone();
                tasks.spawn(async move {
                    let result = app.backend.request(client.id, request).await;
                    let _permit = permit;
                    let _ = sender.send(result).await;
                });
                while tasks.try_join_next().is_some() {}
            }
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
        }
    };
    tokio::pin!(reader);
    let mut heartbeat = tokio::time::interval(IO_TIMEOUT / 3);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let message = tokio::select! {
            _=&mut reader=>break,
            _=heartbeat.tick()=>{
                if !matches!(tokio::time::timeout(IO_TIMEOUT, sink.send(Message::Ping(Vec::new().into()))).await, Ok(Ok(()))) { break; }
                continue;
            },
            result=response_rx.recv()=>match result{Some(value)=>value,None=>break},
            changed=snapshots.changed()=>{if changed.is_err(){break;}let current=snapshots.borrow_and_update().clone();let Some(current)=current else{continue};json!({"event":"state.changed","data":current.as_ref()})},
        };
        let text = message.to_string();
        if text.len() > MAX_MESSAGE_BYTES {
            break;
        }
        if !matches!(
            tokio::time::timeout(IO_TIMEOUT, sink.send(Message::Text(text.into()))).await,
            Ok(Ok(()))
        ) {
            break;
        }
    }
}
pub(crate) async fn request_body(request: Request, maximum: usize) -> Result<Vec<u8>, StatusCode> {
    if request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|n| n > maximum as u64)
    {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    to_bytes(request.into_body(), maximum)
        .await
        .map(|b| b.to_vec())
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)
}
async fn session_data(State(app): State<Arc<App>>, request: Request) -> Response {
    let gzip = request
        .headers()
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|s| s.trim() == "gzip"));
    let Ok(permit) = app.backend.history.clone().try_acquire_owned() else {
        return status(StatusCode::SERVICE_UNAVAILABLE);
    };
    let permit = Arc::new(permit);
    let Ok(client) = Client::new(&app.backend) else {
        return status(StatusCode::SERVICE_UNAVAILABLE);
    };
    let bytes = match request_body(request, MAX_SESSION_DATA_REQUEST_BYTES).await {
        Ok(bytes) => bytes,
        Err(code) => return status(code),
    };
    let request = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => return status(StatusCode::BAD_REQUEST),
    };
    let result = match app
        .backend
        .history(client.id, request, permit.clone())
        .await
    {
        Ok(result) => result,
        Err(_) => return status(StatusCode::BAD_REQUEST),
    };
    let bytes = match crate::encoding::response(result, gzip, permit).await {
        Ok(bytes) => bytes,
        Err(()) => return status(StatusCode::PAYLOAD_TOO_LARGE),
    };
    if gzip {
        return (
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
                (header::CONTENT_ENCODING, "gzip"),
            ],
            bytes,
        )
            .into_response();
    }
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response()
}
struct BoundedListener {
    listener: TcpListener,
    permits: Arc<Semaphore>,
}
struct Socket {
    stream: TcpStream,
    _permit: OwnedSemaphorePermit,
    read_deadline: Pin<Box<Sleep>>,
    write_deadline: Pin<Box<Sleep>>,
}
impl axum::serve::Listener for BoundedListener {
    type Io = Socket;
    type Addr = SocketAddr;
    async fn accept(&mut self) -> (Socket, SocketAddr) {
        loop {
            match self.listener.accept().await {
                Ok((stream, address)) => {
                    if let Ok(permit) = self.permits.clone().try_acquire_owned() {
                        return (
                            Socket {
                                stream,
                                _permit: permit,
                                read_deadline: Box::pin(tokio::time::sleep(IO_TIMEOUT)),
                                write_deadline: Box::pin(tokio::time::sleep(IO_TIMEOUT)),
                            },
                            address,
                        );
                    }
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
            }
        }
    }
    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}
impl AsyncRead for Socket {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match Pin::new(&mut self.stream).poll_read(cx, buffer) {
            Poll::Pending => {
                if self.read_deadline.as_mut().poll(cx).is_ready() {
                    Poll::Ready(Err(io::ErrorKind::TimedOut.into()))
                } else {
                    Poll::Pending
                }
            }
            result => {
                self.read_deadline
                    .as_mut()
                    .reset(Instant::now() + IO_TIMEOUT);
                result
            }
        }
    }
}
impl AsyncWrite for Socket {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        match Pin::new(&mut self.stream).poll_write(cx, bytes) {
            Poll::Pending => {
                if self.write_deadline.as_mut().poll(cx).is_ready() {
                    Poll::Ready(Err(io::ErrorKind::TimedOut.into()))
                } else {
                    Poll::Pending
                }
            }
            result => {
                self.write_deadline
                    .as_mut()
                    .reset(Instant::now() + IO_TIMEOUT);
                result
            }
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}
