//! Process-owned portable backend bridge and minimal serialized actor.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use app_backend::{BackendRuntime, BackendRuntimeConfig, resources_with_index};
use app_coordinator::{ActorError, ApplicationActor, SystemClock, SystemRandom};
use domain_types::{OperationId, RuntimeGeneration};
use host_ports::{
    AdapterEvent, BluetoothNotification, CompletionStatus, HostCommand as PlatformHostCommand,
    MAX_RESOURCE_BYTES, OperationCompletion, PlatformEvent, ResourceNamespace, StorageRootKind,
};

/// Private native ABI version. Hosts and the library are shipped atomically.
pub const ABI_VERSION: u32 = 2;
/// Source identity used by initial smoke products.
pub const BUILD_ID: &str = env!("CARGO_PKG_VERSION");
/// Fixed build identity copied into caller-owned ABI buffers.
pub const BUILD_ID_BYTES: &[u8; 16] = b"float-view-0.1.0";
/// Maximum queued host commands.
pub const COMMAND_QUEUE_ITEMS: usize = 64;
/// Maximum aggregate queued host-command payload bytes.
pub const COMMAND_QUEUE_BYTES: usize = 1024 * 1024;
/// Maximum serialized actor messages awaiting dispatch.
pub const ACTOR_QUEUE_ITEMS: usize = 64;
/// Maximum recent completions retained solely to distinguish duplicate replies.
pub const COMPLETED_OPERATION_HISTORY: usize = 128;
/// Command-header kind whose payload is an encoded [`PlatformCommand`].
pub const HOST_COMMAND_KIND_PLATFORM: u32 = 1;
/// Event kind whose payload is an encoded [`PlatformEvent`].
pub const EVENT_KIND_PLATFORM: u32 = 1;
const SYNTHETIC_OPERATION_START: u64 = 1 << 63;

pub type Generation = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    InvalidArgument,
    WrongState,
    Poisoned,
    QueueFull,
    StaleGeneration,
    DuplicateOperation,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Lifecycle {
    Created,
    Started,
    Stopped,
    Poisoned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostCommand {
    pub kind: u32,
    pub flags: u32,
    pub generation: Generation,
    pub operation_id: u64,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaitCommand {
    Ready,
    Required(usize),
    Timeout,
    Stopped,
}

struct CommandState {
    queue: VecDeque<HostCommand>,
    queued_bytes: usize,
    pending: HashSet<u64>,
    completed: VecDeque<u64>,
    closed: bool,
}

struct CommandQueue {
    state: Mutex<CommandState>,
    changed: Condvar,
}

impl CommandQueue {
    fn new() -> Self {
        Self {
            state: Mutex::new(CommandState {
                queue: VecDeque::new(),
                queued_bytes: 0,
                pending: HashSet::new(),
                completed: VecDeque::new(),
                closed: false,
            }),
            changed: Condvar::new(),
        }
    }

    fn open(&self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::Poisoned)?;
        state.closed = false;
        Ok(())
    }

    fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            state.pending.clear();
            self.changed.notify_all();
        }
    }

    fn enqueue(&self, command: HostCommand) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::Poisoned)?;
        let Some(new_bytes) = state.queued_bytes.checked_add(command.payload.len()) else {
            return Err(RuntimeError::QueueFull);
        };
        if state.closed
            || state.queue.len() >= COMMAND_QUEUE_ITEMS
            || new_bytes > COMMAND_QUEUE_BYTES
        {
            return Err(RuntimeError::QueueFull);
        }
        if state.pending.contains(&command.operation_id)
            || state.completed.contains(&command.operation_id)
        {
            return Err(RuntimeError::DuplicateOperation);
        }
        state.queued_bytes = new_bytes;
        state.pending.insert(command.operation_id);
        state.queue.push_back(command);
        self.changed.notify_one();
        Ok(())
    }

    fn wait(
        &self,
        timeout: Duration,
        payload_capacity: usize,
    ) -> Result<(WaitCommand, Option<HostCommand>), RuntimeError> {
        let state = self.state.lock().map_err(|_| RuntimeError::Poisoned)?;
        let (mut state, _) = self
            .changed
            .wait_timeout_while(state, timeout, |state| {
                state.queue.is_empty() && !state.closed
            })
            .map_err(|_| RuntimeError::Poisoned)?;
        let Some(front) = state.queue.front() else {
            return Ok((
                if state.closed {
                    WaitCommand::Stopped
                } else {
                    WaitCommand::Timeout
                },
                None,
            ));
        };
        if payload_capacity < front.payload.len() {
            return Ok((WaitCommand::Required(front.payload.len()), None));
        }
        let command = state.queue.pop_front().ok_or(RuntimeError::Internal)?;
        state.queued_bytes -= command.payload.len();
        Ok((WaitCommand::Ready, Some(command)))
    }

    fn complete(&self, operation_id: u64) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::Poisoned)?;
        if state.completed.contains(&operation_id) {
            return Err(RuntimeError::DuplicateOperation);
        }
        if !state.pending.remove(&operation_id) {
            return Err(RuntimeError::StaleGeneration);
        }
        if state.completed.len() == COMPLETED_OPERATION_HISTORY {
            state.completed.pop_front();
        }
        state.completed.push_back(operation_id);
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum BootstrapInput {
    DurableRoot,
    FrontendIndex,
    SecureRandom,
}

