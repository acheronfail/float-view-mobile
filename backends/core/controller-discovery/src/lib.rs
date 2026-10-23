//! Bounded direct/CAN controller-discovery decisions.
//!
//! This safe candidate consumes already-decoded facts and returns ordered work
//! for a future Rust runtime. It owns no transport, scheduler, persistence,
//! host ABI, callback, local server, or OS behavior and remains unselected.

#![forbid(unsafe_code)]

use std::fmt::{self, Debug, Formatter};

use float_view_refloat_compatibility::{CompatibilityProfile, TelemetryFamily};
use float_view_vesc_compatibility::{AccessMode, access_mode, supports};
use float_view_vesc_firmware_identity::{FirmwareIdentity, MOTOR_HARDWARE_TYPE};

/// Largest accepted normalized CAN inventory.
pub const MAX_CAN_IDS: usize = 64;
/// Largest publishable motor-controller candidate set.
pub const MAX_CANDIDATES: usize = 16;
/// Highest route accepted by VESC CAN forwarding.
pub const MAX_CAN_ID: u8 = 253;

/// One direct or forwarded controller route.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Route {
    Direct,
    Can(u8),
}

impl Route {
    /// Construct a checked CAN route.
    pub const fn can(can_id: u8) -> Result<Self, DiscoveryInputError> {
        if can_id <= MAX_CAN_ID {
            Ok(Self::Can(can_id))
        } else {
            Err(DiscoveryInputError::InvalidCanId)
        }
    }

    /// Return the CAN ID for a forwarded route.
    #[must_use]
    pub const fn can_id(self) -> Option<u8> {
        match self {
            Self::Direct => None,
            Self::Can(can_id) => Some(can_id),
        }
    }
}

/// A connection-state generation used to reject stale work.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Generation(pub u64);

/// Private learned route facts supplied by the application owner.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RoutingHint {
    pub can_id: u8,
    controller_uuid: [u8; 12],
}

impl RoutingHint {
    /// Construct a hint only for a checked route and meaningful UUID.
    pub fn new(can_id: u8, controller_uuid: [u8; 12]) -> Result<Self, DiscoveryInputError> {
        Route::can(can_id)?;
        if meaningful_uuid(controller_uuid).is_none() {
            return Err(DiscoveryInputError::InvalidControllerUuid);
        }
        Ok(Self {
            can_id,
            controller_uuid,
        })
    }

    fn controller_uuid(self) -> [u8; 12] {
        self.controller_uuid
    }
}

impl Debug for RoutingHint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RoutingHint")
            .field("can_id", &self.can_id)
            .field("controller_uuid", &"[redacted]")
            .finish()
    }
}

/// Active-session identity facts used during reconnect discovery.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Continuity {
    pub has_active_session: bool,
    pub route: Option<Route>,
    controller_uuid: Option<[u8; 12]>,
}

impl Continuity {
    #[must_use]
    pub fn new(
        has_active_session: bool,
        route: Option<Route>,
        controller_uuid: Option<[u8; 12]>,
    ) -> Self {
        Self {
            has_active_session,
            route,
            controller_uuid: controller_uuid.and_then(meaningful_uuid),
        }
    }

    fn controller_uuid(self) -> Option<[u8; 12]> {
        self.controller_uuid
    }
}

impl Default for Continuity {
    fn default() -> Self {
        Self::new(false, None, None)
    }
}

impl Debug for Continuity {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Continuity")
            .field("has_active_session", &self.has_active_session)
            .field("route", &self.route)
            .field(
                "controller_uuid",
                &self.controller_uuid.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

/// One decoded identity associated with its proven route.
#[derive(Clone, Eq, PartialEq)]
pub struct RoutedIdentity {
    pub route: Route,
    pub identity: FirmwareIdentity,
}

impl Debug for RoutedIdentity {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RoutedIdentity")
            .field("route", &self.route)
            .field("identity", &self.identity)
            .finish()
    }
}

/// Display-safe candidate published for explicit selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerCandidate {
    pub can_id: u8,
    pub hardware_name: String,
    pub firmware_name: Option<String>,
    pub vesc_version: String,
    pub vesc_supported: bool,
}

/// Stable discovery failure classes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IssueCode {
    IdentificationTimeout,
    NoControllerFound,
    TooManyCanDevices,
    TooManyControllers,
    PreferredControllerUnavailable,
    ControllerChanged,
    UnsupportedController,
    UnsupportedRefloat,
    RefloatInfoInvalid,
}

