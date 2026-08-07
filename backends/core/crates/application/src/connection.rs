use float_view_domain::RideState;
use float_view_vesc::{ExpectedResponse, FrameStreamDecoder, ProtocolError, response_key};

use crate::ControllerDiscovery;

pub const MAX_VESC_REQUESTS: usize = 64;
pub const MAX_VESC_PAYLOAD_BYTES: usize = 64 * 1024;
pub const MAX_VESC_QUEUE_BYTES: usize = 1024 * 1024;
pub const RIDING_POLL_MILLIS: u64 = 100;
pub const COMPACT_POLL_MILLIS: u64 = 1_000;
pub const REQUEST_TIMEOUT_MILLIS: u64 = 900;
pub const LAYOUT_RETRY_MILLIS: u64 = 1_000;
pub const MAX_LAYOUT_ATTEMPTS: u8 = 5;
pub const MAX_RECONNECT_BACKOFF_MILLIS: u64 = 30_000;
pub const DEFAULT_RECONNECT_DURATION_MILLIS: u64 = 300_000;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RequestPriority {
    Auxiliary,
    ControllerWork,
    RidingTelemetry,
    ControllerIdentification,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestKind {
    ControllerFirmware,
    CanPing,
    AuxiliaryCanPing,
    RefloatInfo,
    RefloatLayout,
    RefloatTelemetry,
    RefloatControl,
    ControllerShutdown,
    RefloatLighting,
    CurrentLimits,
    AuxiliaryFirmware,
    BmsValues,
    RefloatConfiguration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledRequest {
    pub sequence: u64,
    pub payload: Vec<u8>,
    pub expected: ExpectedResponse,
    pub priority: RequestPriority,
    pub kind: RequestKind,
    pub deadline_millis: u64,
    pub generation: u64,
    pub expects_response: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestSpec {
    pub payload: Vec<u8>,
    pub expected: ExpectedResponse,
    pub priority: RequestPriority,
    pub kind: RequestKind,
    pub timeout_millis: u64,
    pub generation: u64,
    pub expects_response: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestCompletion {
    Response {
        request: ScheduledRequest,
        payload: Vec<u8>,
    },
    Sent(ScheduledRequest),
    TimedOut(ScheduledRequest),
    QueueFull,
    Cancelled(ScheduledRequest),
    StaleGeneration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectionAction {
    Write(ScheduledRequest),
    Completed(RequestCompletion),
    UnmatchedResponse(Vec<u8>),
}

#[derive(Debug, Default)]
pub struct VescRequestScheduler {
    generation: u64,
    next_sequence: u64,
    queued_bytes: usize,
    queue: Vec<Queued>,
    in_flight: Option<Queued>,
}

#[derive(Clone, Debug)]
struct Queued {
    request: ScheduledRequest,
    timeout_millis: u64,
}

impl VescRequestScheduler {
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn begin_generation(&mut self) -> Vec<ConnectionAction> {
        self.generation = self.generation.wrapping_add(1);
        self.queued_bytes = 0;
        let mut cancelled: Vec<_> = self
            .queue
            .drain(..)
            .map(|queued| ConnectionAction::Completed(RequestCompletion::Cancelled(queued.request)))
            .collect();
        if let Some(in_flight) = self.in_flight.take() {
            cancelled.push(ConnectionAction::Completed(RequestCompletion::Cancelled(
                in_flight.request,
            )));
        }
        cancelled
    }

    pub fn enqueue(&mut self, now_millis: u64, spec: RequestSpec) -> Vec<ConnectionAction> {
        if spec.generation != self.generation {
            return vec![ConnectionAction::Completed(
                RequestCompletion::StaleGeneration,
            )];
        }
        if spec.timeout_millis == 0 || spec.payload.len() > MAX_VESC_PAYLOAD_BYTES {
            return vec![ConnectionAction::Completed(RequestCompletion::QueueFull)];
        }
        let Some(new_bytes) = self.queued_bytes.checked_add(spec.payload.len()) else {
            return vec![ConnectionAction::Completed(RequestCompletion::QueueFull)];
        };
        if self.queue.len() >= MAX_VESC_REQUESTS || new_bytes > MAX_VESC_QUEUE_BYTES {
            return vec![ConnectionAction::Completed(RequestCompletion::QueueFull)];
        }
        self.next_sequence = self.next_sequence.wrapping_add(1);
        let request = ScheduledRequest {
            sequence: self.next_sequence,
            payload: spec.payload,
            expected: spec.expected,
            priority: spec.priority,
            kind: spec.kind,
            deadline_millis: now_millis.saturating_add(spec.timeout_millis),
            generation: spec.generation,
            expects_response: spec.expects_response,
        };
        self.queued_bytes = new_bytes;
        self.queue.push(Queued {
            request,
            timeout_millis: spec.timeout_millis,
        });
        self.pump(now_millis)
    }

    pub fn receive(&mut self, now_millis: u64, payload: Vec<u8>) -> Vec<ConnectionAction> {
        let Some(key) = response_key(&payload) else {
            return vec![ConnectionAction::UnmatchedResponse(payload)];
        };
        let Some(current) = self.in_flight.as_ref() else {
            return vec![ConnectionAction::UnmatchedResponse(payload)];
        };
        if current.request.generation != self.generation || current.request.expected != key {
            return vec![ConnectionAction::UnmatchedResponse(payload)];
        }
        let Some(completed) = self.in_flight.take() else {
            return vec![ConnectionAction::UnmatchedResponse(payload)];
        };
        let mut actions = vec![ConnectionAction::Completed(RequestCompletion::Response {
            request: completed.request,
            payload,
        })];
        actions.extend(self.pump(now_millis));
        actions
    }

    pub fn tick(&mut self, now_millis: u64) -> Vec<ConnectionAction> {
        let Some(current) = self.in_flight.as_ref() else {
            return self.pump(now_millis);
        };
        if current.request.deadline_millis > now_millis {
            return Vec::new();
        }
        let Some(timed_out) = self.in_flight.take() else {
            return Vec::new();
        };
        let mut actions = vec![ConnectionAction::Completed(RequestCompletion::TimedOut(
            timed_out.request,
        ))];
        actions.extend(self.pump(now_millis));
        actions
    }

    fn pump(&mut self, now_millis: u64) -> Vec<ConnectionAction> {
        if self.in_flight.is_some() || self.queue.is_empty() {
            return Vec::new();
        }
        let index = self
            .queue
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                left.request
                    .priority
                    .cmp(&right.request.priority)
                    .then_with(|| right.request.sequence.cmp(&left.request.sequence))
            })
            .map_or(0, |(index, _)| index);
        let mut queued = self.queue.remove(index);
        self.queued_bytes -= queued.request.payload.len();
        if queued.request.generation != self.generation {
            return vec![ConnectionAction::Completed(
                RequestCompletion::StaleGeneration,
            )];
        }
        queued.request.deadline_millis = now_millis.saturating_add(queued.timeout_millis);
        let request = queued.request.clone();
        if request.expects_response {
            self.in_flight = Some(queued);
            vec![ConnectionAction::Write(request)]
        } else {
            let sent = request.clone();
            let mut actions = vec![
                ConnectionAction::Write(request),
                ConnectionAction::Completed(RequestCompletion::Sent(sent)),
            ];
            actions.extend(self.pump(now_millis));
            actions
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryFamily {
    Legacy,
    Dynamic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PollRequest {
    Layout,
    Realtime,
    Compact,
}

#[derive(Debug)]
pub struct TelemetryPollingModel {
    family: TelemetryFamily,
    idle_interval_millis: u64,
    post_ride_tail_millis: u64,
    previous_ride_state: RideState,
    post_ride_until: Option<u64>,
    next_compact_due: Option<u64>,
    layout_attempts: u8,
}

impl TelemetryPollingModel {
    #[must_use]
    pub const fn new(
        family: TelemetryFamily,
        idle_interval_millis: u64,
        post_ride_tail_millis: u64,
    ) -> Self {
        Self {
            family,
            idle_interval_millis,
            post_ride_tail_millis,
            previous_ride_state: RideState::Unknown,
            post_ride_until: None,
            next_compact_due: None,
            layout_attempts: 0,
        }
    }

    #[must_use]
    pub fn begin(&mut self, _now_millis: u64) -> PollRequest {
        if self.family == TelemetryFamily::Dynamic {
            self.layout_attempts = 1;
            PollRequest::Layout
        } else {
            self.next_compact_due = None;
            PollRequest::Compact
        }
    }

    #[must_use]
    pub fn retry_layout(&mut self) -> Option<PollRequest> {
        if self.family != TelemetryFamily::Dynamic || self.layout_attempts >= MAX_LAYOUT_ATTEMPTS {
            return None;
        }
        self.layout_attempts += 1;
        Some(PollRequest::Layout)
    }

    pub fn layout_established(&mut self, now_millis: u64) {
        self.layout_attempts = 0;
        self.next_compact_due = Some(now_millis.saturating_add(COMPACT_POLL_MILLIS));
    }

    #[must_use]
    pub fn next_request(&mut self, now_millis: u64) -> PollRequest {
        if self.family == TelemetryFamily::Legacy {
            return PollRequest::Compact;
        }
        if self.next_compact_due.is_some_and(|due| now_millis >= due) {
            self.next_compact_due = Some(now_millis.saturating_add(COMPACT_POLL_MILLIS));
            PollRequest::Compact
        } else {
            PollRequest::Realtime
        }
    }

    #[must_use]
    pub fn accept_ride_state(&mut self, state: RideState, now_millis: u64) -> u64 {
        if state == RideState::Riding {
            self.post_ride_until = None;
        } else if self.previous_ride_state == RideState::Riding {
            self.post_ride_until = Some(now_millis.saturating_add(self.post_ride_tail_millis));
        }
        let fast = state == RideState::Riding
            || self
                .post_ride_until
                .is_some_and(|until| now_millis <= until);
        self.previous_ride_state = state;
        if fast {
            RIDING_POLL_MILLIS
        } else {
            self.idle_interval_millis
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconnectDecision {
    ScheduleAfter(u64),
    ConnectNow,
    TimedOut,
    Disabled,
}

#[derive(Debug)]
pub struct ReconnectModel {
    duration_millis: u64,
    started_at: Option<u64>,
    attempt: u8,
}

impl Default for ReconnectModel {
    fn default() -> Self {
        Self::new(DEFAULT_RECONNECT_DURATION_MILLIS)
    }
}

impl ReconnectModel {
    #[must_use]
    pub const fn new(duration_millis: u64) -> Self {
        Self {
            duration_millis,
            started_at: None,
            attempt: 0,
        }
    }

    pub fn begin(&mut self, now_millis: u64) -> ReconnectDecision {
        if self.duration_millis == 0 {
            return ReconnectDecision::Disabled;
        }
        self.started_at = Some(now_millis);
        self.attempt = 0;
        self.schedule(now_millis)
    }

    #[must_use]
    pub fn schedule(&self, now_millis: u64) -> ReconnectDecision {
        let Some(started) = self.started_at else {
            return ReconnectDecision::Disabled;
        };
        let elapsed = now_millis.saturating_sub(started);
        let remaining = self.duration_millis.saturating_sub(elapsed);
        if remaining == 0 {
            return ReconnectDecision::TimedOut;
        }
        let delay = (1_000_u64 << self.attempt.min(5))
            .min(MAX_RECONNECT_BACKOFF_MILLIS)
            .min(remaining);
        ReconnectDecision::ScheduleAfter(delay)
    }

    pub fn timer_fired(&mut self, now_millis: u64, radio_powered_on: bool) -> ReconnectDecision {
        let Some(started) = self.started_at else {
            return ReconnectDecision::Disabled;
        };
        if now_millis.saturating_sub(started) >= self.duration_millis {
            return ReconnectDecision::TimedOut;
        }
        if radio_powered_on {
            self.attempt = self.attempt.saturating_add(1);
            ReconnectDecision::ConnectNow
        } else {
            self.schedule(now_millis)
        }
    }

    pub fn cancel(&mut self) {
        self.started_at = None;
        self.attempt = 0;
    }
}

#[derive(Default)]
pub struct BluetoothStreamPolicy {
    decoder: FrameStreamDecoder,
}

impl BluetoothStreamPolicy {
    /// # Errors
    ///
    /// Rejects malformed, oversized, or CRC-invalid VESC stream data.
    pub fn append(&mut self, notification: &[u8]) -> Result<Vec<Vec<u8>>, ProtocolError> {
        self.decoder.append(notification)
    }
}

#[derive(Default)]
pub struct ConnectionCoordinator {
    pub requests: VescRequestScheduler,
    pub stream: BluetoothStreamPolicy,
    pub controller_discovery: ControllerDiscovery,
    pub auxiliary_discovery: float_view_bms::AuxiliaryDiscovery,
    pub telemetry: Option<TelemetryPollingModel>,
    pub reconnect: ReconnectModel,
}

impl ConnectionCoordinator {
    pub fn begin_generation(&mut self) -> Vec<ConnectionAction> {
        self.stream = BluetoothStreamPolicy::default();
        self.controller_discovery = ControllerDiscovery::default();
        self.auxiliary_discovery = float_view_bms::AuxiliaryDiscovery::default();
        self.telemetry = None;
        self.reconnect.cancel();
        self.requests.begin_generation()
    }

    /// # Errors
    ///
    /// Rejects malformed, oversized, or CRC-invalid VESC stream data.
    pub fn receive_notification(
        &mut self,
        now_millis: u64,
        bytes: &[u8],
    ) -> Result<Vec<ConnectionAction>, ProtocolError> {
        let mut actions = Vec::new();
        for payload in self.stream.append(bytes)? {
            actions.extend(self.requests.receive(now_millis, payload));
        }
        Ok(actions)
    }
}

#[cfg(test)]
mod tests {
    use float_view_vesc::{VescCommand, encode_frame};

    use super::*;

    fn spec(priority: RequestPriority, command: VescCommand, generation: u64) -> RequestSpec {
        RequestSpec {
            payload: vec![command as u8],
            expected: ExpectedResponse::new(command, None),
            priority,
            kind: RequestKind::ControllerFirmware,
            timeout_millis: 100,
            generation,
            expects_response: true,
        }
    }

    #[test]
    fn scheduler_has_one_write_owner_priority_fifo_timeout_and_generation_cancellation() {
        let mut scheduler = VescRequestScheduler::default();
        let generation = scheduler.begin_generation();
        assert!(generation.is_empty());
        let generation = scheduler.generation();
        let first = scheduler.enqueue(
            0,
            spec(
                RequestPriority::Auxiliary,
                VescCommand::BmsGetValues,
                generation,
            ),
        );
        assert!(matches!(first.as_slice(), [ConnectionAction::Write(_)]));
        assert!(
            scheduler
                .enqueue(
                    0,
                    spec(
                        RequestPriority::ControllerWork,
                        VescCommand::GetRealtimeValues,
                        generation
                    )
                )
                .is_empty()
        );
        assert!(
            scheduler
                .enqueue(
                    0,
                    spec(
                        RequestPriority::ControllerIdentification,
                        VescCommand::GetFirmwareVersion,
                        generation
                    )
                )
                .is_empty()
        );
        let timed_out = scheduler.tick(100);
        assert!(matches!(
            timed_out[0],
            ConnectionAction::Completed(RequestCompletion::TimedOut(_))
        ));
        let ConnectionAction::Write(next) = &timed_out[1] else {
            panic!("write");
        };
        assert_eq!(next.expected.command, VescCommand::GetFirmwareVersion);
        let cancelled = scheduler.begin_generation();
        assert_eq!(cancelled.len(), 2);
        assert!(matches!(
            scheduler
                .enqueue(
                    0,
                    spec(
                        RequestPriority::Auxiliary,
                        VescCommand::BmsGetValues,
                        generation
                    )
                )
                .as_slice(),
            [ConnectionAction::Completed(
                RequestCompletion::StaleGeneration
            )]
        ));
    }

    #[test]
    fn stream_matches_only_the_inflight_response_and_handles_chunks() {
        let mut coordinator = ConnectionCoordinator::default();
        coordinator.requests.begin_generation();
        let generation = coordinator.requests.generation();
        let actions = coordinator.requests.enqueue(
            0,
            spec(
                RequestPriority::ControllerWork,
                VescCommand::GetFirmwareVersion,
                generation,
            ),
        );
        assert!(matches!(actions.as_slice(), [ConnectionAction::Write(_)]));
        let frame = encode_frame(&[0, 6, 6]).expect("frame");
        assert!(
            coordinator
                .receive_notification(10, &frame[..2])
                .expect("chunk")
                .is_empty()
        );
        let completed = coordinator
            .receive_notification(10, &frame[2..])
            .expect("chunk");
        assert!(matches!(
            completed.as_slice(),
            [ConnectionAction::Completed(
                RequestCompletion::Response { .. }
            )]
        ));
    }

    #[test]
    fn telemetry_and_reconnect_policies_match_existing_host_constants() {
        let mut polling = TelemetryPollingModel::new(TelemetryFamily::Dynamic, 1_000, 5_000);
        assert_eq!(polling.begin(0), PollRequest::Layout);
        for _ in 1..MAX_LAYOUT_ATTEMPTS {
            assert_eq!(polling.retry_layout(), Some(PollRequest::Layout));
        }
        assert_eq!(polling.retry_layout(), None);
        polling.layout_established(0);
        assert_eq!(polling.next_request(999), PollRequest::Realtime);
        assert_eq!(polling.next_request(1_000), PollRequest::Compact);
        assert_eq!(polling.accept_ride_state(RideState::Riding, 1_000), 100);
        assert_eq!(polling.accept_ride_state(RideState::Idle, 2_000), 100);
        assert_eq!(polling.accept_ride_state(RideState::Idle, 7_001), 1_000);

        let mut reconnect = ReconnectModel::new(300_000);
        assert_eq!(reconnect.begin(0), ReconnectDecision::ScheduleAfter(1_000));
        assert_eq!(
            reconnect.timer_fired(1_000, true),
            ReconnectDecision::ConnectNow
        );
        assert_eq!(
            reconnect.schedule(1_000),
            ReconnectDecision::ScheduleAfter(2_000)
        );
        assert_eq!(
            reconnect.timer_fired(300_000, true),
            ReconnectDecision::TimedOut
        );
    }
}
