//! Pure VESC BMS discovery and polling decisions for a future Rust runtime.
//!
//! This safe candidate composes auxiliary discovery, command-96 decoding, and
//! request-scheduler facts. It owns no clock, timer executor, Bluetooth writer,
//! callback, bridge, persistence, transport, local server, or OS behavior and
//! remains unselected by every production host.

#![forbid(unsafe_code)]

use std::fmt::{self, Debug, Formatter};

use float_view_auxiliary_node_discovery as discovery;
use float_view_vesc_bms_values as values;
use float_view_vesc_request_scheduler as scheduler;

pub use discovery::{Candidate, IssueCode, Route, RoutedIdentity, Status, supports_bms};
pub use scheduler::Generation as SchedulerGeneration;

/// Current auxiliary-discovery generation.
pub type AuxiliaryGeneration = discovery::Generation;

/// Native-equivalent delay between completed BMS values requests.
pub const POLL_INTERVAL_MS: u64 = 1_000;
/// Native-equivalent deadline for one command-96 request or firmware probe.
pub const REQUEST_TIMEOUT_MS: u64 = 1_000;
/// Native-equivalent deadline for one full CAN enumeration.
pub const CAN_SCAN_TIMEOUT_MS: u64 = 10_000;
/// Consecutive failures before a selected BMS becomes unavailable.
pub const FAILURE_THRESHOLD: u8 = 3;

const GET_FIRMWARE_VERSION_COMMAND: u8 = 0;
const FORWARD_CAN_COMMAND: u8 = 34;
const PING_CAN_COMMAND: u8 = 62;

/// Complete owned application observation converted from one bounded response.
#[derive(Clone, PartialEq)]
pub struct BmsObservation {
    pub route: Route,
    pub observed_at_epoch_ms: i64,
    pub total_voltage_v: f64,
    pub charge_voltage_v: f64,
    pub input_current_a: f64,
    pub input_ic_current_a: f64,
    pub session_amp_hours_ah: f64,
    pub session_watt_hours_wh: f64,
    pub cell_voltages_v: Vec<f64>,
    pub balancing: Vec<bool>,
    pub temperatures_c: Vec<f64>,
    pub ic_temperature_c: f64,
    pub humidity_sensor_temperature_c: f64,
    pub humidity_percent: f64,
    pub highest_cell_temperature_c: f64,
    pub state_of_charge: f64,
    pub state_of_health: f64,
    pub reported_can_id: u8,
    pub lifetime_charge_amp_hours_ah: f64,
    pub lifetime_charge_watt_hours_wh: f64,
    pub lifetime_discharge_amp_hours_ah: f64,
    pub lifetime_discharge_watt_hours_wh: f64,
    pub pressure_pa: f64,
    pub data_version: Option<u8>,
    pub status: Option<String>,
}

