//! Bounded evidence for a possible CAN address conflict, never a diagnosis.
use crate::scheduler::{Route, ScheduledRequest};
use std::collections::VecDeque;

const RECENT_LIMIT: usize = 8;
const PAYLOAD_LIMIT: usize = 8_192;
const DUPLICATE_WINDOW_MS: u64 = 1_500;
const EVIDENCE_WINDOW_MS: u64 = 10_000;

pub const WARNING: &str = "Duplicate CAN replies may prevent live values and Refloat configuration from loading. Check that your Bluetooth gateway, motor controller, and every receiver/accessory use different CAN IDs, then reconnect.";

#[derive(Default)]
pub struct ReplyHealth {
    recent: VecDeque<Reply>,
    evidence: VecDeque<u64>,
    pub warned: bool,
}

struct Reply {
    payload: Vec<u8>,
    at_ms: u64,
    counted: bool,
}

impl ReplyHealth {
    // Only solicited, non-streaming, read-only responses on a known CAN route.
    pub fn accepted(&mut self, request: &ScheduledRequest, payload: &[u8], now_ms: u64) {
        if self.warned
            || !matches!(request.context.route(), Some(Route::Can(_)))
            || !matches!(payload.first(), Some(0 | 14 | 50 | 92 | 93 | 94))
            || payload.len() > PAYLOAD_LIMIT
        {
            return;
        }
        if self.recent.len() == RECENT_LIMIT {
            self.recent.pop_front();
        }
        self.recent.push_back(Reply {
            payload: payload.to_vec(),
            at_ms: now_ms,
            counted: false,
        });
    }

    /// Called only for replies that cannot satisfy the current request. Count
    /// at most once per accepted response: one noisy packet cannot trip this.
    pub fn unexpected(&mut self, payload: &[u8], now_ms: u64) -> bool {
        if self.warned {
            return false;
        }
        let Some(reply) = self.recent.iter_mut().rev().find(|r| {
            !r.counted
                && now_ms.saturating_sub(r.at_ms) <= DUPLICATE_WINDOW_MS
                && r.payload == payload
        }) else {
            return false;
        };
        reply.counted = true;
        while self
            .evidence
            .front()
            .is_some_and(|at| now_ms.saturating_sub(*at) > EVIDENCE_WINDOW_MS)
        {
            self.evidence.pop_front();
        }
        self.evidence.push_back(now_ms);
        self.warned = self.evidence.len() >= 3;
        self.warned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{ExpectedResponse, Generation, Priority, RequestContext};

    fn request(route: Route) -> ScheduledRequest {
        ScheduledRequest {
            enqueued_at_ms: 0,
            dispatched_at_ms: Some(0),
            sequence: 0,
            payload: vec![0],
            expected_response: ExpectedResponse {
                command: 0,
                refloat_command: None,
            },
            priority: Priority::ControllerIdentification,
            context: RequestContext::ControllerFirmware(route),
            deadline_monotonic_ms: 1000,
            generation: Generation(0),
        }
    }

    #[test]
    fn direct_unsolicited_expired_and_streamed_replies_do_not_warn() {
        let mut health = ReplyHealth::default();
        for time in 0..3 {
            health.accepted(&request(Route::Direct), &[0, 1], time);
            assert!(!health.unexpected(&[0, 1], time + 1));
            health.accepted(&request(Route::Can(1)), &[4, 1], time);
            assert!(!health.unexpected(&[4, 1], time + 1));
            assert!(!health.unexpected(&[0, 9], time + 1));
        }
        health.accepted(&request(Route::Can(1)), &[0, 1], 10);
        assert!(!health.unexpected(&[0, 1], 1511));
        assert!(!health.warned);
    }

    #[test]
    fn evidence_expires_and_memory_is_bounded() {
        let mut health = ReplyHealth::default();
        for time in [0, 6000, 12000] {
            health.accepted(&request(Route::Can(1)), &[0, 1], time);
            assert!(!health.unexpected(&[0, 1], time + 1));
        }
        for time in 12001..12100 {
            health.accepted(&request(Route::Can(1)), &[0, 1], time);
        }
        assert_eq!(health.recent.len(), RECENT_LIMIT);
        health.accepted(&request(Route::Can(1)), &vec![0; PAYLOAD_LIMIT + 1], 12100);
        assert_eq!(health.recent.len(), RECENT_LIMIT);
        assert_eq!(health.evidence.len(), 2);
    }
}