#[derive(Default)]
struct BootstrapState {
    pending: HashMap<u64, BootstrapInput>,
    durable_root: Option<PathBuf>,
    frontend_index: Option<Arc<[u8]>>,
    random: Option<Vec<u8>>,
}

impl BootstrapState {
    fn ready(&self) -> bool {
        self.pending.is_empty()
            && self.durable_root.is_some()
            && self.frontend_index.is_some()
            && self.random.is_some()
    }
}

struct BackendWorker {
    controls: SyncSender<BackendControl>,
    worker: JoinHandle<Result<(), String>>,
}

enum BackendControl {
    Suspend(SyncSender<Result<(), String>>),
    Resume(SyncSender<Result<String, String>>),
    Stop,
}

impl BackendWorker {
    fn start(config: BackendRuntimeConfig) -> Result<(Self, String), RuntimeError> {
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let (controls, control_receiver) = mpsc::sync_channel(4);
        let worker = thread::Builder::new()
            .name("app-backend".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| error.to_string())?;
                runtime.block_on(async move {
                    let mut backend = match BackendRuntime::start(config).await {
                        Ok(backend) => backend,
                        Err(error) => {
                            let _ = ready_sender.send(Err(error.clone()));
                            return Err(error);
                        }
                    };
                    let endpoint = format!("http://{}", backend.address());
                    let _ = ready_sender.send(Ok(endpoint));
                    loop {
                        match control_receiver.recv() {
                            Ok(BackendControl::Suspend(reply)) => {
                                backend.suspend().await;
                                let _ = reply.send(Ok(()));
                            }
                            Ok(BackendControl::Resume(reply)) => {
                                let result = backend
                                    .resume()
                                    .await
                                    .map(|address| format!("http://{address}"));
                                let _ = reply.send(result);
                            }
                            Ok(BackendControl::Stop) | Err(_) => return backend.stop().await,
                        }
                    }
                })
            })
            .map_err(|_| RuntimeError::Internal)?;
        let endpoint = ready_receiver
            .recv()
            .map_err(|_| RuntimeError::Internal)?
            .map_err(|_| RuntimeError::Internal)?;
        Ok((Self { controls, worker }, endpoint))
    }

    fn suspend(&self) -> Result<(), RuntimeError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.controls
            .send(BackendControl::Suspend(reply))
            .map_err(|_| RuntimeError::WrongState)?;
        result
            .recv()
            .map_err(|_| RuntimeError::Internal)?
            .map_err(|_| RuntimeError::Internal)
    }

    fn resume(&self) -> Result<String, RuntimeError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.controls
            .send(BackendControl::Resume(reply))
            .map_err(|_| RuntimeError::WrongState)?;
        result
            .recv()
            .map_err(|_| RuntimeError::Internal)?
            .map_err(|_| RuntimeError::Internal)
    }

    fn stop(self) -> Result<(), RuntimeError> {
        let _ = self.controls.send(BackendControl::Stop);
        self.worker
            .join()
            .map_err(|_| RuntimeError::Poisoned)?
            .map_err(|_| RuntimeError::Internal)
    }
}