impl Debug for BmsObservation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BmsObservation")
            .field("route", &self.route)
            .field("observed_at_epoch_ms", &self.observed_at_epoch_ms)
            .field("values", &"[redacted]")
            .field("cell_count", &self.cell_voltages_v.len())
            .field("temperature_count", &self.temperatures_c.len())
            .field("data_version", &self.data_version)
            .field("status", &self.status.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

impl BmsObservation {
    /// Keep the firmware's cell maximum separate from circuitry temperatures.
    pub fn highest_circuitry_temperature_c(&self) -> f64 {
        let mut highest = self
            .ic_temperature_c
            .max(self.humidity_sensor_temperature_c);
        // The official VBMS version-1 package reports IC, cell min, cell max,
        // MOSFET, ambient, then cell probes. Other wire families retain their
        // dedicated IC/environment readings without guessing array semantics.
        // https://github.com/vedderb/vesc_pkg/blob/main/vbms32/vbms32.lisp
        if self.data_version == Some(1) && self.temperatures_c.len() >= 5 {
            highest = highest
                .max(self.temperatures_c[0])
                .max(self.temperatures_c[3]);
            if self.temperatures_c[4] > -50.0 {
                highest = highest.max(self.temperatures_c[4]);
            }
        }
        highest
    }

    fn from_decoded(
        decoded: &values::BmsValues<'_>,
        route: Route,
        observed_at_epoch_ms: i64,
    ) -> Self {
        Self {
            route,
            observed_at_epoch_ms,
            total_voltage_v: decoded.total_voltage_v,
            charge_voltage_v: decoded.charge_voltage_v,
            input_current_a: decoded.input_current_a,
            input_ic_current_a: decoded.input_ic_current_a,
            session_amp_hours_ah: decoded.session_amp_hours_ah,
            session_watt_hours_wh: decoded.session_watt_hours_wh,
            cell_voltages_v: decoded.cell_voltages_v().to_vec(),
            balancing: decoded.balancing().to_vec(),
            temperatures_c: decoded.temperatures_c().to_vec(),
            ic_temperature_c: decoded.ic_temperature_c,
            humidity_sensor_temperature_c: decoded.humidity_sensor_temperature_c,
            humidity_percent: decoded.humidity_percent,
            highest_cell_temperature_c: decoded.highest_cell_temperature_c,
            state_of_charge: decoded.state_of_charge,
            state_of_health: decoded.state_of_health,
            reported_can_id: decoded.reported_can_id,
            lifetime_charge_amp_hours_ah: decoded.lifetime_charge_amp_hours_ah,
            lifetime_charge_watt_hours_wh: decoded.lifetime_charge_watt_hours_wh,
            lifetime_discharge_amp_hours_ah: decoded.lifetime_discharge_amp_hours_ah,
            lifetime_discharge_watt_hours_wh: decoded.lifetime_discharge_watt_hours_wh,
            pressure_pa: decoded.pressure_pa,
            data_version: decoded.data_version,
            status: decoded.status.map(str::to_owned),
        }
    }
}

/// Complete input for a replacement BMS service generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Configuration {
    pub service_generation: u64,
    pub scheduler_generation: SchedulerGeneration,
    pub local_identity: RoutedIdentity,
    pub reusable_inventory: Option<Vec<RoutedIdentity>>,
    pub preferred_can_id: Option<u8>,
}

/// Semantic scheduler request owned by this service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestKind {
    CanScan,
    Firmware,
    Values,
}

/// Completion vocabulary shared with the selected native scheduler adapters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestOutcome {
    Response,
    Sent,
    TimedOut,
    QueueFull,
    Cancelled,
    StaleGeneration,
}

/// Identity of one host-owned one-shot poll timer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerToken {
    pub service_generation: u64,
    pub auxiliary_generation: AuxiliaryGeneration,
    pub route: Route,
    pub nonce: u64,
}

/// Stable non-private service diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Diagnostic {
    StaleEvent,
    MismatchedEvent,
    MalformedDiscovery,
    MalformedValues,
    EnqueueRejected,
}

/// Checked public errors that do not partially mutate service state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceError {
    DiscoveryInput(discovery::InputError),
    InvalidRoute,
    InvalidObservationTime,
}

impl From<discovery::InputError> for ServiceError {
    fn from(value: discovery::InputError) -> Self {
        Self::DiscoveryInput(value)
    }
}

/// Bounded public snapshot without private BMS values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub service_generation: Option<u64>,
    pub auxiliary_generation: AuxiliaryGeneration,
    pub scheduler_generation: Option<SchedulerGeneration>,
    pub status: Status,
    pub route: Option<Route>,
    pub candidates: Vec<Candidate>,
    pub consecutive_failures: u8,
    pub has_observation: bool,
    pub pending_request: Option<RequestKind>,
    pub timer: Option<TimerToken>,
    pub issue: Option<IssueCode>,
}

/// Ordered runtime work returned by one transition.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Enqueue {
        kind: RequestKind,
        route: Option<Route>,
        facts: scheduler::RequestFacts,
    },
    ReplaceTimer {
        token: TimerToken,
        interval_ms: u64,
    },
    CancelTimer(TimerToken),
    PublishState(Snapshot),
    PublishObservation(BmsObservation),
    Diagnostic(Diagnostic),
}

