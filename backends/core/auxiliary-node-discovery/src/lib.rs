//! Bounded auxiliary VESC BMS discovery decisions.
//!
//! This safe candidate consumes routed identities from controller discovery and
//! returns ordered work for a future Rust runtime. It owns no transport,
//! scheduler, decoder, persistence, host ABI, callback, local server, or OS
//! behavior and remains unselected by every production host.

#![forbid(unsafe_code)]

use std::fmt::{self, Debug, Formatter};

pub use float_view_controller_discovery::{Generation, Route, RoutedIdentity};
use float_view_controller_discovery::{MAX_CAN_ID, MAX_CAN_IDS, MAX_CANDIDATES};

const STOCK_BMS_HARDWARE_TYPE: u8 = 1;
const CUSTOM_MODULE_HARDWARE_TYPE: u8 = 2;
const VESC_MAJOR: u8 = 6;
const STOCK_BMS_MINOR: u8 = 0;
const VBMS32_MINOR: u8 = 6;
const VBMS32_HARDWARE_NAME: &str = "VBMS32";
const MAX_DISPLAY_CHARACTERS: usize = 80;

/// Largest accepted normalized CAN inventory.
pub const MAX_AUXILIARY_CAN_IDS: usize = MAX_CAN_IDS;
/// Largest publishable BMS candidate set.
pub const MAX_BMS_CANDIDATES: usize = MAX_CANDIDATES;
/// Highest accepted VESC CAN route.
pub const MAX_AUXILIARY_CAN_ID: u8 = MAX_CAN_ID;

/// Public auxiliary BMS lifecycle state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Status {
    #[default]
    Discovering,
    NotFound,
    Attached,
    Unavailable,
    Ambiguous,
}

/// One display-safe BMS candidate.
#[derive(Clone, Eq, PartialEq)]
pub struct Candidate {
    pub route: Route,
    pub hardware_name: String,
    pub vesc_version: String,
}

impl Debug for Candidate {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Candidate")
            .field("route", &self.route)
            .field("hardware_name", &"[redacted]")
            .field("vesc_version", &self.vesc_version)
            .finish()
    }
}

/// Stable auxiliary discovery and liveness failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IssueCode {
    TooManyCanDevices,
    TooManyBmsCandidates,
    PreferredBmsUnavailable,
    DiscoveryUnavailable,
}

impl IssueCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TooManyCanDevices => "too_many_can_devices",
            Self::TooManyBmsCandidates => "too_many_bms_candidates",
            Self::PreferredBmsUnavailable => "preferred_bms_unavailable",
            Self::DiscoveryUnavailable => "bms_unavailable",
        }
    }
}

/// Payload-free issue suitable for future application mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Issue {
    pub code: IssueCode,
    pub message: &'static str,
}

/// Ordered work for the future runtime to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    PingCan {
        generation: Generation,
    },
    ProbeFirmware {
        route: Route,
        generation: Generation,
    },
    RequestValues {
        route: Route,
        generation: Generation,
    },
}

/// Atomic ordered output of one state transition.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Transition {
    pub effects: Vec<Effect>,
}

impl Transition {
    fn one(effect: Effect) -> Self {
        Self {
            effects: vec![effect],
        }
    }

    /// Return whether the transition performs no runtime work.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
}

/// Deterministic public state with an opaque owned last observation.
#[derive(Clone, Eq, PartialEq)]
pub struct State<O> {
    pub status: Status,
    pub connection: Option<Route>,
    pub candidates: Vec<Candidate>,
    pub latest_observation: Option<O>,
    pub issue: Option<Issue>,
}

impl<O> Default for State<O> {
    fn default() -> Self {
        Self {
            status: Status::Discovering,
            connection: None,
            candidates: Vec::new(),
            latest_observation: None,
            issue: None,
        }
    }
}

impl<O> Debug for State<O> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("State")
            .field("status", &self.status)
            .field("connection", &self.connection)
            .field("candidates", &self.candidates)
            .field(
                "latest_observation",
                &self.latest_observation.as_ref().map(|_| "[redacted]"),
            )
            .field("issue", &self.issue)
            .finish()
    }
}

