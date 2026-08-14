//! Connection-scoped device-mutation authorization and serialization.
//!
//! This safe candidate retains only bounded semantic state and returns exact
//! scheduler facts for a future Rust application coordinator to execute. It
//! owns no scheduler, transport, frame stream, clock, callback, persistence,
//! host ABI, or OS behavior and remains unselected by every production host.

#![forbid(unsafe_code)]

use std::fmt::{self, Debug, Formatter};

use float_view_refloat_compatibility as compatibility;
use float_view_refloat_control as control;
use float_view_refloat_telemetry_service as telemetry;
use float_view_vesc_compatibility as vesc_compatibility;
use float_view_vesc_firmware_identity::FirmwareIdentity;
use float_view_vesc_request_scheduler as scheduler;

/// Native-equivalent semantic-control confirmation timeout.
pub const CONTROL_TIMEOUT_MS: u64 = 3_000;
/// Native shutdown-intent window.
pub const SHUTDOWN_TIMEOUT_MS: u64 = 30_000;
/// VESC `COMM_FORWARD_CAN`.
pub const FORWARD_CAN_COMMAND: u8 = 34;
/// VESC accepted-only controller shutdown command.
pub const SHUTDOWN_COMMAND: u8 = 156;
/// Exact accepted-only VESC shutdown payload selected by both native hosts.
pub const SHUTDOWN_REQUEST: [u8; 3] = [SHUTDOWN_COMMAND, 0, 0];

/// Facts that atomically configure one selected controller generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigureFacts {
    pub generation: scheduler::Generation,
    pub route: scheduler::Route,
    pub firmware: FirmwareIdentity,
    pub refloat: compatibility::CompatibilityProfile,
    pub connection_ready: bool,
}

/// Published semantic-control readiness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlStatus {
    Loading,
    Ready,
    Unavailable,
}

/// One frontend-equivalent bounded semantic-control snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Controls {
    pub status: ControlStatus,
    pub writable: bool,
    pub package_enabled: Option<bool>,
    pub flywheel_enabled: Option<bool>,
    pub lighting_available: bool,
    pub leds_enabled: Option<bool>,
    pub headlights_enabled: Option<bool>,
    pub fatal_error_clear_available: bool,
    pub fatal_error_latched: Option<bool>,
    pub fatal_error_active: Option<bool>,
    pub fatal_error_clearable: Option<bool>,
    pub pending_action: Option<PendingKind>,
}

/// Controller-shutdown availability and pending state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControllerActions {
    pub shutdown_available: bool,
    pub shutdown_pending: bool,
}

/// Closed mutation families sharing the sole pending slot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PendingKind {
    Package,
    Flywheel,
    Leds,
    Headlights,
    ClearFatalError,
    ControllerShutdown,
    ConfigurationWrite,
    TuneApply,
}

impl PendingKind {
    const fn is_control(self) -> bool {
        matches!(
            self,
            Self::Package | Self::Flywheel | Self::Leds | Self::Headlights | Self::ClearFatalError
        )
    }

    const fn is_configuration(self) -> bool {
        matches!(self, Self::ConfigurationWrite | Self::TuneApply)
    }
}

/// Exact identity for one reserved mutation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MutationToken {
    pub generation: scheduler::Generation,
    pub sequence: u64,
    pub kind: PendingKind,
}

/// Mutable semantic boolean controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanControl {
    Package,
    Flywheel,
    Leds,
    Headlights,
}

/// Configuration work sharing the mutation lease.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationMutation {
    Write,
    TuneApply,
}

impl ConfigurationMutation {
    const fn pending_kind(self) -> PendingKind {
        match self {
            Self::Write => PendingKind::ConfigurationWrite,
            Self::TuneApply => PendingKind::TuneApply,
        }
    }
}

/// Stable reason for an asynchronously failed accepted mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureReason {
    RequestFailed,
    ConfirmationTimedOut,
    ProhibitedByFlywheel,
    Cancelled,
    Replaced,
}

/// Stable payload-free diagnostics for ignored input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Diagnostic {
    StaleEvent,
    MismatchedToken,
    NonMonotonicTelemetry,
    NonMonotonicLighting,
}