pub struct Runtime {
    generation: Generation,
    lifecycle: Mutex<Lifecycle>,
    commands: Arc<CommandQueue>,
    next_synthetic_operation: AtomicU64,
    application: Mutex<Option<Arc<ApplicationActor>>>,
    bootstrap: Mutex<Option<BootstrapState>>,
    backend: Mutex<Option<BackendWorker>>,
    launch_token: Mutex<Option<String>>,
    requested_port: AtomicU16,
}

impl Runtime {
    #[must_use]
    pub fn new(generation: Generation) -> Self {
        Self {
            generation,
            lifecycle: Mutex::new(Lifecycle::Created),
            commands: Arc::new(CommandQueue::new()),
            next_synthetic_operation: AtomicU64::new(SYNTHETIC_OPERATION_START),
            application: Mutex::new(None),
            bootstrap: Mutex::new(None),
            backend: Mutex::new(None),
            launch_token: Mutex::new(None),
            requested_port: AtomicU16::new(0),
        }
    }

    #[must_use]
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Starts exactly one serialized application worker.
    ///
    /// # Errors
    /// Returns an error unless the runtime is newly created or if synchronization fails.
    pub fn start(&self) -> Result<(), RuntimeError> {
        self.start_on_port(0)
    }

    /// Starts exactly one serialized application worker and requests a loopback port.
    /// Port zero selects an ephemeral port.
    ///
    /// # Errors
    /// Returns an error unless the runtime is newly created or if synchronization fails.
    pub fn start_on_port(&self, requested_port: u16) -> Result<(), RuntimeError> {
        let mut lifecycle = self.lifecycle.lock().map_err(|_| RuntimeError::Poisoned)?;
        if *lifecycle != Lifecycle::Created {
            return Err(if *lifecycle == Lifecycle::Poisoned {
                RuntimeError::Poisoned
            } else {
                RuntimeError::WrongState
            });
        }
        self.commands.open()?;
        self.requested_port.store(requested_port, Ordering::Release);
        let application = ApplicationActor::start(
            RuntimeGeneration(self.generation),
            SystemClock::new(),
            SystemRandom,
        )
        .map_err(map_actor)?;
        *self
            .application
            .lock()
            .map_err(|_| RuntimeError::Poisoned)? = Some(application);
        *lifecycle = Lifecycle::Started;
        Ok(())
    }

