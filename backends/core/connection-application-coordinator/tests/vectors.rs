mod support;

use float_view_connection_application_coordinator::{
    Command, ConnectionApplicationCoordinator, Effect, Phase,
};
use float_view_controller_discovery::Route;

use support::*;

const VECTORS: &str =
    include_str!("../../../fixtures/connection/connection-application-coordinator.tsv");

#[test]
fn shared_native_connection_lifecycle_vectors_match_exactly() {
    let vectors = VECTORS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();
    assert_eq!(vectors.len(), 8);
    for line in vectors {
        let columns = line.split('|').collect::<Vec<_>>();
        assert_eq!(columns.len(), 5, "invalid vector: {line}");
        let snapshot = run(columns[0]);
        assert_eq!(phase_text(snapshot.phase), columns[1], "{}", columns[0]);
        assert_eq!(
            route_text(snapshot.discovery.route),
            columns[2],
            "{}",
            columns[0]
        );
        if !columns[3].is_empty() {
            assert_eq!(
                snapshot.standalone_bms,
                columns[3] == "yes",
                "{}",
                columns[0]
            );
        }
    }
}

fn run(name: &str) -> float_view_connection_application_coordinator::Snapshot {
    match name {
        "direct-legacy-activation" => direct_legacy(),
        "preferred-can-legacy-activation" => preferred_can(),
        "direct-dynamic-telemetry" => direct_dynamic(),
        "standalone-bms-values" => standalone_bms(),
        "ambiguous-explicit-selection" => ambiguous_selection(),
        "stale-hint-fallback" => stale_hint(),
        "two-attempt-identification-timeout" => identification_timeout(),
        "reset-during-identification" => reset_during_identification(),
        _ => panic!("unknown vector {name}"),
    }
}

fn direct_legacy() -> float_view_connection_application_coordinator::Snapshot {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_payload(42), 20);
    let effects = receive(&mut coordinator, legacy_info_payload(), 30);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::PrepareActivation(_)))
    );
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::PublishTelemetry(_)))
    );
    let activation = activate(&mut coordinator, 40);
    assert_eq!(write(&activation).payload, [14]);
    let telemetry_request = receive(
        &mut coordinator,
        current_limits_payload(0x2EFD_0142, Some(15)),
        45,
    );
    assert_eq!(write(&telemetry_request).payload, [36, 101, 10, 4]);
    let telemetry = receive(&mut coordinator, legacy_idle_payload(), 50);
    assert!(
        telemetry
            .iter()
            .any(|effect| matches!(effect, Effect::PublishTelemetry(_)))
    );
    assert_eq!(write(&telemetry).payload, [62]);
    coordinator.snapshot()
}

fn preferred_can() -> float_view_connection_application_coordinator::Snapshot {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    let mut facts = begin_facts(10);
    facts.preferred_controller_can_id = Some(42);
    coordinator.apply(Command::Begin(facts)).unwrap();
    let probe = receive(&mut coordinator, gateway_payload(1), 20);
    assert_eq!(write(&probe).payload, [34, 42, 0]);
    let info = receive(&mut coordinator, motor_payload(42), 30);
    assert_eq!(&write(&info).payload[..2], &[34, 42]);
    receive(&mut coordinator, legacy_info_payload(), 40);
    let activation = activate(&mut coordinator, 50);
    assert_eq!(write(&activation).payload, [34, 42, 14]);
    receive(
        &mut coordinator,
        current_limits_payload(0x2EFD_0142, Some(15)),
        55,
    );
    coordinator.snapshot()
}

