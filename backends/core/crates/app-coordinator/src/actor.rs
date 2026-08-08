use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle, Thread};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use domain_types::{OperationId, RuntimeGeneration};
use host_ports::{
    AdapterEvent, BluetoothAuthorization, BluetoothConnectionState, BluetoothNotification,
    BluetoothRadio, HostCommand, LocationAuthorization, OperationCompletion, PlatformCommand,
    PlatformEvent,
};
use protocol_refloat::Telemetry;
use protocol_refloat::{ConfigurationSchema, ConfigurationValue, TuneIdentity};
use protocol_vesc_bms::{BmsConnection, BmsSnapshot};

use crate::{
    AdaptiveSamplingPolicy, ApplicationModel, AuthoritativeState, ConfigurationContext,
    ConfigurationEffect, ConfigurationSummary, ConnectionAction, ConnectionCoordinator,
    RequestCompletion, RequestSpec, SamplingSettings, TrackingMode,
};

pub const INGRESS_CHANNEL_ITEMS: usize = 128;
pub const HOST_COMMAND_CHANNEL_ITEMS: usize = 64;
pub const STORAGE_CHANNEL_ITEMS: usize = 32;
pub const SERVER_CHANNEL_ITEMS: usize = 64;
pub const MAX_PENDING_OPERATIONS: usize = 64;
pub const MAX_DISCOVERED_DEVICES: usize = 64;
pub const COMPLETED_OPERATION_HISTORY: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorError {
    QueueFull,
    Stopped,
    StaleGeneration,
    StaleConnection,
    StaleOrder,
    StaleOperation,
    DuplicateOperation,
    InvalidInput,
    Internal,
}

pub trait Clock: Send + 'static {
    fn monotonic_millis(&mut self) -> u64;
    fn wall_millis(&mut self) -> i64;
}

pub trait RandomSource: Send + 'static {
    fn fill(&mut self, output: &mut [u8]);
}

#[derive(Debug)]
pub struct SystemClock {
    started: Instant,
}