impl IssueCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IdentificationTimeout => "identification_timeout",
            Self::NoControllerFound => "no_controller_found",
            Self::TooManyCanDevices => "too_many_can_devices",
            Self::TooManyControllers => "too_many_controllers",
            Self::PreferredControllerUnavailable => "preferred_controller_unavailable",
            Self::ControllerChanged => "controller_changed",
            Self::UnsupportedController => "unsupported_controller",
            Self::UnsupportedRefloat => "unsupported_refloat",
            Self::RefloatInfoInvalid => "refloat_info_invalid",
        }
    }
}

/// Payload-free issue suitable for later application mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Issue {
    pub code: IssueCode,
    pub message: &'static str,
}

/// Ordered work for the future runtime to execute.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    ProbeFirmware {
        route: Route,
        identification_generation: Generation,
        probe_generation: Option<Generation>,
    },
    PingCan,
    ClearRoutingHint,
    RequestRefloatInfo {
        route: Route,
        remember_controller: Option<RoutedIdentity>,
    },
    RequestTelemetryLayout,
    Ready,
    SelectingController,
    Failed(Issue),
}

/// Atomic ordered output of one state transition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Transition {
    pub effects: Vec<Effect>,
}

impl Transition {
    fn one(effect: Effect) -> Self {
        Self {
            effects: vec![effect],
        }
    }

    fn clear_then(effect: Effect) -> Self {
        Self {
            effects: vec![Effect::ClearRoutingHint, effect],
        }
    }

    /// Return whether the transition performs no host work.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
}

/// Deterministic public state without private diagnostic material.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub selected: Option<RoutedIdentity>,
    pub transport: Option<FirmwareIdentity>,
    pub selected_access: Option<AccessMode>,
    pub refloat_profile: Option<CompatibilityProfile>,
    pub route: Option<Route>,
    pub candidates: Vec<ControllerCandidate>,
    pub inventory: Vec<RoutedIdentity>,
    pub inventory_complete: bool,
    pub issue: Option<Issue>,
    pub started: bool,
    pub identification_generation: Generation,
    pub probe_generation: Generation,
    pub active_probe_can_id: Option<u8>,
}

/// Checked public-input failures that do not mutate discovery state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveryInputError {
    InvalidCanId,
    InvalidControllerUuid,
    InvalidSelection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProbeIntent {
    Preferred(u8),
    Hint {
        can_id: u8,
        controller_uuid: [u8; 12],
    },
    Enumerating,
}

/// Complete pure direct/CAN discovery owner.
pub struct ControllerDiscovery {
    selected: Option<RoutedIdentity>,
    transport: Option<FirmwareIdentity>,
    refloat_profile: Option<CompatibilityProfile>,
    route: Option<Route>,
    candidates: Vec<ControllerCandidate>,
    inventory: Vec<RoutedIdentity>,
    inventory_complete: bool,
    issue: Option<Issue>,
    started: bool,
    identification_generation: Generation,
    probe_generation: Generation,
    active_probe_can_id: Option<u8>,
    probed_controllers: Vec<RoutedIdentity>,
    probed_nodes: Vec<RoutedIdentity>,
    pending_can_ids: Vec<u8>,
    probe_intent: Option<ProbeIntent>,
    recovery_controller_uuid: Option<[u8; 12]>,
    continuity: Continuity,
}

impl Debug for ControllerDiscovery {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ControllerDiscovery")
            .field("selected", &self.selected)
            .field("transport", &self.transport)
            .field("refloat_profile", &self.refloat_profile)
            .field("route", &self.route)
            .field("candidates", &self.candidates)
            .field("inventory", &self.inventory)
            .field("inventory_complete", &self.inventory_complete)
            .field("issue", &self.issue)
            .field("started", &self.started)
            .field("identification_generation", &self.identification_generation)
            .field("probe_generation", &self.probe_generation)
            .field("active_probe_can_id", &self.active_probe_can_id)
            .field("probed_controllers", &self.probed_controllers)
            .field("probed_nodes", &self.probed_nodes)
            .field("pending_can_ids", &self.pending_can_ids)
            .field("probe_intent", &"[redacted]")
            .field(
                "recovery_controller_uuid",
                &self.recovery_controller_uuid.as_ref().map(|_| "[redacted]"),
            )
            .field("continuity", &self.continuity)
            .finish()
    }
}