/// Closed command surface for a future in-process Rust runtime.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Begin(Configuration),
    AcceptCanIds {
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        can_ids: Vec<u8>,
    },
    AcceptFirmware {
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        identity: RoutedIdentity,
    },
    RequestCompleted {
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        kind: RequestKind,
        route: Option<Route>,
        outcome: RequestOutcome,
    },
    EnqueueFailed {
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        kind: RequestKind,
        route: Option<Route>,
    },
    ReceiveValues {
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        route: Route,
        payload: Vec<u8>,
        observed_at_epoch_ms: i64,
    },
    DecodeFailed {
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        kind: RequestKind,
        route: Option<Route>,
    },
    TimerFired(TimerToken),
    Reset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RequestIdentity {
    service_generation: u64,
    auxiliary_generation: AuxiliaryGeneration,
    kind: RequestKind,
    route: Option<Route>,
}

/// Complete pure BMS discovery and polling application service.
pub struct VescBmsTelemetryService {
    service_generation: Option<u64>,
    scheduler_generation: Option<SchedulerGeneration>,
    discovery: discovery::AuxiliaryNodeDiscovery<BmsObservation>,
    pending_request: Option<RequestIdentity>,
    awaiting_decoded: Option<RequestIdentity>,
    timer: Option<TimerToken>,
    next_timer_nonce: u64,
    consecutive_failures: u8,
}

impl Default for VescBmsTelemetryService {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for VescBmsTelemetryService {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VescBmsTelemetryService")
            .field("service_generation", &self.service_generation)
            .field("scheduler_generation", &self.scheduler_generation)
            .field("discovery", &self.discovery)
            .field("pending_request", &self.pending_request)
            .field("awaiting_decoded", &self.awaiting_decoded)
            .field("timer", &self.timer)
            .field("next_timer_nonce", &self.next_timer_nonce)
            .field("consecutive_failures", &self.consecutive_failures)
            .finish()
    }
}

