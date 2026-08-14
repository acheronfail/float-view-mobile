//! Safe connection-scoped composition for the future unified Rust runtime.
//!
//! The coordinator owns deterministic application decisions and returns owned
//! effects. It deliberately owns no transport, frame stream, clock, timer,
//! callback, persistence, local server, host ABI, executor, or OS behavior.

#![forbid(unsafe_code)]

use std::collections::VecDeque;

use float_view_controller_discovery as discovery;
use float_view_refloat_compatibility as compatibility;
use float_view_refloat_dynamic_telemetry as dynamic;
use float_view_refloat_legacy_telemetry as legacy;
use float_view_refloat_telemetry_service as telemetry;
use float_view_vesc_bms_telemetry_service as bms;
use float_view_vesc_firmware_identity as firmware;
use float_view_vesc_request_scheduler as scheduler;

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
    pub inventory: Vec<discovery::RoutedIdentity>,
    pub inventory_complete: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub generation: Generation,
    pub phase: Phase,
    pub standalone_bms: bool,
    pub awaiting_activation: Option<ActivationFacts>,
    pub discovery: discovery::Snapshot,
    pub scheduler: scheduler::Snapshot,
    pub telemetry: telemetry::Snapshot,
    pub bms: bms::Snapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerToken {
    Refloat(telemetry::TimerToken),
    Bms(bms::TimerToken),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MalformedResponse {
    Empty,
    Firmware,
    CanInventory,
    RefloatInfo,
    RefloatLayout,
    RefloatTelemetry,
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    Discovery(discovery::Issue),
    Telemetry(telemetry::Failure),
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
    PublishBmsState(bms::Snapshot),
    PublishBmsObservation(Box<bms::BmsObservation>),
    Failure(Failure),
    Diagnostic(Diagnostic),
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

pub struct ConnectionApplicationCoordinator {
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
    dynamic_layout: Option<dynamic::DynamicLayout>,
    discovery: discovery::ControllerDiscovery,
    scheduler: scheduler::VescRequestScheduler,
    telemetry: telemetry::RefloatTelemetryService,
    bms: bms::VescBmsTelemetryService,
}

impl Default for ConnectionApplicationCoordinator {
    fn default() -> Self {
        Self {
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
            dynamic_layout: None,
            discovery: discovery::ControllerDiscovery::new(),
            scheduler: scheduler::VescRequestScheduler::default(),
            telemetry: telemetry::RefloatTelemetryService::default(),
            bms: bms::VescBmsTelemetryService::new(),
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
            .finish_non_exhaustive()
    }
}

impl ConnectionApplicationCoordinator {
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            generation: self.generation,
            phase: self.phase,
            standalone_bms: self.standalone_bms,
            awaiting_activation: self.awaiting_activation.clone(),
            discovery: self.discovery.snapshot(),
            scheduler: self.scheduler.snapshot(),
            telemetry: self.telemetry.snapshot(),
            bms: self.bms.snapshot(),
        }
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
            } => self.activate(generation, now_ms, session_started),
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
        self.reset_internal(facts.now_ms, false, &mut effects)?;
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
        let (_, scheduler_effects) = self.scheduler.receive(
            payload,
            expected,
            &scheduler::ContextMatch::Any,
            true,
            now_ms,
        )?;
        let mut effects = Vec::new();
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
        }
        Ok(effects)
    }

    fn activate(
        &mut self,
        generation: Generation,
        now_ms: u64,
        session_started: bool,
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
            settings: self.settings,
        };
        configuration.validate()?;

        let mut effects = Vec::new();
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
        self.reset_internal(now_ms, true, &mut effects)?;
        Ok(effects)
    }

    fn reset_internal(
        &mut self,
        now_ms: u64,
        publish: bool,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let telemetry_effects = self.telemetry.apply(telemetry::Command::Reset)?;
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
            selected,
            transport,
            compatibility: profile,
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
                    push_effect(effects, Effect::Write(request))?;
                }
                scheduler::Effect::Complete { request, outcome } => {
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
            _ => push_effect(effects, Effect::Diagnostic(Diagnostic::UnexpectedResponse)),
        }
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
                let profile = compatibility::decode_info(&payload)
                    .ok()
                    .and_then(compatibility::compatibility);
                match profile {
                    Some(profile) => {
                        let transition = self.discovery.accept_refloat_profile(profile);
                        self.process_discovery(transition, now_ms, effects)
                    }
                    None => self.fail_discovery_response(
                        discovery::IssueCode::UnsupportedRefloat,
                        MalformedResponse::RefloatInfo,
                        now_ms,
                        effects,
                    ),
                }
            }
            scheduler::Completion::TimedOut | scheduler::Completion::QueueFull => {
                let transition = self
                    .discovery
                    .mark_failure(discovery::IssueCode::UnsupportedRefloat);
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
                telemetry::Effect::Publish(sample) => {
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
}