impl Default for ControllerDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl ControllerDiscovery {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selected: None,
            transport: None,
            refloat_profile: None,
            route: None,
            candidates: Vec::new(),
            inventory: Vec::new(),
            inventory_complete: false,
            issue: None,
            started: false,
            identification_generation: Generation(0),
            probe_generation: Generation(0),
            active_probe_can_id: None,
            probed_controllers: Vec::new(),
            probed_nodes: Vec::new(),
            pending_can_ids: Vec::new(),
            probe_intent: None,
            recovery_controller_uuid: None,
            continuity: Continuity {
                has_active_session: false,
                route: None,
                controller_uuid: None,
            },
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            selected: self.selected.clone(),
            transport: self.transport.clone(),
            selected_access: self
                .selected
                .as_ref()
                .and_then(|value| access_mode(&value.identity)),
            refloat_profile: self.refloat_profile,
            route: self.route,
            candidates: self.candidates.clone(),
            inventory: self.inventory.clone(),
            inventory_complete: self.inventory_complete,
            issue: self.issue,
            started: self.started,
            identification_generation: self.identification_generation,
            probe_generation: self.probe_generation,
            active_probe_can_id: self.active_probe_can_id,
        }
    }

    /// Start one discovery generation exactly once.
    pub fn begin(&mut self) -> Transition {
        if self.started {
            return Transition::default();
        }
        self.started = true;
        self.identification_generation = next_generation(self.identification_generation);
        Transition::one(Effect::ProbeFirmware {
            route: Route::Direct,
            identification_generation: self.identification_generation,
            probe_generation: None,
        })
    }

    /// Accept the local transport identity and select direct or gateway work.
    pub fn accept_transport(
        &mut self,
        identity: FirmwareIdentity,
        preferred_can_id: Option<u8>,
        routing_hint: Option<RoutingHint>,
    ) -> Transition {
        if !self.started {
            return Transition::default();
        }
        self.transport = Some(identity.clone());
        self.inventory = vec![RoutedIdentity {
            route: Route::Direct,
            identity: identity.clone(),
        }];
        self.inventory_complete = false;
        if identity.hardware_type == MOTOR_HARDWARE_TYPE {
            let selected = RoutedIdentity {
                route: Route::Direct,
                identity,
            };
            self.route = Some(Route::Direct);
            self.selected = Some(selected);
            return if self
                .selected
                .as_ref()
                .is_some_and(|value| supports(&value.identity))
            {
                Transition::one(Effect::RequestRefloatInfo {
                    route: Route::Direct,
                    remember_controller: None,
                })
            } else {
                self.fail(
                    IssueCode::UnsupportedController,
                    "Direct controller firmware is not supported.",
                )
            };
        }
        if let Some(can_id) = preferred_can_id.filter(|value| *value <= MAX_CAN_ID) {
            if routing_hint.is_some_and(|hint| hint.can_id == can_id) {
                self.recovery_controller_uuid = routing_hint.map(RoutingHint::controller_uuid);
            }
            self.probe_intent = Some(ProbeIntent::Preferred(can_id));
            return self.probe(can_id);
        }
        if let Some(hint) = routing_hint {
            self.recovery_controller_uuid = Some(hint.controller_uuid());
            self.probe_intent = Some(ProbeIntent::Hint {
                can_id: hint.can_id,
                controller_uuid: hint.controller_uuid(),
            });
            return self.probe(hint.can_id);
        }
        self.begin_enumeration()
    }

    /// Accept one routed identity only for the current active probe.
    pub fn accept_can_identity(
        &mut self,
        can_id: u8,
        identity: FirmwareIdentity,
        continuity: Continuity,
    ) -> Transition {
        if can_id > MAX_CAN_ID || self.active_probe_can_id != Some(can_id) {
            return Transition::default();
        }
        self.continuity = continuity;
        self.active_probe_can_id = None;
        let routed = RoutedIdentity {
            route: Route::Can(can_id),
            identity,
        };
        match self.probe_intent {
            Some(ProbeIntent::Preferred(expected)) => {
                if expected != can_id {
                    return Transition::default();
                }
                if routed.identity.hardware_type != MOTOR_HARDWARE_TYPE {
                    return self.fail(
                        IssueCode::PreferredControllerUnavailable,
                        "The configured CAN controller is unavailable.",
                    );
                }
                if self.recovery_controller_uuid.is_some_and(|expected_uuid| {
                    meaningful_uuid(routed.identity.controller_uuid) != Some(expected_uuid)
                }) {
                    return self.fail(
                        IssueCode::ControllerChanged,
                        "The configured CAN route now identifies a different controller.",
                    );
                }
                self.choose(routed)
            }
            Some(ProbeIntent::Hint {
                can_id: expected,
                controller_uuid,
            }) => {
                if expected != can_id {
                    return Transition::default();
                }
                if routed.identity.hardware_type == MOTOR_HARDWARE_TYPE
                    && meaningful_uuid(routed.identity.controller_uuid) == Some(controller_uuid)
                {
                    self.choose(routed)
                } else {
                    let next = self.begin_enumeration_effect();
                    Transition::clear_then(next)
                }
            }
            Some(ProbeIntent::Enumerating) => {
                if self.probed_nodes.len() < MAX_CAN_IDS {
                    self.probed_nodes.push(routed.clone());
                }
                if routed.identity.hardware_type == MOTOR_HARDWARE_TYPE {
                    if self.probed_controllers.len() == MAX_CANDIDATES {
                        return self.fail(
                            IssueCode::TooManyControllers,
                            "More than 16 VESC controllers were found.",
                        );
                    }
                    self.probed_controllers.push(routed);
                }
                self.next_can_probe()
            }
            None => Transition::default(),
        }
    }

    /// Normalize one CAN ping result and begin sequential probing.
    pub fn accept_can_ids(&mut self, raw_can_ids: &[u8], continuity: Continuity) -> Transition {
        if self.probe_intent != Some(ProbeIntent::Enumerating) {
            return Transition::default();
        }
        self.continuity = continuity;
        let mut present = [false; MAX_CAN_ID as usize + 1];
        for &can_id in raw_can_ids {
            if can_id <= MAX_CAN_ID {
                present[can_id as usize] = true;
            }
        }
        let count = present.iter().filter(|value| **value).count();
        if count > MAX_CAN_IDS {
            return self.fail(
                IssueCode::TooManyCanDevices,
                "The CAN network reported more than 64 devices.",
            );
        }
        self.pending_can_ids.clear();
        self.pending_can_ids.extend(
            present
                .iter()
                .enumerate()
                .filter_map(|(can_id, present)| present.then_some(can_id as u8)),
        );
        self.next_can_probe()
    }

    /// Accept the current exact Refloat compatibility profile.
    pub fn accept_refloat_profile(&mut self, profile: CompatibilityProfile) -> Transition {
        if self.selected.is_none() {
            return Transition::default();
        }
        self.refloat_profile = Some(profile);
        if profile.telemetry.family() == TelemetryFamily::Dynamic {
            Transition::one(Effect::RequestTelemetryLayout)
        } else {
            Transition::one(Effect::Ready)
        }
    }

    /// Admit or reject the dynamic layout decision from the safe compatibility owner.
    pub fn accept_telemetry_layout(&mut self, supported: bool) -> Transition {
        if self
            .refloat_profile
            .is_some_and(|profile| profile.telemetry.family() == TelemetryFamily::Dynamic)
            && supported
        {
            Transition::one(Effect::Ready)
        } else {
            self.fail(
                IssueCode::UnsupportedRefloat,
                "This controller's Refloat telemetry layout is not supported.",
            )
        }
    }

    /// Select only a current supported published candidate.
    pub fn select_controller(&mut self, can_id: u8) -> Result<Transition, DiscoveryInputError> {
        if can_id > MAX_CAN_ID
            || !self
                .candidates
                .iter()
                .any(|candidate| candidate.can_id == can_id && candidate.vesc_supported)
        {
            return Err(DiscoveryInputError::InvalidSelection);
        }
        let selected = self
            .probed_controllers
            .iter()
            .find(|value| value.route == Route::Can(can_id))
            .cloned()
            .ok_or(DiscoveryInputError::InvalidSelection)?;
        Ok(self.choose(selected))
    }

    /// Resolve the connection-wide identification deadline.
    pub fn identification_timed_out(&mut self, generation: Generation) -> Transition {
        if generation != self.identification_generation {
            return Transition::default();
        }
        match self.probe_intent {
            Some(ProbeIntent::Preferred(_)) => self.fail(
                IssueCode::PreferredControllerUnavailable,
                "The configured CAN controller did not respond.",
            ),
            Some(ProbeIntent::Hint { .. }) => {
                self.identification_generation = next_generation(self.identification_generation);
                let next = self.begin_enumeration_effect();
                Transition::clear_then(next)
            }
            _ => self.fail(
                IssueCode::IdentificationTimeout,
                "Controller identification timed out.",
            ),
        }
    }

    /// Resolve one routed probe deadline only when its ID and generation match.
    pub fn can_probe_timed_out(&mut self, can_id: u8, generation: Generation) -> Transition {
        if self.active_probe_can_id != Some(can_id) || generation != self.probe_generation {
            return Transition::default();
        }
        self.active_probe_can_id = None;
        match self.probe_intent {
            Some(ProbeIntent::Preferred(_)) => self.fail(
                IssueCode::PreferredControllerUnavailable,
                "The configured CAN controller did not respond.",
            ),
            Some(ProbeIntent::Hint { .. }) => {
                let next = self.begin_enumeration_effect();
                Transition::clear_then(next)
            }
            Some(ProbeIntent::Enumerating) => self.next_can_probe(),
            None => Transition::default(),
        }
    }

    /// Record one stable external discovery failure without payload data.
    pub fn mark_failure(&mut self, code: IssueCode) -> Transition {
        self.fail(code, default_message(code))
    }

    pub fn clear_issue(&mut self) {
        self.issue = None;
    }

    /// Clear all connection state and invalidate every outstanding generation.
    pub fn reset(&mut self) {
        self.selected = None;
        self.transport = None;
        self.refloat_profile = None;
        self.route = None;
        self.candidates.clear();
        self.inventory.clear();
        self.inventory_complete = false;
        self.issue = None;
        self.started = false;
        self.identification_generation = next_generation(self.identification_generation);
        self.probe_generation = next_generation(self.probe_generation);
        self.active_probe_can_id = None;
        self.probed_controllers.clear();
        self.probed_nodes.clear();
        self.pending_can_ids.clear();
        self.probe_intent = None;
        self.recovery_controller_uuid = None;
        self.continuity = Continuity::default();
    }

    fn begin_enumeration(&mut self) -> Transition {
        Transition::one(self.begin_enumeration_effect())
    }

    fn begin_enumeration_effect(&mut self) -> Effect {
        self.probe_intent = Some(ProbeIntent::Enumerating);
        self.pending_can_ids.clear();
        self.probed_controllers.clear();
        self.probed_nodes.clear();
        self.active_probe_can_id = None;
        Effect::PingCan
    }

    fn probe(&mut self, can_id: u8) -> Transition {
        self.active_probe_can_id = Some(can_id);
        self.probe_generation = next_generation(self.probe_generation);
        Transition::one(Effect::ProbeFirmware {
            route: Route::Can(can_id),
            identification_generation: self.identification_generation,
            probe_generation: Some(self.probe_generation),
        })
    }

    fn next_can_probe(&mut self) -> Transition {
        if self.pending_can_ids.is_empty() {
            return self.finish_candidates();
        }
        let can_id = self.pending_can_ids.remove(0);
        self.probe(can_id)
    }

    fn finish_candidates(&mut self) -> Transition {
        self.inventory.clear();
        if let Some(transport) = &self.transport {
            self.inventory.push(RoutedIdentity {
                route: Route::Direct,
                identity: transport.clone(),
            });
        }
        self.probed_nodes.sort_by_key(|value| value.route.can_id());
        self.inventory.extend(self.probed_nodes.iter().cloned());
        self.inventory_complete = true;
        self.probed_controllers
            .sort_by_key(|value| value.route.can_id());
        self.candidates = self
            .probed_controllers
            .iter()
            .filter_map(candidate)
            .collect();

        if self.probed_controllers.is_empty() {
            return if self.continuity.has_active_session {
                self.fail(
                    IssueCode::ControllerChanged,
                    "The controller from the active session was not found.",
                )
            } else {
                self.fail(
                    IssueCode::NoControllerFound,
                    "No VESC motor controller was found.",
                )
            };
        }
        if let Some(expected_uuid) = self.recovery_controller_uuid
            && let Some(recovered) = self
                .probed_controllers
                .iter()
                .find(|value| {
                    meaningful_uuid(value.identity.controller_uuid) == Some(expected_uuid)
                })
                .cloned()
        {
            return self.choose(recovered);
        }
        if self.continuity.has_active_session {
            if let Some(expected_uuid) = self.continuity.controller_uuid() {
                if let Some(recovered) = self
                    .probed_controllers
                    .iter()
                    .find(|value| {
                        meaningful_uuid(value.identity.controller_uuid) == Some(expected_uuid)
                            && supports(&value.identity)
                    })
                    .cloned()
                {
                    return self.choose(recovered);
                }
                return self.fail(
                    IssueCode::ControllerChanged,
                    "The controller from the active session was not found.",
                );
            }
            if let Some(previous_route) = self.continuity.route
                && let Some(recovered) = self
                    .probed_controllers
                    .iter()
                    .find(|value| value.route == previous_route && supports(&value.identity))
                    .cloned()
            {
                return self.choose(recovered);
            }
            return self.fail(
                IssueCode::ControllerChanged,
                "The controller from the active session was not found.",
            );
        }
        if self.probed_controllers.len() == 1 {
            let only = self.probed_controllers[0].clone();
            if !supports(&only.identity) {
                self.route = Some(only.route);
                self.selected = Some(only);
                return self.fail(
                    IssueCode::UnsupportedController,
                    "Controller firmware is not supported.",
                );
            }
            return self.choose(only);
        }
        Transition::one(Effect::SelectingController)
    }

    fn choose(&mut self, value: RoutedIdentity) -> Transition {
        let route = value.route;
        self.route = Some(route);
        self.selected = Some(value.clone());
        self.candidates.clear();
        if !supports(&value.identity) {
            return self.fail(
                IssueCode::UnsupportedController,
                "Controller firmware is not supported.",
            );
        }
        let remember_controller = route
            .can_id()
            .and_then(|_| meaningful_uuid(value.identity.controller_uuid))
            .map(|_| value);
        Transition::one(Effect::RequestRefloatInfo {
            route,
            remember_controller,
        })
    }

    fn fail(&mut self, code: IssueCode, message: &'static str) -> Transition {
        let issue = Issue { code, message };
        self.issue = Some(issue);
        Transition::one(Effect::Failed(issue))
    }
}