/// Stable atomic command failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoordinatorError {
    InvalidState,
    InvalidRoute,
    UnsupportedController,
    InvalidLightingResponse,
}

/// Complete bounded current candidate state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub configured: bool,
    pub generation: scheduler::Generation,
    pub route: Option<scheduler::Route>,
    pub controls: Option<Controls>,
    pub controller_actions: ControllerActions,
    pub pending: Option<MutationToken>,
    pub last_telemetry_sequence: Option<u64>,
    pub last_lighting_sequence: Option<u64>,
}

/// Ordered work/publication returned by one transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    Publish(Snapshot),
    Submit {
        token: MutationToken,
        request: scheduler::RequestFacts,
    },
    Lease(MutationToken),
    Idempotent(Snapshot),
    Confirmed(MutationToken),
    Released(MutationToken),
    Failed {
        token: MutationToken,
        reason: FailureReason,
    },
    Diagnostic(Diagnostic),
}

/// Closed public transition commands.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Configure(ConfigureFacts),
    ObserveTelemetry {
        generation: scheduler::Generation,
        sample: Box<telemetry::NormalizedSample>,
    },
    ObserveLighting {
        generation: scheduler::Generation,
        observation_sequence: u64,
        payload: Vec<u8>,
    },
    SetControl {
        generation: scheduler::Generation,
        control: BooleanControl,
        enabled: bool,
    },
    ClearFatalError {
        generation: scheduler::Generation,
    },
    ShutdownController {
        generation: scheduler::Generation,
    },
    BeginConfiguration {
        generation: scheduler::Generation,
        mutation: ConfigurationMutation,
    },
    CompleteConfiguration {
        token: MutationToken,
    },
    ClearShutdown {
        token: MutationToken,
    },
    Fail {
        token: MutationToken,
        reason: FailureReason,
    },
    Cancel {
        generation: scheduler::Generation,
    },
    Reset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Pending {
    token: MutationToken,
    expected: Option<bool>,
}

/// Stateful safe mutation decision owner.
pub struct DeviceMutationCoordinator {
    configured: bool,
    connection_ready: bool,
    generation: scheduler::Generation,
    route: Option<scheduler::Route>,
    profile: Option<compatibility::ControlProfile>,
    writable: bool,
    shutdown_supported: bool,
    controls: Option<Controls>,
    ride_state: telemetry::RideState,
    package_state: Option<telemetry::PackageState>,
    package_mode: Option<telemetry::PackageMode>,
    footpad_state: Option<telemetry::FootpadState>,
    last_telemetry_sequence: Option<u64>,
    last_lighting_sequence: Option<u64>,
    next_mutation_sequence: u64,
    pending: Option<Pending>,
}

impl Debug for DeviceMutationCoordinator {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceMutationCoordinator")
            .field("snapshot", &self.snapshot())
            .finish()
    }
}

