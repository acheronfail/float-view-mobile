use domain_types::RideState;
use protocol_vesc_bms::{BmsConnection, BmsSnapshot};

pub const DEFAULT_IDLE_PROBE_MILLIS: u64 = 500;
pub const DEFAULT_IDLE_PERSIST_MILLIS: u64 = 1_000;
pub const DEFAULT_RIDING_PERSIST_MILLIS: u64 = 100;
pub const DEFAULT_POST_RIDE_TAIL_MILLIS: u64 = 5_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SamplingSettings {
    pub idle_probe_millis: u64,
    pub idle_persist_millis: u64,
    pub riding_persist_millis: u64,
    pub post_ride_tail_millis: u64,
    pub reconnect_duration_millis: u64,
}

impl Default for SamplingSettings {
    fn default() -> Self {
        Self {
            idle_probe_millis: DEFAULT_IDLE_PROBE_MILLIS,
            idle_persist_millis: DEFAULT_IDLE_PERSIST_MILLIS,
            riding_persist_millis: DEFAULT_RIDING_PERSIST_MILLIS,
            post_ride_tail_millis: DEFAULT_POST_RIDE_TAIL_MILLIS,
            reconnect_duration_millis: crate::DEFAULT_RECONNECT_DURATION_MILLIS,
        }
    }
}

#[derive(Debug, Default)]
pub struct AdaptiveSamplingPolicy {
    pub settings: SamplingSettings,
    last_persisted_millis: Option<u64>,
    previous_ride_state: Option<RideState>,
    tail_until_millis: Option<u64>,
}

impl AdaptiveSamplingPolicy {
    #[must_use]
    pub fn new(settings: SamplingSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    pub fn should_persist(&mut self, ride_state: RideState, elapsed_millis: u64) -> bool {
        if ride_state == RideState::Riding {
            self.tail_until_millis = None;
        } else if self.previous_ride_state == Some(RideState::Riding) {
            self.tail_until_millis =
                Some(elapsed_millis.saturating_add(self.settings.post_ride_tail_millis));
        }
        let in_tail = self
            .tail_until_millis
            .is_some_and(|until| elapsed_millis <= until);
        let interval = if ride_state == RideState::Riding || in_tail {
            self.settings.riding_persist_millis
        } else {
            self.settings.idle_persist_millis
        };
        let transition = self.previous_ride_state != Some(ride_state);
        self.previous_ride_state = Some(ride_state);
        let due_threshold = interval.saturating_sub((interval / 5).min(25));
        let due = self
            .last_persisted_millis
            .is_none_or(|last| elapsed_millis.saturating_sub(last) >= due_threshold);
        if transition || due {
            self.last_persisted_millis = Some(elapsed_millis);
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackingMode {
    Saved,
    Guest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackingPolicy {
    mode: TrackingMode,
}

impl TrackingPolicy {
    #[must_use]
    pub const fn new(mode: TrackingMode) -> Self {
        Self { mode }
    }

    #[must_use]
    pub const fn mode(self) -> TrackingMode {
        self.mode
    }

    #[must_use]
    pub const fn persists_sessions(self) -> bool {
        matches!(self.mode, TrackingMode::Saved)
    }

    #[must_use]
    pub const fn uses_saved_profile(self) -> bool {
        matches!(self.mode, TrackingMode::Saved)
    }

    #[must_use]
    pub const fn reconnects(self) -> bool {
        matches!(self.mode, TrackingMode::Saved | TrackingMode::Guest)
    }
}

impl Default for TrackingPolicy {
    fn default() -> Self {
        Self::new(TrackingMode::Guest)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedBmsState {
    pub generation: u64,
    pub connection: Option<BmsConnection>,
    pub snapshot: Option<BmsSnapshot>,
}

impl OwnedBmsState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            generation: 0,
            connection: None,
            snapshot: None,
        }
    }

    pub fn begin_generation(&mut self, generation: u64) {
        self.generation = generation;
        self.connection = None;
        self.snapshot = None;
    }

    pub fn select(&mut self, generation: u64, connection: BmsConnection) -> bool {
        if generation != self.generation {
            return false;
        }
        self.connection = Some(connection);
        self.snapshot = None;
        true
    }

    pub fn accept(&mut self, generation: u64, snapshot: BmsSnapshot) -> bool {
        if generation != self.generation || self.connection.is_none() {
            return false;
        }
        self.snapshot = Some(snapshot);
        true
    }
}

impl Default for OwnedBmsState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainConnectionState {
    Disconnected,
    Connecting,
    Identifying,
    SelectingController,
    Ready,
    Reconnecting,
    Disconnecting,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuthoritativeState {
    pub revision: u64,
    pub connection: DomainConnectionState,
    pub tracking: TrackingMode,
    pub ride_state: RideState,
    pub telemetry_sequence: u64,
    pub session_active: bool,
    pub bms: OwnedBmsState,
}

impl Default for AuthoritativeState {
    fn default() -> Self {
        Self {
            revision: 0,
            connection: DomainConnectionState::Disconnected,
            tracking: TrackingMode::Guest,
            ride_state: RideState::Unknown,
            telemetry_sequence: 0,
            session_active: false,
            bms: OwnedBmsState::default(),
        }
    }
}

impl AuthoritativeState {
    pub fn connect(&mut self, tracking: TrackingMode) {
        self.tracking = tracking;
        self.connection = DomainConnectionState::Connecting;
        self.session_active = false;
        self.changed();
    }

    pub fn ready(&mut self) {
        self.connection = DomainConnectionState::Ready;
        self.session_active = self.tracking == TrackingMode::Saved;
        self.changed();
    }

    pub fn observe_telemetry(&mut self, ride_state: RideState) {
        self.ride_state = ride_state;
        self.telemetry_sequence = self.telemetry_sequence.saturating_add(1);
        self.changed();
    }

    pub fn disconnect(&mut self) {
        self.connection = DomainConnectionState::Disconnected;
        self.ride_state = RideState::Unknown;
        self.session_active = false;
        self.changed();
    }

    fn changed(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_sampling_matches_riding_tail_transition_and_jitter_policy() {
        let mut policy = AdaptiveSamplingPolicy::default();
        assert!(policy.should_persist(RideState::Idle, 0));
        assert!(!policy.should_persist(RideState::Idle, 799));
        assert!(policy.should_persist(RideState::Idle, 975));
        assert!(policy.should_persist(RideState::Riding, 1_000));
        assert!(!policy.should_persist(RideState::Riding, 1_074));
        assert!(policy.should_persist(RideState::Riding, 1_080));
        assert!(policy.should_persist(RideState::Idle, 2_000));
        assert!(!policy.should_persist(RideState::Idle, 2_079));
        assert!(policy.should_persist(RideState::Idle, 2_080));
        assert!(policy.should_persist(RideState::Idle, 7_001));
    }

    #[test]
    fn guest_never_owns_durable_sessions_or_saved_profiles() {
        let guest = TrackingPolicy::new(TrackingMode::Guest);
        assert!(!guest.persists_sessions());
        assert!(!guest.uses_saved_profile());
        assert!(guest.reconnects());
        let mut state = AuthoritativeState::default();
        state.connect(TrackingMode::Guest);
        state.ready();
        assert!(!state.session_active);
        state.connect(TrackingMode::Saved);
        state.ready();
        assert!(state.session_active);
    }
}
