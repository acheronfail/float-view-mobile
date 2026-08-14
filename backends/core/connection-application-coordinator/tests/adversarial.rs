mod support;

use float_view_connection_application_coordinator::{
    Command, ConnectionApplicationCoordinator, CoordinatorError, Diagnostic, Effect, Phase,
    TimerToken,
};
use float_view_refloat_telemetry_service::{Diagnostic as TelemetryDiagnostic, Settings};
use float_view_vesc_bms_telemetry_service::Status as BmsStatus;

use support::*;

fn ready_legacy() -> (ConnectionApplicationCoordinator, Vec<Effect>) {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_payload(42), 20);
    receive(&mut coordinator, legacy_info_payload(), 30);
    let activation = activate(&mut coordinator, 40);
    (coordinator, activation)
}

fn ready_standalone() -> (ConnectionApplicationCoordinator, Vec<Effect>) {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    let effects = receive(&mut coordinator, stock_bms_payload(), 20);
    (coordinator, effects)
}

#[test]
fn activation_is_generation_qualified_and_exactly_once() {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_payload(42), 20);
    receive(&mut coordinator, legacy_info_payload(), 30);
    let before = coordinator.snapshot();
    let stale = coordinator
        .apply(Command::Activate {
            generation: stale_generation(before.generation),
            now_ms: 40,
            session_started: true,
            cached_configuration_schema: None,
        })
        .unwrap();
    assert_eq!(stale, [Effect::Diagnostic(Diagnostic::StaleEvent)]);
    assert_eq!(coordinator.snapshot(), before);
    activate(&mut coordinator, 50);
    let active = coordinator.snapshot();
    assert_eq!(active.phase, Phase::Ready);
    let repeated = coordinator
        .apply(Command::Activate {
            generation: active.generation,
            now_ms: 60,
            session_started: true,
            cached_configuration_schema: None,
        })
        .unwrap();
    assert_eq!(repeated, [Effect::Diagnostic(Diagnostic::StaleEvent)]);
    assert_eq!(coordinator.snapshot(), active);
}

#[test]
fn malformed_telemetry_clears_only_the_pending_sample_and_polling_recovers() {
    let (mut coordinator, activation) = ready_legacy();
    assert!(activation.iter().any(|effect| matches!(
        effect,
        Effect::ReplaceTimer {
            token: TimerToken::Refloat(_),
            ..
        }
    )));
    let malformed = receive(&mut coordinator, vec![36, 101, 10], 50);
    assert!(malformed.iter().any(|effect| matches!(
        effect,
        Effect::Diagnostic(Diagnostic::Telemetry(
            TelemetryDiagnostic::MalformedTelemetry
        ))
    )));
    assert_eq!(coordinator.snapshot().telemetry.pending_request, None);
    let retry = coordinator
        .apply(Command::Refresh {
            generation: coordinator.snapshot().generation,
            now_ms: 60,
        })
        .unwrap();
    assert!(!retry.iter().any(|effect| matches!(
        effect,
        Effect::Diagnostic(Diagnostic::Telemetry(
            TelemetryDiagnostic::UnexpectedTelemetryResponse
        ))
    )));
    assert_eq!(
        coordinator.snapshot().telemetry.pending_request,
        Some(float_view_refloat_telemetry_service::RequestKind::Legacy)
    );
}

#[test]
fn three_bms_timeouts_become_unavailable_and_valid_response_recovers() {
    let (mut coordinator, mut request) = ready_standalone();
    let mut now = 1_020;
    for failure in 1..=3 {
        let timeout = coordinator
            .apply(Command::DeadlineFired {
                generation: coordinator.snapshot().generation,
                token: deadline(&request),
                now_ms: now,
            })
            .unwrap();
        assert_eq!(coordinator.snapshot().bms.consecutive_failures, failure);
        let timer = timeout
            .iter()
            .find_map(|effect| match effect {
                Effect::ReplaceTimer {
                    token: TimerToken::Bms(token),
                    ..
                } => Some(*token),
                _ => None,
            })
            .expect("replacement timer");
        now += 1_000;
        request = coordinator
            .apply(Command::TimerFired {
                generation: coordinator.snapshot().generation,
                token: TimerToken::Bms(timer),
                now_ms: now,
            })
            .unwrap();
        now += 1_000;
    }
    assert_eq!(coordinator.snapshot().bms.status, BmsStatus::Unavailable);
    let recovered = receive(&mut coordinator, bms_values_payload(), now);
    assert!(
        recovered
            .iter()
            .any(|effect| matches!(effect, Effect::PublishBmsObservation(_)))
    );
    assert_eq!(coordinator.snapshot().bms.status, BmsStatus::Attached);
    assert_eq!(coordinator.snapshot().bms.consecutive_failures, 0);
}

