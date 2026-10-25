//! Safe connection-scoped composition for the future unified Rust runtime.
//!
//! The coordinator owns deterministic application decisions and returns owned
//! effects. It deliberately owns no transport, frame stream, clock, timer,
//! callback, persistence, local server, host ABI, executor, or OS behavior.

#![forbid(unsafe_code)]

use std::collections::VecDeque;

mod reply_health;
pub use reply_health::WARNING as CAN_REPLY_WARNING;

pub use float_view_controller_discovery as discovery;
pub use float_view_device_mutation_coordinator as mutation;
pub use float_view_refloat_compatibility as compatibility;
pub use float_view_refloat_configuration_codec as configuration_codec;
pub use float_view_refloat_configuration_service as configuration;
use float_view_refloat_dynamic_telemetry as dynamic;
use float_view_refloat_legacy_telemetry as legacy;
pub use float_view_refloat_telemetry_service as telemetry;
pub use float_view_vesc_bms_telemetry_service as bms;
pub use float_view_vesc_controller_data as controller_data;
use float_view_vesc_custom_configuration as custom_configuration;
pub use float_view_vesc_firmware_identity as firmware;
pub use float_view_vesc_request_scheduler as scheduler;

pub const INITIAL_IDENTIFICATION_ATTEMPT_TIMEOUT_MS: u64 = 2_000;
pub const IDENTIFICATION_TIMEOUT_MS: u64 = 10_000;
pub const CAN_PROBE_TIMEOUT_MS: u64 = 1_000;
pub const MAX_TRANSITION_EFFECTS: usize = 512;

