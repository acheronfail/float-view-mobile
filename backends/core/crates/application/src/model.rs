use float_view_bms::{BmsConnection, BmsSnapshot};
use float_view_domain::RideState;
use float_view_refloat::{MutationCoordinator, Telemetry};

use crate::{
    AdaptiveSamplingPolicy, AuthoritativeState, ConfigurationService, TrackingMode, TrackingPolicy,
};

#[derive(Default)]
pub struct ApplicationModel {
    pub state: AuthoritativeState,
    pub sampling: AdaptiveSamplingPolicy,
    pub tracking: TrackingPolicy,
    pub mutations: Option<MutationCoordinator>,
    pub configuration: ConfigurationService,
}

impl ApplicationModel {
    pub fn connect(&mut self, mode: TrackingMode) {
        self.tracking = TrackingPolicy::new(mode);
        self.state.connect(mode);
    }

    pub fn begin_generation(&mut self, generation: u64) {
        self.state.bms.begin_generation(generation);
        self.mutations = None;
        self.configuration = ConfigurationService::default();
    }

    pub fn configure_mutations(&mut self, generation: u64, writable: bool) {
        self.mutations = Some(MutationCoordinator::new(generation, writable));
    }

    pub fn ready(&mut self) {
        self.state.ready();
    }

    pub fn disconnected(&mut self) {
        if let Some(mutations) = self.mutations.as_mut() {
            mutations.cancel();
        }
        self.configuration = ConfigurationService::default();
        self.state.disconnect();
    }

    pub fn observe_telemetry(&mut self, value: &Telemetry, elapsed_millis: u64) -> bool {
        self.state.observe_telemetry(value.ride_state);
        if let Some(mutations) = self.mutations.as_mut() {
            mutations.observe(
                value.package_state,
                value.package_mode,
                None,
                value.fatal_error_latched,
                value
                    .fatal_error_latched
                    .map(|latched| latched && value.firmware_fault.is_some()),
            );
        }
        self.tracking.persists_sessions()
            && self
                .sampling
                .should_persist(value.ride_state, elapsed_millis)
    }

    pub fn select_bms(&mut self, generation: u64, connection: BmsConnection) -> bool {
        self.state.bms.select(generation, connection)
    }

    pub fn accept_bms(&mut self, generation: u64, snapshot: BmsSnapshot) -> bool {
        let accepted = self.state.bms.accept(generation, snapshot);
        if accepted {
            self.state.revision = self.state.revision.saturating_add(1);
        }
        accepted
    }

    #[must_use]
    pub const fn ride_state(&self) -> RideState {
        self.state.ride_state
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use float_view_refloat::{
        AlertReason, FootpadState, PackageMode, PackageState, SetpointAdjustment, StopCondition,
    };

    use super::*;

    fn telemetry(state: RideState) -> Telemetry {
        Telemetry {
            ride_state: state,
            package_state: PackageState::Ready,
            package_mode: PackageMode::Normal,
            footpad_state: FootpadState::None,
            stop_condition: StopCondition::None,
            setpoint_adjustment: SetpointAdjustment::None,
            wheelslip: false,
            darkride: false,
            charging: false,
            alert_reason: AlertReason::None,
            firmware_fault: None,
            fatal_error_latched: Some(false),
            numeric: BTreeMap::new(),
            extensions: BTreeMap::new(),
        }
    }

    #[test]
    fn authoritative_mapping_keeps_guest_live_but_non_persistent() {
        let mut model = ApplicationModel::default();
        model.connect(TrackingMode::Guest);
        model.begin_generation(1);
        model.ready();
        assert!(!model.observe_telemetry(&telemetry(RideState::Riding), 0));
        assert_eq!(model.state.telemetry_sequence, 1);
        assert!(!model.state.session_active);

        model.connect(TrackingMode::Saved);
        model.ready();
        assert!(model.observe_telemetry(&telemetry(RideState::Riding), 100));
        assert!(model.state.session_active);
    }
}