/// Checked caller errors that do not mutate discovery state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputError {
    InvalidLocalRoute,
    InvalidPreferredCanId,
    InvalidInventoryRoute,
    InvalidInventorySize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Idle,
    AwaitingCan,
    ProbingAutomatic,
    ProbingPreferred,
    Selected,
    Complete,
}

/// Complete pure auxiliary VBMS discovery and availability owner.
pub struct AuxiliaryNodeDiscovery<O> {
    generation: Generation,
    state: State<O>,
    phase: Phase,
    active_probe: Option<Route>,
    pending_can_ids: Vec<u8>,
    discovered: Vec<RoutedIdentity>,
}

impl<O> Default for AuxiliaryNodeDiscovery<O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O> Debug for AuxiliaryNodeDiscovery<O> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuxiliaryNodeDiscovery")
            .field("generation", &self.generation)
            .field("state", &self.state)
            .field("phase", &self.phase)
            .field("active_probe", &self.active_probe)
            .field("pending_can_ids", &self.pending_can_ids)
            .field("discovered_count", &self.discovered.len())
            .finish()
    }
}

impl<O> AuxiliaryNodeDiscovery<O> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            generation: Generation(0),
            state: State {
                status: Status::Discovering,
                connection: None,
                candidates: Vec::new(),
                latest_observation: None,
                issue: None,
            },
            phase: Phase::Idle,
            active_probe: None,
            pending_can_ids: Vec::new(),
            discovered: Vec::new(),
        }
    }

    /// Current auxiliary generation.
    #[must_use]
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Current public state.
    #[must_use]
    pub const fn state(&self) -> &State<O> {
        &self.state
    }

    /// Current firmware probe route, when one is pending.
    #[must_use]
    pub const fn active_probe(&self) -> Option<Route> {
        self.active_probe
    }

    /// Begin a new generation from already decoded and attributed identities.
    pub fn begin(
        &mut self,
        local_identity: RoutedIdentity,
        reusable_inventory: Option<&[RoutedIdentity]>,
        preferred_can_id: Option<u8>,
    ) -> Result<Transition, InputError> {
        validate_begin(&local_identity, reusable_inventory, preferred_can_id)?;
        self.clear_next_generation();

        if supports_bms(&local_identity) {
            return Ok(self.select(Route::Direct));
        }

        if let Some(can_id) = preferred_can_id {
            let route = Route::Can(can_id);
            if let Some(identity) = reusable_inventory
                .into_iter()
                .flatten()
                .find(|identity| identity.route == route)
            {
                return Ok(if supports_bms(identity) {
                    self.select(route)
                } else {
                    self.fail(
                        IssueCode::PreferredBmsUnavailable,
                        "The configured BMS route is unavailable.",
                    )
                });
            }
            self.phase = Phase::ProbingPreferred;
            return Ok(self.probe(route));
        }

        if let Some(inventory) = reusable_inventory {
            self.discovered.extend(
                inventory
                    .iter()
                    .filter(|value| matches!(value.route, Route::Can(_)) && supports_bms(value))
                    .cloned(),
            );
            return Ok(self.finish());
        }

        self.phase = Phase::AwaitingCan;
        Ok(Transition::one(Effect::PingCan {
            generation: self.generation,
        }))
    }

    /// Normalize one CAN ping result and begin sequential probing.
    pub fn accept_can_ids(&mut self, raw_can_ids: &[u8], generation: Generation) -> Transition {
        if generation != self.generation || self.phase != Phase::AwaitingCan {
            return Transition::default();
        }
        let mut present = [false; MAX_CAN_ID as usize + 1];
        for &can_id in raw_can_ids {
            if can_id <= MAX_CAN_ID {
                present[can_id as usize] = true;
            }
        }
        if present.iter().filter(|value| **value).count() > MAX_CAN_IDS {
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
                .filter_map(|(can_id, is_present)| is_present.then_some(can_id as u8)),
        );
        self.phase = Phase::ProbingAutomatic;
        self.next_probe()
    }

    /// Accept one decoded identity only for the current attributed probe.
    pub fn accept_firmware(&mut self, value: RoutedIdentity, generation: Generation) -> Transition {
        if generation != self.generation
            || self.active_probe != Some(value.route)
            || !valid_can_route(value.route)
        {
            return Transition::default();
        }
        self.active_probe = None;
        match self.phase {
            Phase::ProbingPreferred => {
                if supports_bms(&value) {
                    self.select(value.route)
                } else {
                    self.fail(
                        IssueCode::PreferredBmsUnavailable,
                        "The configured BMS route is unavailable.",
                    )
                }
            }
            Phase::ProbingAutomatic => {
                if supports_bms(&value) {
                    if self.discovered.len() == MAX_BMS_CANDIDATES {
                        return self.fail(
                            IssueCode::TooManyBmsCandidates,
                            "More than 16 compatible BMS nodes were found.",
                        );
                    }
                    self.discovered.push(value);
                }
                self.next_probe()
            }
            _ => Transition::default(),
        }
    }

    /// Resolve one current firmware-probe timeout.
    pub fn probe_timed_out(&mut self, route: Route, generation: Generation) -> Transition {
        if generation != self.generation || self.active_probe != Some(route) {
            return Transition::default();
        }
        self.active_probe = None;
        match self.phase {
            Phase::ProbingPreferred => self.fail(
                IssueCode::PreferredBmsUnavailable,
                "The configured BMS route did not respond.",
            ),
            Phase::ProbingAutomatic => self.next_probe(),
            _ => Transition::default(),
        }
    }

    /// Resolve one current CAN-scan timeout or scheduling failure.
    pub fn scan_failed(&mut self, generation: Generation) -> Transition {
        if generation != self.generation || self.phase != Phase::AwaitingCan {
            return Transition::default();
        }
        self.fail(
            IssueCode::DiscoveryUnavailable,
            "BMS discovery is unavailable.",
        )
    }

    /// Attach a current observation to the selected route.
    pub fn accept_observation(
        &mut self,
        route: Route,
        generation: Generation,
        observation: O,
    ) -> Transition {
        if generation != self.generation || self.state.connection != Some(route) {
            return Transition::default();
        }
        self.state.status = Status::Attached;
        self.state.latest_observation = Some(observation);
        self.state.issue = None;
        Transition::default()
    }

    /// Mark only the current selected route unavailable while retaining its observation.
    pub fn mark_unavailable(&mut self, route: Route, generation: Generation) -> Transition {
        if generation != self.generation || self.state.connection != Some(route) {
            return Transition::default();
        }
        self.state.status = Status::Unavailable;
        self.state.issue = Some(issue(
            IssueCode::DiscoveryUnavailable,
            "BMS values are unavailable.",
        ));
        Transition::default()
    }

    /// Clear all state and invalidate work from the prior generation.
    pub fn reset(&mut self) {
        self.clear_next_generation();
        self.phase = Phase::Idle;
    }

    fn clear_next_generation(&mut self) {
        self.generation = Generation(self.generation.0.wrapping_add(1));
        self.state = State::default();
        self.phase = Phase::Idle;
        self.active_probe = None;
        self.pending_can_ids.clear();
        self.discovered.clear();
    }

    fn probe(&mut self, route: Route) -> Transition {
        self.active_probe = Some(route);
        Transition::one(Effect::ProbeFirmware {
            route,
            generation: self.generation,
        })
    }

    fn next_probe(&mut self) -> Transition {
        if self.pending_can_ids.is_empty() {
            return self.finish();
        }
        let can_id = self.pending_can_ids.remove(0);
        self.probe(Route::Can(can_id))
    }

    fn finish(&mut self) -> Transition {
        self.discovered.sort_by_key(|value| route_key(value.route));
        if self.discovered.len() > MAX_BMS_CANDIDATES {
            return self.fail(
                IssueCode::TooManyBmsCandidates,
                "More than 16 compatible BMS nodes were found.",
            );
        }
        self.state.candidates = self.discovered.iter().map(candidate).collect();
        match self.discovered.len() {
            0 => {
                self.phase = Phase::Complete;
                self.state.status = Status::NotFound;
                Transition::default()
            }
            1 => self.select(self.discovered[0].route),
            _ => {
                self.phase = Phase::Complete;
                self.state.status = Status::Ambiguous;
                Transition::default()
            }
        }
    }

    fn select(&mut self, route: Route) -> Transition {
        self.phase = Phase::Selected;
        self.active_probe = None;
        self.pending_can_ids.clear();
        self.state.status = Status::Discovering;
        self.state.connection = Some(route);
        self.state.candidates.clear();
        self.state.issue = None;
        Transition::one(Effect::RequestValues {
            route,
            generation: self.generation,
        })
    }

    fn fail(&mut self, code: IssueCode, message: &'static str) -> Transition {
        self.phase = Phase::Complete;
        self.active_probe = None;
        self.pending_can_ids.clear();
        self.discovered.clear();
        self.state.status = Status::Unavailable;
        self.state.connection = None;
        self.state.candidates.clear();
        self.state.issue = Some(issue(code, message));
        Transition::default()
    }
}