impl VescBmsTelemetryService {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            service_generation: None,
            scheduler_generation: None,
            discovery: discovery::AuxiliaryNodeDiscovery::new(),
            pending_request: None,
            awaiting_decoded: None,
            timer: None,
            next_timer_nonce: 0,
            consecutive_failures: 0,
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let state = self.discovery.state();
        Snapshot {
            service_generation: self.service_generation,
            auxiliary_generation: self.discovery.generation(),
            scheduler_generation: self.scheduler_generation,
            status: state.status,
            route: state.connection,
            candidates: state.candidates.clone(),
            consecutive_failures: self.consecutive_failures,
            has_observation: state.latest_observation.is_some(),
            pending_request: self.pending_request.map(|value| value.kind),
            timer: self.timer,
            issue: state.issue.map(|value| value.code),
        }
    }

    #[must_use]
    pub const fn observation(&self) -> Option<&BmsObservation> {
        self.discovery.state().latest_observation.as_ref()
    }

    pub fn apply(&mut self, command: Command) -> Result<Vec<Effect>, ServiceError> {
        match command {
            Command::Begin(configuration) => self.begin(configuration),
            Command::AcceptCanIds {
                service_generation,
                auxiliary_generation,
                can_ids,
            } => Ok(self.accept_can_ids(service_generation, auxiliary_generation, &can_ids)),
            Command::AcceptFirmware {
                service_generation,
                auxiliary_generation,
                identity,
            } => Ok(self.accept_firmware(service_generation, auxiliary_generation, identity)),
            Command::RequestCompleted {
                service_generation,
                auxiliary_generation,
                kind,
                route,
                outcome,
            } => Ok(self.request_completed(
                service_generation,
                auxiliary_generation,
                kind,
                route,
                outcome,
            )),
            Command::EnqueueFailed {
                service_generation,
                auxiliary_generation,
                kind,
                route,
            } => Ok(self.enqueue_failed(service_generation, auxiliary_generation, kind, route)),
            Command::ReceiveValues {
                service_generation,
                auxiliary_generation,
                route,
                payload,
                observed_at_epoch_ms,
            } => self.receive_values(
                service_generation,
                auxiliary_generation,
                route,
                &payload,
                observed_at_epoch_ms,
            ),
            Command::DecodeFailed {
                service_generation,
                auxiliary_generation,
                kind,
                route,
            } => Ok(self.decode_failed(service_generation, auxiliary_generation, kind, route)),
            Command::TimerFired(token) => Ok(self.timer_fired(token)),
            Command::Reset => Ok(self.reset()),
        }
    }

    pub fn begin(&mut self, configuration: Configuration) -> Result<Vec<Effect>, ServiceError> {
        validate_route(configuration.local_identity.route)?;
        if let Some(inventory) = &configuration.reusable_inventory {
            for value in inventory {
                validate_route(value.route)?;
            }
        }
        let transition = self.discovery.begin(
            configuration.local_identity,
            configuration.reusable_inventory.as_deref(),
            configuration.preferred_can_id,
        )?;
        let mut effects = self.cancel_timer_effect();
        self.service_generation = Some(configuration.service_generation);
        self.scheduler_generation = Some(configuration.scheduler_generation);
        self.pending_request = None;
        self.awaiting_decoded = None;
        self.consecutive_failures = 0;
        effects.extend(self.translate_transition(transition));
        Ok(effects)
    }

    pub fn accept_can_ids(
        &mut self,
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        can_ids: &[u8],
    ) -> Vec<Effect> {
        let expected = RequestIdentity {
            service_generation,
            auxiliary_generation,
            kind: RequestKind::CanScan,
            route: None,
        };
        if !self.consume_decoded(expected) {
            return self.stale_or_mismatched(expected);
        }
        let transition = self.discovery.accept_can_ids(can_ids, auxiliary_generation);
        self.translate_transition(transition)
    }

    pub fn accept_firmware(
        &mut self,
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        identity: RoutedIdentity,
    ) -> Vec<Effect> {
        let expected = RequestIdentity {
            service_generation,
            auxiliary_generation,
            kind: RequestKind::Firmware,
            route: Some(identity.route),
        };
        if !self.consume_decoded(expected) {
            return self.stale_or_mismatched(expected);
        }
        let transition = self
            .discovery
            .accept_firmware(identity, auxiliary_generation);
        self.translate_transition(transition)
    }

    pub fn request_completed(
        &mut self,
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        kind: RequestKind,
        route: Option<Route>,
        outcome: RequestOutcome,
    ) -> Vec<Effect> {
        let expected = RequestIdentity {
            service_generation,
            auxiliary_generation,
            kind,
            route,
        };
        if self.pending_request != Some(expected) {
            return self.stale_or_mismatched(expected);
        }
        self.pending_request = None;
        match outcome {
            RequestOutcome::Response => {
                self.awaiting_decoded = Some(expected);
                if kind == RequestKind::Values {
                    self.replace_poll_timer(expected)
                } else {
                    Vec::new()
                }
            }
            RequestOutcome::Sent => {
                if kind == RequestKind::Values {
                    self.replace_poll_timer(expected)
                } else {
                    Vec::new()
                }
            }
            RequestOutcome::TimedOut | RequestOutcome::QueueFull => self.failed_request(expected),
            RequestOutcome::Cancelled | RequestOutcome::StaleGeneration => Vec::new(),
        }
    }

    pub fn enqueue_failed(
        &mut self,
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        kind: RequestKind,
        route: Option<Route>,
    ) -> Vec<Effect> {
        let expected = RequestIdentity {
            service_generation,
            auxiliary_generation,
            kind,
            route,
        };
        if self.pending_request != Some(expected) {
            return self.stale_or_mismatched(expected);
        }
        self.pending_request = None;
        let mut effects = vec![Effect::Diagnostic(Diagnostic::EnqueueRejected)];
        effects.extend(self.failed_request(expected));
        effects
    }

    pub fn receive_values(
        &mut self,
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        route: Route,
        payload: &[u8],
        observed_at_epoch_ms: i64,
    ) -> Result<Vec<Effect>, ServiceError> {
        if observed_at_epoch_ms <= 0 {
            return Err(ServiceError::InvalidObservationTime);
        }
        let expected = RequestIdentity {
            service_generation,
            auxiliary_generation,
            kind: RequestKind::Values,
            route: Some(route),
        };
        if !self.consume_decoded(expected) {
            return Ok(self.stale_or_mismatched(expected));
        }
        let expected_can_id = match route {
            Route::Direct => None,
            Route::Can(can_id) if can_id <= 253 => Some(can_id),
            Route::Can(_) => {
                return Ok(vec![Effect::Diagnostic(Diagnostic::MismatchedEvent)]);
            }
        };
        let decoded = match values::decode(payload, expected_can_id) {
            Ok(value) => value,
            Err(_) => {
                let mut effects = vec![Effect::Diagnostic(Diagnostic::MalformedValues)];
                effects.extend(self.record_values_failure(route, auxiliary_generation));
                return Ok(effects);
            }
        };
        let observation = BmsObservation::from_decoded(&decoded, route, observed_at_epoch_ms);
        self.consecutive_failures = 0;
        self.discovery
            .accept_observation(route, auxiliary_generation, observation.clone());
        Ok(vec![
            Effect::PublishObservation(observation),
            Effect::PublishState(self.snapshot()),
        ])
    }

    pub fn decode_failed(
        &mut self,
        service_generation: u64,
        auxiliary_generation: AuxiliaryGeneration,
        kind: RequestKind,
        route: Option<Route>,
    ) -> Vec<Effect> {
        let expected = RequestIdentity {
            service_generation,
            auxiliary_generation,
            kind,
            route,
        };
        if !self.consume_decoded(expected) {
            return self.stale_or_mismatched(expected);
        }
        let mut effects = vec![Effect::Diagnostic(Diagnostic::MalformedDiscovery)];
        effects.extend(match (kind, route) {
            (RequestKind::CanScan, None) => {
                let transition = self.discovery.scan_failed(auxiliary_generation);
                self.translate_transition(transition)
            }
            (RequestKind::Firmware, Some(route)) => {
                let transition = self.discovery.probe_timed_out(route, auxiliary_generation);
                self.translate_transition(transition)
            }
            (RequestKind::Values, Some(route)) => {
                self.record_values_failure(route, auxiliary_generation)
            }
            _ => vec![Effect::Diagnostic(Diagnostic::MismatchedEvent)],
        });
        effects
    }

    pub fn timer_fired(&mut self, token: TimerToken) -> Vec<Effect> {
        if self.timer != Some(token)
            || self.service_generation != Some(token.service_generation)
            || self.discovery.generation() != token.auxiliary_generation
            || self.discovery.state().connection != Some(token.route)
        {
            return vec![Effect::Diagnostic(Diagnostic::StaleEvent)];
        }
        self.timer = None;
        self.enqueue(
            RequestKind::Values,
            Some(token.route),
            token.auxiliary_generation,
        )
    }

    pub fn reset(&mut self) -> Vec<Effect> {
        let mut effects = self.cancel_timer_effect();
        self.discovery.reset();
        self.service_generation = None;
        self.scheduler_generation = None;
        self.pending_request = None;
        self.awaiting_decoded = None;
        self.consecutive_failures = 0;
        effects.push(Effect::PublishState(self.snapshot()));
        effects
    }

    fn translate_transition(&mut self, transition: discovery::Transition) -> Vec<Effect> {
        if transition.effects.is_empty() {
            return vec![Effect::PublishState(self.snapshot())];
        }
        let mut effects = Vec::with_capacity(transition.effects.len());
        for effect in transition.effects {
            effects.extend(match effect {
                discovery::Effect::PingCan { generation } => {
                    self.enqueue(RequestKind::CanScan, None, generation)
                }
                discovery::Effect::ProbeFirmware { route, generation } => {
                    self.enqueue(RequestKind::Firmware, Some(route), generation)
                }
                discovery::Effect::RequestValues { route, generation } => {
                    self.enqueue(RequestKind::Values, Some(route), generation)
                }
            });
        }
        effects
    }

    fn enqueue(
        &mut self,
        kind: RequestKind,
        route: Option<Route>,
        auxiliary_generation: AuxiliaryGeneration,
    ) -> Vec<Effect> {
        let Some(service_generation) = self.service_generation else {
            return vec![Effect::Diagnostic(Diagnostic::StaleEvent)];
        };
        let Some(scheduler_generation) = self.scheduler_generation else {
            return vec![Effect::Diagnostic(Diagnostic::StaleEvent)];
        };
        if self.pending_request.is_some() {
            return vec![Effect::Diagnostic(Diagnostic::MismatchedEvent)];
        }
        let facts = match request_facts(kind, route, scheduler_generation) {
            Ok(value) => value,
            Err(_) => return vec![Effect::Diagnostic(Diagnostic::MismatchedEvent)],
        };
        self.pending_request = Some(RequestIdentity {
            service_generation,
            auxiliary_generation,
            kind,
            route,
        });
        vec![Effect::Enqueue { kind, route, facts }]
    }

    fn failed_request(&mut self, request: RequestIdentity) -> Vec<Effect> {
        match request.kind {
            RequestKind::CanScan => {
                let transition = self.discovery.scan_failed(request.auxiliary_generation);
                self.translate_transition(transition)
            }
            RequestKind::Firmware => {
                let Some(route) = request.route else {
                    return vec![Effect::Diagnostic(Diagnostic::MismatchedEvent)];
                };
                let transition = self
                    .discovery
                    .probe_timed_out(route, request.auxiliary_generation);
                self.translate_transition(transition)
            }
            RequestKind::Values => {
                let Some(route) = request.route else {
                    return vec![Effect::Diagnostic(Diagnostic::MismatchedEvent)];
                };
                let mut effects = self.record_values_failure(route, request.auxiliary_generation);
                effects.extend(self.replace_poll_timer(request));
                effects
            }
        }
    }

    fn record_values_failure(
        &mut self,
        route: Route,
        auxiliary_generation: AuxiliaryGeneration,
    ) -> Vec<Effect> {
        if self.discovery.generation() != auxiliary_generation
            || self.discovery.state().connection != Some(route)
        {
            return vec![Effect::Diagnostic(Diagnostic::StaleEvent)];
        }
        self.consecutive_failures = self
            .consecutive_failures
            .saturating_add(1)
            .min(FAILURE_THRESHOLD);
        if self.consecutive_failures < FAILURE_THRESHOLD {
            return Vec::new();
        }
        self.discovery.mark_unavailable(route, auxiliary_generation);
        vec![Effect::PublishState(self.snapshot())]
    }

    fn replace_poll_timer(&mut self, request: RequestIdentity) -> Vec<Effect> {
        let Some(route) = request.route else {
            return vec![Effect::Diagnostic(Diagnostic::MismatchedEvent)];
        };
        if request.kind != RequestKind::Values
            || self.service_generation != Some(request.service_generation)
            || self.discovery.generation() != request.auxiliary_generation
            || self.discovery.state().connection != Some(route)
        {
            return vec![Effect::Diagnostic(Diagnostic::StaleEvent)];
        }
        let mut effects = self.cancel_timer_effect();
        self.next_timer_nonce = self.next_timer_nonce.wrapping_add(1);
        let token = TimerToken {
            service_generation: request.service_generation,
            auxiliary_generation: request.auxiliary_generation,
            route,
            nonce: self.next_timer_nonce,
        };
        self.timer = Some(token);
        effects.push(Effect::ReplaceTimer {
            token,
            interval_ms: POLL_INTERVAL_MS,
        });
        effects
    }

    fn cancel_timer_effect(&mut self) -> Vec<Effect> {
        self.timer
            .take()
            .map(Effect::CancelTimer)
            .into_iter()
            .collect()
    }

    fn consume_decoded(&mut self, expected: RequestIdentity) -> bool {
        if self.awaiting_decoded != Some(expected) {
            return false;
        }
        self.awaiting_decoded = None;
        true
    }

    fn stale_or_mismatched(&self, event: RequestIdentity) -> Vec<Effect> {
        let diagnostic = if self.service_generation != Some(event.service_generation)
            || self.discovery.generation() != event.auxiliary_generation
        {
            Diagnostic::StaleEvent
        } else {
            Diagnostic::MismatchedEvent
        };
        vec![Effect::Diagnostic(diagnostic)]
    }
}