#[test]
fn reset_cancels_current_work_and_invalidates_deadline_and_timer_tokens() {
    let (mut coordinator, activation) = ready_legacy();
    let old_generation = coordinator.snapshot().generation;
    let old_deadline = deadline(&activation);
    let old_timer = activation
        .iter()
        .find_map(|effect| match effect {
            Effect::ReplaceTimer { token, .. } => Some(*token),
            _ => None,
        })
        .unwrap();
    let reset = coordinator.apply(Command::Reset { now_ms: 50 }).unwrap();
    assert!(
        reset
            .iter()
            .any(|effect| matches!(effect, Effect::CancelDeadline(_)))
    );
    assert!(
        reset
            .iter()
            .any(|effect| matches!(effect, Effect::CancelTimer(_)))
    );
    let snapshot = coordinator.snapshot();
    assert_eq!(snapshot.phase, Phase::Disconnected);
    assert_ne!(snapshot.generation, old_generation);
    let before = snapshot.clone();
    assert_eq!(
        coordinator
            .apply(Command::DeadlineFired {
                generation: old_generation,
                token: old_deadline,
                now_ms: 60,
            })
            .unwrap(),
        [Effect::Diagnostic(Diagnostic::StaleEvent)]
    );
    assert_eq!(
        coordinator
            .apply(Command::TimerFired {
                generation: old_generation,
                token: old_timer,
                now_ms: 60,
            })
            .unwrap(),
        [Effect::Diagnostic(Diagnostic::StaleEvent)]
    );
    assert_eq!(coordinator.snapshot(), before);
}

#[test]
fn malformed_and_overflowing_public_input_is_atomic() {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    let before = coordinator.snapshot();
    assert_eq!(
        coordinator.apply(Command::Receive {
            generation: before.generation,
            payload: Vec::new(),
            now_ms: 20,
            observed_at_epoch_ms: 1,
        }),
        Err(CoordinatorError::InvalidPayload)
    );
    assert_eq!(coordinator.snapshot(), before);
    assert_eq!(
        coordinator.apply(Command::Reset { now_ms: u64::MAX }),
        Err(CoordinatorError::InvalidMonotonicTime)
    );
    assert_eq!(coordinator.snapshot(), before);
    assert_eq!(
        coordinator.apply(Command::Receive {
            generation: before.generation,
            payload: vec![0],
            now_ms: 20,
            observed_at_epoch_ms: 0,
        }),
        Err(CoordinatorError::InvalidObservationTime)
    );
    assert_eq!(coordinator.snapshot(), before);
}

#[test]
fn maximum_can_inventory_fails_boundedly_without_more_probes() {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, gateway_payload(1), 20);
    let mut inventory = vec![62];
    inventory.extend(0_u8..=64);
    let failed = receive(&mut coordinator, inventory, 30);
    assert_eq!(coordinator.snapshot().phase, Phase::Failed);
    assert!(
        !failed
            .iter()
            .any(|effect| matches!(effect, Effect::Write(_)))
    );
}

#[test]
fn settings_validation_and_stale_updates_do_not_partially_mutate() {
    let (mut coordinator, _) = ready_legacy();
    let before = coordinator.snapshot();
    let invalid = Settings {
        idle_interval_ms: 0,
        post_ride_tail_ms: 0,
    };
    assert!(matches!(
        coordinator.apply(Command::UpdateSettings {
            generation: before.generation,
            settings: invalid,
            now_ms: 50,
        }),
        Err(CoordinatorError::Telemetry(_))
    ));
    assert_eq!(coordinator.snapshot(), before);
    let stale = coordinator
        .apply(Command::UpdateSettings {
            generation: stale_generation(before.generation),
            settings: Settings::new(2_000, 10_000).unwrap(),
            now_ms: 50,
        })
        .unwrap();
    assert_eq!(stale, [Effect::Diagnostic(Diagnostic::StaleEvent)]);
    assert_eq!(coordinator.snapshot(), before);
}

#[test]
fn private_bms_values_and_identity_do_not_appear_in_debug_output() {
    let (mut coordinator, _) = ready_standalone();
    let effects = receive(&mut coordinator, bms_values_payload(), 30);
    let debug = format!("{coordinator:?} {effects:?}");
    assert!(!debug.contains("Operational"));
    assert!(!debug.contains("THOR300"));
    assert!(!debug.contains("50.4"));
    assert!(debug.contains("[redacted]"));
}

#[test]
fn every_emitted_request_is_read_only_and_bounded() {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    let mut effects = coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    effects.extend(receive(&mut coordinator, gateway_payload(1), 20));
    effects.extend(receive(&mut coordinator, vec![62, 42], 30));
    effects.extend(receive(&mut coordinator, motor_payload(42), 40));
    effects.extend(receive(&mut coordinator, legacy_info_payload(), 50));
    effects.extend(activate(&mut coordinator, 60));
    let writes = effects.iter().filter_map(|effect| match effect {
        Effect::Write(request) => Some(&request.payload),
        _ => None,
    });
    for payload in writes {
        assert!(payload.len() <= 7);
        let command = if payload.first() == Some(&34) {
            payload[2]
        } else {
            payload[0]
        };
        assert!(
            matches!(command, 0 | 36 | 62 | 96),
            "mutating command {command}"
        );
    }
}