fn validate_begin(
    local_identity: &RoutedIdentity,
    reusable_inventory: Option<&[RoutedIdentity]>,
    preferred_can_id: Option<u8>,
) -> Result<(), InputError> {
    if local_identity.route != Route::Direct {
        return Err(InputError::InvalidLocalRoute);
    }
    if preferred_can_id.is_some_and(|can_id| can_id > MAX_CAN_ID) {
        return Err(InputError::InvalidPreferredCanId);
    }
    if let Some(inventory) = reusable_inventory {
        let mut direct_seen = false;
        let mut can_seen = [false; MAX_CAN_ID as usize + 1];
        let mut can_count = 0_usize;
        for value in inventory {
            match value.route {
                Route::Direct if !direct_seen => direct_seen = true,
                Route::Direct => return Err(InputError::InvalidInventoryRoute),
                Route::Can(can_id) if can_id <= MAX_CAN_ID && !can_seen[can_id as usize] => {
                    can_seen[can_id as usize] = true;
                    can_count += 1;
                    if can_count > MAX_CAN_IDS {
                        return Err(InputError::InvalidInventorySize);
                    }
                }
                Route::Can(_) => return Err(InputError::InvalidInventoryRoute),
            }
        }
    }
    Ok(())
}

fn valid_can_route(route: Route) -> bool {
    matches!(route, Route::Can(can_id) if can_id <= MAX_CAN_ID)
}