/// Construct the exact unframed read-only request facts for one BMS service action.
pub fn request_facts(
    kind: RequestKind,
    route: Option<Route>,
    generation: SchedulerGeneration,
) -> Result<scheduler::RequestFacts, ServiceError> {
    let (payload, expected_command, context, timeout_ms) = match kind {
        RequestKind::CanScan if route.is_none() => (
            vec![PING_CAN_COMMAND],
            PING_CAN_COMMAND,
            scheduler::RequestContext::AuxiliaryCanPing,
            CAN_SCAN_TIMEOUT_MS,
        ),
        RequestKind::Firmware => {
            let route = route.ok_or(ServiceError::InvalidRoute)?;
            let scheduler_route = scheduler_route(route)?;
            let payload = routed_payload(route, GET_FIRMWARE_VERSION_COMMAND)?;
            (
                payload,
                GET_FIRMWARE_VERSION_COMMAND,
                scheduler::RequestContext::AuxiliaryFirmware(scheduler_route),
                REQUEST_TIMEOUT_MS,
            )
        }
        RequestKind::Values => {
            let route = route.ok_or(ServiceError::InvalidRoute)?;
            let scheduler_route = scheduler_route(route)?;
            let payload = routed_payload(route, values::COMM_BMS_GET_VALUES)?;
            (
                payload,
                values::COMM_BMS_GET_VALUES,
                scheduler::RequestContext::BmsValues(scheduler_route),
                REQUEST_TIMEOUT_MS,
            )
        }
        RequestKind::CanScan => return Err(ServiceError::InvalidRoute),
    };
    Ok(scheduler::RequestFacts {
        payload,
        expected_response: scheduler::ExpectedResponse {
            command: expected_command,
            refloat_command: None,
        },
        priority: scheduler::Priority::Auxiliary,
        context,
        timeout_ms,
        generation,
        expects_response: true,
    })
}