fn candidate(value: &RoutedIdentity) -> Option<ControllerCandidate> {
    let can_id = value.route.can_id()?;
    Some(ControllerCandidate {
        can_id,
        hardware_name: value.identity.hardware_name.clone(),
        firmware_name: value.identity.firmware_name.clone(),
        vesc_version: format!("{}.{:02}", value.identity.major, value.identity.minor),
        vesc_supported: supports(&value.identity),
    })
}

fn meaningful_uuid(value: [u8; 12]) -> Option<[u8; 12]> {
    if value.iter().all(|byte| *byte == 0) || value.iter().all(|byte| *byte == u8::MAX) {
        None
    } else {
        Some(value)
    }
}

const fn next_generation(value: Generation) -> Generation {
    Generation(value.0.wrapping_add(1))
}

const fn default_message(code: IssueCode) -> &'static str {
    match code {
        IssueCode::IdentificationTimeout => "Controller identification timed out.",
        IssueCode::NoControllerFound => "No VESC motor controller was found.",
        IssueCode::TooManyCanDevices => "The CAN network reported more than 64 devices.",
        IssueCode::TooManyControllers => "More than 16 VESC controllers were found.",
        IssueCode::PreferredControllerUnavailable => {
            "The configured CAN controller is unavailable."
        }
        IssueCode::ControllerChanged => "The controller from the active session was not found.",
        IssueCode::UnsupportedController => "Controller firmware is not supported.",
        IssueCode::UnsupportedRefloat => "This controller's Refloat package is not supported.",
        IssueCode::RefloatInfoInvalid => {
            "The controller returned an unreadable Refloat package response. Please try again."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_wrap_is_explicit_and_reset_is_complete() {
        let mut discovery = ControllerDiscovery::new();
        discovery.identification_generation = Generation(u64::MAX);
        discovery.probe_generation = Generation(u64::MAX);
        assert_eq!(
            discovery.begin().effects[0],
            Effect::ProbeFirmware {
                route: Route::Direct,
                identification_generation: Generation(0),
                probe_generation: None,
            }
        );
        discovery.reset();
        let snapshot = discovery.snapshot();
        assert_eq!(snapshot.identification_generation, Generation(1));
        assert_eq!(snapshot.probe_generation, Generation(0));
        assert!(!snapshot.started);
        assert!(snapshot.selected.is_none());
        assert!(snapshot.transport.is_none());
        assert!(snapshot.candidates.is_empty());
        assert!(snapshot.inventory.is_empty());
        assert!(snapshot.issue.is_none());
    }
}