fn direct_dynamic() -> float_view_connection_application_coordinator::Snapshot {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    receive(&mut coordinator, motor_payload(42), 20);
    let layout_request = receive(&mut coordinator, dynamic_info_payload(), 30);
    assert_eq!(write(&layout_request).payload, [36, 101, 32]);
    receive(&mut coordinator, dynamic_layout_payload(), 40);
    let activation = activate(&mut coordinator, 50);
    assert_eq!(write(&activation).payload, [14]);
    let telemetry_request = receive(
        &mut coordinator,
        current_limits_payload(0xDEAD_BEEF, None),
        55,
    );
    assert_eq!(write(&telemetry_request).payload, [36, 101, 31]);
    let sample = receive(&mut coordinator, dynamic_idle_payload(), 60);
    assert!(
        sample
            .iter()
            .any(|effect| matches!(effect, Effect::PublishTelemetry(_)))
    );
    coordinator.snapshot()
}

fn standalone_bms() -> float_view_connection_application_coordinator::Snapshot {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    let values_request = receive(&mut coordinator, stock_bms_payload(), 20);
    assert_eq!(write(&values_request).payload, [96]);
    assert!(
        !values_request
            .iter()
            .any(|effect| matches!(effect, Effect::PrepareActivation(_)))
    );
    let values = receive(&mut coordinator, bms_values_payload(), 30);
    assert!(
        values
            .iter()
            .any(|effect| matches!(effect, Effect::PublishBmsObservation(_)))
    );
    coordinator.snapshot()
}

fn ambiguous_selection() -> float_view_connection_application_coordinator::Snapshot {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    let scan = receive(&mut coordinator, gateway_payload(1), 20);
    assert_eq!(write(&scan).payload, [62]);
    let first = receive(&mut coordinator, vec![62, 42, 7], 30);
    assert_eq!(write(&first).payload, [34, 7, 0]);
    let second = receive(&mut coordinator, motor_payload(7), 40);
    assert_eq!(write(&second).payload, [34, 42, 0]);
    receive(&mut coordinator, motor_payload(42), 50);
    assert_eq!(coordinator.snapshot().phase, Phase::SelectingController);
    let selected = coordinator
        .apply(Command::SelectController {
            generation: coordinator.snapshot().generation,
            can_id: 42,
            now_ms: 60,
        })
        .unwrap();
    assert_eq!(&write(&selected).payload[..2], &[34, 42]);
    receive(&mut coordinator, legacy_info_payload(), 70);
    coordinator.snapshot()
}

fn stale_hint() -> float_view_connection_application_coordinator::Snapshot {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    let mut facts = begin_facts(10);
    facts.routing_hint = Some(hint(7, 42));
    coordinator.apply(Command::Begin(facts)).unwrap();
    receive(&mut coordinator, gateway_payload(1), 20);
    let fallback = receive(&mut coordinator, gateway_payload(8), 30);
    assert!(
        fallback
            .iter()
            .any(|effect| matches!(effect, Effect::ClearRoutingHint))
    );
    assert_eq!(write(&fallback).payload, [62]);
    coordinator.snapshot()
}

fn identification_timeout() -> float_view_connection_application_coordinator::Snapshot {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    let first = coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    let second = coordinator
        .apply(Command::DeadlineFired {
            generation: coordinator.snapshot().generation,
            token: deadline(&first),
            now_ms: 2_010,
        })
        .unwrap();
    assert_eq!(write(&second).payload, [0]);
    coordinator
        .apply(Command::DeadlineFired {
            generation: coordinator.snapshot().generation,
            token: deadline(&second),
            now_ms: 10_010,
        })
        .unwrap();
    coordinator.snapshot()
}

fn reset_during_identification() -> float_view_connection_application_coordinator::Snapshot {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    coordinator.apply(Command::Reset { now_ms: 20 }).unwrap();
    coordinator.snapshot()
}

fn phase_text(phase: Phase) -> &'static str {
    match phase {
        Phase::Disconnected => "disconnected",
        Phase::Identifying => "identifying",
        Phase::SelectingController => "selecting_controller",
        Phase::AwaitingActivation => "awaiting_activation",
        Phase::Ready => "ready",
        Phase::Failed => "failed",
    }
}

#[test]
fn vector_routes_use_only_checked_forms() {
    assert_eq!(route_text(Some(Route::Can(253))), "can:253");
}
