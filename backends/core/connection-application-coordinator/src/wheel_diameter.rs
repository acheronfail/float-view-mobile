use super::*;
use controller_data::wheel_diameter as wire;
use scheduler::WheelDiameterStage as Stage;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WheelDiameter {
    pub current_mm: Option<f64>,
    pub supported: bool,
    pub pending: bool,
    pub reading: bool,
    pub outcome: Option<bool>,
}
#[derive(Default)]
pub(super) struct Owner {
    pub snapshot: WheelDiameter,
    pub token: Option<mutation::MutationToken>,
    expected_mm: f64,
    diameter_mm: f64,
    pub(super) written: Vec<u8>,
}
impl ConnectionApplicationCoordinator {
    pub(super) fn read_wheel_diameter(
        &mut self,
        generation: Generation,
        now: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        if generation != self.generation
            || self.phase != Phase::Ready
            || self.motor.snapshot.pending
            || self.motor.snapshot.reading
        {
            return Err(CoordinatorError::InvalidPayload);
        }
        let route = self
            .mutations
            .snapshot()
            .route
            .ok_or(CoordinatorError::InvalidPayload)?;
        self.motor.snapshot.reading = true;
        let mut effects = Vec::new();
        if let Err(error) = self.wheel_request(
            route,
            Stage::ReadCurrent,
            vec![wire::GET],
            now,
            &mut effects,
        ) {
            self.motor.snapshot.reading = false;
            return Err(error);
        }
        self.publish_state(&mut effects)?;
        Ok(effects)
    }
    pub(super) fn wheel_diameter_authorized(&mut self, now: u64) -> bool {
        self.motor
            .token
            .is_some_and(|t| self.mutations.wheel_diameter_lease_valid(t))
            && self
                .last_telemetry_ms
                .is_some_and(|at| now.saturating_sub(at) <= 1_500)
            && self.require_inactive_imu_timeout(now).is_ok()
    }
    pub(super) fn begin_wheel_diameter(
        &mut self,
        generation: Generation,
        expected_mm: f64,
        diameter_mm: f64,
        now: u64,
    ) -> Result<Vec<Effect>, CoordinatorError> {
        if generation != self.generation
            || self.phase != Phase::Ready
            || !self.motor.snapshot.supported
            || !wire::valid(expected_mm)
            || !wire::valid(diameter_mm)
        {
            return Err(CoordinatorError::InvalidPayload);
        }
        let child = self
            .mutations
            .apply(mutation::Command::BeginConfiguration {
                generation: self.scheduler.snapshot().generation,
                mutation: mutation::ConfigurationMutation::WheelDiameter,
            })?;
        let mut effects = Vec::new();
        self.process_mutation(child, now, &mut effects)?;
        self.motor.expected_mm = expected_mm;
        self.motor.diameter_mm = diameter_mm;
        self.motor.snapshot.pending = true;
        self.motor.snapshot.outcome = None;
        let route = self
            .mutations
            .snapshot()
            .route
            .ok_or(CoordinatorError::InvalidPayload)?;
        self.wheel_request(route, Stage::ReadBase, vec![wire::GET], now, &mut effects)?;
        self.publish_state(&mut effects)?;
        Ok(effects)
    }
    fn wheel_request(
        &mut self,
        route: scheduler::Route,
        stage: Stage,
        payload: Vec<u8>,
        now: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        let command = payload[0];
        let result = self.enqueue(
            scheduler::RequestFacts {
                payload: routed_scheduler_payload(route, &payload)?,
                expected_response: scheduler::ExpectedResponse {
                    command,
                    refloat_command: None,
                },
                priority: scheduler::Priority::ControllerWork,
                context: scheduler::RequestContext::WheelDiameter { route, stage },
                timeout_ms: 2_000,
                generation: self.scheduler.snapshot().generation,
                expects_response: true,
            },
            now,
            effects,
        );
        if result.is_err() {
            self.motor.snapshot.pending = false;
            self.motor.snapshot.outcome = Some(false);
            self.motor.written.clear();
            if let Some(token) = self.motor.token.take() {
                let child = self.mutations.apply(mutation::Command::Fail {
                    token,
                    reason: mutation::FailureReason::RequestFailed,
                })?;
                self.process_mutation(child, now, effects)?;
            }
        }
        result
    }
    pub(super) fn complete_wheel_diameter(
        &mut self,
        route: scheduler::Route,
        stage: Stage,
        outcome: scheduler::Completion,
        now: u64,
        effects: &mut Vec<Effect>,
    ) -> Result<(), CoordinatorError> {
        if stage == Stage::ReadCurrent {
            self.motor.snapshot.reading = false;
            self.motor.snapshot.current_mm = None;
            self.motor.snapshot.supported = false;
            if let scheduler::Completion::Response(payload) = outcome {
                self.motor.snapshot.current_mm = wire::diameter_mm(&payload);
                self.motor.snapshot.supported = self
                    .discovery
                    .snapshot()
                    .selected
                    .as_ref()
                    .is_some_and(|s| {
                        wire::supported_for(&payload, s.identity.major, s.identity.minor)
                    });
            }
            return self.publish_state(effects);
        }
        let Some(token) = self.motor.token else {
            return Ok(());
        };
        if let scheduler::Completion::Response(payload) = outcome {
            let supported = self
                .discovery
                .snapshot()
                .selected
                .as_ref()
                .is_some_and(|s| wire::supported_for(&payload, s.identity.major, s.identity.minor));
            if stage == Stage::ReadBase && supported && self.wheel_diameter_authorized(now) {
                if let Ok(patched) =
                    wire::patch(&payload, self.motor.expected_mm, self.motor.diameter_mm)
                {
                    self.motor.written = patched.clone();
                    return self.wheel_request(route, Stage::Write, patched, now, effects);
                }
            } else if stage == Stage::Write && payload == [wire::SET] {
                return self.wheel_request(route, Stage::Verify, vec![wire::GET], now, effects);
            } else if stage == Stage::Verify && wire::verified(&self.motor.written, &payload) {
                self.motor.snapshot.current_mm = wire::diameter_mm(&payload);
                self.motor.snapshot.pending = false;
                self.motor.snapshot.outcome = Some(true);
                self.motor.token = None;
                let child = self
                    .mutations
                    .apply(mutation::Command::CompleteConfiguration { token })?;
                self.process_mutation(child, now, effects)?;
                self.motor.written.clear();
                return self.publish_state(effects);
            }
        }
        self.motor.snapshot.pending = false;
        self.motor.snapshot.outcome = Some(false);
        self.motor.token = None;
        self.motor.written.clear();
        let child = self.mutations.apply(mutation::Command::Fail {
            token,
            reason: mutation::FailureReason::RequestFailed,
        })?;
        self.process_mutation(child, now, effects)?;
        self.publish_state(effects)
    }
}