impl SystemClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn monotonic_millis(&mut self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn wall_millis(&mut self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemRandom;

impl RandomSource for SystemRandom {
    fn fill(&mut self, output: &mut [u8]) {
        getrandom::fill(output).expect("operating-system random source is unavailable");
    }
}

#[derive(Clone, Debug)]
pub struct DeterministicClock {
    monotonic: u64,
    wall: i64,
    step: u64,
}

impl DeterministicClock {
    #[must_use]
    pub const fn new(monotonic: u64, wall: i64, step: u64) -> Self {
        Self {
            monotonic,
            wall,
            step,
        }
    }
}

impl Clock for DeterministicClock {
    fn monotonic_millis(&mut self) -> u64 {
        let value = self.monotonic;
        self.monotonic = self.monotonic.saturating_add(self.step);
        value
    }

    fn wall_millis(&mut self) -> i64 {
        self.wall
    }
}

#[derive(Clone, Debug)]
pub struct DeterministicRandom {
    state: u64,
}

impl DeterministicRandom {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }
}

impl RandomSource for DeterministicRandom {
    fn fill(&mut self, output: &mut [u8]) {
        for byte in output {
            self.state ^= self.state << 13;
            self.state ^= self.state >> 7;
            self.state ^= self.state << 17;
            *byte = self.state.to_le_bytes()[0];
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageWork {
    Open,
    Flush,
    Query { request_id: u64, payload: Vec<u8> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageResult {
    pub request_id: u64,
    pub succeeded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredBluetoothDevice {
    pub device_token: Vec<u8>,
    pub name: Option<String>,
    pub rssi: i16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationSnapshot {
    pub generation: RuntimeGeneration,
    pub revision: u64,
    pub foreground: bool,
    pub background_eligible: bool,
    pub bluetooth_radio: BluetoothRadio,
    pub bluetooth_authorization: BluetoothAuthorization,
    pub bluetooth_ready: bool,
    pub bluetooth_connection: Option<BluetoothConnectionState>,
    pub discovered_devices: Vec<DiscoveredBluetoothDevice>,
    pub connection_generation: u64,
    pub location_authorization: LocationAuthorization,
    pub location_sequence: u64,
    pub latest_location: Option<(u64, u64, Option<u64>)>,
    pub notification_count: u64,
    pub pending_operations: usize,
    pub monotonic_millis: u64,
    pub wall_millis: i64,
    pub instance_nonce: [u8; 16],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerPublication {
    StateChanged(ApplicationSnapshot),
    StorageCompleted(StorageResult),
    VescCompleted(RequestCompletion),
}

enum ActorInput {
    PlatformEvent(PlatformEvent, SyncSender<Result<(), ActorError>>),
    Bluetooth(BluetoothNotification, SyncSender<Result<(), ActorError>>),
    Completion(OperationCompletion, SyncSender<Result<(), ActorError>>),
    RequestHost(HostCommand, SyncSender<Result<OperationId, ActorError>>),
    RequestStorage(StorageWork, SyncSender<Result<(), ActorError>>),
    StorageResult(StorageResult, SyncSender<Result<(), ActorError>>),
    BeginVescGeneration(SyncSender<Result<u64, ActorError>>),
    ScheduleVesc(
        RequestSpec,
        SyncSender<Result<Vec<RequestCompletion>, ActorError>>,
    ),
    TickVesc(u64, SyncSender<Result<Vec<RequestCompletion>, ActorError>>),
    TickVescNow(SyncSender<Result<Vec<RequestCompletion>, ActorError>>),
    StartTracking(TrackingMode, SyncSender<Result<(), ActorError>>),
    ConfigureSampling(SamplingSettings, SyncSender<Result<(), ActorError>>),
    BeginConfiguration(
        ConfigurationContext,
        SyncSender<Result<ConfigurationEffect, ActorError>>,
    ),
    ConfigurationSummary(SyncSender<Result<ConfigurationSummary, ActorError>>),
    ConfigurationSchema(SyncSender<Result<Option<ConfigurationSchema>, ActorError>>),
    ConfigurationSchemaBytes(SyncSender<Result<Option<Vec<u8>>, ActorError>>),
    ConfigurationValues(
        bool,
        SyncSender<Result<Option<HashMap<String, ConfigurationValue>>, ActorError>>,
    ),
    RequestConfigurationValues(bool, SyncSender<Result<ConfigurationEffect, ActorError>>),
    RequestConfigurationWrite(
        TuneIdentity,
        u64,
        HashMap<String, ConfigurationValue>,
        SyncSender<Result<ConfigurationEffect, ActorError>>,
    ),
    AcceptConfigurationResponse(
        u64,
        u64,
        Vec<u8>,
        SyncSender<Result<ConfigurationEffect, ActorError>>,
    ),
    ObserveTelemetry(Telemetry, u64, SyncSender<Result<bool, ActorError>>),
    SelectBms(BmsConnection, SyncSender<Result<bool, ActorError>>),
    AcceptBms(BmsSnapshot, SyncSender<Result<bool, ActorError>>),
    DomainSnapshot(SyncSender<Result<AuthoritativeState, ActorError>>),
    Snapshot(SyncSender<Result<ApplicationSnapshot, ActorError>>),
    Stop,
}

struct ActorState<C> {
    snapshot: ApplicationSnapshot,
    next_operation: u64,
    pending: HashSet<u64>,
    completed: VecDeque<u64>,
    last_adapter_order: Option<u64>,
    connection: ConnectionCoordinator,
    model: ApplicationModel,
    clock: C,
}

impl<C: Clock> ActorState<C> {
    fn new<R: RandomSource>(generation: RuntimeGeneration, mut clock: C, mut random: R) -> Self {
        let mut nonce = [0; 16];
        random.fill(&mut nonce);
        let monotonic_millis = clock.monotonic_millis();
        let wall_millis = clock.wall_millis();
        Self {
            snapshot: ApplicationSnapshot {
                generation,
                revision: 0,
                foreground: true,
                background_eligible: false,
                bluetooth_radio: BluetoothRadio::Unknown,
                bluetooth_authorization: BluetoothAuthorization::Unknown,
                bluetooth_ready: false,
                bluetooth_connection: None,
                discovered_devices: Vec::new(),
                connection_generation: 0,
                location_authorization: LocationAuthorization::Unknown,
                location_sequence: 0,
                latest_location: None,
                notification_count: 0,
                pending_operations: 0,
                monotonic_millis,
                wall_millis,
                instance_nonce: nonce,
            },
            next_operation: 1,
            pending: HashSet::new(),
            completed: VecDeque::new(),
            last_adapter_order: None,
            connection: ConnectionCoordinator::default(),
            model: ApplicationModel::default(),
            clock,
        }
    }

    fn update_time(&mut self) {
        self.snapshot.monotonic_millis = self.clock.monotonic_millis();
        self.snapshot.wall_millis = self.clock.wall_millis();
    }

    fn changed(&mut self) {
        self.snapshot.revision = self.snapshot.revision.saturating_add(1);
        self.update_time();
        self.snapshot.pending_operations = self.pending.len();
    }

    fn accept_context(
        &mut self,
        generation: RuntimeGeneration,
        connection_generation: u64,
        adapter_order: u64,
    ) -> Result<(), ActorError> {
        if generation != self.snapshot.generation {
            return Err(ActorError::StaleGeneration);
        }
        if connection_generation < self.snapshot.connection_generation {
            return Err(ActorError::StaleConnection);
        }
        if self
            .last_adapter_order
            .is_some_and(|last| adapter_order <= last)
        {
            return Err(ActorError::StaleOrder);
        }
        self.last_adapter_order = Some(adapter_order);
        self.snapshot.connection_generation = connection_generation;
        Ok(())
    }
}

pub struct ApplicationActor {
    generation: RuntimeGeneration,
    ingress: SyncSender<ActorInput>,
    host_commands: Mutex<Receiver<PlatformCommand>>,
    storage_work: Mutex<Receiver<StorageWork>>,
    publications: Mutex<Receiver<ServerPublication>>,
    pump_waker: Arc<Mutex<Option<Thread>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl ApplicationActor {
    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn start<C: Clock, R: RandomSource>(
        generation: RuntimeGeneration,
        clock: C,
        random: R,
    ) -> Result<Arc<Self>, ActorError> {
        let (ingress, inputs) = mpsc::sync_channel(INGRESS_CHANNEL_ITEMS);
        let (host_sender, host_commands) = mpsc::sync_channel(HOST_COMMAND_CHANNEL_ITEMS);
        let (storage_sender, storage_work) = mpsc::sync_channel(STORAGE_CHANNEL_ITEMS);
        let (server_sender, publications) = mpsc::sync_channel(SERVER_CHANNEL_ITEMS);
        let pump_waker = Arc::new(Mutex::new(None));
        let actor = Arc::new(Self {
            generation,
            ingress,
            host_commands: Mutex::new(host_commands),
            storage_work: Mutex::new(storage_work),
            publications: Mutex::new(publications),
            pump_waker: Arc::clone(&pump_waker),
            worker: Mutex::new(None),
        });
        let worker = thread::Builder::new()
            .name("app-coordinator".into())
            .spawn(move || {
                run_actor(
                    &inputs,
                    ActorPorts {
                        host_commands: host_sender,
                        storage_work: storage_sender,
                        publications: server_sender,
                        pump_waker,
                    },
                    generation,
                    clock,
                    random,
                );
            })
            .map_err(|_| ActorError::Internal)?;
        *actor.worker.lock().map_err(|_| ActorError::Internal)? = Some(worker);
        Ok(actor)
    }

    #[must_use]
    pub const fn generation(&self) -> RuntimeGeneration {
        self.generation
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn submit_platform_event(&self, event: PlatformEvent) -> Result<(), ActorError> {
        self.request(|result| ActorInput::PlatformEvent(event, result))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn submit_bluetooth(&self, notification: BluetoothNotification) -> Result<(), ActorError> {
        self.request(|result| ActorInput::Bluetooth(notification, result))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn complete(&self, completion: OperationCompletion) -> Result<(), ActorError> {
        self.request(|result| ActorInput::Completion(completion, result))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn request_host(&self, command: HostCommand) -> Result<OperationId, ActorError> {
        self.request(|result| ActorInput::RequestHost(command, result))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn request_storage(&self, work: StorageWork) -> Result<(), ActorError> {
        self.request(|result| ActorInput::RequestStorage(work, result))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn submit_storage_result(&self, result: StorageResult) -> Result<(), ActorError> {
        self.request(|reply| ActorInput::StorageResult(result, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn begin_vesc_generation(&self) -> Result<u64, ActorError> {
        self.request(ActorInput::BeginVescGeneration)
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn schedule_vesc(
        &self,
        request: RequestSpec,
    ) -> Result<Vec<RequestCompletion>, ActorError> {
        self.request(|reply| ActorInput::ScheduleVesc(request, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn tick_vesc(&self, now_millis: u64) -> Result<Vec<RequestCompletion>, ActorError> {
        self.request(|reply| ActorInput::TickVesc(now_millis, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn tick_vesc_now(&self) -> Result<Vec<RequestCompletion>, ActorError> {
        self.request(ActorInput::TickVescNow)
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn start_tracking(&self, mode: TrackingMode) -> Result<(), ActorError> {
        self.request(|reply| ActorInput::StartTracking(mode, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn configure_sampling(&self, settings: SamplingSettings) -> Result<(), ActorError> {
        self.request(|reply| ActorInput::ConfigureSampling(settings, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn begin_configuration(
        &self,
        context: ConfigurationContext,
    ) -> Result<ConfigurationEffect, ActorError> {
        self.request(|reply| ActorInput::BeginConfiguration(context, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn configuration_summary(&self) -> Result<ConfigurationSummary, ActorError> {
        self.request(ActorInput::ConfigurationSummary)
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn configuration_schema(&self) -> Result<Option<ConfigurationSchema>, ActorError> {
        self.request(ActorInput::ConfigurationSchema)
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn configuration_schema_bytes(&self) -> Result<Option<Vec<u8>>, ActorError> {
        self.request(ActorInput::ConfigurationSchemaBytes)
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn configuration_values(
        &self,
        defaults: bool,
    ) -> Result<Option<HashMap<String, ConfigurationValue>>, ActorError> {
        self.request(|reply| ActorInput::ConfigurationValues(defaults, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn request_configuration_values(
        &self,
        defaults: bool,
    ) -> Result<ConfigurationEffect, ActorError> {
        self.request(|reply| ActorInput::RequestConfigurationValues(defaults, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn request_configuration_write(
        &self,
        identity: TuneIdentity,
        base_revision: u64,
        values: HashMap<String, ConfigurationValue>,
    ) -> Result<ConfigurationEffect, ActorError> {
        self.request(|reply| {
            ActorInput::RequestConfigurationWrite(identity, base_revision, values, reply)
        })
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn accept_configuration_response(
        &self,
        generation: u64,
        operation: u64,
        response: Vec<u8>,
    ) -> Result<ConfigurationEffect, ActorError> {
        self.request(|reply| {
            ActorInput::AcceptConfigurationResponse(generation, operation, response, reply)
        })
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn observe_telemetry(
        &self,
        telemetry: Telemetry,
        elapsed_millis: u64,
    ) -> Result<bool, ActorError> {
        self.request(|reply| ActorInput::ObserveTelemetry(telemetry, elapsed_millis, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn select_bms(&self, connection: BmsConnection) -> Result<bool, ActorError> {
        self.request(|reply| ActorInput::SelectBms(connection, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn accept_bms(&self, snapshot: BmsSnapshot) -> Result<bool, ActorError> {
        self.request(|reply| ActorInput::AcceptBms(snapshot, reply))
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn domain_snapshot(&self) -> Result<AuthoritativeState, ActorError> {
        self.request(ActorInput::DomainSnapshot)
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn snapshot(&self) -> Result<ApplicationSnapshot, ActorError> {
        self.request(ActorInput::Snapshot)
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn try_host_command(&self) -> Result<Option<PlatformCommand>, ActorError> {
        self.host_commands
            .lock()
            .map_err(|_| ActorError::Internal)?
            .try_recv()
            .map(Some)
            .or_else(|error| match error {
                mpsc::TryRecvError::Empty => Ok(None),
                mpsc::TryRecvError::Disconnected => Err(ActorError::Stopped),
            })
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn try_storage_work(&self) -> Result<Option<StorageWork>, ActorError> {
        self.storage_work
            .lock()
            .map_err(|_| ActorError::Internal)?
            .try_recv()
            .map(Some)
            .or_else(|error| match error {
                mpsc::TryRecvError::Empty => Ok(None),
                mpsc::TryRecvError::Disconnected => Err(ActorError::Stopped),
            })
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn try_publication(&self) -> Result<Option<ServerPublication>, ActorError> {
        self.publications
            .lock()
            .map_err(|_| ActorError::Internal)?
            .try_recv()
            .map(Some)
            .or_else(|error| match error {
                mpsc::TryRecvError::Empty => Ok(None),
                mpsc::TryRecvError::Disconnected => Err(ActorError::Stopped),
            })
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn register_pump_thread(&self, thread: Thread) -> Result<(), ActorError> {
        *self.pump_waker.lock().map_err(|_| ActorError::Internal)? = Some(thread);
        Ok(())
    }

    /// # Errors
    /// Returns an actor error when the bounded actor is stopped, overloaded, or rejects the request.
    pub fn stop(&self) -> Result<(), ActorError> {
        self.ingress
            .send(ActorInput::Stop)
            .map_err(|_| ActorError::Stopped)?;
        let worker = self.worker.lock().map_err(|_| ActorError::Internal)?.take();
        if let Some(worker) = worker {
            worker.join().map_err(|_| ActorError::Internal)?;
        }
        Ok(())
    }

    fn request<T>(
        &self,
        build: impl FnOnce(SyncSender<Result<T, ActorError>>) -> ActorInput,
    ) -> Result<T, ActorError> {
        let (sender, receiver) = mpsc::sync_channel(1);
        self.ingress.try_send(build(sender)).map_err(map_send)?;
        receiver.recv().map_err(|_| ActorError::Stopped)?
    }
}

fn map_send<T>(error: TrySendError<T>) -> ActorError {
    match error {
        TrySendError::Full(value) => {
            drop(value);
            ActorError::QueueFull
        }
        TrySendError::Disconnected(value) => {
            drop(value);
            ActorError::Stopped
        }
    }
}

fn wake_pump(waker: &Mutex<Option<Thread>>) {
    if let Ok(waker) = waker.lock()
        && let Some(thread) = waker.as_ref()
    {
        thread.unpark();
    }
}

struct ActorPorts {
    host_commands: SyncSender<PlatformCommand>,
    storage_work: SyncSender<StorageWork>,
    publications: SyncSender<ServerPublication>,
    pump_waker: Arc<Mutex<Option<Thread>>>,
}

fn run_actor<C: Clock, R: RandomSource>(
    inputs: &Receiver<ActorInput>,
    ports: ActorPorts,
    generation: RuntimeGeneration,
    clock: C,
    random: R,
) {
    let ActorPorts {
        host_commands,
        storage_work,
        publications,
        pump_waker,
    } = ports;
    let mut state = ActorState::new(generation, clock, random);
    while let Ok(input) = inputs.recv() {
        match input {
            ActorInput::PlatformEvent(event, reply) => {
                let result = accept_platform_event(&mut state, event)
                    .and_then(|changed| publish_if_changed(&state, changed, &publications));
                let _ = reply.send(result);
                wake_pump(&pump_waker);
            }
            ActorInput::Bluetooth(notification, reply) => {
                let result = accept_bluetooth_notification(
                    &mut state,
                    &notification,
                    &host_commands,
                    &publications,
                );
                let _ = reply.send(result);
                wake_pump(&pump_waker);
            }
            ActorInput::Completion(completion, reply) => {
                let result = accept_completion(&mut state, &completion)
                    .and_then(|()| publish_if_changed(&state, true, &publications));
                let _ = reply.send(result);
                wake_pump(&pump_waker);
            }
            ActorInput::RequestHost(command, reply) => {
                let result =
                    request_host(&mut state, command, &host_commands).and_then(|operation| {
                        publish_if_changed(&state, true, &publications).map(|()| operation)
                    });
                let _ = reply.send(result);
            }
            ActorInput::RequestStorage(work, reply) => {
                let result = storage_work.try_send(work).map_err(map_send);
                let _ = reply.send(result);
            }
            ActorInput::StorageResult(result, reply) => {
                let published = publications
                    .try_send(ServerPublication::StorageCompleted(result))
                    .map_err(map_send);
                let _ = reply.send(published);
                wake_pump(&pump_waker);
            }
            input @ (ActorInput::BeginVescGeneration(..)
            | ActorInput::ScheduleVesc(..)
            | ActorInput::TickVesc(..)
            | ActorInput::TickVescNow(..)) => {
                handle_vesc_input(&mut state, input, &host_commands, &publications);
            }
            ActorInput::StartTracking(mode, reply) => {
                state.model.connect(mode);
                let _ = reply.send(Ok(()));
            }
            ActorInput::ConfigureSampling(settings, reply) => {
                state.model.sampling = AdaptiveSamplingPolicy::new(settings);
                let _ = reply.send(Ok(()));
            }
            input @ (ActorInput::BeginConfiguration(..)
            | ActorInput::ConfigurationSummary(..)
            | ActorInput::ConfigurationSchema(..)
            | ActorInput::ConfigurationSchemaBytes(..)
            | ActorInput::ConfigurationValues(..)
            | ActorInput::RequestConfigurationValues(..)
            | ActorInput::RequestConfigurationWrite(..)
            | ActorInput::AcceptConfigurationResponse(..)) => {
                handle_configuration_input(&mut state, input);
            }
            ActorInput::ObserveTelemetry(telemetry, elapsed_millis, reply) => {
                let persisted = state.model.observe_telemetry(&telemetry, elapsed_millis);
                let _ = reply.send(Ok(persisted));
            }
            ActorInput::SelectBms(connection, reply) => {
                let generation = state.connection.requests.generation();
                let _ = reply.send(Ok(state.model.select_bms(generation, connection)));
            }
            ActorInput::AcceptBms(snapshot, reply) => {
                let generation = state.connection.requests.generation();
                let _ = reply.send(Ok(state.model.accept_bms(generation, snapshot)));
            }
            ActorInput::DomainSnapshot(reply) => {
                let _ = reply.send(Ok(state.model.state.clone()));
            }
            ActorInput::Snapshot(reply) => {
                let _ = reply.send(Ok(state.snapshot.clone()));
            }
            ActorInput::Stop => break,
        }
    }
}

fn handle_vesc_input<C: Clock>(
    state: &mut ActorState<C>,
    input: ActorInput,
    host_commands: &SyncSender<PlatformCommand>,
    publications: &SyncSender<ServerPublication>,
) {
    match input {
        ActorInput::BeginVescGeneration(reply) => {
            let actions = state.connection.begin_generation();
            let result = apply_connection_actions(state, actions, host_commands).map(|_| {
                let generation = state.connection.requests.generation();
                state.model.begin_generation(generation);
                generation
            });
            let _ = reply.send(result);
        }
        ActorInput::ScheduleVesc(request, reply) => {
            let revision = state.snapshot.revision;
            let actions = state
                .connection
                .requests
                .enqueue(state.snapshot.monotonic_millis, request);
            let result =
                apply_connection_actions(state, actions, host_commands).and_then(|completions| {
                    publish_if_changed(state, state.snapshot.revision != revision, publications)?;
                    Ok(completions)
                });
            let _ = reply.send(result);
        }
        ActorInput::TickVesc(now_millis, reply) => {
            let revision = state.snapshot.revision;
            let actions = state.connection.requests.tick(now_millis);
            let result =
                apply_connection_actions(state, actions, host_commands).and_then(|completions| {
                    publish_if_changed(state, state.snapshot.revision != revision, publications)?;
                    Ok(completions)
                });
            let _ = reply.send(result);
        }
        ActorInput::TickVescNow(reply) => {
            state.update_time();
            let actions = state
                .connection
                .requests
                .tick(state.snapshot.monotonic_millis);
            let result =
                apply_connection_actions(state, actions, host_commands).and_then(|completions| {
                    for completion in &completions {
                        publications
                            .try_send(ServerPublication::VescCompleted(completion.clone()))
                            .map_err(map_send)?;
                    }
                    Ok(completions)
                });
            let _ = reply.send(result);
        }
        _ => unreachable!("only VESC inputs are routed here"),
    }
}

fn accept_bluetooth_notification<C: Clock>(
    state: &mut ActorState<C>,
    notification: &BluetoothNotification,
    host_commands: &SyncSender<PlatformCommand>,
    publications: &SyncSender<ServerPublication>,
) -> Result<(), ActorError> {
    notification
        .validate()
        .map_err(|_| ActorError::InvalidInput)?;
    state.accept_context(
        notification.generation,
        notification.connection_generation,
        notification.adapter_order,
    )?;
    let revision = state.snapshot.revision;
    let actions = state
        .connection
        .receive_notification(state.snapshot.monotonic_millis, &notification.bytes)
        .map_err(|_| ActorError::InvalidInput)?;
    let completions = apply_connection_actions(state, actions, host_commands)?;
    for completion in completions {
        publications
            .try_send(ServerPublication::VescCompleted(completion))
            .map_err(map_send)?;
    }
    state.snapshot.notification_count = state.snapshot.notification_count.saturating_add(1);
    state.changed();
    publish_if_changed(state, state.snapshot.revision != revision, publications)
}

fn handle_configuration_input<C: Clock>(state: &mut ActorState<C>, input: ActorInput) {
    match input {
        ActorInput::BeginConfiguration(context, reply) => {
            let _ = reply.send(Ok(state.model.configuration.begin_owned(&context)));
        }
        ActorInput::ConfigurationSummary(reply) => {
            let _ = reply.send(Ok(state.model.configuration.summary()));
        }
        ActorInput::ConfigurationSchema(reply) => {
            let _ = reply.send(Ok(state.model.configuration.schema().cloned()));
        }
        ActorInput::ConfigurationSchemaBytes(reply) => {
            let bytes = state.model.configuration.schema_bytes().map(<[u8]>::to_vec);
            let _ = reply.send(Ok(bytes));
        }
        ActorInput::ConfigurationValues(defaults, reply) => {
            let _ = reply.send(Ok(state.model.configuration.values(defaults).cloned()));
        }
        ActorInput::RequestConfigurationValues(defaults, reply) => {
            let result = state
                .model
                .configuration
                .request_values(defaults)
                .map_err(|_| ActorError::InvalidInput);
            let _ = reply.send(result);
        }
        ActorInput::RequestConfigurationWrite(identity, revision, values, reply) => {
            let result = state
                .model
                .configuration
                .request_write(&identity, revision, values)
                .map_err(|_| ActorError::InvalidInput);
            let _ = reply.send(result);
        }
        ActorInput::AcceptConfigurationResponse(generation, operation, response, reply) => {
            let result = state
                .model
                .configuration
                .accept_response(generation, operation, &response)
                .map_err(|_| ActorError::InvalidInput);
            let _ = reply.send(result);
        }
        _ => unreachable!("only configuration inputs are routed here"),
    }
}

fn apply_connection_actions<C: Clock>(
    state: &mut ActorState<C>,
    actions: Vec<ConnectionAction>,
    host_commands: &SyncSender<PlatformCommand>,
) -> Result<Vec<RequestCompletion>, ActorError> {
    let mut completions = Vec::new();
    for action in actions {
        match action {
            ConnectionAction::Write(request) => {
                let connection_generation = state.snapshot.connection_generation;
                request_host(
                    state,
                    HostCommand::BluetoothWrite {
                        connection_generation,
                        bytes: request.payload,
                    },
                    host_commands,
                )?;
            }
            ConnectionAction::Completed(completion) => completions.push(completion),
            ConnectionAction::UnmatchedResponse(_) => {}
        }
    }
    Ok(completions)
}

fn accept_platform_event<C: Clock>(
    state: &mut ActorState<C>,
    event: PlatformEvent,
) -> Result<bool, ActorError> {
    state.accept_context(
        event.generation,
        event.connection_generation,
        event.adapter_order,
    )?;
    let changed = match event.event {
        AdapterEvent::BluetoothState {
            radio,
            authorization,
        } => {
            let changed = state.snapshot.bluetooth_radio != radio
                || state.snapshot.bluetooth_authorization != authorization;
            state.snapshot.bluetooth_radio = radio;
            state.snapshot.bluetooth_authorization = authorization;
            changed
        }
        AdapterEvent::BluetoothDevice {
            device_token,
            name,
            rssi,
        } => update_discovered_device(state, device_token, name, rssi),
        AdapterEvent::LocationAuthorization(authorization) => {
            let changed = state.snapshot.location_authorization != authorization;
            state.snapshot.location_authorization = authorization;
            changed
        }
        AdapterEvent::Location {
            latitude,
            longitude,
            accuracy_m,
        } => {
            validate_location(latitude, longitude, accuracy_m)?;
            state.snapshot.location_sequence = state.snapshot.location_sequence.saturating_add(1);
            state.snapshot.latest_location = Some((
                latitude.to_bits(),
                longitude.to_bits(),
                accuracy_m.map(f64::to_bits),
            ));
            true
        }
        AdapterEvent::Lifecycle {
            foreground,
            background_eligible,
        } => {
            let changed = state.snapshot.foreground != foreground
                || state.snapshot.background_eligible != background_eligible;
            state.snapshot.foreground = foreground;
            state.snapshot.background_eligible = background_eligible;
            changed
        }
        AdapterEvent::BluetoothConnection {
            connection_generation,
            state: connection_state,
        } => {
            if connection_generation != event.connection_generation {
                return Err(ActorError::StaleConnection);
            }
            let ready = connection_state == BluetoothConnectionState::Ready;
            let changed = state.snapshot.bluetooth_ready != ready
                || state.snapshot.bluetooth_connection != Some(connection_state);
            state.snapshot.bluetooth_ready = ready;
            state.snapshot.bluetooth_connection = Some(connection_state);
            match connection_state {
                BluetoothConnectionState::Ready => state.model.ready(),
                BluetoothConnectionState::Disconnected | BluetoothConnectionState::Failed => {
                    state.model.disconnected();
                }
                BluetoothConnectionState::Connecting => {}
            }
            changed
        }
        _ => false,
    };
    if changed {
        state.changed();
    }
    Ok(changed)
}

fn update_discovered_device<C: Clock>(
    state: &mut ActorState<C>,
    device_token: Vec<u8>,
    name: Option<String>,
    rssi: i16,
) -> bool {
    if let Some(device) = state
        .snapshot
        .discovered_devices
        .iter_mut()
        .find(|device| device.device_token == device_token)
    {
        let changed = device.name != name || device.rssi != rssi;
        device.name = name;
        device.rssi = rssi;
        changed
    } else if state.snapshot.discovered_devices.len() < MAX_DISCOVERED_DEVICES {
        state
            .snapshot
            .discovered_devices
            .push(DiscoveredBluetoothDevice {
                device_token,
                name,
                rssi,
            });
        true
    } else {
        false
    }
}

fn validate_location(
    latitude: f64,
    longitude: f64,
    accuracy_m: Option<f64>,
) -> Result<(), ActorError> {
    if !latitude.is_finite()
        || !longitude.is_finite()
        || !(-90.0..=90.0).contains(&latitude)
        || !(-180.0..=180.0).contains(&longitude)
        || accuracy_m.is_some_and(|accuracy| !accuracy.is_finite() || accuracy < 0.0)
    {
        Err(ActorError::InvalidInput)
    } else {
        Ok(())
    }
}

fn accept_completion<C: Clock>(
    state: &mut ActorState<C>,
    completion: &OperationCompletion,
) -> Result<(), ActorError> {
    if completion.generation != state.snapshot.generation {
        return Err(ActorError::StaleGeneration);
    }
    let operation = completion.operation.0;
    if state.completed.contains(&operation) {
        return Err(ActorError::DuplicateOperation);
    }
    if !state.pending.remove(&operation) {
        return Err(ActorError::StaleOperation);
    }
    if state.completed.len() == COMPLETED_OPERATION_HISTORY {
        state.completed.pop_front();
    }
    state.completed.push_back(operation);
    state.changed();
    Ok(())
}

fn request_host<C: Clock>(
    state: &mut ActorState<C>,
    command: HostCommand,
    output: &SyncSender<PlatformCommand>,
) -> Result<OperationId, ActorError> {
    if state.pending.len() >= MAX_PENDING_OPERATIONS {
        return Err(ActorError::QueueFull);
    }
    let operation = OperationId(state.next_operation);
    let envelope = PlatformCommand {
        generation: state.snapshot.generation,
        operation,
        command,
    };
    envelope.encode().map_err(|_| ActorError::InvalidInput)?;
    output.try_send(envelope).map_err(map_send)?;
    state.next_operation = state.next_operation.saturating_add(1);
    state.pending.insert(operation.0);
    state.changed();
    Ok(operation)
}

fn publish_if_changed<C>(
    state: &ActorState<C>,
    changed: bool,
    output: &SyncSender<ServerPublication>,
) -> Result<(), ActorError> {
    if changed {
        output
            .try_send(ServerPublication::StateChanged(state.snapshot.clone()))
            .map_err(map_send)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use host_ports::CompletionStatus;
    use protocol_vesc::{ExpectedResponse, VescCommand, encode_frame};

    use super::*;

    fn actor() -> Arc<ApplicationActor> {
        ApplicationActor::start(
            RuntimeGeneration(7),
            DeterministicClock::new(100, 1_700_000_000_000, 10),
            DeterministicRandom::new(42),
        )
        .expect("actor")
    }

    fn request(command: VescCommand, generation: u64) -> RequestSpec {
        RequestSpec {
            payload: encode_frame(&[command as u8]).expect("request frame"),
            expected: ExpectedResponse::new(command, None),
            priority: crate::RequestPriority::ControllerWork,
            kind: crate::RequestKind::ControllerFirmware,
            timeout_millis: 900,
            generation,
            expects_response: true,
        }
    }

    #[test]
    fn operations_are_generation_owned_completed_once_and_published_monotonically() {
        let actor = actor();
        let operation = actor
            .request_host(HostCommand::PresentSettings)
            .expect("request");
        let command = actor.try_host_command().expect("channel").expect("command");
        assert_eq!(command.operation, operation);
        assert_eq!(command.generation, RuntimeGeneration(7));
        let first = actor
            .try_publication()
            .expect("channel")
            .expect("publication");
        let ServerPublication::StateChanged(first) = first else {
            panic!("state");
        };
        assert_eq!(first.revision, 1);
        assert_eq!(first.pending_operations, 1);

        let completion = OperationCompletion {
            generation: RuntimeGeneration(7),
            operation,
            status: CompletionStatus::Success,
            payload: Vec::new(),
        };
        actor.complete(completion.clone()).expect("complete");
        assert_eq!(
            actor.complete(completion),
            Err(ActorError::DuplicateOperation)
        );
        let second = actor
            .try_publication()
            .expect("channel")
            .expect("publication");
        let ServerPublication::StateChanged(second) = second else {
            panic!("state");
        };
        assert_eq!(second.revision, 2);
        assert_eq!(second.pending_operations, 0);
        actor.stop().expect("stop");
    }

    #[test]
    fn completed_operation_replay_history_is_bounded() {
        let actor = actor();
        let mut first = None;
        for _ in 0..=COMPLETED_OPERATION_HISTORY {
            let operation = actor
                .request_host(HostCommand::PresentSettings)
                .expect("request");
            first.get_or_insert(operation);
            actor.try_host_command().expect("channel").expect("command");
            actor
                .complete(OperationCompletion {
                    generation: RuntimeGeneration(7),
                    operation,
                    status: CompletionStatus::Success,
                    payload: Vec::new(),
                })
                .expect("complete");
            while actor
                .try_publication()
                .expect("publication channel")
                .is_some()
            {}
        }
        assert_eq!(
            actor.complete(OperationCompletion {
                generation: RuntimeGeneration(7),
                operation: first.expect("first operation"),
                status: CompletionStatus::Success,
                payload: Vec::new(),
            }),
            Err(ActorError::StaleOperation)
        );
        actor.stop().expect("stop");
    }

    #[test]
    fn production_clock_tick_releases_a_timed_out_vesc_request() {
        let actor = ApplicationActor::start(
            RuntimeGeneration(7),
            DeterministicClock::new(0, 1_700_000_000_000, 1_000),
            DeterministicRandom::new(42),
        )
        .expect("actor");
        let generation = actor.begin_vesc_generation().expect("generation");
        actor
            .schedule_vesc(request(VescCommand::GetFirmwareVersion, generation))
            .expect("schedule");
        actor.try_host_command().expect("channel").expect("write");
        assert!(matches!(
            actor.tick_vesc_now().expect("tick").as_slice(),
            [RequestCompletion::TimedOut(_)]
        ));
        actor.stop().expect("stop");
    }

    #[test]
    fn platform_order_connection_generation_and_state_revisions_are_strict() {
        let actor = actor();
        actor
            .start_tracking(TrackingMode::Saved)
            .expect("tracking mode");
        let event = PlatformEvent {
            generation: RuntimeGeneration(7),
            connection_generation: 2,
            adapter_order: 1,
            event: AdapterEvent::BluetoothConnection {
                connection_generation: 2,
                state: BluetoothConnectionState::Ready,
            },
        };
        actor.submit_platform_event(event.clone()).expect("event");
        assert_eq!(
            actor.submit_platform_event(event),
            Err(ActorError::StaleOrder)
        );
        let stale = PlatformEvent {
            generation: RuntimeGeneration(6),
            connection_generation: 2,
            adapter_order: 2,
            event: AdapterEvent::Lifecycle {
                foreground: false,
                background_eligible: true,
            },
        };
        assert_eq!(
            actor.submit_platform_event(stale),
            Err(ActorError::StaleGeneration)
        );
        let snapshot = actor.snapshot().expect("snapshot");
        assert_eq!(snapshot.revision, 1);
        assert!(snapshot.bluetooth_ready);
        let domain = actor.domain_snapshot().expect("domain snapshot");
        assert_eq!(domain.connection, crate::DomainConnectionState::Ready);
        assert_eq!(domain.tracking, TrackingMode::Saved);
        assert!(domain.session_active);
        actor.stop().expect("stop");
    }

    #[test]
    fn raw_bluetooth_storage_server_and_deterministic_adapters_are_bounded() {
        let actor = actor();
        actor
            .submit_bluetooth(BluetoothNotification {
                generation: RuntimeGeneration(7),
                connection_generation: 1,
                adapter_order: 1,
                bytes: vec![2, 0, 3],
            })
            .expect("notification");
        assert_eq!(actor.snapshot().expect("snapshot").notification_count, 1);
        assert!(matches!(
            actor.try_publication(),
            Ok(Some(ServerPublication::StateChanged(_)))
        ));
        actor.request_storage(StorageWork::Open).expect("storage");
        assert_eq!(
            actor.try_storage_work().expect("channel"),
            Some(StorageWork::Open)
        );
        let result = StorageResult {
            request_id: 4,
            succeeded: true,
        };
        actor
            .submit_storage_result(result.clone())
            .expect("storage result");
        assert_eq!(
            actor.try_publication().expect("channel"),
            Some(ServerPublication::StorageCompleted(result))
        );
        let first_nonce = actor.snapshot().expect("snapshot").instance_nonce;
        assert_ne!(first_nonce, [0; 16]);
        actor.stop().expect("stop");
    }

    #[test]
    fn actor_is_the_single_vesc_write_owner_across_streamed_responses() {
        let actor = actor();
        actor
            .submit_platform_event(PlatformEvent {
                generation: RuntimeGeneration(7),
                connection_generation: 1,
                adapter_order: 1,
                event: AdapterEvent::BluetoothConnection {
                    connection_generation: 1,
                    state: BluetoothConnectionState::Ready,
                },
            })
            .expect("ready");
        let generation = actor.begin_vesc_generation().expect("generation");

        let firmware = request(VescCommand::GetFirmwareVersion, generation);
        let firmware_bytes = firmware.payload.clone();
        assert!(actor.schedule_vesc(firmware).expect("schedule").is_empty());
        let first = actor
            .try_host_command()
            .expect("host channel")
            .expect("first write");
        assert_eq!(
            first.command,
            HostCommand::BluetoothWrite {
                connection_generation: 1,
                bytes: firmware_bytes,
            }
        );

        let bms = request(VescCommand::BmsGetValues, generation);
        let bms_bytes = bms.payload.clone();
        assert!(actor.schedule_vesc(bms).expect("queue").is_empty());
        assert_eq!(actor.try_host_command().expect("host channel"), None);

        actor
            .submit_bluetooth(BluetoothNotification {
                generation: RuntimeGeneration(7),
                connection_generation: 1,
                adapter_order: 2,
                bytes: encode_frame(&[VescCommand::GetFirmwareVersion as u8, 6, 6])
                    .expect("response frame"),
            })
            .expect("response");
        let second = actor
            .try_host_command()
            .expect("host channel")
            .expect("second write");
        assert_eq!(
            second.command,
            HostCommand::BluetoothWrite {
                connection_generation: 1,
                bytes: bms_bytes,
            }
        );
        actor.stop().expect("stop");
    }
}
