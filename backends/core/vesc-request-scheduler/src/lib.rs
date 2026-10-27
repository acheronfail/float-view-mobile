//! Connection-wide VESC request arbitration decisions.
//!
//! This safe candidate returns ordered work for a future Rust runtime to apply.
//! It owns no clock, timer, Bluetooth writer, callback, protocol codec, host ABI,
//! persistence, transport, or OS behavior and remains unselected by every host.

#![forbid(unsafe_code)]

use std::fmt;

pub const MAX_QUEUED_REQUESTS: usize = 64;
pub const MAX_PAYLOAD_BYTES: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Route {
    Direct,
    Can(u8),
}

impl Route {
    pub fn can(can_id: u8) -> Result<Self, SchedulerError> {
        (can_id <= 253)
            .then_some(Self::Can(can_id))
            .ok_or(SchedulerError::InvalidCanId)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Priority {
    Auxiliary,
    ControllerWork,
    RidingTelemetry,
    ControllerIdentification,
}

impl Priority {
    const fn rank(self) -> u8 {
        match self {
            Self::Auxiliary => 0,
            Self::ControllerWork => 1,
            Self::RidingTelemetry => 2,
            Self::ControllerIdentification => 3,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum RequestContext {
    ControllerFirmware(Route),
    ControllerRouteVerification(Route),
    CanPing,
    AuxiliaryCanPing,
    RefloatInfo(Route),
    RefloatLayout(Route),
    RefloatTelemetry(Route),
    RefloatControl(Route),
    ControllerShutdown(Route),
    RefloatLighting(Route),
    CurrentLimits(Route),
    AuxiliaryFirmware(Route),
    BmsValues(Route),
    RefloatConfigurationSchema {
        route: Route,
        index: u8,
        offset: usize,
        operation: u64,
    },
    RefloatConfigurationValues {
        route: Route,
        index: u8,
        defaults: bool,
        operation: u64,
    },
    RefloatConfigurationWrite {
        route: Route,
        index: u8,
        operation: u64,
    },
    RefloatConfigurationVerification {
        route: Route,
        index: u8,
        operation: u64,
    },
}

impl RequestContext {
    pub const fn route(&self) -> Option<Route> {
        match *self {
            Self::CanPing | Self::AuxiliaryCanPing => None,
            Self::ControllerFirmware(route)
            | Self::ControllerRouteVerification(route)
            | Self::RefloatInfo(route)
            | Self::RefloatLayout(route)
            | Self::RefloatTelemetry(route)
            | Self::RefloatControl(route)
            | Self::ControllerShutdown(route)
            | Self::RefloatLighting(route)
            | Self::CurrentLimits(route)
            | Self::AuxiliaryFirmware(route)
            | Self::BmsValues(route)
            | Self::RefloatConfigurationSchema { route, .. }
            | Self::RefloatConfigurationValues { route, .. }
            | Self::RefloatConfigurationWrite { route, .. }
            | Self::RefloatConfigurationVerification { route, .. } => Some(route),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Generation(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeadlineToken {
    pub sequence: u64,
    pub generation: Generation,
    nonce: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpectedResponse {
    pub command: u8,
    pub refloat_command: Option<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestFacts {
    pub payload: Vec<u8>,
    pub expected_response: ExpectedResponse,
    pub priority: Priority,
    pub context: RequestContext,
    pub timeout_ms: u64,
    pub generation: Generation,
    pub expects_response: bool,
}

impl RequestFacts {
    pub fn validate(&self) -> Result<(), SchedulerError> {
        if self.payload.len() > MAX_PAYLOAD_BYTES {
            return Err(SchedulerError::PayloadTooLarge);
        }
        if self.timeout_ms == 0 {
            return Err(SchedulerError::InvalidTimeout);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledRequest {
    pub enqueued_at_ms: u64,
    pub dispatched_at_ms: Option<u64>,
    pub sequence: u64,
    pub payload: Vec<u8>,
    pub expected_response: ExpectedResponse,
    pub priority: Priority,
    pub context: RequestContext,
    pub deadline_monotonic_ms: u64,
    pub generation: Generation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Completion {
    Response(Vec<u8>),
    Sent,
    TimedOut,
    QueueFull,
    Cancelled,
    StaleGeneration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Diagnostic {
    QueueFull,
    StaleGeneration,
    UnmatchedOrLateResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    CancelDeadline(DeadlineToken),
    ArmDeadline {
        token: DeadlineToken,
        timeout_ms: u64,
    },
    Write(ScheduledRequest),
    Complete {
        request: ScheduledRequest,
        outcome: Completion,
    },
    Diagnostic(Diagnostic),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub generation: Generation,
    pub queued_count: usize,
    pub in_flight: Option<ScheduledRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextMatch {
    Any,
    Exact(RequestContext),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CancellationSelector {
    Exact(RequestContext),
    FlywheelProhibited { preserve_refloat_control: bool },
}

impl CancellationSelector {
    fn matches(&self, context: &RequestContext) -> bool {
        match self {
            Self::Exact(expected) => context == expected,
            Self::FlywheelProhibited {
                preserve_refloat_control,
            } => match context {
                RequestContext::RefloatControl(_) => !preserve_refloat_control,
                RequestContext::ControllerShutdown(_)
                | RequestContext::RefloatConfigurationWrite { .. }
                | RequestContext::RefloatConfigurationVerification { .. } => true,
                RequestContext::RefloatConfigurationValues { defaults, .. } => !defaults,
                _ => false,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerError {
    InvalidCanId,
    PayloadTooLarge,
    InvalidTimeout,
    MonotonicOverflow,
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidCanId => "invalid_can_id",
            Self::PayloadTooLarge => "payload_too_large",
            Self::InvalidTimeout => "invalid_timeout",
            Self::MonotonicOverflow => "monotonic_overflow",
        })
    }
}

impl std::error::Error for SchedulerError {}

#[derive(Clone, Debug)]
struct Pending {
    request: ScheduledRequest,
    timeout_ms: u64,
    expects_response: bool,
}

#[derive(Debug)]
pub struct VescRequestScheduler {
    generation: Generation,
    next_sequence: u64,
    next_deadline_nonce: u64,
    queue: Vec<Pending>,
    in_flight: Option<(Pending, DeadlineToken)>,
}

impl Default for VescRequestScheduler {
    fn default() -> Self {
        Self {
            generation: Generation(0),
            next_sequence: 0,
            next_deadline_nonce: 0,
            queue: Vec::new(),
            in_flight: None,
        }
    }
}

impl VescRequestScheduler {
    /// Whether this context already has queued or in-flight work.
    pub fn has_pending(&self, context: &RequestContext) -> bool {
        self.in_flight
            .as_ref()
            .is_some_and(|(pending, _)| &pending.request.context == context)
            || self
                .queue
                .iter()
                .any(|pending| &pending.request.context == context)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            generation: self.generation,
            queued_count: self.queue.len(),
            in_flight: self.in_flight.as_ref().map(|value| value.0.request.clone()),
        }
    }

    pub fn begin_generation(&mut self, now_ms: u64) -> Result<Vec<Effect>, SchedulerError> {
        self.generation = Generation(self.generation.0.wrapping_add(1));
        let mut effects = Vec::with_capacity(self.queue.len() + 2);
        if let Some((_, token)) = &self.in_flight {
            effects.push(Effect::CancelDeadline(*token));
        }
        for pending in self.queue.drain(..) {
            effects.push(Self::completion(pending.request, Completion::Cancelled));
        }
        if let Some((pending, _)) = self.in_flight.take() {
            effects.push(Self::completion(pending.request, Completion::Cancelled));
        }
        self.pump(now_ms, &mut effects)?;
        Ok(effects)
    }

    pub fn enqueue(
        &mut self,
        facts: RequestFacts,
        now_ms: u64,
    ) -> Result<Vec<Effect>, SchedulerError> {
        facts.validate()?;
        let initial_deadline = now_ms
            .checked_add(facts.timeout_ms)
            .ok_or(SchedulerError::MonotonicOverflow)?;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        let request = ScheduledRequest {
            enqueued_at_ms: now_ms,
            dispatched_at_ms: None,
            sequence: self.next_sequence,
            payload: facts.payload,
            expected_response: facts.expected_response,
            priority: facts.priority,
            context: facts.context,
            deadline_monotonic_ms: initial_deadline,
            generation: facts.generation,
        };
        let pending = Pending {
            request,
            timeout_ms: facts.timeout_ms,
            expects_response: facts.expects_response,
        };
        let mut effects = Vec::new();
        if facts.generation != self.generation {
            effects.push(Effect::Diagnostic(Diagnostic::StaleGeneration));
            effects.push(Self::completion(
                pending.request,
                Completion::StaleGeneration,
            ));
            return Ok(effects);
        }
        if self.queue.len() >= MAX_QUEUED_REQUESTS {
            effects.push(Effect::Diagnostic(Diagnostic::QueueFull));
            effects.push(Self::completion(pending.request, Completion::QueueFull));
            return Ok(effects);
        }
        self.queue.push(pending);
        self.pump(now_ms, &mut effects)?;
        Ok(effects)
    }

    pub fn receive(
        &mut self,
        payload: Vec<u8>,
        expected: ExpectedResponse,
        context_match: &ContextMatch,
        defer_next: bool,
        now_ms: u64,
    ) -> Result<(bool, Vec<Effect>), SchedulerError> {
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(SchedulerError::PayloadTooLarge);
        }
        let matches = self.in_flight.as_ref().is_some_and(|(pending, _)| {
            pending.request.generation == self.generation
                && pending.request.expected_response == expected
                && match context_match {
                    ContextMatch::Any => true,
                    ContextMatch::Exact(context) => pending.request.context == *context,
                }
        });
        if !matches {
            return Ok((
                false,
                vec![Effect::Diagnostic(Diagnostic::UnmatchedOrLateResponse)],
            ));
        }
        let (pending, token) = self.in_flight.take().expect("matched in-flight request");
        let mut effects = vec![
            Effect::CancelDeadline(token),
            Self::completion(pending.request, Completion::Response(payload)),
        ];
        if !defer_next {
            self.pump(now_ms, &mut effects)?;
        }
        Ok((true, effects))
    }

    pub fn expire_deadline(
        &mut self,
        token: DeadlineToken,
        now_ms: u64,
    ) -> Result<Vec<Effect>, SchedulerError> {
        let Some((pending, current_token)) = self.in_flight.as_ref() else {
            return Ok(Vec::new());
        };
        if *current_token != token || pending.request.generation != self.generation {
            return Ok(Vec::new());
        }
        let (pending, _) = self
            .in_flight
            .take()
            .expect("deadline matched in-flight request");
        let mut effects = vec![Self::completion(pending.request, Completion::TimedOut)];
        self.pump(now_ms, &mut effects)?;
        Ok(effects)
    }

    pub fn cancel_matching(
        &mut self,
        selector: &CancellationSelector,
        resume: bool,
        now_ms: u64,
    ) -> Result<Vec<Effect>, SchedulerError> {
        let mut cancelled = Vec::new();
        let mut retained = Vec::with_capacity(self.queue.len());
        for pending in self.queue.drain(..) {
            if selector.matches(&pending.request.context) {
                cancelled.push(pending.request);
            } else {
                retained.push(pending);
            }
        }
        self.queue = retained;
        let mut deadline = None;
        if self
            .in_flight
            .as_ref()
            .is_some_and(|value| selector.matches(&value.0.request.context))
        {
            let (pending, token) = self
                .in_flight
                .take()
                .expect("selector matched in-flight request");
            deadline = Some(token);
            cancelled.push(pending.request);
        }
        let mut effects = Vec::with_capacity(cancelled.len() + usize::from(deadline.is_some()));
        if let Some(token) = deadline {
            effects.push(Effect::CancelDeadline(token));
        }
        effects.extend(
            cancelled
                .into_iter()
                .map(|request| Self::completion(request, Completion::Cancelled)),
        );
        if resume {
            self.pump(now_ms, &mut effects)?;
        }
        Ok(effects)
    }

    pub fn cancel_generation(
        &mut self,
        generation: Generation,
        now_ms: u64,
    ) -> Result<Vec<Effect>, SchedulerError> {
        let mut cancelled = Vec::new();
        let mut retained = Vec::with_capacity(self.queue.len());
        for pending in self.queue.drain(..) {
            if pending.request.generation == generation {
                cancelled.push(pending.request);
            } else {
                retained.push(pending);
            }
        }
        self.queue = retained;
        let mut deadline = None;
        if self
            .in_flight
            .as_ref()
            .is_some_and(|value| value.0.request.generation == generation)
        {
            let (pending, token) = self
                .in_flight
                .take()
                .expect("generation matched in-flight request");
            deadline = Some(token);
            cancelled.push(pending.request);
        }
        let mut effects = Vec::with_capacity(cancelled.len() + usize::from(deadline.is_some()));
        if let Some(token) = deadline {
            effects.push(Effect::CancelDeadline(token));
        }
        effects.extend(
            cancelled
                .into_iter()
                .map(|request| Self::completion(request, Completion::Cancelled)),
        );
        self.pump(now_ms, &mut effects)?;
        Ok(effects)
    }

    pub fn resume(&mut self, now_ms: u64) -> Result<Vec<Effect>, SchedulerError> {
        let mut effects = Vec::new();
        self.pump(now_ms, &mut effects)?;
        Ok(effects)
    }

    fn pump(&mut self, now_ms: u64, effects: &mut Vec<Effect>) -> Result<(), SchedulerError> {
        while self.in_flight.is_none() && !self.queue.is_empty() {
            let Some(index) = self
                .queue
                .iter()
                .enumerate()
                .max_by(|left, right| {
                    left.1
                        .request
                        .priority
                        .rank()
                        .cmp(&right.1.request.priority.rank())
                        .then_with(|| right.0.cmp(&left.0))
                })
                .map(|value| value.0)
            else {
                break;
            };
            let mut pending = self.queue.remove(index);
            if pending.request.generation != self.generation {
                effects.push(Self::completion(
                    pending.request,
                    Completion::StaleGeneration,
                ));
                continue;
            }
            pending.request.dispatched_at_ms = Some(now_ms);
            pending.request.deadline_monotonic_ms = now_ms
                .checked_add(pending.timeout_ms)
                .ok_or(SchedulerError::MonotonicOverflow)?;
            self.next_deadline_nonce = self.next_deadline_nonce.wrapping_add(1);
            let token = DeadlineToken {
                sequence: pending.request.sequence,
                generation: pending.request.generation,
                nonce: self.next_deadline_nonce,
            };
            effects.push(Effect::ArmDeadline {
                token,
                timeout_ms: pending.timeout_ms,
            });
            effects.push(Effect::Write(pending.request.clone()));
            if pending.expects_response {
                self.in_flight = Some((pending, token));
                break;
            }
            effects.push(Effect::CancelDeadline(token));
            effects.push(Self::completion(pending.request, Completion::Sent));
        }
        Ok(())
    }

    fn completion(request: ScheduledRequest, outcome: Completion) -> Effect {
        Effect::Complete { request, outcome }
    }

    #[cfg(test)]
    fn force_counters(&mut self, sequence: u64, generation: u64, deadline_nonce: u64) {
        self.next_sequence = sequence;
        self.generation = Generation(generation);
        self.next_deadline_nonce = deadline_nonce;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(
        byte: u8,
        priority: Priority,
        context: RequestContext,
        generation: Generation,
    ) -> RequestFacts {
        RequestFacts {
            payload: vec![byte],
            expected_response: ExpectedResponse {
                command: 1,
                refloat_command: None,
            },
            priority,
            context,
            timeout_ms: 100,
            generation,
            expects_response: true,
        }
    }

    fn writes(effects: &[Effect]) -> Vec<u8> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Write(request) => request.payload.first().copied(),
                _ => None,
            })
            .collect()
    }

    fn token(effects: &[Effect]) -> DeadlineToken {
        effects
            .iter()
            .find_map(|effect| match effect {
                Effect::ArmDeadline { token, .. } => Some(*token),
                _ => None,
            })
            .expect("deadline effect")
    }

    fn outcomes(effects: &[Effect]) -> Vec<Completion> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Complete { outcome, .. } => Some(outcome.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn prioritizes_controller_and_riding_work_without_preemption() {
        let mut scheduler = VescRequestScheduler::default();
        let generation = Generation(1);
        scheduler.begin_generation(0).unwrap();
        let first = scheduler
            .enqueue(
                facts(
                    1,
                    Priority::ControllerWork,
                    RequestContext::CurrentLimits(Route::Direct),
                    generation,
                ),
                0,
            )
            .unwrap();
        assert_eq!(writes(&first), [1]);
        scheduler
            .enqueue(
                facts(
                    2,
                    Priority::Auxiliary,
                    RequestContext::BmsValues(Route::Can(42)),
                    generation,
                ),
                0,
            )
            .unwrap();
        scheduler
            .enqueue(
                facts(
                    3,
                    Priority::RidingTelemetry,
                    RequestContext::RefloatTelemetry(Route::Direct),
                    generation,
                ),
                0,
            )
            .unwrap();
        scheduler
            .enqueue(
                facts(
                    4,
                    Priority::ControllerIdentification,
                    RequestContext::ControllerFirmware(Route::Can(7)),
                    generation,
                ),
                0,
            )
            .unwrap();
        let (_, next) = scheduler
            .receive(
                vec![1],
                ExpectedResponse {
                    command: 1,
                    refloat_command: None,
                },
                &ContextMatch::Any,
                false,
                0,
            )
            .unwrap();
        assert_eq!(writes(&next), [4]);
        let (_, next) = scheduler
            .receive(
                vec![4],
                ExpectedResponse {
                    command: 1,
                    refloat_command: None,
                },
                &ContextMatch::Any,
                false,
                0,
            )
            .unwrap();
        assert_eq!(writes(&next), [3]);
        let (_, next) = scheduler
            .receive(
                vec![3],
                ExpectedResponse {
                    command: 1,
                    refloat_command: None,
                },
                &ContextMatch::Any,
                false,
                0,
            )
            .unwrap();
        assert_eq!(writes(&next), [2]);
    }

    #[test]
    fn preserves_fifo_within_each_priority() {
        let mut scheduler = VescRequestScheduler::default();
        let generation = Generation(1);
        scheduler.begin_generation(0).unwrap();
        scheduler
            .enqueue(
                facts(0, Priority::Auxiliary, RequestContext::CanPing, generation),
                0,
            )
            .unwrap();
        for byte in 1..=3 {
            scheduler
                .enqueue(
                    facts(
                        byte,
                        Priority::ControllerWork,
                        RequestContext::RefloatInfo(Route::Direct),
                        generation,
                    ),
                    0,
                )
                .unwrap();
        }
        let mut observed = Vec::new();
        for response in [0, 1, 2] {
            let (_, effects) = scheduler
                .receive(
                    vec![response],
                    ExpectedResponse {
                        command: 1,
                        refloat_command: None,
                    },
                    &ContextMatch::Any,
                    false,
                    0,
                )
                .unwrap();
            observed.extend(writes(&effects));
        }
        assert_eq!(observed, [1, 2, 3]);
    }

    #[test]
    fn starts_deadline_when_queued_work_is_dispatched() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let generation = Generation(1);
        let mut first = facts(
            62,
            Priority::Auxiliary,
            RequestContext::AuxiliaryCanPing,
            generation,
        );
        first.timeout_ms = 10_000;
        scheduler.enqueue(first, 0).unwrap();
        let mut second = facts(
            31,
            Priority::ControllerWork,
            RequestContext::RefloatTelemetry(Route::Direct),
            generation,
        );
        second.timeout_ms = 900;
        scheduler.enqueue(second, 0).unwrap();
        let (_, effects) = scheduler
            .receive(
                vec![62],
                ExpectedResponse {
                    command: 1,
                    refloat_command: None,
                },
                &ContextMatch::Any,
                false,
                2_550,
            )
            .unwrap();
        assert_eq!(writes(&effects), [31]);
        let dispatched = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Write(request) => Some(request),
                _ => None,
            })
            .unwrap();
        assert_eq!(dispatched.enqueued_at_ms, 0);
        assert_eq!(dispatched.dispatched_at_ms, Some(2_550));
        assert_eq!(
            scheduler
                .snapshot()
                .in_flight
                .unwrap()
                .deadline_monotonic_ms,
            3_450
        );
    }

    #[test]
    fn timeout_advances_and_stale_deadline_does_nothing() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let generation = Generation(1);
        let effects = scheduler
            .enqueue(
                facts(
                    7,
                    Priority::Auxiliary,
                    RequestContext::ControllerFirmware(Route::Can(7)),
                    generation,
                ),
                0,
            )
            .unwrap();
        let first_token = token(&effects);
        scheduler
            .enqueue(
                facts(
                    42,
                    Priority::Auxiliary,
                    RequestContext::AuxiliaryFirmware(Route::Can(42)),
                    generation,
                ),
                0,
            )
            .unwrap();
        let effects = scheduler.expire_deadline(first_token, 100).unwrap();
        assert_eq!(outcomes(&effects), [Completion::TimedOut]);
        assert_eq!(writes(&effects), [42]);
        assert!(
            scheduler
                .expire_deadline(first_token, 200)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn bounds_queue_replaces_generation_and_rejects_stale_work() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let generation = Generation(1);
        scheduler
            .enqueue(
                facts(
                    0,
                    Priority::Auxiliary,
                    RequestContext::BmsValues(Route::Direct),
                    generation,
                ),
                0,
            )
            .unwrap();
        for value in 1..=MAX_QUEUED_REQUESTS {
            scheduler
                .enqueue(
                    facts(
                        value as u8,
                        Priority::Auxiliary,
                        RequestContext::BmsValues(Route::Direct),
                        generation,
                    ),
                    0,
                )
                .unwrap();
        }
        let rejected = scheduler
            .enqueue(
                facts(
                    255,
                    Priority::Auxiliary,
                    RequestContext::BmsValues(Route::Direct),
                    generation,
                ),
                0,
            )
            .unwrap();
        assert_eq!(outcomes(&rejected), [Completion::QueueFull]);
        let cancelled = scheduler.begin_generation(0).unwrap();
        assert_eq!(
            outcomes(&cancelled)
                .iter()
                .filter(|value| **value == Completion::Cancelled)
                .count(),
            65
        );
        let stale = scheduler
            .enqueue(
                facts(
                    9,
                    Priority::Auxiliary,
                    RequestContext::BmsValues(Route::Direct),
                    generation,
                ),
                0,
            )
            .unwrap();
        assert_eq!(outcomes(&stale), [Completion::StaleGeneration]);
    }

    #[test]
    fn exact_response_context_controls_attribution_and_deferred_resume() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let generation = Generation(1);
        let expected_context = RequestContext::RefloatConfigurationSchema {
            route: Route::Can(42),
            index: 0,
            offset: 400,
            operation: 7,
        };
        scheduler
            .enqueue(
                facts(
                    92,
                    Priority::ControllerWork,
                    expected_context.clone(),
                    generation,
                ),
                0,
            )
            .unwrap();
        scheduler
            .enqueue(
                facts(
                    31,
                    Priority::RidingTelemetry,
                    RequestContext::RefloatTelemetry(Route::Can(42)),
                    generation,
                ),
                0,
            )
            .unwrap();
        let (matched, effects) = scheduler
            .receive(
                vec![92],
                ExpectedResponse {
                    command: 1,
                    refloat_command: None,
                },
                &ContextMatch::Exact(RequestContext::RefloatConfigurationSchema {
                    route: Route::Can(42),
                    index: 0,
                    offset: 0,
                    operation: 7,
                }),
                false,
                0,
            )
            .unwrap();
        assert!(!matched);
        assert_eq!(
            effects,
            [Effect::Diagnostic(Diagnostic::UnmatchedOrLateResponse)]
        );
        let (matched, effects) = scheduler
            .receive(
                vec![92],
                ExpectedResponse {
                    command: 1,
                    refloat_command: None,
                },
                &ContextMatch::Exact(expected_context),
                true,
                0,
            )
            .unwrap();
        assert!(matched);
        assert!(writes(&effects).is_empty());
        assert_eq!(writes(&scheduler.resume(0).unwrap()), [31]);
    }

    #[test]
    fn one_way_requests_complete_and_chain_without_waiting() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let generation = Generation(1);
        let mut one_way = facts(
            12,
            Priority::ControllerWork,
            RequestContext::RefloatTelemetry(Route::Direct),
            generation,
        );
        one_way.expects_response = false;
        let effects = scheduler.enqueue(one_way, 0).unwrap();
        assert_eq!(writes(&effects), [12]);
        assert_eq!(outcomes(&effects), [Completion::Sent]);
        assert!(scheduler.snapshot().in_flight.is_none());
    }

    #[test]
    fn flywheel_selector_matches_only_prohibited_work() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let generation = Generation(1);
        let contexts = [
            RequestContext::RefloatControl(Route::Direct),
            RequestContext::ControllerShutdown(Route::Direct),
            RequestContext::RefloatConfigurationWrite {
                route: Route::Direct,
                index: 0,
                operation: 1,
            },
            RequestContext::RefloatConfigurationVerification {
                route: Route::Direct,
                index: 0,
                operation: 1,
            },
            RequestContext::RefloatConfigurationValues {
                route: Route::Direct,
                index: 0,
                defaults: false,
                operation: 1,
            },
            RequestContext::RefloatConfigurationValues {
                route: Route::Direct,
                index: 0,
                defaults: true,
                operation: 1,
            },
        ];
        for (index, context) in contexts.into_iter().enumerate() {
            scheduler
                .enqueue(
                    facts(index as u8, Priority::Auxiliary, context, generation),
                    0,
                )
                .unwrap();
        }
        let effects = scheduler
            .cancel_matching(
                &CancellationSelector::FlywheelProhibited {
                    preserve_refloat_control: true,
                },
                false,
                0,
            )
            .unwrap();
        assert_eq!(outcomes(&effects).len(), 4);
        assert_eq!(scheduler.snapshot().queued_count, 1);
    }

    #[test]
    fn validates_inputs_atomically() {
        assert_eq!(Route::can(254), Err(SchedulerError::InvalidCanId));
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let before = scheduler.snapshot();
        let mut invalid = facts(
            1,
            Priority::Auxiliary,
            RequestContext::CanPing,
            Generation(1),
        );
        invalid.payload = vec![0; MAX_PAYLOAD_BYTES + 1];
        assert_eq!(
            scheduler.enqueue(invalid, 0),
            Err(SchedulerError::PayloadTooLarge)
        );
        let mut invalid = facts(
            1,
            Priority::Auxiliary,
            RequestContext::CanPing,
            Generation(1),
        );
        invalid.timeout_ms = 0;
        assert_eq!(
            scheduler.enqueue(invalid, 0),
            Err(SchedulerError::InvalidTimeout)
        );
        assert_eq!(scheduler.snapshot(), before);
    }

    #[test]
    fn counters_wrap_without_reusing_deadline_identity() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.force_counters(u64::MAX, u64::MAX, u64::MAX);
        scheduler.begin_generation(0).unwrap();
        assert_eq!(scheduler.snapshot().generation, Generation(0));
        let effects = scheduler
            .enqueue(
                facts(
                    1,
                    Priority::Auxiliary,
                    RequestContext::CanPing,
                    Generation(0),
                ),
                0,
            )
            .unwrap();
        let wrapped = token(&effects);
        assert_eq!(wrapped.sequence, 0);
        assert_eq!(wrapped.nonce, 0);
    }

    #[test]
    fn every_context_family_preserves_route_and_equality() {
        let direct = Route::Direct;
        let can = Route::can(253).unwrap();
        let contexts = vec![
            RequestContext::ControllerFirmware(direct),
            RequestContext::ControllerRouteVerification(can),
            RequestContext::CanPing,
            RequestContext::AuxiliaryCanPing,
            RequestContext::RefloatInfo(can),
            RequestContext::RefloatLayout(direct),
            RequestContext::RefloatTelemetry(can),
            RequestContext::RefloatControl(direct),
            RequestContext::ControllerShutdown(can),
            RequestContext::RefloatLighting(direct),
            RequestContext::CurrentLimits(can),
            RequestContext::AuxiliaryFirmware(direct),
            RequestContext::BmsValues(can),
            RequestContext::RefloatConfigurationSchema {
                route: direct,
                index: u8::MAX,
                offset: usize::MAX,
                operation: u64::MAX,
            },
            RequestContext::RefloatConfigurationValues {
                route: can,
                index: 0,
                defaults: true,
                operation: 0,
            },
            RequestContext::RefloatConfigurationWrite {
                route: direct,
                index: 0,
                operation: 0,
            },
            RequestContext::RefloatConfigurationVerification {
                route: can,
                index: 0,
                operation: 0,
            },
        ];
        assert_eq!(
            contexts
                .iter()
                .filter(|context| context.route().is_none())
                .count(),
            2
        );
        assert_eq!(
            contexts
                .iter()
                .filter(|context| context.route().is_some())
                .count(),
            15
        );
    }

    #[test]
    fn response_identifiers_and_payload_bounds_are_exact() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let mut request = facts(
            31,
            Priority::RidingTelemetry,
            RequestContext::RefloatTelemetry(Route::Direct),
            Generation(1),
        );
        request.expected_response = ExpectedResponse {
            command: 36,
            refloat_command: Some(31),
        };
        scheduler.enqueue(request, 0).unwrap();
        for expected in [
            ExpectedResponse {
                command: 35,
                refloat_command: Some(31),
            },
            ExpectedResponse {
                command: 36,
                refloat_command: Some(10),
            },
        ] {
            assert!(
                !scheduler
                    .receive(Vec::new(), expected, &ContextMatch::Any, false, 0)
                    .unwrap()
                    .0
            );
        }
        assert_eq!(
            scheduler.receive(
                vec![0; MAX_PAYLOAD_BYTES + 1],
                ExpectedResponse {
                    command: 36,
                    refloat_command: Some(31),
                },
                &ContextMatch::Any,
                false,
                0,
            ),
            Err(SchedulerError::PayloadTooLarge)
        );
        assert!(scheduler.snapshot().in_flight.is_some());
    }

    #[test]
    fn cancellation_cancels_deadline_before_completing_and_can_defer_pump() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let generation = Generation(1);
        let first_context = RequestContext::RefloatConfigurationValues {
            route: Route::Direct,
            index: 0,
            defaults: false,
            operation: 4,
        };
        scheduler
            .enqueue(
                facts(
                    93,
                    Priority::ControllerWork,
                    first_context.clone(),
                    generation,
                ),
                0,
            )
            .unwrap();
        scheduler
            .enqueue(
                facts(
                    31,
                    Priority::RidingTelemetry,
                    RequestContext::RefloatTelemetry(Route::Direct),
                    generation,
                ),
                0,
            )
            .unwrap();
        let effects = scheduler
            .cancel_matching(&CancellationSelector::Exact(first_context), false, 0)
            .unwrap();
        assert!(matches!(effects.first(), Some(Effect::CancelDeadline(_))));
        assert_eq!(outcomes(&effects), [Completion::Cancelled]);
        assert!(writes(&effects).is_empty());
        assert_eq!(writes(&scheduler.resume(0).unwrap()), [31]);
    }

    #[test]
    fn monotonic_overflow_does_not_mutate_state() {
        let mut scheduler = VescRequestScheduler::default();
        scheduler.begin_generation(0).unwrap();
        let before = scheduler.snapshot();
        let request = facts(
            1,
            Priority::Auxiliary,
            RequestContext::CanPing,
            Generation(1),
        );
        assert_eq!(
            scheduler.enqueue(request, u64::MAX),
            Err(SchedulerError::MonotonicOverflow)
        );
        assert_eq!(scheduler.snapshot(), before);
    }
}