impl Default for DeviceMutationCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceMutationCoordinator {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            configured: false,
            connection_ready: false,
            generation: scheduler::Generation(0),
            route: None,
            profile: None,
            writable: false,
            shutdown_supported: false,
            controls: None,
            ride_state: telemetry::RideState::Unknown,
            package_state: None,
            package_mode: None,
            footpad_state: None,
            last_telemetry_sequence: None,
            last_lighting_sequence: None,
            next_mutation_sequence: 0,
            pending: None,
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let mut controls = self.controls.clone();
        if let Some(controls) = &mut controls {
            controls.pending_action = self
                .pending
                .map(|pending| pending.token.kind)
                .filter(|kind| kind.is_control());
        }
        Snapshot {
            configured: self.configured,
            generation: self.generation,
            route: self.route,
            controls,
            controller_actions: ControllerActions {
                shutdown_available: self.shutdown_available(),
                shutdown_pending: self
                    .pending
                    .is_some_and(|pending| pending.token.kind == PendingKind::ControllerShutdown),
            },
            pending: self.pending.map(|pending| pending.token),
            last_telemetry_sequence: self.last_telemetry_sequence,
            last_lighting_sequence: self.last_lighting_sequence,
        }
    }

    pub fn apply(&mut self, command: Command) -> Result<Vec<Effect>, CoordinatorError> {
        match command {
            Command::Configure(facts) => self.configure(facts),
            Command::ObserveTelemetry { generation, sample } => {
                self.observe_telemetry(generation, *sample)
            }
            Command::ObserveLighting {
                generation,
                observation_sequence,
                payload,
            } => self.observe_lighting(generation, observation_sequence, &payload),
            Command::SetControl {
                generation,
                control,
                enabled,
            } => self.authorize_control(generation, control, enabled),
            Command::ClearFatalError { generation } => self.authorize_fatal_clear(generation),
            Command::ShutdownController { generation } => self.authorize_shutdown(generation),
            Command::BeginConfiguration {
                generation,
                mutation,
            } => self.authorize_configuration(generation, mutation),
            Command::CompleteConfiguration { token } => self.complete_configuration(token),
            Command::ClearShutdown { token } => self.clear_shutdown(token),
            Command::Fail { token, reason } => self.fail(token, reason),
            Command::Cancel { generation } => self.cancel(generation),
            Command::Reset => Ok(self.reset()),
        }
    }

    fn configure(&mut self, facts: ConfigureFacts) -> Result<Vec<Effect>, CoordinatorError> {
        validate_route(facts.route)?;
        if !vesc_compatibility::supports(&facts.firmware) {
            return Err(CoordinatorError::UnsupportedController);
        }
        let writable = facts.connection_ready
            && vesc_compatibility::permits(vesc_compatibility::Operation::Write, &facts.firmware);
        let shutdown_supported =
            facts.connection_ready && vesc_compatibility::supports_shutdown(&facts.firmware);
        let prior = self.pending.map(|pending| pending.token);
        self.configured = true;
        self.connection_ready = facts.connection_ready;
        self.generation = facts.generation;
        self.route = Some(facts.route);
        self.profile = Some(facts.refloat.control);
        self.writable = writable;
        self.shutdown_supported = shutdown_supported;
        self.ride_state = telemetry::RideState::Unknown;
        self.package_state = None;
        self.package_mode = None;
        self.footpad_state = None;
        self.last_telemetry_sequence = None;
        self.last_lighting_sequence = None;
        self.pending = None;
        let fatal_available = facts.refloat.control.fatal_clear_available;
        self.controls = Some(Controls {
            status: if facts.connection_ready {
                ControlStatus::Loading
            } else {
                ControlStatus::Unavailable
            },
            writable,
            package_enabled: None,
            flywheel_enabled: None,
            lighting_available: true,
            leds_enabled: None,
            headlights_enabled: None,
            fatal_error_clear_available: fatal_available,
            fatal_error_latched: (!fatal_available).then_some(false),
            fatal_error_active: (!fatal_available).then_some(false),
            fatal_error_clearable: (!fatal_available).then_some(false),
            pending_action: None,
        });
        let mut effects = Vec::with_capacity(2);
        if let Some(token) = prior {
            effects.push(Effect::Failed {
                token,
                reason: FailureReason::Replaced,
            });
        }
        effects.push(Effect::Publish(self.snapshot()));
        Ok(effects)
    }

    fn observe_telemetry(
        &mut self,
        generation: scheduler::Generation,
        sample: telemetry::NormalizedSample,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        if !self.is_current(generation) {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        if sample.sequence == 0
            || self
                .last_telemetry_sequence
                .is_some_and(|sequence| sample.sequence <= sequence)
        {
            return Ok(vec![Effect::Diagnostic(Diagnostic::NonMonotonicTelemetry)]);
        }
        let controls = self
            .controls
            .as_mut()
            .ok_or(CoordinatorError::InvalidState)?;
        self.last_telemetry_sequence = Some(sample.sequence);
        self.ride_state = sample.ride_state;
        if let Some(state) = sample.package_state {
            self.package_state = Some(state);
            controls.package_enabled = Some(state != telemetry::PackageState::Disabled);
        }
        if let Some(mode) = sample.package_mode {
            self.package_mode = Some(mode);
            controls.flywheel_enabled = Some(mode == telemetry::PackageMode::Flywheel);
        }
        if let Some(footpad) = sample.footpad_state {
            self.footpad_state = Some(footpad);
        }
        if let Some(latched) = sample.fatal_error_latched {
            controls.fatal_error_latched = Some(latched);
            controls.fatal_error_active = Some(sample.firmware_fault.is_some());
            controls.fatal_error_clearable = Some(latched && sample.firmware_fault.is_none());
        }
        refresh_status(self.connection_ready, controls);

        let confirmed = self.confirmed_by_authoritative_state();
        if confirmed.is_some() {
            self.pending = None;
        }
        let prohibited = if self.package_mode == Some(telemetry::PackageMode::Flywheel) {
            self.pending
                .filter(|pending| !is_preserved_flywheel_disable(*pending))
                .map(|pending| pending.token)
        } else {
            None
        };
        if prohibited.is_some() {
            self.pending = None;
        }

        let mut effects = vec![Effect::Publish(self.snapshot())];
        if let Some(token) = confirmed {
            effects.push(Effect::Confirmed(token));
        }
        if let Some(token) = prohibited {
            effects.push(Effect::Failed {
                token,
                reason: FailureReason::ProhibitedByFlywheel,
            });
        }
        Ok(effects)
    }

    fn observe_lighting(
        &mut self,
        generation: scheduler::Generation,
        observation_sequence: u64,
        payload: &[u8],
    ) -> Result<Vec<Effect>, CoordinatorError> {
        if !self.is_current(generation) {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        if observation_sequence == 0
            || self
                .last_lighting_sequence
                .is_some_and(|sequence| observation_sequence <= sequence)
        {
            return Ok(vec![Effect::Diagnostic(Diagnostic::NonMonotonicLighting)]);
        }
        let profile = self.profile.ok_or(CoordinatorError::InvalidState)?;
        let state = control::decode_lighting_response(payload, lighting_profile(profile.lighting))
            .map_err(|_| CoordinatorError::InvalidLightingResponse)?;
        let controls = self
            .controls
            .as_mut()
            .ok_or(CoordinatorError::InvalidState)?;
        self.last_lighting_sequence = Some(observation_sequence);
        controls.leds_enabled = Some(state.leds_enabled);
        controls.headlights_enabled = Some(state.headlights_enabled);
        refresh_status(self.connection_ready, controls);
        let confirmed = self.confirmed_by_authoritative_state();
        if confirmed.is_some() {
            self.pending = None;
        }
        let mut effects = vec![Effect::Publish(self.snapshot())];
        if let Some(token) = confirmed {
            effects.push(Effect::Confirmed(token));
        }
        Ok(effects)
    }

    fn authorize_control(
        &mut self,
        generation: scheduler::Generation,
        target: BooleanControl,
        enabled: bool,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        self.validate_mutable(generation)?;
        let controls = self
            .controls
            .as_ref()
            .ok_or(CoordinatorError::InvalidState)?;
        let active_flywheel_disable = target == BooleanControl::Flywheel
            && !enabled
            && self.package_mode == Some(telemetry::PackageMode::Flywheel);
        if controls.flywheel_enabled == Some(true) && !active_flywheel_disable {
            return Err(CoordinatorError::InvalidState);
        }
        if (matches!(target, BooleanControl::Package)
            || (target == BooleanControl::Flywheel && !active_flywheel_disable))
            && self.ride_state != telemetry::RideState::Idle
        {
            return Err(CoordinatorError::InvalidState);
        }
        if target == BooleanControl::Flywheel {
            if enabled
                && (self.package_state != Some(telemetry::PackageState::Ready)
                    || self.package_mode != Some(telemetry::PackageMode::Normal))
            {
                return Err(CoordinatorError::InvalidState);
            }
            if !enabled && self.package_mode != Some(telemetry::PackageMode::Flywheel) {
                return Err(CoordinatorError::InvalidState);
            }
        }
        if matches!(target, BooleanControl::Leds | BooleanControl::Headlights)
            && !controls.lighting_available
        {
            return Err(CoordinatorError::InvalidState);
        }
        let confirmed = match target {
            BooleanControl::Package => controls.package_enabled,
            BooleanControl::Flywheel => controls.flywheel_enabled,
            BooleanControl::Leds => controls.leds_enabled,
            BooleanControl::Headlights => controls.headlights_enabled,
        };
        if confirmed == Some(enabled) {
            return Ok(vec![Effect::Idempotent(self.snapshot())]);
        }
        let (kind, payload, expects_response) = match target {
            BooleanControl::Package => (
                PendingKind::Package,
                control::package_request(enabled),
                false,
            ),
            BooleanControl::Flywheel => (
                PendingKind::Flywheel,
                control::flywheel_request(enabled),
                false,
            ),
            BooleanControl::Leds | BooleanControl::Headlights => {
                let current = control::LightingState {
                    leds_enabled: controls
                        .leds_enabled
                        .ok_or(CoordinatorError::InvalidState)?,
                    headlights_enabled: controls
                        .headlights_enabled
                        .ok_or(CoordinatorError::InvalidState)?,
                };
                let profile = self.profile.ok_or(CoordinatorError::InvalidState)?;
                let (kind, target) = match target {
                    BooleanControl::Leds => (PendingKind::Leds, control::LightingTarget::Leds),
                    BooleanControl::Headlights => {
                        (PendingKind::Headlights, control::LightingTarget::Headlights)
                    }
                    _ => unreachable!(),
                };
                (
                    kind,
                    control::lighting_request(
                        lighting_profile(profile.lighting),
                        Some(target),
                        enabled,
                        current,
                    ),
                    true,
                )
            }
        };
        self.authorize_request(kind, Some(enabled), payload.as_slice(), expects_response)
    }

    fn authorize_fatal_clear(
        &mut self,
        generation: scheduler::Generation,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        self.validate_mutable(generation)?;
        let controls = self
            .controls
            .as_ref()
            .ok_or(CoordinatorError::InvalidState)?;
        if controls.flywheel_enabled == Some(true)
            || self.ride_state != telemetry::RideState::Idle
            || !controls.fatal_error_clear_available
            || controls.fatal_error_latched != Some(true)
            || controls.fatal_error_active != Some(false)
            || controls.fatal_error_clearable != Some(true)
        {
            return Err(CoordinatorError::InvalidState);
        }
        let profile = self.profile.ok_or(CoordinatorError::InvalidState)?;
        let payload = control::fatal_clear_request(control_profile(profile))
            .map_err(|_| CoordinatorError::InvalidState)?;
        self.authorize_request(
            PendingKind::ClearFatalError,
            Some(false),
            payload.as_slice(),
            false,
        )
    }

    fn authorize_shutdown(
        &mut self,
        generation: scheduler::Generation,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        self.validate_mutable(generation)?;
        if !self.shutdown_supported
            || self.ride_state != telemetry::RideState::Idle
            || self.package_mode == Some(telemetry::PackageMode::Flywheel)
        {
            return Err(CoordinatorError::InvalidState);
        }
        let token = self.reserve(PendingKind::ControllerShutdown, None);
        let route = self.route.ok_or(CoordinatorError::InvalidState)?;
        let request = scheduler::RequestFacts {
            payload: routed_payload(route, &SHUTDOWN_REQUEST)?,
            expected_response: scheduler::ExpectedResponse {
                command: SHUTDOWN_COMMAND,
                refloat_command: None,
            },
            priority: scheduler::Priority::RidingTelemetry,
            context: scheduler::RequestContext::ControllerShutdown(route),
            timeout_ms: SHUTDOWN_TIMEOUT_MS,
            generation: self.generation,
            expects_response: false,
        };
        Ok(vec![
            Effect::Publish(self.snapshot()),
            Effect::Submit { token, request },
        ])
    }

    fn authorize_configuration(
        &mut self,
        generation: scheduler::Generation,
        mutation: ConfigurationMutation,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        self.validate_mutable(generation)?;
        if self.ride_state != telemetry::RideState::Idle
            || self.footpad_state != Some(telemetry::FootpadState::None)
            || self.package_state != Some(telemetry::PackageState::Ready)
            || self.package_mode != Some(telemetry::PackageMode::Normal)
        {
            return Err(CoordinatorError::InvalidState);
        }
        let token = self.reserve(mutation.pending_kind(), None);
        Ok(vec![Effect::Publish(self.snapshot()), Effect::Lease(token)])
    }

    fn complete_configuration(
        &mut self,
        token: MutationToken,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        if !token.kind.is_configuration() {
            return Err(CoordinatorError::InvalidState);
        }
        self.complete_exact(token, true)
    }

    fn clear_shutdown(&mut self, token: MutationToken) -> Result<Vec<Effect>, CoordinatorError> {
        if token.kind != PendingKind::ControllerShutdown {
            return Err(CoordinatorError::InvalidState);
        }
        self.complete_exact(token, false)
    }

    fn complete_exact(
        &mut self,
        token: MutationToken,
        confirmed: bool,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        if !self.is_current(token.generation) {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        if self.pending.map(|pending| pending.token) != Some(token) {
            return Ok(vec![Effect::Diagnostic(Diagnostic::MismatchedToken)]);
        }
        self.pending = None;
        Ok(vec![
            Effect::Publish(self.snapshot()),
            if confirmed {
                Effect::Confirmed(token)
            } else {
                Effect::Released(token)
            },
        ])
    }

    fn fail(
        &mut self,
        token: MutationToken,
        reason: FailureReason,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        if !self.is_current(token.generation) {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        if self.pending.map(|pending| pending.token) != Some(token) {
            return Ok(vec![Effect::Diagnostic(Diagnostic::MismatchedToken)]);
        }
        self.pending = None;
        Ok(vec![
            Effect::Publish(self.snapshot()),
            Effect::Failed { token, reason },
        ])
    }

    fn cancel(
        &mut self,
        generation: scheduler::Generation,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        if !self.is_current(generation) {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        let Some(token) = self.pending.map(|pending| pending.token) else {
            return Ok(vec![Effect::Publish(self.snapshot())]);
        };
        self.pending = None;
        Ok(vec![
            Effect::Publish(self.snapshot()),
            Effect::Failed {
                token,
                reason: FailureReason::Cancelled,
            },
        ])
    }

    fn reset(&mut self) -> Vec<Effect> {
        let prior = self.pending.map(|pending| pending.token);
        self.configured = false;
        self.connection_ready = false;
        self.route = None;
        self.profile = None;
        self.writable = false;
        self.shutdown_supported = false;
        self.controls = None;
        self.ride_state = telemetry::RideState::Unknown;
        self.package_state = None;
        self.package_mode = None;
        self.footpad_state = None;
        self.last_telemetry_sequence = None;
        self.last_lighting_sequence = None;
        self.pending = None;
        let mut effects = Vec::with_capacity(2);
        if let Some(token) = prior {
            effects.push(Effect::Failed {
                token,
                reason: FailureReason::Cancelled,
            });
        }
        effects.push(Effect::Publish(self.snapshot()));
        effects
    }

    fn validate_mutable(&self, generation: scheduler::Generation) -> Result<(), CoordinatorError> {
        if !self.is_current(generation)
            || !self.connection_ready
            || !self.writable
            || self.pending.is_some()
            || self
                .controls
                .as_ref()
                .is_none_or(|controls| controls.status != ControlStatus::Ready)
        {
            return Err(CoordinatorError::InvalidState);
        }
        Ok(())
    }

    fn authorize_request(
        &mut self,
        kind: PendingKind,
        expected: Option<bool>,
        payload: &[u8],
        expects_response: bool,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        let route = self.route.ok_or(CoordinatorError::InvalidState)?;
        let refloat_command = payload.get(2).copied();
        let token = self.reserve(kind, expected);
        let request = scheduler::RequestFacts {
            payload: routed_payload(route, payload)?,
            expected_response: scheduler::ExpectedResponse {
                command: control::CUSTOM_APP_DATA_COMMAND,
                refloat_command,
            },
            priority: scheduler::Priority::RidingTelemetry,
            context: scheduler::RequestContext::RefloatControl(route),
            timeout_ms: CONTROL_TIMEOUT_MS,
            generation: self.generation,
            expects_response,
        };
        Ok(vec![
            Effect::Publish(self.snapshot()),
            Effect::Submit { token, request },
        ])
    }

    fn reserve(&mut self, kind: PendingKind, expected: Option<bool>) -> MutationToken {
        self.next_mutation_sequence = self.next_mutation_sequence.wrapping_add(1);
        let token = MutationToken {
            generation: self.generation,
            sequence: self.next_mutation_sequence,
            kind,
        };
        self.pending = Some(Pending { token, expected });
        token
    }

    fn confirmed_by_authoritative_state(&self) -> Option<MutationToken> {
        let pending = self.pending?;
        let controls = self.controls.as_ref()?;
        let satisfied = match pending.token.kind {
            PendingKind::Package => controls.package_enabled == pending.expected,
            PendingKind::Flywheel => controls.flywheel_enabled == pending.expected,
            PendingKind::Leds => controls.leds_enabled == pending.expected,
            PendingKind::Headlights => controls.headlights_enabled == pending.expected,
            PendingKind::ClearFatalError => {
                pending.expected == Some(false)
                    && controls.fatal_error_latched == Some(false)
                    && controls.fatal_error_active == Some(false)
            }
            PendingKind::ControllerShutdown
            | PendingKind::ConfigurationWrite
            | PendingKind::TuneApply => false,
        };
        satisfied.then_some(pending.token)
    }

    fn shutdown_available(&self) -> bool {
        self.configured
            && self.connection_ready
            && self.shutdown_supported
            && self.ride_state == telemetry::RideState::Idle
            && self.package_mode != Some(telemetry::PackageMode::Flywheel)
            && self.pending.is_none()
    }

    const fn is_current(&self, generation: scheduler::Generation) -> bool {
        self.configured && self.generation.0 == generation.0
    }
}

fn refresh_status(connection_ready: bool, controls: &mut Controls) {
    controls.status = if !connection_ready {
        ControlStatus::Unavailable
    } else if controls.package_enabled.is_some()
        && controls.flywheel_enabled.is_some()
        && (!controls.lighting_available
            || (controls.leds_enabled.is_some() && controls.headlights_enabled.is_some()))
        && (!controls.fatal_error_clear_available
            || (controls.fatal_error_latched.is_some() && controls.fatal_error_active.is_some()))
    {
        ControlStatus::Ready
    } else {
        ControlStatus::Loading
    };
}

fn is_preserved_flywheel_disable(pending: Pending) -> bool {
    pending.token.kind == PendingKind::Flywheel && matches!(pending.expected, Some(false))
}

const fn lighting_profile(profile: compatibility::LightingProfile) -> control::LightingProfile {
    match profile {
        compatibility::LightingProfile::LegacyCommand202 => {
            control::LightingProfile::LegacyCommand202
        }
        compatibility::LightingProfile::StableCommand20 => {
            control::LightingProfile::StableCommand20
        }
    }
}

const fn control_profile(profile: compatibility::ControlProfile) -> control::ControlProfile {
    control::ControlProfile {
        lighting: lighting_profile(profile.lighting),
        fatal_clear_available: profile.fatal_clear_available,
    }
}

fn validate_route(route: scheduler::Route) -> Result<(), CoordinatorError> {
    match route {
        scheduler::Route::Direct | scheduler::Route::Can(0..=253) => Ok(()),
        scheduler::Route::Can(_) => Err(CoordinatorError::InvalidRoute),
    }
}

fn routed_payload(route: scheduler::Route, payload: &[u8]) -> Result<Vec<u8>, CoordinatorError> {
    validate_route(route)?;
    match route {
        scheduler::Route::Direct => Ok(payload.to_vec()),
        scheduler::Route::Can(can_id) => {
            let mut routed = Vec::with_capacity(payload.len() + 2);
            routed.extend([FORWARD_CAN_COMMAND, can_id]);
            routed.extend_from_slice(payload);
            Ok(routed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_counter_wraps_with_generation_identity_intact() {
        let mut coordinator = DeviceMutationCoordinator::new();
        coordinator.configured = true;
        coordinator.generation = scheduler::Generation(9);
        coordinator.next_mutation_sequence = u64::MAX;
        let token = coordinator.reserve(PendingKind::ConfigurationWrite, None);
        assert_eq!(token.sequence, 0);
        assert_eq!(token.generation, scheduler::Generation(9));
    }

    #[test]
    fn route_forwarding_is_exact_and_checked() {
        assert_eq!(
            routed_payload(scheduler::Route::Direct, &[36, 101, 12]).unwrap(),
            [36, 101, 12]
        );
        assert_eq!(
            routed_payload(scheduler::Route::Can(253), &[156, 0, 0]).unwrap(),
            [34, 253, 156, 0, 0]
        );
        assert_eq!(
            routed_payload(scheduler::Route::Can(254), &[156]),
            Err(CoordinatorError::InvalidRoute)
        );
    }
}
