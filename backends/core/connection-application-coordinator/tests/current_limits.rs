mod support;

use float_view_connection_application_coordinator::{
    Command, ConnectionApplicationCoordinator, Diagnostic, Effect, MalformedResponse, Phase,
};
use float_view_vesc_request_scheduler::Diagnostic as SchedulerDiagnostic;

use support::*;

const VECTORS: &str = include_str!("../../../fixtures/connection/controller-current-limits.tsv");

fn awaiting_direct() -> ConnectionApplicationCoordinator {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_payload(42), 20);
    receive(&mut coordinator, legacy_info_payload(), 30);
    assert_eq!(coordinator.snapshot().phase, Phase::AwaitingActivation);
    coordinator
}

fn awaiting_can() -> ConnectionApplicationCoordinator {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    let mut facts = begin_facts(10);
    facts.preferred_controller_can_id = Some(42);
    coordinator.apply(Command::Begin(facts)).unwrap();
    receive(&mut coordinator, gateway_payload(1), 20);
    receive(&mut coordinator, motor_payload(42), 30);
    receive(&mut coordinator, legacy_info_payload(), 40);
    assert_eq!(coordinator.snapshot().phase, Phase::AwaitingActivation);
    coordinator
}

#[test]
fn source_derived_vectors_cover_routes_signatures_and_standalone_exclusion() {
    let rows = VECTORS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 4);
    assert!(
        rows.iter()
            .any(|row| row.starts_with("direct-known-signature|"))
    );
    assert!(
        rows.iter()
            .any(|row| row.starts_with("direct-unknown-signature|"))
    );
    assert!(
        rows.iter()
            .any(|row| row.starts_with("can-known-signature|"))
    );
    assert!(rows.iter().any(|row| row.starts_with("standalone-bms|")));

    let mut direct = awaiting_direct();
    let direct_activation = activate(&mut direct, 40);
    assert_eq!(write(&direct_activation).payload, [14]);
    let published = receive(
        &mut direct,
        current_limits_payload(0x2EFD_0142, Some(15)),
        50,
    );
    assert!(
        published
            .iter()
            .any(|effect| matches!(effect, Effect::PublishState(_)))
    );
    let limits = direct.snapshot().current_limits.expect("current limits");
    assert_eq!(limits.motor_current_max_a, 100.0);
    assert_eq!(limits.motor_current_min_a, -100.0);
    assert_eq!(limits.battery_current_max_a, 50.0);
    assert_eq!(limits.battery_current_min_a, -20.0);
    assert_eq!(limits.battery_series_cells, Some(15));

    let mut can = awaiting_can();
    let can_activation = activate(&mut can, 50);
    assert_eq!(write(&can_activation).payload, [34, 42, 14]);
    receive(&mut can, current_limits_payload(0xDEAD_BEEF, None), 60);
    assert_eq!(
        can.snapshot()
            .current_limits
            .expect("unknown signature limits")
            .battery_series_cells,
        None
    );

    let mut bms = ConnectionApplicationCoordinator::default();
    bms.apply(Command::Begin(begin_facts(10))).unwrap();
    let effects = receive(&mut bms, stock_bms_payload(), 20);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Write(request)
        if request.payload == [14] || request.payload.ends_with(&[14])))
    );
    assert_eq!(bms.snapshot().current_limits, None);
}

#[test]
fn malformed_mismatched_timeout_repeated_and_reset_inputs_are_soft_and_isolated() {
    let mut malformed = awaiting_direct();
    activate(&mut malformed, 40);
    let before = malformed.snapshot();
    let wrong = receive(&mut malformed, vec![4; 30], 45);
    assert!(wrong.iter().any(|effect| matches!(
        effect,
        Effect::Diagnostic(Diagnostic::Scheduler(
            SchedulerDiagnostic::UnmatchedOrLateResponse
        ))
    )));
    assert_eq!(malformed.snapshot().current_limits, None);
    assert_eq!(malformed.snapshot().phase, before.phase);

    let mut zero = current_limits_payload(0x2EFD_0142, Some(15));
    zero[9..13].fill(0);
    let rejected = receive(&mut malformed, zero, 50);
    assert!(rejected.iter().any(|effect| matches!(
        effect,
        Effect::Diagnostic(Diagnostic::MalformedResponse(
            MalformedResponse::CurrentLimits
        ))
    )));
    assert_eq!(malformed.snapshot().current_limits, None);
    assert_eq!(malformed.snapshot().phase, Phase::Ready);

    let repeated = receive(
        &mut malformed,
        current_limits_payload(0x2EFD_0142, Some(15)),
        55,
    );
    assert!(repeated.iter().any(|effect| matches!(
        effect,
        Effect::Diagnostic(Diagnostic::Scheduler(
            SchedulerDiagnostic::UnmatchedOrLateResponse
        ))
    )));
    assert_eq!(malformed.snapshot().current_limits, None);

    let mut timed_out = awaiting_direct();
    let activation = activate(&mut timed_out, 40);
    timed_out
        .apply(Command::DeadlineFired {
            generation: timed_out.snapshot().generation,
            token: deadline(&activation),
            now_ms: 1_040,
        })
        .unwrap();
    assert_eq!(timed_out.snapshot().phase, Phase::Ready);
    assert_eq!(timed_out.snapshot().current_limits, None);

    let mut valid = awaiting_direct();
    activate(&mut valid, 40);
    receive(
        &mut valid,
        current_limits_payload(0x2EFD_0142, Some(15)),
        50,
    );
    assert!(valid.snapshot().current_limits.is_some());
    valid.apply(Command::Reset { now_ms: 60 }).unwrap();
    assert_eq!(valid.snapshot().current_limits, None);
    assert_eq!(valid.snapshot().phase, Phase::Disconnected);
}

#[test]
fn debug_output_discloses_presence_but_no_numeric_current_limits() {
    let mut coordinator = awaiting_direct();
    activate(&mut coordinator, 40);
    let effects = receive(
        &mut coordinator,
        current_limits_payload(0x2EFD_0142, Some(15)),
        50,
    );
    let debug = format!("{coordinator:?} {effects:?}");
    assert!(debug.contains("has_current_limits: true"));
    for private in ["100.0", "-100.0", "50.0", "-20.0"] {
        assert!(!debug.contains(private), "leaked {private}: {debug}");
    }
}