    /// Sends copied input through the bounded serialized actor.
    ///
    /// # Errors
    /// Returns a stable lifecycle, actor-capacity, or command-capacity failure.
    pub fn dispatch(&self, kind: u32, payload: &[u8]) -> Result<u64, RuntimeError> {
        self.require_started()?;
        let operation_id = self
            .next_synthetic_operation
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| RuntimeError::Internal)?;
        self.commands.enqueue(HostCommand {
            kind,
            flags: 0,
            generation: self.generation,
            operation_id,
            payload: payload.to_vec(),
        })?;
        Ok(operation_id)
    }

    /// Decodes and submits one bounded platform event to the application actor.
    ///
    /// # Errors
    /// Returns stable lifecycle, codec, ordering, generation, or actor-capacity failures.
    pub fn submit_platform_event(&self, payload: &[u8]) -> Result<(), RuntimeError> {
        self.require_started()?;
        let event = PlatformEvent::decode(payload).map_err(|_| RuntimeError::InvalidArgument)?;
        let foreground = match &event.event {
            AdapterEvent::Lifecycle { foreground, .. } => Some(*foreground),
            _ => None,
        };
        self.application()?
            .submit_platform_event(event)
            .map_err(map_actor)?;
        if let Some(foreground) = foreground {
            self.update_loopback_lifecycle(foreground)?;
        }
        self.ensure_bootstrap()
    }

    /// Copies and submits one ordered Bluetooth notification to the application actor.
    ///
    /// # Errors
    /// Returns stable lifecycle, bounds, ordering, generation, or actor-capacity failures.
    pub fn submit_bluetooth(
        &self,
        connection_generation: u64,
        adapter_order: u64,
        bytes: &[u8],
    ) -> Result<(), RuntimeError> {
        self.require_started()?;
        self.application()?
            .submit_bluetooth(BluetoothNotification {
                generation: RuntimeGeneration(self.generation),
                connection_generation,
                adapter_order,
                bytes: bytes.to_vec(),
            })
            .map_err(map_actor)
    }

    /// Enqueues a typed platform command through the production application actor.
    ///
    /// # Errors
    /// Returns stable lifecycle, validation, pending-operation, or queue-capacity failures.
    pub fn request_host(&self, command: PlatformHostCommand) -> Result<u64, RuntimeError> {
        self.require_started()?;
        let operation = self
            .application()?
            .request_host(command)
            .map_err(map_actor)?;
        self.drain_application_commands()?;
        Ok(operation.0)
    }

    /// Retrieves one command without dequeueing on insufficient capacity.
    ///
    /// # Errors
    /// Returns a stable lifecycle or synchronization failure.
    pub fn wait_command(
        &self,
        timeout: Duration,
        payload_capacity: usize,
    ) -> Result<(WaitCommand, Option<HostCommand>), RuntimeError> {
        self.require_started_or_stopped()?;
        if self.require_started().is_ok() {
            self.drain_application_commands()?;
        }
        self.commands.wait(timeout, payload_capacity)
    }

    /// Accepts an operation completion at most once.
    ///
    /// # Errors
    /// Returns duplicate/stale operation or lifecycle failures.
    pub fn complete(&self, operation_id: u64) -> Result<(), RuntimeError> {
        self.require_started()?;
        self.commands.complete(operation_id)
    }

    /// Accepts a typed platform operation completion at most once.
    ///
    /// # Errors
    /// Returns stable lifecycle, result-kind, generation, or operation-lifecycle failures.
    pub fn complete_platform(
        &self,
        operation_id: u64,
        result_kind: u32,
        payload: &[u8],
    ) -> Result<(), RuntimeError> {
        self.require_started()?;
        let status = match result_kind {
            0 => CompletionStatus::Success,
            1 => CompletionStatus::Cancelled,
            2 => CompletionStatus::PermissionDenied,
            3 => CompletionStatus::Unavailable,
            4 => CompletionStatus::Failed,
            _ => return Err(RuntimeError::InvalidArgument),
        };
        self.application()?
            .complete(OperationCompletion {
                generation: RuntimeGeneration(self.generation),
                operation: OperationId(operation_id),
                status,
                payload: payload.to_vec(),
            })
            .map_err(map_actor)?;
        self.commands.complete(operation_id)?;
        self.advance_bootstrap(operation_id, status, payload)
    }

    /// Stops the actor, closes the command queue, and wakes blocked waiters.
    ///
    /// # Errors
    /// Returns an error unless currently started or if the worker cannot join.
    pub fn stop(&self) -> Result<(), RuntimeError> {
        {
            let lifecycle = self.lifecycle.lock().map_err(|_| RuntimeError::Poisoned)?;
            if *lifecycle != Lifecycle::Started {
                return Err(if *lifecycle == Lifecycle::Poisoned {
                    RuntimeError::Poisoned
                } else {
                    RuntimeError::WrongState
                });
            }
        }
        let backend = self
            .backend
            .lock()
            .map_err(|_| RuntimeError::Poisoned)?
            .take();
        if let Some(backend) = backend {
            backend.stop()?;
            let _ = self
                .application
                .lock()
                .map_err(|_| RuntimeError::Poisoned)?
                .take();
        } else if let Some(application) = self
            .application
            .lock()
            .map_err(|_| RuntimeError::Poisoned)?
            .take()
        {
            application.stop().map_err(map_actor)?;
        }
        *self.bootstrap.lock().map_err(|_| RuntimeError::Poisoned)? = None;
        *self
            .launch_token
            .lock()
            .map_err(|_| RuntimeError::Poisoned)? = None;
        self.commands.close();
        *self.lifecycle.lock().map_err(|_| RuntimeError::Poisoned)? = Lifecycle::Stopped;
        Ok(())
    }

    /// Reports whether host bridge work has detached and destruction is safe.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Poisoned`] after an internal panic.
    pub fn can_destroy(&self) -> Result<bool, RuntimeError> {
        let lifecycle = self.lifecycle.lock().map_err(|_| RuntimeError::Poisoned)?;
        match *lifecycle {
            Lifecycle::Created | Lifecycle::Stopped | Lifecycle::Poisoned => Ok(true),
            Lifecycle::Started => Ok(false),
        }
    }

    pub fn poison(&self) {
        if let Ok(mut lifecycle) = self.lifecycle.lock() {
            *lifecycle = Lifecycle::Poisoned;
        }
        self.commands.close();
        if let Ok(mut application) = self.application.lock()
            && let Some(application) = application.take()
        {
            let _ = application.stop();
        }
    }

    fn require_started(&self) -> Result<(), RuntimeError> {
        let lifecycle = self.lifecycle.lock().map_err(|_| RuntimeError::Poisoned)?;
        match *lifecycle {
            Lifecycle::Started => Ok(()),
            Lifecycle::Poisoned => Err(RuntimeError::Poisoned),
            Lifecycle::Created | Lifecycle::Stopped => Err(RuntimeError::WrongState),
        }
    }

    fn require_started_or_stopped(&self) -> Result<(), RuntimeError> {
        let lifecycle = self.lifecycle.lock().map_err(|_| RuntimeError::Poisoned)?;
        match *lifecycle {
            Lifecycle::Started | Lifecycle::Stopped => Ok(()),
            Lifecycle::Poisoned => Err(RuntimeError::Poisoned),
            Lifecycle::Created => Err(RuntimeError::WrongState),
        }
    }

    fn application(&self) -> Result<Arc<ApplicationActor>, RuntimeError> {
        self.application
            .lock()
            .map_err(|_| RuntimeError::Poisoned)?
            .as_ref()
            .cloned()
            .ok_or(RuntimeError::WrongState)
    }

    fn drain_application_commands(&self) -> Result<(), RuntimeError> {
        let application = self.application()?;
        while let Some(command) = application.try_host_command().map_err(map_actor)? {
            let payload = command.encode().map_err(|_| RuntimeError::Internal)?;
            self.commands.enqueue(HostCommand {
                kind: HOST_COMMAND_KIND_PLATFORM,
                flags: 0,
                generation: self.generation,
                operation_id: command.operation.0,
                payload,
            })?;
        }
        Ok(())
    }

    fn ensure_bootstrap(&self) -> Result<(), RuntimeError> {
        let mut bootstrap = self.bootstrap.lock().map_err(|_| RuntimeError::Poisoned)?;
        if bootstrap.is_some()
            || self
                .backend
                .lock()
                .map_err(|_| RuntimeError::Poisoned)?
                .is_some()
        {
            return Ok(());
        }
        let application = self.application()?;
        let requests = [
            (
                PlatformHostCommand::ResolveStorageRoot {
                    kind: StorageRootKind::Durable,
                },
                BootstrapInput::DurableRoot,
            ),
            (
                PlatformHostCommand::ReadResource {
                    namespace: ResourceNamespace::Frontend,
                    logical_name: "index.html".into(),
                    maximum_bytes: MAX_RESOURCE_BYTES,
                },
                BootstrapInput::FrontendIndex,
            ),
            (
                PlatformHostCommand::SecureRandom { count: 32 },
                BootstrapInput::SecureRandom,
            ),
        ];
        let mut state = BootstrapState::default();
        for (command, input) in requests {
            let operation = application.request_host(command).map_err(map_actor)?;
            state.pending.insert(operation.0, input);
        }
        *bootstrap = Some(state);
        drop(bootstrap);
        self.drain_application_commands()
    }

    fn advance_bootstrap(
        &self,
        operation_id: u64,
        status: CompletionStatus,
        payload: &[u8],
    ) -> Result<(), RuntimeError> {
        let config = {
            let mut bootstrap = self.bootstrap.lock().map_err(|_| RuntimeError::Poisoned)?;
            let Some(state) = bootstrap.as_mut() else {
                return Ok(());
            };
            let Some(input) = state.pending.remove(&operation_id) else {
                return Ok(());
            };
            if status != CompletionStatus::Success {
                return Err(RuntimeError::Internal);
            }
            match input {
                BootstrapInput::DurableRoot => {
                    let value =
                        std::str::from_utf8(payload).map_err(|_| RuntimeError::InvalidArgument)?;
                    let path = PathBuf::from(value);
                    if value.is_empty() || value.len() > 1024 || !path.is_absolute() {
                        return Err(RuntimeError::InvalidArgument);
                    }
                    state.durable_root = Some(path);
                }
                BootstrapInput::FrontendIndex => {
                    if payload.is_empty()
                        || u64::try_from(payload.len()).unwrap_or(u64::MAX) > MAX_RESOURCE_BYTES
                    {
                        return Err(RuntimeError::InvalidArgument);
                    }
                    state.frontend_index = Some(Arc::from(payload));
                }
                BootstrapInput::SecureRandom => {
                    if payload.len() != 32 {
                        return Err(RuntimeError::InvalidArgument);
                    }
                    state.random = Some(payload.to_vec());
                }
            }
            if !state.ready() {
                return Ok(());
            }
            let mut token = String::with_capacity(64);
            for byte in state.random.take().ok_or(RuntimeError::Internal)? {
                write!(&mut token, "{byte:02x}").map_err(|_| RuntimeError::Internal)?;
            }
            BackendRuntimeConfig {
                generation: self.generation,
                requested_port: self.requested_port.load(Ordering::Acquire),
                storage_root: state.durable_root.take().ok_or(RuntimeError::Internal)?,
                platform: platform_name().into(),
                launch_token: token,
                resources: resources_with_index(
                    state.frontend_index.take().ok_or(RuntimeError::Internal)?,
                ),
                application: Some(self.application()?),
            }
        };
        let launch_token = config.launch_token.clone();
        let (worker, endpoint) = BackendWorker::start(config)?;
        *self.backend.lock().map_err(|_| RuntimeError::Poisoned)? = Some(worker);
        *self
            .launch_token
            .lock()
            .map_err(|_| RuntimeError::Poisoned)? = Some(launch_token.clone());
        *self.bootstrap.lock().map_err(|_| RuntimeError::Poisoned)? = None;
        self.request_host(PlatformHostCommand::RuntimeReady {
            endpoint,
            launch_token,
        })?;
        Ok(())
    }

    fn update_loopback_lifecycle(&self, foreground: bool) -> Result<(), RuntimeError> {
        let endpoint = {
            let backend = self.backend.lock().map_err(|_| RuntimeError::Poisoned)?;
            let Some(backend) = backend.as_ref() else {
                return Ok(());
            };
            if foreground {
                Some(backend.resume()?)
            } else {
                backend.suspend()?;
                None
            }
        };
        if let Some(endpoint) = endpoint {
            let launch_token = self
                .launch_token
                .lock()
                .map_err(|_| RuntimeError::Poisoned)?
                .clone()
                .ok_or(RuntimeError::Internal)?;
            self.request_host(PlatformHostCommand::RuntimeReady {
                endpoint,
                launch_token,
            })?;
        }
        Ok(())
    }
}