const GET_FIRMWARE_VERSION_COMMAND: u8 = 0;
const FORWARD_CAN_COMMAND: u8 = 34;
const PING_CAN_COMMAND: u8 = 62;
const CUSTOM_APP_DATA_COMMAND: u8 = 36;
const REFLOAT_INFO_COMMAND: u8 = 0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Generation(pub u64);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Phase {
    #[default]
    Disconnected,
    Identifying,
    SelectingController,
    AwaitingActivation,
    Ready,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeginFacts {
    pub now_ms: u64,
    pub settings: telemetry::Settings,
    pub preferred_controller_can_id: Option<u8>,
    pub routing_hint: Option<discovery::RoutingHint>,
    pub continuity: discovery::Continuity,
    pub preferred_bms_can_id: Option<u8>,
}

impl BeginFacts {
    pub fn validate(&self) -> Result<(), CoordinatorError> {
        self.settings
            .validate()
            .map_err(CoordinatorError::Telemetry)?;
        validate_optional_can_id(self.preferred_controller_can_id)?;
        validate_optional_can_id(self.preferred_bms_can_id)?;
        validate_optional_route(self.continuity.route)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationFacts {
    pub generation: Generation,
    pub selected: discovery::RoutedIdentity,
    pub transport: firmware::FirmwareIdentity,
    pub compatibility: compatibility::CompatibilityProfile,
    pub refloat_version: compatibility::WireVersion,
    pub configuration_cache_identity: Option<configuration::CacheIdentity>,
    pub inventory: Vec<discovery::RoutedIdentity>,
    pub inventory_complete: bool,
}

/// Highest-priority blocker for an edited configuration write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationWriteBlockReason {
    Reconnecting,
    Loading,
    Unavailable,
    ReadOnly,
    Busy,
    Flywheel,
    Riding,
    StateUnknown,
    PackageNotReady,
    ModeNotNormal,
    CurrentRequired,
}

#[derive(Clone, PartialEq)]
pub struct Snapshot {
    pub reported_refloat_version: Option<String>,
    pub possible_can_id_conflict: bool,
    pub generation: Generation,
    pub phase: Phase,
    pub standalone_bms: bool,
    pub awaiting_activation: Option<ActivationFacts>,
    pub discovery: discovery::Snapshot,
    pub scheduler: scheduler::Snapshot,
    pub telemetry: telemetry::Snapshot,
    pub bms: bms::Snapshot,
    pub mutations: mutation::Snapshot,
    pub configuration: configuration::Snapshot,
    pub configuration_write_block_reason: Option<ConfigurationWriteBlockReason>,
    pub current_limits: Option<controller_data::CurrentLimits>,
}

impl std::fmt::Debug for Snapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Snapshot")
            .field("possible_can_id_conflict", &self.possible_can_id_conflict)
            .field("generation", &self.generation)
            .field("phase", &self.phase)
            .field("standalone_bms", &self.standalone_bms)
            .field("awaiting_activation", &self.awaiting_activation)
            .field("discovery", &self.discovery)
            .field("scheduler", &self.scheduler)
            .field("telemetry", &self.telemetry)
            .field("bms", &self.bms)
            .field("mutations", &self.mutations)
            .field(
                "configuration_availability",
                &self.configuration.availability,
            )
            .field("configuration_writable", &self.configuration.writable)
            .field("configuration_identity", &self.configuration.identity)
            .field(
                "has_configuration_current",
                &self.configuration.current.is_some(),
            )
            .field(
                "has_configuration_defaults",
                &self.configuration.defaults.is_some(),
            )
            .field("configuration_pending", &self.configuration.pending)
            .field("has_current_limits", &self.current_limits.is_some())
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct ConfigurationCacheEffect(configuration::CacheEffect);

impl ConfigurationCacheEffect {
    #[must_use]
    pub const fn as_inner(&self) -> &configuration::CacheEffect {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> configuration::CacheEffect {
        self.0
    }
}

impl std::fmt::Debug for ConfigurationCacheEffect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            configuration::CacheEffect::Store { bytes, .. } => formatter
                .debug_struct("ConfigurationCacheStore")
                .field("identity", &"[redacted]")
                .field("bytes", &bytes.len())
                .finish(),
            configuration::CacheEffect::Remove { .. } => formatter
                .debug_struct("ConfigurationCacheRemove")
                .field("identity", &"[redacted]")
                .finish(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutationTimerToken {
    Confirmation(mutation::MutationToken),
    Shutdown(mutation::MutationToken),
    LightingRefresh(scheduler::Generation),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerToken {
    Refloat(telemetry::TimerToken),
    Bms(bms::TimerToken),
    Mutation(MutationTimerToken),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MalformedResponse {
    Empty,
    Firmware,
    CanInventory,
    RefloatInfo,
    RefloatLayout,
    RefloatTelemetry,
    RefloatConfiguration,
    CurrentLimits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Diagnostic {
    StaleEvent,
    UnexpectedResponse,
    MalformedResponse(MalformedResponse),
    Scheduler(scheduler::Diagnostic),
    Telemetry(telemetry::Diagnostic),
    Bms(bms::Diagnostic),
    EnqueueRejected,
    Mutation(mutation::Diagnostic),
    ConfigurationStale,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    Discovery(discovery::Issue),
    Telemetry(telemetry::Failure),
    Configuration(configuration::ServiceError),
    Mutation {
        token: mutation::MutationToken,
        reason: mutation::FailureReason,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutationEvent {
    Idempotent,
    Confirmed(mutation::MutationToken),
    Released(mutation::MutationToken),
    Failed {
        token: mutation::MutationToken,
        reason: mutation::FailureReason,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    CancelDeadline(scheduler::DeadlineToken),
    ArmDeadline {
        token: scheduler::DeadlineToken,
        timeout_ms: u64,
    },
    Write(scheduler::ScheduledRequest),
    ReplaceTimer {
        token: TimerToken,
        interval_ms: u64,
    },
    CancelTimer(TimerToken),
    ClearRoutingHint,
    StoreRoutingHint(discovery::RoutedIdentity),
    PrepareActivation(ActivationFacts),
    PublishState(Box<Snapshot>),
    PublishTelemetry(Box<telemetry::NormalizedSample>),
    PublishFaultHistory(telemetry::FaultHistoryObservation),
    PublishBmsState(bms::Snapshot),
    PublishBmsObservation(Box<bms::BmsObservation>),
    ApplyConfigurationCache(ConfigurationCacheEffect),
    Mutation(MutationEvent),
    Failure(Failure),
    Diagnostic(Diagnostic),
}

#[derive(Clone, PartialEq)]
pub enum Command {
    Begin(BeginFacts),
    SelectController {
        generation: Generation,
        can_id: u8,
        now_ms: u64,
    },
    Receive {
        generation: Generation,
        payload: Vec<u8>,
        now_ms: u64,
        observed_at_epoch_ms: i64,
    },
    DeadlineFired {
        generation: Generation,
        token: scheduler::DeadlineToken,
        now_ms: u64,
    },
    TimerFired {
        generation: Generation,
        token: TimerToken,
        now_ms: u64,
    },
    Activate {
        generation: Generation,
        now_ms: u64,
        session_started: bool,
        cached_configuration_schema: Option<Vec<u8>>,
    },
    SetControl {
        generation: Generation,
        control: mutation::BooleanControl,
        enabled: bool,
        now_ms: u64,
    },
    ClearFatalError {
        generation: Generation,
        now_ms: u64,
    },
    ShutdownController {
        generation: Generation,
        now_ms: u64,
    },
    ReadConfiguration {
        generation: Generation,
        defaults: bool,
        now_ms: u64,
    },
    WriteConfiguration {
        generation: Generation,
        mutation: mutation::ConfigurationMutation,
        package_version: String,
        fingerprint: String,
        wire_signature: u32,
        base_revision: u64,
        values: configuration_codec::ConfigurationValues,
        now_ms: u64,
    },
    UpdateSettings {
        generation: Generation,
        settings: telemetry::Settings,
        now_ms: u64,
    },
    Refresh {
        generation: Generation,
        now_ms: u64,
    },
    Reset {
        now_ms: u64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoordinatorError {
    InvalidCanId,
    InvalidObservationTime,
    InvalidMonotonicTime,
    InvalidPayload,
    EffectLimit,
    Discovery(discovery::DiscoveryInputError),
    Scheduler(scheduler::SchedulerError),
    Telemetry(telemetry::ServiceError),
    Bms(bms::ServiceError),
    Mutation(mutation::CoordinatorError),
    Configuration(configuration::ServiceError),
    CustomConfiguration(custom_configuration::CustomConfigurationError),
}

impl From<discovery::DiscoveryInputError> for CoordinatorError {
    fn from(value: discovery::DiscoveryInputError) -> Self {
        Self::Discovery(value)
    }
}

impl From<scheduler::SchedulerError> for CoordinatorError {
    fn from(value: scheduler::SchedulerError) -> Self {
        Self::Scheduler(value)
    }
}

impl From<telemetry::ServiceError> for CoordinatorError {
    fn from(value: telemetry::ServiceError) -> Self {
        Self::Telemetry(value)
    }
}

impl From<bms::ServiceError> for CoordinatorError {
    fn from(value: bms::ServiceError) -> Self {
        Self::Bms(value)
    }
}

impl From<mutation::CoordinatorError> for CoordinatorError {
    fn from(value: mutation::CoordinatorError) -> Self {
        Self::Mutation(value)
    }
}

impl From<configuration::ServiceError> for CoordinatorError {
    fn from(value: configuration::ServiceError) -> Self {
        Self::Configuration(value)
    }
}

impl From<custom_configuration::CustomConfigurationError> for CoordinatorError {
    fn from(value: custom_configuration::CustomConfigurationError) -> Self {
        Self::CustomConfiguration(value)
    }
}

#[derive(Clone, Debug)]
pub struct RequestTiming {
    pub generation: u64,
    pub sequence: u64,
    pub kind: &'static str,
    pub outcome: &'static str,
    pub queue_ms: u64,
    pub response_ms: u64,
}

pub struct ConnectionApplicationCoordinator {
    reply_health: reply_health::ReplyHealth,
    timings: VecDeque<RequestTiming>,
    generation: Generation,
    phase: Phase,
    standalone_bms: bool,
    settings: telemetry::Settings,
    preferred_controller_can_id: Option<u8>,
    routing_hint: Option<discovery::RoutingHint>,
    continuity: discovery::Continuity,
    preferred_bms_can_id: Option<u8>,
    awaiting_activation: Option<ActivationFacts>,
    direct_firmware_attempt: u8,
    unverified_controller: Option<discovery::RoutedIdentity>,
    reported_refloat_version: Option<String>,
    refloat_version: Option<compatibility::WireVersion>,
    refloat_gnss: bool,
    dynamic_layout: Option<dynamic::DynamicLayout>,
    configuration_token: Option<mutation::MutationToken>,
    discovery: discovery::ControllerDiscovery,
    scheduler: scheduler::VescRequestScheduler,
    telemetry: telemetry::RefloatTelemetryService,
    bms: bms::VescBmsTelemetryService,
    mutations: mutation::DeviceMutationCoordinator,
    configuration: configuration::ConfigurationService,
    current_limits: Option<controller_data::CurrentLimits>,
}

impl Default for ConnectionApplicationCoordinator {
    fn default() -> Self {
        Self {
            reply_health: reply_health::ReplyHealth::default(),
            timings: VecDeque::new(),
            generation: Generation(0),
            phase: Phase::Disconnected,
            standalone_bms: false,
            settings: telemetry::Settings {
                idle_interval_ms: 1_000,
                post_ride_tail_ms: 5_000,
            },
            preferred_controller_can_id: None,
            routing_hint: None,
            continuity: discovery::Continuity::default(),
            preferred_bms_can_id: None,
            awaiting_activation: None,
            direct_firmware_attempt: 0,
            unverified_controller: None,
            reported_refloat_version: None,
            refloat_version: None,
            refloat_gnss: false,
            dynamic_layout: None,
            configuration_token: None,
            discovery: discovery::ControllerDiscovery::new(),
            scheduler: scheduler::VescRequestScheduler::default(),
            telemetry: telemetry::RefloatTelemetryService::default(),
            bms: bms::VescBmsTelemetryService::new(),
            mutations: mutation::DeviceMutationCoordinator::new(),
            configuration: configuration::ConfigurationService::new(),
            current_limits: None,
        }
    }
}

impl std::fmt::Debug for ConnectionApplicationCoordinator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConnectionApplicationCoordinator")
            .field("snapshot", &self.snapshot())
            .field("settings", &self.settings)
            .field("routing_hint", &self.routing_hint)
            .field("continuity", &self.continuity)
            .field("has_dynamic_layout", &self.dynamic_layout.is_some())
            .field("has_refloat_version", &self.refloat_version.is_some())
            .field("configuration_token", &self.configuration_token)
            .field("has_current_limits", &self.current_limits.is_some())
            .finish_non_exhaustive()
    }
}

impl ConnectionApplicationCoordinator {
    pub fn clear_recording(&mut self) {
        self.telemetry = telemetry::RefloatTelemetryService::default();
    }

    pub fn restore_recording(
        &mut self,
        now_ms: u64,
        elapsed_ms: u64,
        sequence: u64,
        riding_ms: u64,
        max_speed_kph: Option<f64>,
    ) {
        self.telemetry
            .restore_recording(now_ms, elapsed_ms, sequence, riding_ms, max_speed_kph);
    }

    pub fn take_request_timings(&mut self) -> Vec<RequestTiming> {
        self.timings.drain(..).collect()
    }

    fn record_request(
        &mut self,
        request: &scheduler::ScheduledRequest,
        outcome: &'static str,
        now_ms: u64,
    ) {
        use scheduler::RequestContext as C;
        let kind = match request.context {
            C::RefloatTelemetry(_) => {
                let offset = usize::from(request.payload.first() == Some(&34)) * 2;
                let payload = request.payload.get(offset..).unwrap_or_default();
                if payload == dynamic::selectable_fast_request() {
                    "fast"
                } else if payload.get(2) == Some(&33) {
                    "slow"
                } else if payload == [36, 101, 10, 0] {
                    "fault"
                } else {
                    "telemetry"
                }
            }
            C::RefloatLighting(_) => "lighting",
            C::BmsValues(_) => "bms",
            C::CurrentLimits(_) => "limits",
            C::RefloatConfigurationSchema { .. }
            | C::RefloatConfigurationValues { .. }
            | C::RefloatConfigurationWrite { .. }
            | C::RefloatConfigurationVerification { .. } => "configuration",
            _ => "discovery_or_control",
        };
        if self.timings.len() == 128 {
            self.timings.pop_front();
        }
        self.timings.push_back(RequestTiming {
            generation: request.generation.0,
            sequence: request.sequence,
            kind,
            outcome,
            queue_ms: request
                .dispatched_at_ms
                .unwrap_or(now_ms)
                .saturating_sub(request.enqueued_at_ms),
            response_ms: request
                .dispatched_at_ms
                .map_or(0, |start| now_ms.saturating_sub(start)),
        });
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            reported_refloat_version: self.reported_refloat_version.clone(),
            possible_can_id_conflict: self.reply_health.warned,
            generation: self.generation,
            phase: self.phase,
            standalone_bms: self.standalone_bms,
            awaiting_activation: self.awaiting_activation.clone(),
            discovery: self.discovery.snapshot(),
            scheduler: self.scheduler.snapshot(),
            telemetry: self.telemetry.snapshot(),
            bms: self.bms.snapshot(),
            mutations: self.mutations.snapshot(),
            configuration: self.configuration.snapshot(),
            configuration_write_block_reason: self.configuration_write_block_reason(),
            current_limits: self.current_limits,
        }
    }

    fn configuration_write_block_reason(&self) -> Option<ConfigurationWriteBlockReason> {
        use ConfigurationWriteBlockReason as R;
        use mutation::ConfigurationBlockReason as M;
        if self.phase != Phase::Ready || self.standalone_bms {
            return Some(R::Reconnecting);
        }
        let configuration = self.configuration.snapshot();
        match configuration.availability {
            configuration::Availability::Loading => return Some(R::Loading),
            configuration::Availability::Unavailable => return Some(R::Unavailable),
            configuration::Availability::Ready => {}
        }
        if let Some(reason) = self.mutations.configuration_block_reason(
            self.scheduler.snapshot().generation,
            mutation::ConfigurationMutation::Write,
        ) {
            return Some(match reason {
                M::Reconnecting => R::Reconnecting,
                M::ReadOnly => R::ReadOnly,
                M::Busy => R::Busy,
                M::Flywheel => R::Flywheel,
                M::Riding => R::Riding,
                M::StateUnknown => R::StateUnknown,
                M::PackageNotReady => R::PackageNotReady,
                M::ModeNotNormal => R::ModeNotNormal,
            });
        }
        if configuration.pending.is_some() {
            return Some(R::Busy);
        }
        if configuration.current.is_none() {
            return Some(R::CurrentRequired);
        }
        None
    }

    #[must_use]
    pub fn configuration_schema(&self) -> Option<&configuration_codec::ConfigurationSchema> {
        self.configuration.loaded_schema()
    }

    pub fn apply(&mut self, command: Command) -> Result<Vec<Effect>, CoordinatorError> {
        match command {
            Command::Begin(facts) => self.begin(facts),
            Command::SelectController {
                generation,
                can_id,
                now_ms,
            } => self.select_controller(generation, can_id, now_ms),
            Command::Receive {
                generation,
                payload,
                now_ms,
                observed_at_epoch_ms,
            } => self.receive(generation, payload, now_ms, observed_at_epoch_ms),
            Command::DeadlineFired {
                generation,
                token,
                now_ms,
            } => self.deadline_fired(generation, token, now_ms),
            Command::TimerFired {
                generation,
                token,
                now_ms,
            } => self.timer_fired(generation, token, now_ms),
            Command::Activate {
                generation,
                now_ms,
                session_started,
                cached_configuration_schema,
            } => self.activate(
                generation,
                now_ms,
                session_started,
                cached_configuration_schema,
            ),
            Command::SetControl {
                generation,
                control,
                enabled,
                now_ms,
            } => self.set_control(generation, control, enabled, now_ms),
            Command::ClearFatalError { generation, now_ms } => {
                self.clear_fatal_error(generation, now_ms)
            }
            Command::ShutdownController { generation, now_ms } => {
                self.shutdown_controller(generation, now_ms)
            }
            Command::ReadConfiguration {
                generation,
                defaults,
                now_ms,
            } => self.read_configuration(generation, defaults, now_ms),
            Command::WriteConfiguration {
                generation,
                mutation,
                package_version,
                fingerprint,
                wire_signature,
                base_revision,
                values,
                now_ms,
            } => self.write_configuration(
                generation,
                mutation,
                &package_version,
                &fingerprint,
                wire_signature,
                base_revision,
                &values,
                now_ms,
            ),
            Command::UpdateSettings {
                generation,
                settings,
                now_ms,
            } => self.update_settings(generation, settings, now_ms),
            Command::Refresh { generation, now_ms } => self.refresh(generation, now_ms),
            Command::Reset { now_ms } => self.reset(now_ms),
        }
    }

    fn begin(&mut self, facts: BeginFacts) -> Result<Vec<Effect>, CoordinatorError> {
        facts.validate()?;
        validate_now(facts.now_ms)?;
        let mut effects = Vec::new();
        self.reset_internal(
            facts.now_ms,
            false,
            facts.continuity.has_active_session,
            &mut effects,
        )?;
        self.settings = facts.settings;
        self.preferred_controller_can_id = facts.preferred_controller_can_id;
        self.routing_hint = facts.routing_hint;
        self.continuity = facts.continuity;
        self.preferred_bms_can_id = facts.preferred_bms_can_id;
        self.phase = Phase::Identifying;
        self.direct_firmware_attempt = 1;
        let transition = self.discovery.begin();
        self.process_discovery(transition, facts.now_ms, &mut effects)?;
        self.publish_state(&mut effects)?;
        Ok(effects)
    }

    fn select_controller(
        &mut self,
        generation: Generation,
        can_id: u8,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        validate_now(now_ms)?;
        if generation != self.generation || self.phase != Phase::SelectingController {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        validate_can_id(can_id)?;
        let transition = self.discovery.select_controller(can_id)?;
        let mut effects = Vec::new();
        self.phase = Phase::Identifying;
        self.process_discovery(transition, now_ms, &mut effects)?;
        self.publish_state(&mut effects)?;
        Ok(effects)
    }

    fn receive(
        &mut self,
        generation: Generation,
        payload: Vec<u8>,
        now_ms: u64,
        observed_at_epoch_ms: i64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        validate_now(now_ms)?;
        if generation != self.generation {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        if observed_at_epoch_ms <= 0 {
            return Err(CoordinatorError::InvalidObservationTime);
        }
        let expected = expected_response(&payload)?;
        // Forwarded replies omit their source route. Require the controller's
        // own CAN ID before admitting a motor identity to discovery. A late ID
        // reply for another route must leave this request and deadline intact.
        if expected.command == controller_data::CONTROLLER_CAN_ID_REQUEST[0]
            && let Some(request) = self.scheduler.snapshot().in_flight
            && let scheduler::RequestContext::ControllerRouteVerification(scheduler::Route::Can(
                can_id,
            )) = request.context
            && let Ok(reported_id) = controller_data::decode_controller_can_id(&payload)
            && reported_id != can_id
        {
            return self.unexpected_reply(&payload, now_ms, Diagnostic::UnexpectedResponse);
        }
        // Schema pages share command 92. Correlate the echoed index/offset
        // before the scheduler retires a request; delayed pages must not fail
        // an otherwise valid cold download or cancel its current deadline.
        if expected.command == custom_configuration::GET_SCHEMA_COMMAND
            && let Some(request) = self.scheduler.snapshot().in_flight
            && let scheduler::RequestContext::RefloatConfigurationSchema { index, offset, .. } =
                request.context
            && let Ok(configuration::Response::SchemaChunk {
                index: actual_index,
                offset: actual_offset,
                ..
            }) = configuration_response(&payload)
            && (actual_index != index || actual_offset != offset)
        {
            return self.unexpected_reply(&payload, now_ms, Diagnostic::ConfigurationStale);
        }
        // Fast and slow command-33 replies share a command ID, but echo different
        // field masks. A late reply must not retire the other request's deadline.
        if expected.command == CUSTOM_APP_DATA_COMMAND
            && expected.refloat_command == Some(dynamic::SELECTABLE_REALTIME_DATA_COMMAND)
            && let Some(header) = payload.get(3..12)
            && let Some(request) = self.scheduler.snapshot().in_flight
            && request.expected_response == expected
            && let scheduler::RequestContext::RefloatTelemetry(route) = request.context
        {
            let offset = match route {
                scheduler::Route::Direct => 0,
                scheduler::Route::Can(_) => 2,
            };
            if request.payload.get(offset + 3..offset + 12) != Some(header) {
                return self.unexpected_reply(&payload, now_ms, Diagnostic::UnexpectedResponse);
            }
        }
        let warning_changed = self
            .scheduler
            .snapshot()
            .in_flight
            .is_none_or(|request| request.expected_response != expected)
            && self.reply_health.unexpected(&payload, now_ms);
        let (_, scheduler_effects) = self.scheduler.receive(
            payload,
            expected,
            &scheduler::ContextMatch::Any,
            true,
            now_ms,
        )?;
        let mut effects = Vec::new();
        if warning_changed {
            self.publish_state(&mut effects)?;
        }
        self.process_scheduler(
            scheduler_effects,
            now_ms,
            observed_at_epoch_ms,
            &mut effects,
        )?;
        let resumed = self.scheduler.resume(now_ms)?;
        self.process_scheduler(resumed, now_ms, observed_at_epoch_ms, &mut effects)?;
        Ok(effects)
    }

    fn unexpected_reply(
        &mut self,
        payload: &[u8],
        now_ms: u64,
        diagnostic: Diagnostic,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        let mut effects = vec![Effect::Diagnostic(diagnostic)];
        if self.reply_health.unexpected(payload, now_ms) {
            self.publish_state(&mut effects)?;
        }
        Ok(effects)
    }

    fn deadline_fired(
        &mut self,
        generation: Generation,
        token: scheduler::DeadlineToken,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        validate_now(now_ms)?;
        if generation != self.generation {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        let scheduler_effects = self.scheduler.expire_deadline(token, now_ms)?;
        let mut effects = Vec::new();
        self.process_scheduler(scheduler_effects, now_ms, 1, &mut effects)?;
        Ok(effects)
    }

    fn timer_fired(
        &mut self,
        generation: Generation,
        token: TimerToken,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        validate_now(now_ms)?;
        if generation != self.generation {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        let mut effects = Vec::new();
        match token {
            TimerToken::Refloat(token) => {
                let child = self
                    .telemetry
                    .apply(telemetry::Command::TimerFired { token, now_ms })?;
                self.process_telemetry(child, now_ms, &mut effects)?;
            }
            TimerToken::Bms(token) => {
                let child = self.bms.apply(bms::Command::TimerFired(token))?;
                self.process_bms(child, now_ms, &mut effects)?;
            }
            TimerToken::Mutation(MutationTimerToken::Confirmation(token)) => {
                let child = self.mutations.apply(mutation::Command::Fail {
                    token,
                    reason: mutation::FailureReason::ConfirmationTimedOut,
                })?;
                self.process_mutation(child, now_ms, &mut effects)?;
                if let Some(route) = self.mutations.snapshot().route {
                    let scheduler_effects = self.scheduler.cancel_matching(
                        &scheduler::CancellationSelector::Exact(
                            scheduler::RequestContext::RefloatControl(route),
                        ),
                        true,
                        now_ms,
                    )?;
                    self.process_scheduler(scheduler_effects, now_ms, 1, &mut effects)?;
                }
            }
            TimerToken::Mutation(MutationTimerToken::Shutdown(token)) => {
                let child = self
                    .mutations
                    .apply(mutation::Command::ClearShutdown { token })?;
                self.process_mutation(child, now_ms, &mut effects)?;
                if let Some(route) = self.mutations.snapshot().route {
                    let scheduler_effects = self.scheduler.cancel_matching(
                        &scheduler::CancellationSelector::Exact(
                            scheduler::RequestContext::ControllerShutdown(route),
                        ),
                        true,
                        now_ms,
                    )?;
                    self.process_scheduler(scheduler_effects, now_ms, 1, &mut effects)?;
                }
            }
            TimerToken::Mutation(MutationTimerToken::LightingRefresh(scheduler_generation)) => {
                let child = self.mutations.apply(mutation::Command::RefreshLighting {
                    generation: scheduler_generation,
                })?;
                self.process_mutation(child, now_ms, &mut effects)?;
                if self.phase == Phase::Ready
                    && self.scheduler.snapshot().generation == scheduler_generation
                {
                    push_effect(
                        &mut effects,
                        Effect::ReplaceTimer {
                            token: TimerToken::Mutation(MutationTimerToken::LightingRefresh(
                                scheduler_generation,
                            )),
                            interval_ms: mutation::LIGHTING_TIMEOUT_MS,
                        },
                    )?;
                }
            }
        }
        Ok(effects)
    }

    fn activate(
        &mut self,
        generation: Generation,
        now_ms: u64,
        session_started: bool,
        cached_configuration_schema: Option<Vec<u8>>,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        validate_now(now_ms)?;
        if generation != self.generation
            || self.phase != Phase::AwaitingActivation
            || self.standalone_bms
        {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        let Some(activation) = self.awaiting_activation.clone() else {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        };
        let scheduler_generation = self.scheduler.snapshot().generation;
        let route = scheduler_route(activation.selected.route)?;
        let family = telemetry_family(activation.compatibility.telemetry);
        let configuration = telemetry::Configuration {
            service_generation: self.generation.0,
            scheduler_generation,
            route,
            family,
            selectable_gnss: self.refloat_gnss,
            alerts_available: activation.compatibility.control.fatal_clear_available,
            settings: self.settings,
        };
        configuration.validate()?;

        let mut effects = Vec::new();
        let child =
            self.mutations
                .apply(mutation::Command::Configure(mutation::ConfigureFacts {
                    generation: scheduler_generation,
                    route,
                    firmware: activation.selected.identity.clone(),
                    refloat: activation.compatibility,
                    connection_ready: true,
                }))?;
        self.process_mutation(child, now_ms, &mut effects)?;
        let child = self.configuration.begin(configuration::BeginFacts {
            route: configuration_route(route),
            generation: configuration::Generation(scheduler_generation.0),
            firmware: configuration_firmware(&activation.selected.identity),
            reported_version: activation.refloat_version.to_string(),
            writable: self
                .mutations
                .snapshot()
                .controls
                .is_some_and(|controls| controls.writable),
            flywheel: false,
            cached_schema: cached_configuration_schema,
        });
        self.process_configuration(child, now_ms, &mut effects)?;
        self.enqueue(
            current_limits_request_facts(route, scheduler_generation)?,
            now_ms,
            &mut effects,
        )?;
        let refresh_generation = scheduler_generation;
        push_effect(
            &mut effects,
            Effect::ReplaceTimer {
                token: TimerToken::Mutation(MutationTimerToken::LightingRefresh(
                    refresh_generation,
                )),
                interval_ms: mutation::LIGHTING_TIMEOUT_MS,
            },
        )?;
        let child = self.telemetry.apply(telemetry::Command::Start {
            configuration,
            now_ms,
        })?;
        self.process_telemetry(child, now_ms, &mut effects)?;
        if session_started {
            let child = self
                .telemetry
                .apply(telemetry::Command::MarkSessionStarted {
                    service_generation: self.generation.0,
                    now_ms,
                })?;
            self.process_telemetry(child, now_ms, &mut effects)?;
        }

        let child = self.bms.apply(bms::Command::Begin(bms::Configuration {
            service_generation: self.generation.0,
            scheduler_generation,
            local_identity: discovery::RoutedIdentity {
                route: discovery::Route::Direct,
                identity: activation.transport.clone(),
            },
            reusable_inventory: activation
                .inventory_complete
                .then_some(activation.inventory.clone()),
            preferred_can_id: self.preferred_bms_can_id,
        }))?;
        self.process_bms(child, now_ms, &mut effects)?;

        self.awaiting_activation = None;
        self.phase = Phase::Ready;
        self.publish_state(&mut effects)?;
        Ok(effects)
    }

    fn set_control(
        &mut self,
        generation: Generation,
        control: mutation::BooleanControl,
        enabled: bool,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        let scheduler_generation = self.ready_generation(generation, now_ms)?;
        let child = self.mutations.apply(mutation::Command::SetControl {
            generation: scheduler_generation,
            control,
            enabled,
        })?;
        let mut effects = Vec::new();
        self.process_mutation(child, now_ms, &mut effects)?;
        Ok(effects)
    }

    fn clear_fatal_error(
        &mut self,
        generation: Generation,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        let scheduler_generation = self.ready_generation(generation, now_ms)?;
        let child = self.mutations.apply(mutation::Command::ClearFatalError {
            generation: scheduler_generation,
        })?;
        let mut effects = Vec::new();
        self.process_mutation(child, now_ms, &mut effects)?;
        Ok(effects)
    }

    fn shutdown_controller(
        &mut self,
        generation: Generation,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        let scheduler_generation = self.ready_generation(generation, now_ms)?;
        let child = self
            .mutations
            .apply(mutation::Command::ShutdownController {
                generation: scheduler_generation,
            })?;
        let mut effects = Vec::new();
        self.process_mutation(child, now_ms, &mut effects)?;
        Ok(effects)
    }

    fn read_configuration(
        &mut self,
        generation: Generation,
        defaults: bool,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        self.ready_generation(generation, now_ms)?;
        let child = self.configuration.request_values(defaults)?;
        let mut effects = Vec::new();
        self.process_configuration(child, now_ms, &mut effects)?;
        Ok(effects)
    }

    #[allow(clippy::too_many_arguments)]
    fn write_configuration(
        &mut self,
        generation: Generation,
        configuration_mutation: mutation::ConfigurationMutation,
        package_version: &str,
        fingerprint: &str,
        wire_signature: u32,
        base_revision: u64,
        values: &configuration_codec::ConfigurationValues,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        let scheduler_generation = self.ready_generation(generation, now_ms)?;
        let transition = self.configuration.request_write(
            package_version,
            fingerprint,
            wire_signature,
            base_revision,
            values,
        )?;
        let lease = match self.mutations.apply(mutation::Command::BeginConfiguration {
            generation: scheduler_generation,
            mutation: configuration_mutation,
        }) {
            Ok(effects) => effects,
            Err(error) => {
                self.configuration
                    .abort_active_operation(configuration::Generation(scheduler_generation.0));
                return Err(error.into());
            }
        };
        let mut effects = Vec::new();
        self.process_mutation(lease, now_ms, &mut effects)?;
        self.process_configuration(transition, now_ms, &mut effects)?;
        Ok(effects)
    }

    fn ready_generation(
        &self,
        generation: Generation,
        now_ms: u64,
    ) -> Result<scheduler::Generation, CoordinatorError> {
        validate_now(now_ms)?;
        if generation != self.generation || self.phase != Phase::Ready || self.standalone_bms {
            return Err(CoordinatorError::Mutation(
                mutation::CoordinatorError::InvalidState,
            ));
        }
        Ok(self.scheduler.snapshot().generation)
    }

    fn update_settings(
        &mut self,
        generation: Generation,
        settings: telemetry::Settings,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        validate_now(now_ms)?;
        settings.validate()?;
        if generation != self.generation {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        self.settings = settings;
        let child = self.telemetry.apply(telemetry::Command::UpdateSettings {
            service_generation: generation.0,
            settings,
            now_ms,
        })?;
        let mut effects = Vec::new();
        self.process_telemetry(child, now_ms, &mut effects)?;
        Ok(effects)
    }

    fn refresh(
        &mut self,
        generation: Generation,
        now_ms: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        validate_now(now_ms)?;
        if generation != self.generation || self.phase != Phase::Ready || self.standalone_bms {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        let child = self.telemetry.apply(telemetry::Command::Refresh {
            service_generation: generation.0,
            now_ms,
        })?;
        let mut effects = Vec::new();
        self.process_telemetry(child, now_ms, &mut effects)?;
        Ok(effects)
    }

    fn reset(&mut self, now_ms: u64) -> Result<Vec<Effect>, CoordinatorError> {
        validate_now(now_ms)?;
        let mut effects = Vec::new();
        self.reset_internal(now_ms, true, false, &mut effects)?;
        Ok(effects)
    }

    fn reset_internal(
        &mut self,
        now_ms: u64,
        publish: bool,
        retain_session: bool,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        if self.mutations.snapshot().configured {
            push_effect(
                effects,
                Effect::CancelTimer(TimerToken::Mutation(MutationTimerToken::LightingRefresh(
                    self.scheduler.snapshot().generation,
                ))),
            )?;
        }
        let mutation_effects = self.mutations.apply(mutation::Command::Reset)?;
        self.process_mutation(mutation_effects, now_ms, effects)?;
        self.configuration.reset();
        self.current_limits = None;
        self.reply_health = reply_health::ReplyHealth::default();
        self.configuration_token = None;
        // A reconnect resets connection work, but the retained ride still owns
        // its original monotonic origin and unique sample sequence.
        let telemetry_effects = self.telemetry.apply(if retain_session {
            telemetry::Command::ResetConnection
        } else {
            telemetry::Command::Reset
        })?;
        self.process_telemetry(telemetry_effects, now_ms, effects)?;
        let bms_effects = self.bms.apply(bms::Command::Reset)?;
        self.process_bms(bms_effects, now_ms, effects)?;
        let scheduler_effects = self.scheduler.begin_generation(now_ms)?;
        self.process_scheduler(scheduler_effects, now_ms, 1, effects)?;
        self.discovery.reset();
        self.generation = Generation(self.generation.0.wrapping_add(1));
        self.phase = Phase::Disconnected;
        self.standalone_bms = false;
        self.awaiting_activation = None;
        self.direct_firmware_attempt = 0;
        self.unverified_controller = None;
        self.reported_refloat_version = None;
        self.refloat_version = None;
        self.refloat_gnss = false;
        self.dynamic_layout = None;
        self.preferred_controller_can_id = None;
        self.routing_hint = None;
        self.continuity = discovery::Continuity::default();
        self.preferred_bms_can_id = None;
        if publish {
            self.publish_state(effects)?;
        }
        Ok(())
    }

    fn process_discovery(
        &mut self,
        transition: discovery::Transition,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        for effect in transition.effects {
            match effect {
                discovery::Effect::ProbeFirmware { route, .. } => {
                    if route == discovery::Route::Direct && self.direct_firmware_attempt == 0 {
                        self.direct_firmware_attempt = 1;
                    }
                    let facts = discovery_request_facts(
                        DiscoveryRequest::Firmware {
                            route,
                            direct_attempt: self.direct_firmware_attempt,
                        },
                        self.scheduler.snapshot().generation,
                    )?;
                    self.enqueue(facts, now_ms, effects)?;
                }
                discovery::Effect::PingCan => {
                    let facts = discovery_request_facts(
                        DiscoveryRequest::CanPing,
                        self.scheduler.snapshot().generation,
                    )?;
                    self.enqueue(facts, now_ms, effects)?;
                }
                discovery::Effect::ClearRoutingHint => {
                    self.routing_hint = None;
                    push_effect(effects, Effect::ClearRoutingHint)?;
                }
                discovery::Effect::RequestRefloatInfo {
                    route,
                    remember_controller,
                } => {
                    if let Some(selected) = remember_controller {
                        push_effect(effects, Effect::StoreRoutingHint(selected))?;
                    }
                    let facts = discovery_request_facts(
                        DiscoveryRequest::RefloatInfo { route },
                        self.scheduler.snapshot().generation,
                    )?;
                    self.enqueue(facts, now_ms, effects)?;
                }
                discovery::Effect::RequestTelemetryLayout => {
                    let Some(route) = self.discovery.snapshot().route else {
                        continue;
                    };
                    let child = self.telemetry.apply(telemetry::Command::BeginLayout {
                        service_generation: self.generation.0,
                        scheduler_generation: self.scheduler.snapshot().generation,
                        route: scheduler_route(route)?,
                    })?;
                    self.process_telemetry(child, now_ms, effects)?;
                }
                discovery::Effect::Ready => {
                    self.prepare_activation(effects)?;
                }
                discovery::Effect::SelectingController => {
                    self.phase = Phase::SelectingController;
                    self.publish_state(effects)?;
                }
                discovery::Effect::Failed(issue) => {
                    self.phase = Phase::Failed;
                    self.awaiting_activation = None;
                    push_effect(effects, Effect::Failure(Failure::Discovery(issue)))?;
                    self.publish_state(effects)?;
                }
            }
        }
        Ok(())
    }

    fn prepare_activation(&mut self, effects: &mut Vec<Effect>) -> Result<(), CoordinatorError> {
        let snapshot = self.discovery.snapshot();
        let (Some(selected), Some(transport), Some(profile)) = (
            snapshot.selected,
            snapshot.transport,
            snapshot.refloat_profile,
        ) else {
            return Ok(());
        };
        let facts = ActivationFacts {
            generation: self.generation,
            selected: selected.clone(),
            transport,
            compatibility: profile,
            refloat_version: self
                .refloat_version
                .ok_or(CoordinatorError::InvalidPayload)?,
            configuration_cache_identity: configuration::cache_identity(
                &configuration_firmware(&selected.identity),
                &self
                    .refloat_version
                    .ok_or(CoordinatorError::InvalidPayload)?
                    .to_string(),
            ),
            inventory: snapshot.inventory,
            inventory_complete: snapshot.inventory_complete,
        };
        self.phase = Phase::AwaitingActivation;
        self.awaiting_activation = Some(facts.clone());
        push_effect(effects, Effect::PrepareActivation(facts))?;
        self.publish_state(effects)
    }

    fn start_standalone_bms(
        &mut self,
        identity: firmware::FirmwareIdentity,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        self.standalone_bms = true;
        self.phase = Phase::Ready;
        self.awaiting_activation = None;
        self.publish_state(effects)?;
        let child = self.bms.apply(bms::Command::Begin(bms::Configuration {
            service_generation: self.generation.0,
            scheduler_generation: self.scheduler.snapshot().generation,
            local_identity: bms::RoutedIdentity {
                route: bms::Route::Direct,
                identity,
            },
            reusable_inventory: None,
            preferred_can_id: None,
        }))?;
        self.process_bms(child, now_ms, effects)
    }

    fn enqueue(
        &mut self,
        facts: scheduler::RequestFacts,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let scheduler_effects = self.scheduler.enqueue(facts, now_ms)?;
        self.process_scheduler(scheduler_effects, now_ms, 1, effects)
    }

    fn process_scheduler(
        &mut self,
        scheduler_effects: Vec<scheduler::Effect>,
        now_ms: u64,
        observed_at_epoch_ms: i64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let mut pending = VecDeque::from(scheduler_effects);
        while let Some(effect) = pending.pop_front() {
            match effect {
                scheduler::Effect::CancelDeadline(token) => {
                    push_effect(effects, Effect::CancelDeadline(token))?;
                }
                scheduler::Effect::ArmDeadline { token, timeout_ms } => {
                    push_effect(effects, Effect::ArmDeadline { token, timeout_ms })?;
                }
                scheduler::Effect::Write(request) => {
                    self.record_request(&request, "sent", now_ms);
                    push_effect(effects, Effect::Write(request))?;
                }
                scheduler::Effect::Complete { request, outcome } => {
                    if let scheduler::Completion::Response(ref payload) = outcome {
                        self.reply_health.accepted(&request, payload, now_ms);
                    }
                    self.record_request(
                        &request,
                        match outcome {
                            scheduler::Completion::Response(_) => "response",
                            scheduler::Completion::Sent => "sent_only",
                            scheduler::Completion::TimedOut => "timeout",
                            scheduler::Completion::QueueFull => "queue_full",
                            scheduler::Completion::Cancelled => "cancelled",
                            scheduler::Completion::StaleGeneration => "stale",
                        },
                        now_ms,
                    );
                    self.dispatch_completion(
                        request,
                        outcome,
                        now_ms,
                        observed_at_epoch_ms,
                        effects,
                    )?;
                }
                scheduler::Effect::Diagnostic(value) => {
                    push_effect(effects, Effect::Diagnostic(Diagnostic::Scheduler(value)))?;
                }
            }
        }
        Ok(())
    }

    fn dispatch_completion(
        &mut self,
        request: scheduler::ScheduledRequest,
        outcome: scheduler::Completion,
        now_ms: u64,
        observed_at_epoch_ms: i64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        match request.context.clone() {
            scheduler::RequestContext::ControllerFirmware(route) => {
                self.complete_controller_firmware(route, outcome, now_ms, effects)
            }
            scheduler::RequestContext::ControllerRouteVerification(route) => {
                self.complete_controller_route_verification(route, outcome, now_ms, effects)
            }
            scheduler::RequestContext::CanPing => {
                self.complete_controller_can_ping(outcome, now_ms, effects)
            }
            scheduler::RequestContext::RefloatInfo(_) => {
                self.complete_refloat_info(outcome, now_ms, effects)
            }
            scheduler::RequestContext::RefloatLayout(_) => {
                self.complete_refloat_layout(outcome, now_ms, effects)
            }
            scheduler::RequestContext::RefloatTelemetry(_) => {
                self.complete_refloat_telemetry(outcome, now_ms, observed_at_epoch_ms, effects)
            }
            scheduler::RequestContext::CurrentLimits(_) => {
                self.complete_current_limits(outcome, effects)
            }
            scheduler::RequestContext::AuxiliaryCanPing => self.complete_bms(
                request,
                bms::RequestKind::CanScan,
                None,
                outcome,
                now_ms,
                observed_at_epoch_ms,
                effects,
            ),
            scheduler::RequestContext::AuxiliaryFirmware(route) => self.complete_bms(
                request,
                bms::RequestKind::Firmware,
                Some(bms_route(route)?),
                outcome,
                now_ms,
                observed_at_epoch_ms,
                effects,
            ),
            scheduler::RequestContext::BmsValues(route) => self.complete_bms(
                request,
                bms::RequestKind::Values,
                Some(bms_route(route)?),
                outcome,
                now_ms,
                observed_at_epoch_ms,
                effects,
            ),
            scheduler::RequestContext::RefloatControl(_)
            | scheduler::RequestContext::RefloatLighting(_) => {
                self.complete_mutation_request(request, outcome, now_ms, effects)
            }
            scheduler::RequestContext::ControllerShutdown(_) => {
                self.complete_shutdown_request(outcome, now_ms, effects)
            }
            scheduler::RequestContext::RefloatConfigurationSchema { .. }
            | scheduler::RequestContext::RefloatConfigurationValues { .. }
            | scheduler::RequestContext::RefloatConfigurationWrite { .. }
            | scheduler::RequestContext::RefloatConfigurationVerification { .. } => {
                self.complete_configuration_request(request, outcome, now_ms, effects)
            }
        }
    }

    fn complete_mutation_request(
        &mut self,
        request: scheduler::ScheduledRequest,
        outcome: scheduler::Completion,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        match outcome {
            scheduler::Completion::Response(payload) => {
                let child = self.mutations.apply(mutation::Command::ObserveLighting {
                    generation: request.generation,
                    observation_sequence: request.sequence,
                    payload,
                })?;
                self.process_mutation(child, now_ms, effects)
            }
            scheduler::Completion::Sent => Ok(()),
            scheduler::Completion::TimedOut
            | scheduler::Completion::QueueFull
            | scheduler::Completion::Cancelled
            | scheduler::Completion::StaleGeneration => {
                let Some(token) = self.mutations.snapshot().pending.filter(|token| {
                    matches!(
                        token.kind,
                        mutation::PendingKind::Package
                            | mutation::PendingKind::Flywheel
                            | mutation::PendingKind::Leds
                            | mutation::PendingKind::Headlights
                            | mutation::PendingKind::ClearFatalError
                    )
                }) else {
                    return Ok(());
                };
                let child = self.mutations.apply(mutation::Command::Fail {
                    token,
                    reason: mutation::FailureReason::RequestFailed,
                })?;
                self.process_mutation(child, now_ms, effects)
            }
        }
    }

    fn complete_current_limits(
        &mut self,
        outcome: scheduler::Completion,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let scheduler::Completion::Response(payload) = outcome else {
            return Ok(());
        };
        if self.phase != Phase::Ready {
            return Ok(());
        }
        match controller_data::decode_current_limits(&payload) {
            Ok(current_limits) => {
                self.current_limits = Some(current_limits);
                self.publish_state(effects)
            }
            Err(_) => push_effect(
                effects,
                Effect::Diagnostic(Diagnostic::MalformedResponse(
                    MalformedResponse::CurrentLimits,
                )),
            ),
        }
    }

    fn complete_shutdown_request(
        &mut self,
        outcome: scheduler::Completion,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        if outcome == scheduler::Completion::Sent {
            return Ok(());
        }
        let Some(token) = self
            .mutations
            .snapshot()
            .pending
            .filter(|token| token.kind == mutation::PendingKind::ControllerShutdown)
        else {
            return Ok(());
        };
        let child = self
            .mutations
            .apply(mutation::Command::ClearShutdown { token })?;
        self.process_mutation(child, now_ms, effects)
    }

    fn complete_configuration_request(
        &mut self,
        request: scheduler::ScheduledRequest,
        outcome: scheduler::Completion,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let context = match configuration_context(&request.context) {
            Ok(context) => context,
            Err(error) => {
                push_effect(effects, Effect::Diagnostic(Diagnostic::ConfigurationStale))?;
                return Err(error);
            }
        };
        let generation = configuration::Generation(request.generation.0);
        let scheduler::Completion::Response(payload) = outcome else {
            if outcome != scheduler::Completion::Sent {
                self.configuration.request_failed(context, generation);
                push_effect(
                    effects,
                    Effect::Failure(Failure::Configuration(
                        configuration::ServiceError::Unavailable,
                    )),
                )?;
                self.fail_configuration_token(
                    mutation::FailureReason::RequestFailed,
                    now_ms,
                    effects,
                )?;
                self.publish_state(effects)?;
            }
            return Ok(());
        };
        let response = match configuration_response(&payload) {
            Ok(response) => response,
            Err(_) => {
                self.configuration.request_failed(context, generation);
                push_effect(
                    effects,
                    Effect::Diagnostic(Diagnostic::MalformedResponse(
                        MalformedResponse::RefloatConfiguration,
                    )),
                )?;
                push_effect(
                    effects,
                    Effect::Failure(Failure::Configuration(
                        configuration::ServiceError::MalformedResponse,
                    )),
                )?;
                self.fail_configuration_token(
                    mutation::FailureReason::RequestFailed,
                    now_ms,
                    effects,
                )?;
                self.publish_state(effects)?;
                return Ok(());
            }
        };
        match self.configuration.accept(response, context, generation) {
            Ok(transition) => {
                self.process_configuration(transition, now_ms, effects)?;
                if self.configuration_token.is_some()
                    && self.configuration.snapshot().pending.is_none()
                {
                    self.complete_configuration_token(now_ms, effects)?;
                }
            }
            Err(error) => {
                self.configuration.request_failed(context, generation);
                push_effect(effects, Effect::Failure(Failure::Configuration(error)))?;
                self.fail_configuration_token(
                    mutation::FailureReason::RequestFailed,
                    now_ms,
                    effects,
                )?;
                self.publish_state(effects)?;
            }
        }
        Ok(())
    }

    fn complete_controller_firmware(
        &mut self,
        route: scheduler::Route,
        outcome: scheduler::Completion,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        match outcome {
            scheduler::Completion::Response(payload) => match firmware::decode(&payload) {
                Ok(identity) => {
                    let route = discovery_route(route)?;
                    if route == discovery::Route::Direct {
                        let routed = bms::RoutedIdentity {
                            route,
                            identity: identity.clone(),
                        };
                        if bms::supports_bms(&routed) {
                            return self.start_standalone_bms(identity, now_ms, effects);
                        }
                        let transition = self.discovery.accept_transport(
                            identity,
                            self.preferred_controller_can_id,
                            self.routing_hint,
                        );
                        self.process_discovery(transition, now_ms, effects)
                    } else {
                        let discovery::Route::Can(can_id) = route else {
                            unreachable!();
                        };
                        if identity.hardware_type == firmware::MOTOR_HARDWARE_TYPE {
                            self.unverified_controller =
                                Some(discovery::RoutedIdentity { route, identity });
                            let facts = scheduler::RequestFacts {
                                payload: routed_payload(
                                    route,
                                    &controller_data::CONTROLLER_CAN_ID_REQUEST,
                                )?,
                                expected_response: scheduler::ExpectedResponse {
                                    command: controller_data::CONTROLLER_CAN_ID_REQUEST[0],
                                    refloat_command: None,
                                },
                                priority: scheduler::Priority::ControllerIdentification,
                                context: scheduler::RequestContext::ControllerRouteVerification(
                                    scheduler::Route::Can(can_id),
                                ),
                                timeout_ms: CAN_PROBE_TIMEOUT_MS,
                                generation: self.scheduler.snapshot().generation,
                                expects_response: true,
                            };
                            return self.enqueue(facts, now_ms, effects);
                        }
                        let transition =
                            self.discovery
                                .accept_can_identity(can_id, identity, self.continuity);
                        self.process_discovery(transition, now_ms, effects)
                    }
                }
                Err(_) => self.fail_discovery_response(
                    discovery::IssueCode::UnsupportedController,
                    MalformedResponse::Firmware,
                    now_ms,
                    effects,
                ),
            },
            scheduler::Completion::TimedOut | scheduler::Completion::QueueFull => match route {
                scheduler::Route::Direct if self.direct_firmware_attempt == 1 => {
                    self.direct_firmware_attempt = 2;
                    let facts = discovery_request_facts(
                        DiscoveryRequest::Firmware {
                            route: discovery::Route::Direct,
                            direct_attempt: 2,
                        },
                        self.scheduler.snapshot().generation,
                    )?;
                    self.enqueue(facts, now_ms, effects)
                }
                scheduler::Route::Direct => {
                    let generation = self.discovery.snapshot().identification_generation;
                    let transition = self.discovery.identification_timed_out(generation);
                    self.process_discovery(transition, now_ms, effects)
                }
                scheduler::Route::Can(can_id) => {
                    let generation = self.discovery.snapshot().probe_generation;
                    let transition = self.discovery.can_probe_timed_out(can_id, generation);
                    self.process_discovery(transition, now_ms, effects)
                }
            },
            scheduler::Completion::Cancelled | scheduler::Completion::StaleGeneration => Ok(()),
            scheduler::Completion::Sent => {
                push_effect(effects, Effect::Diagnostic(Diagnostic::UnexpectedResponse))
            }
        }
    }

    fn complete_controller_route_verification(
        &mut self,
        route: scheduler::Route,
        outcome: scheduler::Completion,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let Some(pending) = self.unverified_controller.take() else {
            return Ok(());
        };
        let scheduler::Route::Can(can_id) = route else {
            return Err(CoordinatorError::InvalidPayload);
        };
        if pending.route != discovery::Route::Can(can_id) {
            return Err(CoordinatorError::InvalidPayload);
        }
        match outcome {
            scheduler::Completion::Response(payload)
                if controller_data::decode_controller_can_id(&payload) == Ok(can_id) =>
            {
                let transition =
                    self.discovery
                        .accept_can_identity(can_id, pending.identity, self.continuity);
                self.process_discovery(transition, now_ms, effects)
            }
            scheduler::Completion::Cancelled | scheduler::Completion::StaleGeneration => Ok(()),
            _ => {
                // A false scan address, malformed reply or timeout cannot
                // become a candidate or overwrite a saved routing hint.
                push_effect(effects, Effect::Diagnostic(Diagnostic::UnexpectedResponse))?;
                let generation = self.discovery.snapshot().probe_generation;
                let transition = self.discovery.can_probe_timed_out(can_id, generation);
                self.process_discovery(transition, now_ms, effects)
            }
        }
    }

    fn complete_controller_can_ping(
        &mut self,
        outcome: scheduler::Completion,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        match outcome {
            scheduler::Completion::Response(payload) => match can_ids(&payload) {
                Ok(can_ids) => {
                    let transition = self.discovery.accept_can_ids(&can_ids, self.continuity);
                    self.process_discovery(transition, now_ms, effects)
                }
                Err(kind) => self.fail_discovery_response(
                    discovery::IssueCode::TooManyCanDevices,
                    kind,
                    now_ms,
                    effects,
                ),
            },
            scheduler::Completion::TimedOut | scheduler::Completion::QueueFull => {
                let generation = self.discovery.snapshot().identification_generation;
                let transition = self.discovery.identification_timed_out(generation);
                self.process_discovery(transition, now_ms, effects)
            }
            scheduler::Completion::Cancelled | scheduler::Completion::StaleGeneration => Ok(()),
            scheduler::Completion::Sent => {
                push_effect(effects, Effect::Diagnostic(Diagnostic::UnexpectedResponse))
            }
        }
    }

    fn complete_refloat_info(
        &mut self,
        outcome: scheduler::Completion,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        match outcome {
            scheduler::Completion::Response(payload) => {
                let identity = match compatibility::decode_info_identity(&payload) {
                    Ok(identity) => identity,
                    Err(_) => {
                        return self.fail_discovery_response(
                            discovery::IssueCode::RefloatInfoInvalid,
                            MalformedResponse::RefloatInfo,
                            now_ms,
                            effects,
                        );
                    }
                };
                let version = identity.version;
                self.reported_refloat_version = Some(identity.to_string());
                let admitted = compatibility::decode_info(&payload)
                    .ok()
                    .and_then(compatibility::compatibility)
                    .map(|profile| (version, profile));
                match admitted {
                    Some((version, profile)) => {
                        self.refloat_version = Some(version);
                        self.refloat_gnss = compatibility::decode_info_capabilities(&payload)
                            .is_ok_and(|capabilities| {
                                capabilities & compatibility::INFO_CAPABILITY_GNSS != 0
                            });
                        let transition = self.discovery.accept_refloat_profile(profile);
                        self.process_discovery(transition, now_ms, effects)
                    }
                    None => {
                        let transition = self
                            .discovery
                            .mark_failure(discovery::IssueCode::UnsupportedRefloat);
                        self.process_discovery(transition, now_ms, effects)
                    }
                }
            }
            scheduler::Completion::TimedOut | scheduler::Completion::QueueFull => {
                let transition = self
                    .discovery
                    .mark_failure(discovery::IssueCode::IdentificationTimeout);
                self.process_discovery(transition, now_ms, effects)
            }
            scheduler::Completion::Cancelled | scheduler::Completion::StaleGeneration => Ok(()),
            scheduler::Completion::Sent => {
                push_effect(effects, Effect::Diagnostic(Diagnostic::UnexpectedResponse))
            }
        }
    }

    fn complete_refloat_layout(
        &mut self,
        outcome: scheduler::Completion,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let request_outcome = telemetry_outcome(&outcome);
        let child = self.telemetry.apply(telemetry::Command::RequestCompleted {
            service_generation: self.generation.0,
            kind: telemetry::RequestKind::Layout,
            outcome: request_outcome,
        })?;
        self.process_telemetry(child, now_ms, effects)?;
        let scheduler::Completion::Response(payload) = outcome else {
            return Ok(());
        };
        let layout = dynamic::decode_layout(&payload);
        let supported = layout.as_ref().is_ok_and(layout_is_admitted);
        let child = self
            .telemetry
            .apply(telemetry::Command::LayoutEstablished {
                service_generation: self.generation.0,
            })?;
        self.process_telemetry(child, now_ms, effects)?;
        if let Ok(layout) = layout {
            if supported {
                self.dynamic_layout = Some(layout);
            }
        } else {
            push_effect(
                effects,
                Effect::Diagnostic(Diagnostic::MalformedResponse(
                    MalformedResponse::RefloatLayout,
                )),
            )?;
        }
        let transition = self.discovery.accept_telemetry_layout(supported);
        self.process_discovery(transition, now_ms, effects)
    }

    fn complete_refloat_telemetry(
        &mut self,
        outcome: scheduler::Completion,
        now_ms: u64,
        observed_at_epoch_ms: i64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let pending = self.telemetry.snapshot().pending_request;
        let Some(kind) = pending else {
            return push_effect(effects, Effect::Diagnostic(Diagnostic::UnexpectedResponse));
        };
        let child = self.telemetry.apply(telemetry::Command::RequestCompleted {
            service_generation: self.generation.0,
            kind,
            outcome: telemetry_outcome(&outcome),
        })?;
        self.process_telemetry(child, now_ms, effects)?;
        let scheduler::Completion::Response(payload) = outcome else {
            return Ok(());
        };
        let decoded = match kind {
            telemetry::RequestKind::Realtime => self
                .dynamic_layout
                .as_ref()
                .and_then(|layout| dynamic::decode_realtime(layout, &payload).ok())
                .map(telemetry::DecodedTelemetry::Dynamic),
            telemetry::RequestKind::Legacy | telemetry::RequestKind::Compact => {
                legacy::decode(&payload)
                    .ok()
                    .map(telemetry::DecodedTelemetry::Legacy)
            }
            telemetry::RequestKind::SelectableFast => {
                dynamic::decode_selectable(dynamic::SELECTABLE_FAST_MASK1, 0, &payload)
                    .ok()
                    .map(telemetry::DecodedTelemetry::Selectable)
            }
            telemetry::RequestKind::SelectableSlow => dynamic::decode_selectable(
                dynamic::SELECTABLE_SLOW_MASK1,
                dynamic::SELECTABLE_SLOW_MASK2
                    | if self.refloat_gnss {
                        dynamic::SELECTABLE_GNSS_MASK2
                    } else {
                        0
                    },
                &payload,
            )
            .ok()
            .map(telemetry::DecodedTelemetry::Selectable),
            telemetry::RequestKind::SelectableFault => legacy::decode_fault_only(&payload)
                .ok()
                .map(telemetry::DecodedTelemetry::SelectableFault),
            telemetry::RequestKind::Alerts => dynamic::alerts::decode_alerts(&payload)
                .ok()
                .map(telemetry::DecodedTelemetry::Alerts),
            telemetry::RequestKind::Layout => None,
        };
        let Some(decoded) = decoded else {
            let child = self.telemetry.apply(telemetry::Command::DecodeFailed {
                service_generation: self.generation.0,
                kind,
            })?;
            self.process_telemetry(child, now_ms, effects)?;
            return push_effect(
                effects,
                Effect::Diagnostic(Diagnostic::MalformedResponse(
                    MalformedResponse::RefloatTelemetry,
                )),
            );
        };
        let child = self.telemetry.apply(telemetry::Command::Receive {
            service_generation: self.generation.0,
            telemetry: Box::new(decoded),
            now_ms,
            observed_at_epoch_ms,
        })?;
        self.process_telemetry(child, now_ms, effects)
    }

    #[allow(clippy::too_many_arguments)]
    fn complete_bms(
        &mut self,
        _request: scheduler::ScheduledRequest,
        kind: bms::RequestKind,
        route: Option<bms::Route>,
        outcome: scheduler::Completion,
        now_ms: u64,
        observed_at_epoch_ms: i64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let snapshot = self.bms.snapshot();
        let child = self.bms.apply(bms::Command::RequestCompleted {
            service_generation: self.generation.0,
            auxiliary_generation: snapshot.auxiliary_generation,
            kind,
            route,
            outcome: bms_outcome(&outcome),
        })?;
        self.process_bms(child, now_ms, effects)?;
        let scheduler::Completion::Response(payload) = outcome else {
            return Ok(());
        };
        let snapshot = self.bms.snapshot();
        let child = match kind {
            bms::RequestKind::CanScan => match can_ids(&payload) {
                Ok(can_ids) => self.bms.apply(bms::Command::AcceptCanIds {
                    service_generation: self.generation.0,
                    auxiliary_generation: snapshot.auxiliary_generation,
                    can_ids,
                })?,
                Err(_) => self.bms.apply(bms::Command::DecodeFailed {
                    service_generation: self.generation.0,
                    auxiliary_generation: snapshot.auxiliary_generation,
                    kind,
                    route,
                })?,
            },
            bms::RequestKind::Firmware => match (route, firmware::decode(&payload)) {
                (Some(route), Ok(identity)) => self.bms.apply(bms::Command::AcceptFirmware {
                    service_generation: self.generation.0,
                    auxiliary_generation: snapshot.auxiliary_generation,
                    identity: bms::RoutedIdentity { route, identity },
                })?,
                _ => self.bms.apply(bms::Command::DecodeFailed {
                    service_generation: self.generation.0,
                    auxiliary_generation: snapshot.auxiliary_generation,
                    kind,
                    route,
                })?,
            },
            bms::RequestKind::Values => match route {
                Some(route) => self.bms.apply(bms::Command::ReceiveValues {
                    service_generation: self.generation.0,
                    auxiliary_generation: snapshot.auxiliary_generation,
                    route,
                    payload,
                    observed_at_epoch_ms,
                })?,
                None => Vec::new(),
            },
        };
        self.process_bms(child, now_ms, effects)
    }

    fn process_mutation(
        &mut self,
        child_effects: Vec<mutation::Effect>,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        for effect in child_effects {
            match effect {
                mutation::Effect::Publish(_) => self.publish_state(effects)?,
                mutation::Effect::Submit { token, request } => {
                    self.enqueue(request, now_ms, effects)?;
                    if self.mutations.snapshot().pending == Some(token)
                        && let Some((timer, interval_ms)) = mutation_timer(token)
                    {
                        push_effect(
                            effects,
                            Effect::ReplaceTimer {
                                token: TimerToken::Mutation(timer),
                                interval_ms,
                            },
                        )?;
                    }
                }
                mutation::Effect::SubmitObservation(request) => {
                    self.enqueue(request, now_ms, effects)?;
                }
                mutation::Effect::Lease(token) => {
                    self.configuration_token = Some(token);
                }
                mutation::Effect::Idempotent(_) => {
                    push_effect(effects, Effect::Mutation(MutationEvent::Idempotent))?;
                    self.publish_state(effects)?;
                }
                mutation::Effect::Confirmed(token) => {
                    if let Some((timer, _)) = mutation_timer(token) {
                        push_effect(effects, Effect::CancelTimer(TimerToken::Mutation(timer)))?;
                    }
                    if self.configuration_token == Some(token) {
                        self.configuration_token = None;
                    }
                    push_effect(effects, Effect::Mutation(MutationEvent::Confirmed(token)))?;
                }
                mutation::Effect::Released(token) => {
                    if let Some((timer, _)) = mutation_timer(token) {
                        push_effect(effects, Effect::CancelTimer(TimerToken::Mutation(timer)))?;
                    }
                    if self.configuration_token == Some(token) {
                        self.configuration_token = None;
                    }
                    push_effect(effects, Effect::Mutation(MutationEvent::Released(token)))?;
                }
                mutation::Effect::Failed { token, reason } => {
                    if let Some((timer, _)) = mutation_timer(token) {
                        push_effect(effects, Effect::CancelTimer(TimerToken::Mutation(timer)))?;
                    }
                    if self.configuration_token == Some(token) {
                        self.configuration
                            .abort_active_operation(configuration::Generation(token.generation.0));
                        self.configuration_token = None;
                    }
                    push_effect(
                        effects,
                        Effect::Failure(Failure::Mutation { token, reason }),
                    )?;
                    push_effect(
                        effects,
                        Effect::Mutation(MutationEvent::Failed { token, reason }),
                    )?;
                }
                mutation::Effect::Diagnostic(value) => {
                    push_effect(effects, Effect::Diagnostic(Diagnostic::Mutation(value)))?;
                }
            }
        }
        Ok(())
    }

    fn process_configuration(
        &mut self,
        transition: configuration::Transition,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        if let Some(cache) = transition.cache {
            push_effect(
                effects,
                Effect::ApplyConfigurationCache(ConfigurationCacheEffect(cache)),
            )?;
        }
        if let Some(request) = transition.request {
            let facts = configuration_request_facts(request, self.scheduler.snapshot().generation)?;
            self.enqueue(facts, now_ms, effects)?;
        }
        self.publish_state(effects)
    }

    fn complete_configuration_token(
        &mut self,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let Some(token) = self.configuration_token else {
            return Ok(());
        };
        let child = self
            .mutations
            .apply(mutation::Command::CompleteConfiguration { token })?;
        self.process_mutation(child, now_ms, effects)
    }

    fn fail_configuration_token(
        &mut self,
        reason: mutation::FailureReason,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let Some(token) = self.configuration_token else {
            return Ok(());
        };
        let child = self
            .mutations
            .apply(mutation::Command::Fail { token, reason })?;
        self.process_mutation(child, now_ms, effects)
    }

    fn process_telemetry(
        &mut self,
        child_effects: Vec<telemetry::Effect>,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        for effect in child_effects {
            match effect {
                telemetry::Effect::ReplaceTimer { token, interval_ms } => push_effect(
                    effects,
                    Effect::ReplaceTimer {
                        token: TimerToken::Refloat(token),
                        interval_ms,
                    },
                )?,
                telemetry::Effect::CancelTimer(token) => {
                    push_effect(effects, Effect::CancelTimer(TimerToken::Refloat(token)))?;
                }
                telemetry::Effect::Enqueue { facts, .. } => {
                    self.enqueue(facts, now_ms, effects)?;
                }
                telemetry::Effect::PublishFaultHistory(observation) => {
                    push_effect(effects, Effect::PublishFaultHistory(observation))?;
                }
                telemetry::Effect::Publish(sample) => {
                    let mutation_effects =
                        self.mutations.apply(mutation::Command::ObserveTelemetry {
                            generation: self.scheduler.snapshot().generation,
                            sample: sample.clone(),
                        })?;
                    self.process_mutation(mutation_effects, now_ms, effects)?;
                    if let Some(mode) = sample.package_mode {
                        let flywheel = mode == telemetry::PackageMode::Flywheel;
                        let transition = self.configuration.set_flywheel(flywheel);
                        self.process_configuration(transition, now_ms, effects)?;
                        if flywheel {
                            let scheduler_effects = self.scheduler.cancel_matching(
                                &scheduler::CancellationSelector::FlywheelProhibited {
                                    preserve_refloat_control: self
                                        .mutations
                                        .snapshot()
                                        .pending
                                        .is_some_and(|token| {
                                            token.kind == mutation::PendingKind::Flywheel
                                        }),
                                },
                                true,
                                now_ms,
                            )?;
                            self.process_scheduler(scheduler_effects, now_ms, 1, effects)?;
                        }
                    }
                    push_effect(effects, Effect::PublishTelemetry(sample))?;
                }
                telemetry::Effect::Failure(failure) => {
                    push_effect(effects, Effect::Failure(Failure::Telemetry(failure)))?;
                }
                telemetry::Effect::Diagnostic(value) => {
                    push_effect(effects, Effect::Diagnostic(Diagnostic::Telemetry(value)))?;
                }
            }
        }
        Ok(())
    }

    fn process_bms(
        &mut self,
        child_effects: Vec<bms::Effect>,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        for effect in child_effects {
            match effect {
                bms::Effect::Enqueue { facts, .. } => self.enqueue(facts, now_ms, effects)?,
                bms::Effect::ReplaceTimer { token, interval_ms } => push_effect(
                    effects,
                    Effect::ReplaceTimer {
                        token: TimerToken::Bms(token),
                        interval_ms,
                    },
                )?,
                bms::Effect::CancelTimer(token) => {
                    push_effect(effects, Effect::CancelTimer(TimerToken::Bms(token)))?;
                }
                bms::Effect::PublishState(snapshot) => {
                    push_effect(effects, Effect::PublishBmsState(snapshot))?;
                }
                bms::Effect::PublishObservation(observation) => {
                    push_effect(
                        effects,
                        Effect::PublishBmsObservation(Box::new(observation)),
                    )?;
                }
                bms::Effect::Diagnostic(value) => {
                    push_effect(effects, Effect::Diagnostic(Diagnostic::Bms(value)))?;
                }
            }
        }
        Ok(())
    }

    fn fail_discovery_response(
        &mut self,
        code: discovery::IssueCode,
        malformed: MalformedResponse,
        now_ms: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        push_effect(
            effects,
            Effect::Diagnostic(Diagnostic::MalformedResponse(malformed)),
        )?;
        let transition = self.discovery.mark_failure(code);
        self.process_discovery(transition, now_ms, effects)
    }

    fn publish_state(&self, effects: &mut Vec<Effect>) -> Result<(), CoordinatorError> {
        push_effect(effects, Effect::PublishState(Box::new(self.snapshot())))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DiscoveryRequest {
    Firmware {
        route: discovery::Route,
        direct_attempt: u8,
    },
    CanPing,
    RefloatInfo {
        route: discovery::Route,
    },
}

fn discovery_request_facts(
    request: DiscoveryRequest,
    generation: scheduler::Generation,
) -> Result<scheduler::RequestFacts, CoordinatorError> {
    let (payload, expected_response, context, timeout_ms) = match request {
        DiscoveryRequest::Firmware {
            route,
            direct_attempt,
        } => {
            let timeout_ms = match (route, direct_attempt) {
                (discovery::Route::Direct, 1) => INITIAL_IDENTIFICATION_ATTEMPT_TIMEOUT_MS,
                (discovery::Route::Direct, _) => {
                    IDENTIFICATION_TIMEOUT_MS - INITIAL_IDENTIFICATION_ATTEMPT_TIMEOUT_MS
                }
                (discovery::Route::Can(_), _) => CAN_PROBE_TIMEOUT_MS,
            };
            let scheduler_route = scheduler_route(route)?;
            (
                routed_payload(route, &[GET_FIRMWARE_VERSION_COMMAND])?,
                scheduler::ExpectedResponse {
                    command: GET_FIRMWARE_VERSION_COMMAND,
                    refloat_command: None,
                },
                scheduler::RequestContext::ControllerFirmware(scheduler_route),
                timeout_ms,
            )
        }
        DiscoveryRequest::CanPing => (
            vec![PING_CAN_COMMAND],
            scheduler::ExpectedResponse {
                command: PING_CAN_COMMAND,
                refloat_command: None,
            },
            scheduler::RequestContext::CanPing,
            IDENTIFICATION_TIMEOUT_MS,
        ),
        DiscoveryRequest::RefloatInfo { route } => {
            let scheduler_route = scheduler_route(route)?;
            (
                routed_payload(route, &compatibility::info_request())?,
                scheduler::ExpectedResponse {
                    command: CUSTOM_APP_DATA_COMMAND,
                    refloat_command: Some(REFLOAT_INFO_COMMAND),
                },
                scheduler::RequestContext::RefloatInfo(scheduler_route),
                IDENTIFICATION_TIMEOUT_MS,
            )
        }
    };
    Ok(scheduler::RequestFacts {
        payload,
        expected_response,
        priority: scheduler::Priority::ControllerIdentification,
        context,
        timeout_ms,
        generation,
        expects_response: true,
    })
}

fn current_limits_request_facts(
    route: scheduler::Route,
    generation: scheduler::Generation,
) -> Result<scheduler::RequestFacts, CoordinatorError> {
    Ok(scheduler::RequestFacts {
        payload: routed_scheduler_payload(route, &[controller_data::COMM_GET_MCCONF])?,
        expected_response: scheduler::ExpectedResponse {
            command: controller_data::COMM_GET_MCCONF,
            refloat_command: None,
        },
        priority: scheduler::Priority::ControllerWork,
        context: scheduler::RequestContext::CurrentLimits(route),
        timeout_ms: CAN_PROBE_TIMEOUT_MS,
        generation,
        expects_response: true,
    })
}

fn routed_payload(route: discovery::Route, payload: &[u8]) -> Result<Vec<u8>, CoordinatorError> {
    match route {
        discovery::Route::Direct => Ok(payload.to_vec()),
        discovery::Route::Can(can_id) => {
            validate_can_id(can_id)?;
            let mut routed = Vec::with_capacity(payload.len() + 2);
            routed.extend([FORWARD_CAN_COMMAND, can_id]);
            routed.extend(payload);
            Ok(routed)
        }
    }
}

fn expected_response(payload: &[u8]) -> Result<scheduler::ExpectedResponse, CoordinatorError> {
    let Some(&command) = payload.first() else {
        return Err(CoordinatorError::InvalidPayload);
    };
    let refloat_command = (command == CUSTOM_APP_DATA_COMMAND)
        .then(|| payload.get(2).copied())
        .flatten();
    Ok(scheduler::ExpectedResponse {
        command,
        refloat_command,
    })
}

fn can_ids(payload: &[u8]) -> Result<Vec<u8>, MalformedResponse> {
    if payload.first() != Some(&PING_CAN_COMMAND) {
        return Err(MalformedResponse::CanInventory);
    }
    let mut present = [false; 254];
    for &can_id in &payload[1..] {
        if can_id > 253 {
            return Err(MalformedResponse::CanInventory);
        }
        present[usize::from(can_id)] = true;
    }
    Ok(present
        .iter()
        .enumerate()
        .filter_map(|(can_id, present)| present.then_some(can_id as u8))
        .collect())
}

fn layout_is_admitted(layout: &dynamic::DynamicLayout) -> bool {
    let fields: Vec<&[u8]> = layout
        .always_present()
        .iter()
        .map(String::as_bytes)
        .collect();
    let runtime: Vec<&[u8]> = layout.running_only().iter().map(String::as_bytes).collect();
    compatibility::admit_dynamic_layout(&fields, &runtime).is_ok()
}

fn telemetry_family(profile: compatibility::TelemetryProfile) -> telemetry::TelemetryFamily {
    match profile.family() {
        compatibility::TelemetryFamily::LegacyFixed => telemetry::TelemetryFamily::LegacyFixed,
        compatibility::TelemetryFamily::Dynamic => telemetry::TelemetryFamily::Dynamic,
        compatibility::TelemetryFamily::Selectable => telemetry::TelemetryFamily::Selectable,
    }
}

fn telemetry_outcome(outcome: &scheduler::Completion) -> telemetry::RequestOutcome {
    match outcome {
        scheduler::Completion::Response(_) => telemetry::RequestOutcome::Response,
        scheduler::Completion::Sent => telemetry::RequestOutcome::Sent,
        scheduler::Completion::TimedOut => telemetry::RequestOutcome::TimedOut,
        scheduler::Completion::QueueFull => telemetry::RequestOutcome::QueueFull,
        scheduler::Completion::Cancelled => telemetry::RequestOutcome::Cancelled,
        scheduler::Completion::StaleGeneration => telemetry::RequestOutcome::StaleGeneration,
    }
}

fn bms_outcome(outcome: &scheduler::Completion) -> bms::RequestOutcome {
    match outcome {
        scheduler::Completion::Response(_) => bms::RequestOutcome::Response,
        scheduler::Completion::Sent => bms::RequestOutcome::Sent,
        scheduler::Completion::TimedOut => bms::RequestOutcome::TimedOut,
        scheduler::Completion::QueueFull => bms::RequestOutcome::QueueFull,
        scheduler::Completion::Cancelled => bms::RequestOutcome::Cancelled,
        scheduler::Completion::StaleGeneration => bms::RequestOutcome::StaleGeneration,
    }
}

fn mutation_timer(token: mutation::MutationToken) -> Option<(MutationTimerToken, u64)> {
    match token.kind {
        mutation::PendingKind::ControllerShutdown => Some((
            MutationTimerToken::Shutdown(token),
            mutation::SHUTDOWN_TIMEOUT_MS,
        )),
        mutation::PendingKind::Package
        | mutation::PendingKind::Flywheel
        | mutation::PendingKind::Leds
        | mutation::PendingKind::Headlights
        | mutation::PendingKind::ClearFatalError => Some((
            MutationTimerToken::Confirmation(token),
            mutation::CONTROL_TIMEOUT_MS,
        )),
        mutation::PendingKind::ConfigurationWrite | mutation::PendingKind::TuneApply => None,
    }
}

fn configuration_firmware(identity: &firmware::FirmwareIdentity) -> configuration::FirmwareFacts {
    configuration::FirmwareFacts {
        major: identity.major,
        minor: identity.minor,
        hardware_name: identity.hardware_name.clone(),
        custom_configuration_count: identity.custom_configuration_count,
        hardware_configuration_crc: identity.hardware_configuration_crc,
    }
}

const fn configuration_route(route: scheduler::Route) -> configuration::Route {
    match route {
        scheduler::Route::Direct => configuration::Route(0),
        scheduler::Route::Can(can_id) => configuration::Route(can_id as u32 + 1),
    }
}

fn scheduler_configuration_route(
    route: configuration::Route,
) -> Result<scheduler::Route, CoordinatorError> {
    match route.0 {
        0 => Ok(scheduler::Route::Direct),
        1..=254 => Ok(scheduler::Route::Can((route.0 - 1) as u8)),
        _ => Err(CoordinatorError::InvalidCanId),
    }
}

fn configuration_request_facts(
    request: configuration::RequestEffect,
    generation: scheduler::Generation,
) -> Result<scheduler::RequestFacts, CoordinatorError> {
    let (route, body, expected_command, context) = match request {
        configuration::RequestEffect::SchemaChunk {
            route,
            length,
            offset,
            operation,
        } => {
            let body = custom_configuration::schema_request(
                configuration::REFLOAT_INDEX,
                i64::try_from(length).map_err(|_| CoordinatorError::InvalidPayload)?,
                i64::try_from(offset).map_err(|_| CoordinatorError::InvalidPayload)?,
            )?
            .to_vec();
            let scheduler_route = scheduler_configuration_route(route)?;
            (
                scheduler_route,
                body,
                custom_configuration::GET_SCHEMA_COMMAND,
                scheduler::RequestContext::RefloatConfigurationSchema {
                    route: scheduler_route,
                    index: configuration::REFLOAT_INDEX,
                    offset,
                    operation: operation.0,
                },
            )
        }
        configuration::RequestEffect::Values {
            route,
            defaults,
            operation,
        } => {
            let scheduler_route = scheduler_configuration_route(route)?;
            let command = if defaults {
                custom_configuration::GET_DEFAULT_COMMAND
            } else {
                custom_configuration::GET_CURRENT_COMMAND
            };
            (
                scheduler_route,
                custom_configuration::values_request(configuration::REFLOAT_INDEX, defaults)
                    .to_vec(),
                command,
                scheduler::RequestContext::RefloatConfigurationValues {
                    route: scheduler_route,
                    index: configuration::REFLOAT_INDEX,
                    defaults,
                    operation: operation.0,
                },
            )
        }
        configuration::RequestEffect::Write {
            route,
            payload,
            operation,
        } => {
            let scheduler_route = scheduler_configuration_route(route)?;
            (
                scheduler_route,
                custom_configuration::write_request(configuration::REFLOAT_INDEX, &payload)?,
                custom_configuration::SET_CONFIGURATION_COMMAND,
                scheduler::RequestContext::RefloatConfigurationWrite {
                    route: scheduler_route,
                    index: configuration::REFLOAT_INDEX,
                    operation: operation.0,
                },
            )
        }
        configuration::RequestEffect::Verification { route, operation } => {
            let scheduler_route = scheduler_configuration_route(route)?;
            (
                scheduler_route,
                custom_configuration::values_request(configuration::REFLOAT_INDEX, false).to_vec(),
                custom_configuration::GET_CURRENT_COMMAND,
                scheduler::RequestContext::RefloatConfigurationVerification {
                    route: scheduler_route,
                    index: configuration::REFLOAT_INDEX,
                    operation: operation.0,
                },
            )
        }
    };
    Ok(scheduler::RequestFacts {
        payload: routed_scheduler_payload(route, &body)?,
        expected_response: scheduler::ExpectedResponse {
            command: expected_command,
            refloat_command: None,
        },
        priority: scheduler::Priority::ControllerWork,
        context,
        timeout_ms: configuration::REQUEST_TIMEOUT_MS,
        generation,
        expects_response: true,
    })
}

fn configuration_context(
    context: &scheduler::RequestContext,
) -> Result<configuration::RequestContext, CoordinatorError> {
    Ok(match *context {
        scheduler::RequestContext::RefloatConfigurationSchema {
            route,
            index,
            offset,
            operation,
        } => configuration::RequestContext::Schema {
            route: configuration_route(route),
            index,
            offset,
            operation: configuration::Operation(operation),
        },
        scheduler::RequestContext::RefloatConfigurationValues {
            route,
            index,
            defaults,
            operation,
        } => configuration::RequestContext::Values {
            route: configuration_route(route),
            index,
            defaults,
            operation: configuration::Operation(operation),
        },
        scheduler::RequestContext::RefloatConfigurationWrite {
            route,
            index,
            operation,
        } => configuration::RequestContext::Write {
            route: configuration_route(route),
            index,
            operation: configuration::Operation(operation),
        },
        scheduler::RequestContext::RefloatConfigurationVerification {
            route,
            index,
            operation,
        } => configuration::RequestContext::Verification {
            route: configuration_route(route),
            index,
            operation: configuration::Operation(operation),
        },
        _ => return Err(CoordinatorError::InvalidPayload),
    })
}

fn configuration_response(
    payload: &[u8],
) -> Result<configuration::Response, custom_configuration::CustomConfigurationError> {
    Ok(match custom_configuration::decode_response(payload)? {
        custom_configuration::CustomConfigurationResponse::SchemaChunk {
            index,
            total_length,
            offset,
            bytes,
        } => configuration::Response::SchemaChunk {
            index,
            total: total_length,
            offset,
            bytes: bytes.to_vec(),
        },
        custom_configuration::CustomConfigurationResponse::Values {
            command,
            index,
            bytes,
        } => configuration::Response::Values {
            command: match command {
                custom_configuration::ValuesCommand::Current => {
                    configuration::ValuesCommand::Current
                }
                custom_configuration::ValuesCommand::Default => {
                    configuration::ValuesCommand::Default
                }
            },
            index,
            bytes: bytes.to_vec(),
        },
        custom_configuration::CustomConfigurationResponse::WriteAcknowledgement => {
            configuration::Response::WriteAcknowledgement
        }
    })
}

fn routed_scheduler_payload(
    route: scheduler::Route,
    payload: &[u8],
) -> Result<Vec<u8>, CoordinatorError> {
    routed_payload(discovery_route(route)?, payload)
}

fn scheduler_route(route: discovery::Route) -> Result<scheduler::Route, CoordinatorError> {
    match route {
        discovery::Route::Direct => Ok(scheduler::Route::Direct),
        discovery::Route::Can(can_id) => {
            validate_can_id(can_id)?;
            Ok(scheduler::Route::Can(can_id))
        }
    }
}

fn discovery_route(route: scheduler::Route) -> Result<discovery::Route, CoordinatorError> {
    match route {
        scheduler::Route::Direct => Ok(discovery::Route::Direct),
        scheduler::Route::Can(can_id) => {
            validate_can_id(can_id)?;
            Ok(discovery::Route::Can(can_id))
        }
    }
}

fn bms_route(route: scheduler::Route) -> Result<bms::Route, CoordinatorError> {
    discovery_route(route)
}

fn validate_optional_route(route: Option<discovery::Route>) -> Result<(), CoordinatorError> {
    match route {
        None | Some(discovery::Route::Direct) => Ok(()),
        Some(discovery::Route::Can(can_id)) => validate_can_id(can_id),
    }
}

fn validate_optional_can_id(can_id: Option<u8>) -> Result<(), CoordinatorError> {
    can_id.map_or(Ok(()), validate_can_id)
}

fn validate_can_id(can_id: u8) -> Result<(), CoordinatorError> {
    if can_id <= 253 {
        Ok(())
    } else {
        Err(CoordinatorError::InvalidCanId)
    }
}

fn validate_now(now_ms: u64) -> Result<(), CoordinatorError> {
    if now_ms <= u64::MAX - IDENTIFICATION_TIMEOUT_MS {
        Ok(())
    } else {
        Err(CoordinatorError::InvalidMonotonicTime)
    }
}

fn push_effect(effects: &mut Vec<Effect>, effect: Effect) -> Result<(), CoordinatorError> {
    if effects.len() >= MAX_TRANSITION_EFFECTS {
        return Err(CoordinatorError::EffectLimit);
    }
    effects.push(effect);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn begin_facts() -> BeginFacts {
        BeginFacts {
            now_ms: 10,
            settings: telemetry::Settings::new(1_000, 5_000).unwrap(),
            preferred_controller_can_id: None,
            routing_hint: None,
            continuity: discovery::Continuity::default(),
            preferred_bms_can_id: None,
        }
    }

    #[test]
    fn selectable_mismatches_preserve_direct_and_can_requests_until_deadline() {
        for route in [scheduler::Route::Direct, scheduler::Route::Can(42)] {
            for slow in [false, true] {
                let mut coordinator = ConnectionApplicationCoordinator::default();
                let request = if slow {
                    dynamic::selectable_slow_request(false)
                } else {
                    dynamic::selectable_fast_request()
                };
                let other = if slow {
                    dynamic::selectable_fast_request()
                } else {
                    dynamic::selectable_slow_request(false)
                };
                let effects = coordinator
                    .scheduler
                    .enqueue(
                        scheduler::RequestFacts {
                            payload: routed_scheduler_payload(route, &request).unwrap(),
                            expected_response: expected_response(&request).unwrap(),
                            priority: scheduler::Priority::RidingTelemetry,
                            context: scheduler::RequestContext::RefloatTelemetry(route),
                            timeout_ms: 900,
                            generation: coordinator.scheduler.snapshot().generation,
                            expects_response: true,
                        },
                        10,
                    )
                    .unwrap();
                let token = effects
                    .iter()
                    .find_map(|effect| match effect {
                        scheduler::Effect::ArmDeadline { token, .. } => Some(*token),
                        _ => None,
                    })
                    .unwrap();
                let pending = coordinator.scheduler.snapshot().in_flight;
                let effects = coordinator
                    .receive(coordinator.generation, other.to_vec(), 20, 20)
                    .unwrap();
                assert_eq!(
                    effects,
                    [Effect::Diagnostic(Diagnostic::UnexpectedResponse)]
                );
                assert_eq!(coordinator.scheduler.snapshot().in_flight, pending);
                coordinator
                    .deadline_fired(coordinator.generation, token, 910)
                    .unwrap();
                assert!(coordinator.scheduler.snapshot().in_flight.is_none());
            }
        }
    }

    #[test]
    fn begin_emits_exact_direct_firmware_request() {
        let mut coordinator = ConnectionApplicationCoordinator::default();
        let effects = coordinator.apply(Command::Begin(begin_facts())).unwrap();
        let write = effects.iter().find_map(|effect| match effect {
            Effect::Write(request) => Some(request),
            _ => None,
        });
        let write = write.expect("direct firmware write");
        assert_eq!(write.payload, [GET_FIRMWARE_VERSION_COMMAND]);
        assert_eq!(
            write.context,
            scheduler::RequestContext::ControllerFirmware(scheduler::Route::Direct)
        );
        assert_eq!(write.deadline_monotonic_ms, 2_010);
        assert_eq!(coordinator.snapshot().phase, Phase::Identifying);
    }

    #[test]
    fn invalid_begin_is_atomic() {
        let mut coordinator = ConnectionApplicationCoordinator::default();
        let before = coordinator.snapshot();
        let mut facts = begin_facts();
        facts.preferred_controller_can_id = Some(254);
        assert_eq!(
            coordinator.apply(Command::Begin(facts)),
            Err(CoordinatorError::InvalidCanId)
        );
        assert_eq!(coordinator.snapshot(), before);
    }

    #[test]
    fn route_translation_is_exact_and_checked() {
        let generation = scheduler::Generation(7);
        let direct = discovery_request_facts(
            DiscoveryRequest::RefloatInfo {
                route: discovery::Route::Direct,
            },
            generation,
        )
        .unwrap();
        assert_eq!(direct.payload, compatibility::info_request());
        let can = discovery_request_facts(
            DiscoveryRequest::RefloatInfo {
                route: discovery::Route::Can(253),
            },
            generation,
        )
        .unwrap();
        assert_eq!(&can.payload[..2], &[FORWARD_CAN_COMMAND, 253]);
        assert_eq!(&can.payload[2..], compatibility::info_request());
        assert_eq!(
            routed_payload(discovery::Route::Can(254), &[0]),
            Err(CoordinatorError::InvalidCanId)
        );
    }

    #[test]
    fn stale_public_events_do_not_mutate() {
        let mut coordinator = ConnectionApplicationCoordinator::default();
        coordinator.apply(Command::Begin(begin_facts())).unwrap();
        let before = coordinator.snapshot();
        let effects = coordinator
            .apply(Command::SelectController {
                generation: Generation(before.generation.0.wrapping_sub(1)),
                can_id: 1,
                now_ms: 20,
            })
            .unwrap();
        assert_eq!(effects, [Effect::Diagnostic(Diagnostic::StaleEvent)]);
        assert_eq!(coordinator.snapshot(), before);
    }

    #[test]
    fn composition_effect_bound_is_exact() {
        let mut effects = vec![Effect::Diagnostic(Diagnostic::StaleEvent); MAX_TRANSITION_EFFECTS];
        assert_eq!(
            push_effect(&mut effects, Effect::Diagnostic(Diagnostic::StaleEvent)),
            Err(CoordinatorError::EffectLimit)
        );
        assert_eq!(effects.len(), MAX_TRANSITION_EFFECTS);
    }

    #[test]
    fn configuration_request_adapters_are_exact_and_once_routed() {
        let generation = scheduler::Generation(9);
        let direct = configuration_request_facts(
            configuration::RequestEffect::SchemaChunk {
                route: configuration::Route(0),
                length: 400,
                offset: 12,
                operation: configuration::Operation(7),
            },
            generation,
        )
        .unwrap();
        assert_eq!(direct.payload, [92, 0, 0, 0, 1, 144, 0, 0, 0, 12]);
        assert_eq!(direct.generation, generation);
        assert_eq!(direct.priority, scheduler::Priority::ControllerWork);

        let can = configuration_request_facts(
            configuration::RequestEffect::Values {
                route: configuration::Route(254),
                defaults: true,
                operation: configuration::Operation(8),
            },
            generation,
        )
        .unwrap();
        assert_eq!(can.payload, [34, 253, 94, 0]);
        assert_eq!(
            scheduler_configuration_route(configuration::Route(255)),
            Err(CoordinatorError::InvalidCanId)
        );
    }
}
