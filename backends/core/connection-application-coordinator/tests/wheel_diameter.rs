mod support;
use float_view_connection_application_coordinator::{
    Command, ConnectionApplicationCoordinator, Effect, Phase,
};
use float_view_vesc_request_scheduler::{RequestContext, WheelDiameterStage};
use support::*;

fn config() -> Vec<u8> {
    let mut raw = current_limits_payload(0x2EFD_0142, None);
    raw.resize(484, 0);
    raw[453..457].copy_from_slice(&0.28_f32.to_be_bytes());
    raw
}
fn ready(can: bool) -> ConnectionApplicationCoordinator {
    let mut c = ConnectionApplicationCoordinator::default();
    let mut facts = begin_facts(10);
    if can {
        facts.preferred_controller_can_id = Some(42);
    }
    c.apply(Command::Begin(facts)).unwrap();
    if can {
        receive(&mut c, gateway_payload(1), 20);
        receive_can_motor(&mut c, motor_payload(42), 42, 30);
    } else {
        receive(&mut c, motor_payload(42), 20);
    }
    receive(&mut c, legacy_info_payload(), 40);
    assert_eq!(c.snapshot().phase, Phase::AwaitingActivation);
    activate(&mut c, 50);
    receive(&mut c, config(), 60);
    let mut idle = legacy_idle_payload();
    idle[10] = 14;
    receive(&mut c, idle, 70);
    if c.snapshot()
        .scheduler
        .in_flight
        .as_ref()
        .is_some_and(|r| r.expected_response.command == 62)
    {
        receive(&mut c, vec![62], 75);
    }
    c
}
fn apply(c: &mut ConnectionApplicationCoordinator, expected: f64) -> Vec<Effect> {
    c.apply(Command::ApplyWheelDiameter {
        generation: c.snapshot().generation,
        expected_mm: expected,
        diameter_mm: 268.8,
        now_ms: 80,
    })
    .unwrap()
}
fn pending_motor(c: &ConnectionApplicationCoordinator, stage: WheelDiameterStage) -> bool {
    c.snapshot()
        .scheduler
        .in_flight
        .as_ref()
        .is_some_and(|r| matches!(r.context,RequestContext::WheelDiameter{stage:s,..} if s==stage))
}
#[test]
fn full_write_preserves_bytes_and_completes_only_after_readback_on_both_routes() {
    for can in [false, true] {
        let mut c = ready(can);
        let effects = apply(&mut c, 280.);
        let request = write(&effects);
        assert_eq!(
            request.payload,
            if can { vec![34, 42, 14] } else { vec![14] }
        );
        assert!(pending_motor(&c, WheelDiameterStage::ReadBase));
        let effects = receive(&mut c, config(), 90);
        let wire = write(&effects).payload.clone();
        let offset = if can { 2 } else { 0 };
        let mut expected = config();
        expected[0] = 13;
        expected[453..457].copy_from_slice(&0.2688_f32.to_be_bytes());
        assert_eq!(&wire[offset..], expected);
        assert!(c.snapshot().wheel_diameter.pending);
        receive(&mut c, vec![13], 100);
        assert!(pending_motor(&c, WheelDiameterStage::Verify));
        let mut readback = expected;
        readback[0] = 14;
        receive(&mut c, readback, 110);
        assert_eq!(c.snapshot().wheel_diameter.outcome, Some(true));
        assert!(c.snapshot().mutations.pending.is_none());
    }
}
#[test]
fn changed_base_or_unrelated_readback_changes_never_report_success() {
    let mut c = ready(false);
    apply(&mut c, 281.);
    let effects = receive(&mut c, config(), 90);
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e,Effect::Write(r) if r.payload.first()==Some(&13)))
    );
    assert_eq!(c.snapshot().wheel_diameter.outcome, Some(false));
    let mut c = ready(false);
    apply(&mut c, 280.);
    let written = write(&receive(&mut c, config(), 90)).payload.clone();
    receive(&mut c, vec![13], 100);
    let mut readback = written;
    readback[0] = 14;
    readback[40] ^= 1;
    receive(&mut c, readback, 110);
    assert_eq!(c.snapshot().wheel_diameter.outcome, Some(false));
    assert!(c.snapshot().mutations.pending.is_none());
}
#[test]
fn stale_generation_and_unknown_layout_cannot_begin_write() {
    let mut c = ready(false);
    let generation = c.snapshot().generation;
    c.apply(Command::Reset { now_ms: 80 }).unwrap();
    assert!(
        c.apply(Command::ApplyWheelDiameter {
            generation,
            expected_mm: 280.,
            diameter_mm: 269.,
            now_ms: 90
        })
        .is_err()
    );
}

#[test]
fn acknowledgement_timeout_does_not_retry_a_persistent_write() {
    let mut c = ready(false);
    apply(&mut c, 280.);
    let effects = receive(&mut c, config(), 90);
    let token = deadline(&effects);
    let result = c
        .apply(Command::DeadlineFired {
            generation: c.snapshot().generation,
            token,
            now_ms: 2_091,
        })
        .unwrap();
    assert_eq!(c.snapshot().wheel_diameter.outcome, Some(false));
    assert!(c.snapshot().mutations.pending.is_none());
    assert!(
        !result
            .iter()
            .any(|e| matches!(e,Effect::Write(r) if r.payload.first()==Some(&13)))
    );
}

#[test]
fn stale_telemetry_before_write_releases_the_reservation_without_mutation() {
    let mut c = ready(false);
    apply(&mut c, 280.);
    let result = receive(&mut c, config(), 1_700);
    assert_eq!(c.snapshot().wheel_diameter.outcome, Some(false));
    assert!(c.snapshot().mutations.pending.is_none());
    assert!(
        !result
            .iter()
            .any(|e| matches!(e,Effect::Write(r) if r.payload.first()==Some(&13)))
    );
}

#[test]
fn preparation_reads_the_current_diameter_instead_of_the_connection_cache() {
    let mut c = ready(false);
    let effects = c
        .apply(Command::ReadWheelDiameter {
            generation: c.snapshot().generation,
            now_ms: 80,
        })
        .unwrap();
    assert_eq!(write(&effects).payload, [14]);
    assert!(c.snapshot().wheel_diameter.reading);
    let mut current = config();
    current[453..457].copy_from_slice(&0.3_f32.to_be_bytes());
    receive(&mut c, current, 90);
    assert!(!c.snapshot().wheel_diameter.reading);
    assert!((c.snapshot().wheel_diameter.current_mm.unwrap() - 300.).abs() < 0.001);
    assert!(c.snapshot().mutations.pending.is_none());
}