fn routed_payload(route: Route, command: u8) -> Result<Vec<u8>, ServiceError> {
    match route {
        Route::Direct => Ok(vec![command]),
        Route::Can(can_id) if can_id <= 253 => Ok(vec![FORWARD_CAN_COMMAND, can_id, command]),
        Route::Can(_) => Err(ServiceError::InvalidRoute),
    }
}

fn scheduler_route(route: Route) -> Result<scheduler::Route, ServiceError> {
    match route {
        Route::Direct => Ok(scheduler::Route::Direct),
        Route::Can(can_id) if can_id <= 253 => Ok(scheduler::Route::Can(can_id)),
        Route::Can(_) => Err(ServiceError::InvalidRoute),
    }
}

fn validate_route(route: Route) -> Result<(), ServiceError> {
    match route {
        Route::Direct | Route::Can(0..=253) => Ok(()),
        Route::Can(_) => Err(ServiceError::InvalidRoute),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_view_auxiliary_node_discovery::RoutedIdentity;
    use float_view_vesc_firmware_identity::FirmwareIdentity;

    #[test]
    fn timer_nonce_rollover_remains_deterministic_and_bounded() {
        let mut service = VescBmsTelemetryService::new();
        let effects = service
            .begin(Configuration {
                service_generation: 9,
                scheduler_generation: SchedulerGeneration(19),
                local_identity: RoutedIdentity {
                    route: Route::Direct,
                    identity: FirmwareIdentity {
                        major: 6,
                        minor: 6,
                        hardware_name: "VBMS32".to_owned(),
                        firmware_name: None,
                        controller_uuid: [1; 12],
                        test_version: 0,
                        hardware_type: 2,
                        custom_configuration_count: 0,
                        hardware_configuration_crc: None,
                    },
                },
                reusable_inventory: None,
                preferred_can_id: None,
            })
            .unwrap();
        assert!(matches!(effects.as_slice(), [Effect::Enqueue { .. }]));
        service.next_timer_nonce = u64::MAX;
        let generation = service.discovery.generation();
        let effects = service.request_completed(
            9,
            generation,
            RequestKind::Values,
            Some(Route::Direct),
            RequestOutcome::Response,
        );
        assert!(matches!(
            effects.as_slice(),
            [Effect::ReplaceTimer {
                token: TimerToken { nonce: 0, .. },
                interval_ms: POLL_INTERVAL_MS,
            }]
        ));
    }
}