/// Exact audited VBMS compatibility policy over a routed decoded identity.
#[must_use]
pub fn supports_bms(value: &RoutedIdentity) -> bool {
    let identity = &value.identity;
    (identity.hardware_type == STOCK_BMS_HARDWARE_TYPE
        && identity.major == VESC_MAJOR
        && identity.minor == STOCK_BMS_MINOR)
        || (identity.hardware_type == CUSTOM_MODULE_HARDWARE_TYPE
            && identity.major == VESC_MAJOR
            && identity.minor == VBMS32_MINOR
            && identity.hardware_name == VBMS32_HARDWARE_NAME)
}

fn candidate(value: &RoutedIdentity) -> Candidate {
    Candidate {
        route: value.route,
        hardware_name: value
            .identity
            .hardware_name
            .chars()
            .take(MAX_DISPLAY_CHARACTERS)
            .collect(),
        vesc_version: format!("{}.{:02}", value.identity.major, value.identity.minor),
    }
}

fn route_key(route: Route) -> i16 {
    match route {
        Route::Direct => -1,
        Route::Can(can_id) => i16::from(can_id),
    }
}

const fn issue(code: IssueCode, message: &'static str) -> Issue {
    Issue { code, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_wrap_and_reset_are_explicit() {
        let mut discovery = AuxiliaryNodeDiscovery::<u8>::new();
        discovery.generation = Generation(u64::MAX);
        discovery.reset();
        assert_eq!(discovery.generation(), Generation(0));
        assert_eq!(discovery.state(), &State::default());
        assert_eq!(discovery.phase, Phase::Idle);
    }
}