const fn platform_name() -> &'static str {
    if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else {
        "macos"
    }
}

fn map_actor(error: ActorError) -> RuntimeError {
    match error {
        ActorError::QueueFull => RuntimeError::QueueFull,
        ActorError::Stopped => RuntimeError::WrongState,
        ActorError::StaleGeneration
        | ActorError::StaleConnection
        | ActorError::StaleOrder
        | ActorError::StaleOperation => RuntimeError::StaleGeneration,
        ActorError::DuplicateOperation => RuntimeError::DuplicateOperation,
        ActorError::InvalidInput => RuntimeError::InvalidArgument,
        ActorError::Internal => RuntimeError::Internal,
    }
}

#[must_use]
pub const fn identity() -> (u32, &'static str) {
    (ABI_VERSION, BUILD_ID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use host_ports::{
        AdapterEvent, BluetoothAuthorization, BluetoothRadio, HostCommand as PlatformHostCommand,
        PlatformCommand,
    };

    #[test]
    fn actor_dispatches_and_completes_once() {
        let runtime = Runtime::new(7);
        runtime.start().unwrap();
        let operation = runtime.dispatch(42, b"copied").unwrap();
        let (outcome, command) = runtime.wait_command(Duration::ZERO, 32).unwrap();
        assert_eq!(outcome, WaitCommand::Ready);
        let command = command.unwrap();
        assert_eq!(command.operation_id, operation);
        assert_eq!(command.payload, b"copied");
        assert_eq!(runtime.complete(operation), Ok(()));
        assert_eq!(
            runtime.complete(operation),
            Err(RuntimeError::DuplicateOperation)
        );
        runtime.stop().unwrap();
        assert_eq!(runtime.can_destroy(), Ok(true));
    }

    #[test]
    fn insufficient_capacity_does_not_dequeue() {
        let runtime = Runtime::new(8);
        runtime.start().unwrap();
        runtime.dispatch(1, b"payload").unwrap();
        assert_eq!(
            runtime.wait_command(Duration::ZERO, 2).unwrap().0,
            WaitCommand::Required(7)
        );
        assert_eq!(
            runtime.wait_command(Duration::ZERO, 7).unwrap().0,
            WaitCommand::Ready
        );
        runtime.stop().unwrap();
    }

    #[test]
    fn stop_wakes_waiter() {
        let runtime = Arc::new(Runtime::new(9));
        runtime.start().unwrap();
        let waiter_runtime = Arc::clone(&runtime);
        let waiter = thread::spawn(move || {
            waiter_runtime
                .wait_command(Duration::from_secs(30), 0)
                .unwrap()
                .0
        });
        thread::sleep(Duration::from_millis(20));
        runtime.stop().unwrap();
        assert_eq!(waiter.join().unwrap(), WaitCommand::Stopped);
    }

    #[test]
    fn queue_item_and_byte_bounds_are_stable() {
        let queue = CommandQueue::new();
        queue.open().unwrap();
        for operation_id in 1..=COMMAND_QUEUE_ITEMS as u64 {
            queue
                .enqueue(HostCommand {
                    kind: 1,
                    flags: 0,
                    generation: 1,
                    operation_id,
                    payload: Vec::new(),
                })
                .unwrap();
        }
        assert_eq!(
            queue.enqueue(HostCommand {
                kind: 1,
                flags: 0,
                generation: 1,
                operation_id: 100,
                payload: Vec::new(),
            }),
            Err(RuntimeError::QueueFull)
        );

        let byte_queue = CommandQueue::new();
        byte_queue.open().unwrap();
        assert_eq!(
            byte_queue.enqueue(HostCommand {
                kind: 1,
                flags: 0,
                generation: 1,
                operation_id: 1,
                payload: vec![0; COMMAND_QUEUE_BYTES + 1],
            }),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn completed_command_replay_history_is_bounded() {
        let queue = CommandQueue::new();
        queue.open().unwrap();
        for operation_id in 1..=COMPLETED_OPERATION_HISTORY as u64 + 1 {
            queue
                .enqueue(HostCommand {
                    kind: 1,
                    flags: 0,
                    generation: 1,
                    operation_id,
                    payload: Vec::new(),
                })
                .unwrap();
            queue.wait(Duration::ZERO, 0).unwrap();
            queue.complete(operation_id).unwrap();
        }
        assert_eq!(queue.complete(1), Err(RuntimeError::StaleGeneration));
        assert_eq!(
            queue.complete(COMPLETED_OPERATION_HISTORY as u64 + 1),
            Err(RuntimeError::DuplicateOperation)
        );
    }

    #[test]
    fn platform_events_enforce_generation_and_adapter_order() {
        let runtime = Runtime::new(10);
        runtime.start().unwrap();
        let event = PlatformEvent {
            generation: RuntimeGeneration(10),
            connection_generation: 0,
            adapter_order: 1,
            event: AdapterEvent::BluetoothState {
                radio: BluetoothRadio::PoweredOn,
                authorization: BluetoothAuthorization::Granted,
            },
        }
        .encode()
        .unwrap();
        assert_eq!(runtime.submit_platform_event(&event), Ok(()));
        assert_eq!(
            runtime.submit_platform_event(&event),
            Err(RuntimeError::StaleGeneration)
        );
        runtime.stop().unwrap();
    }

    #[test]
    fn typed_platform_commands_cross_the_bounded_queue() {
        let runtime = Runtime::new(11);
        runtime.start().unwrap();
        let operation = runtime
            .request_host(PlatformHostCommand::BluetoothStartScan)
            .unwrap();
        let (outcome, command) = runtime.wait_command(Duration::ZERO, 128).unwrap();
        assert_eq!(outcome, WaitCommand::Ready);
        let command = command.unwrap();
        assert_eq!(command.kind, HOST_COMMAND_KIND_PLATFORM);
        assert_eq!(command.operation_id, operation);
        assert_eq!(
            PlatformCommand::decode(&command.payload).unwrap().command,
            PlatformHostCommand::BluetoothStartScan
        );
        assert_eq!(runtime.complete_platform(operation, 0, &[]), Ok(()));
        assert_eq!(
            runtime.complete_platform(operation, 0, &[]),
            Err(RuntimeError::DuplicateOperation)
        );
        runtime.stop().unwrap();
    }

    #[test]
    fn typed_host_bootstrap_starts_the_shared_production_composition() {
        let runtime = Runtime::new(12);
        runtime.start().unwrap();
        let event = PlatformEvent {
            generation: RuntimeGeneration(12),
            connection_generation: 0,
            adapter_order: 1,
            event: AdapterEvent::Lifecycle {
                foreground: true,
                background_eligible: false,
            },
        }
        .encode()
        .unwrap();
        runtime.submit_platform_event(&event).unwrap();

        let root = std::env::temp_dir().join(format!(
            "host-runtime-bootstrap-{}-{}",
            std::process::id(),
            runtime.generation()
        ));
        let _ = std::fs::remove_dir_all(&root);
        for _ in 0..3 {
            let (_, command) = runtime.wait_command(Duration::ZERO, 4096).unwrap();
            let command = command.unwrap();
            let decoded = PlatformCommand::decode(&command.payload).unwrap();
            let payload = match decoded.command {
                PlatformHostCommand::ResolveStorageRoot { .. } => {
                    root.to_string_lossy().as_bytes().to_vec()
                }
                PlatformHostCommand::ReadResource { .. } => b"<!doctype html>".to_vec(),
                PlatformHostCommand::SecureRandom { count } => vec![7; usize::from(count)],
                other => panic!("unexpected bootstrap command: {other:?}"),
            };
            runtime
                .complete_platform(command.operation_id, 0, &payload)
                .unwrap();
        }

        let (_, ready) = runtime.wait_command(Duration::from_secs(1), 4096).unwrap();
        let ready = ready.unwrap();
        let decoded = PlatformCommand::decode(&ready.payload).unwrap();
        match decoded.command {
            PlatformHostCommand::RuntimeReady {
                endpoint,
                launch_token,
            } => {
                assert!(endpoint.starts_with("http://127.0.0.1:"));
                assert_eq!(launch_token, "07".repeat(32));
            }
            other => panic!("unexpected readiness command: {other:?}"),
        }
        runtime
            .complete_platform(ready.operation_id, 0, &[])
            .unwrap();
        for (order, foreground) in [(2, false), (3, true)] {
            let lifecycle = PlatformEvent {
                generation: RuntimeGeneration(12),
                connection_generation: 0,
                adapter_order: order,
                event: AdapterEvent::Lifecycle {
                    foreground,
                    background_eligible: false,
                },
            }
            .encode()
            .unwrap();
            runtime.submit_platform_event(&lifecycle).unwrap();
        }
        let (_, resumed) = runtime.wait_command(Duration::from_secs(1), 4096).unwrap();
        let resumed = resumed.unwrap();
        let resumed = PlatformCommand::decode(&resumed.payload).unwrap();
        assert!(matches!(
            resumed.command,
            PlatformHostCommand::RuntimeReady { .. }
        ));
        runtime
            .complete_platform(resumed.operation.0, 0, &[])
            .unwrap();
        runtime.stop().unwrap();
        let _ = std::fs::remove_dir_all(root);
    }
}
