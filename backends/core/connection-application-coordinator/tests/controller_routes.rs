mod support;

use float_view_connection_application_coordinator::{
    Command, ConnectionApplicationCoordinator, Diagnostic, Effect, Phase,
};
use float_view_controller_discovery::Route;
use support::*;

fn scanning(ids: &[u8]) -> ConnectionApplicationCoordinator {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    coordinator.apply(Command::Begin(begin_facts(10))).unwrap();
    let mut gateway = gateway_payload(1);
    gateway[2] = 6; // VESC Express 6.06, matching the reported topology.
    receive(&mut coordinator, gateway, 20);
    let mut inventory = vec![62];
    inventory.extend_from_slice(ids);
    receive(&mut coordinator, inventory, 30);
    coordinator
}

fn expire(
    coordinator: &mut ConnectionApplicationCoordinator,
    effects: &[Effect],
    now_ms: u64,
) -> Vec<Effect> {
    coordinator
        .apply(Command::DeadlineFired {
            generation: coordinator.snapshot().generation,
            token: deadline(effects),
            now_ms,
        })
        .unwrap()
}

#[test]
fn duplicate_thor_reply_and_false_express_scan_entry_do_not_create_a_second_controller() {
    let mut coordinator = scanning(&[1, 2]);
    let identity = motor_payload(42);
    let verify_first = receive(&mut coordinator, identity.clone(), 40);
    assert_eq!(write(&verify_first).payload, [34, 1, 50, 0, 2, 0, 0]);
    // Duplicate firmware while the CAN ID read is pending cannot advance discovery.
    let before = coordinator.snapshot();
    receive(&mut coordinator, identity.clone(), 41);
    assert_eq!(coordinator.snapshot(), before);
    let second = receive(&mut coordinator, vec![50, 0, 2, 0, 0, 1], 42);
    assert_eq!(write(&second).payload, [34, 2, 0]);
    // The same physical identity arrives again after the false CAN 2 probe starts.
    let verify_second = receive(&mut coordinator, identity, 43);
    assert_eq!(write(&verify_second).payload, [34, 2, 50, 0, 2, 0, 0]);
    assert_eq!(coordinator.snapshot().discovery.candidates.len(), 0);
    // A delayed CAN 1 ID reply must not retire CAN 2's verification deadline.
    let before = coordinator.snapshot();
    let rejected = receive(&mut coordinator, vec![50, 0, 2, 0, 0, 1], 44);
    assert_eq!(
        rejected,
        [Effect::Diagnostic(Diagnostic::UnexpectedResponse)]
    );
    assert_eq!(coordinator.snapshot(), before);
    let finished = expire(&mut coordinator, &verify_second, 1_043);
    let snapshot = coordinator.snapshot();
    assert_eq!(snapshot.discovery.route, Some(Route::Can(1)));
    assert_eq!(snapshot.phase, Phase::Identifying);
    assert!(snapshot.discovery.candidates.is_empty());
    assert_eq!(snapshot.discovery.inventory.len(), 2); // Gateway plus one verified motor.
    assert_eq!(write(&finished).payload, [34, 1, 36, 101, 0, 2, 0]);
    assert!(finished.iter().all(|effect| !matches!(effect,
        Effect::StoreRoutingHint(value) if value.route == Route::Can(2)
    )));
}

#[test]
fn matching_names_and_versions_with_distinct_verified_routes_still_require_selection() {
    let mut coordinator = scanning(&[1, 2]);
    receive_can_motor(&mut coordinator, motor_payload(42), 1, 40);
    let verify_second = receive(&mut coordinator, motor_payload(43), 50);
    let before = coordinator.snapshot();
    receive(&mut coordinator, vec![50, 0, 2, 0, 0, 1], 51);
    assert_eq!(coordinator.snapshot(), before);
    receive(&mut coordinator, vec![50, 0, 2, 0, 0, 2], 52);
    let snapshot = coordinator.snapshot();
    assert_eq!(snapshot.phase, Phase::SelectingController);
    assert_eq!(
        snapshot
            .discovery
            .candidates
            .iter()
            .map(|v| v.can_id)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    // A completed verification's deadline is harmless.
    let before = coordinator.snapshot();
    expire(&mut coordinator, &verify_second, 1_050);
    assert_eq!(coordinator.snapshot(), before);
}

#[test]
fn malformed_or_missing_route_proof_never_admits_a_controller() {
    for response in [
        None,
        Some(vec![50, 0, 2, 0]),
        Some(vec![50, 0, 0, 0, 0, 1]),
        Some(vec![50, 0, 2, 0, 0, 254]),
    ] {
        let mut coordinator = scanning(&[1]);
        let pending = receive(&mut coordinator, motor_payload(42), 40);
        if let Some(payload) = response {
            receive(&mut coordinator, payload, 41);
        } else {
            expire(&mut coordinator, &pending, 1_040);
        }
        let snapshot = coordinator.snapshot();
        assert_eq!(snapshot.phase, Phase::Failed);
        assert!(snapshot.discovery.selected.is_none());
        assert!(snapshot.discovery.candidates.is_empty());
        assert_eq!(snapshot.discovery.inventory.len(), 1);
    }
}

#[test]
fn preferred_route_cannot_bypass_verification_and_reset_discards_pending_identity() {
    let mut coordinator = ConnectionApplicationCoordinator::default();
    let mut facts = begin_facts(10);
    facts.preferred_controller_can_id = Some(2);
    coordinator.apply(Command::Begin(facts)).unwrap();
    receive(&mut coordinator, gateway_payload(1), 20);
    let pending = receive(&mut coordinator, motor_payload(42), 30);
    receive(&mut coordinator, vec![50, 0, 2, 0, 0, 1], 31);
    expire(&mut coordinator, &pending, 1_030);
    assert_eq!(coordinator.snapshot().phase, Phase::Failed);
    assert!(coordinator.snapshot().discovery.selected.is_none());

    let mut coordinator = scanning(&[1]);
    let pending = receive(&mut coordinator, motor_payload(42), 40);
    coordinator.apply(Command::Reset { now_ms: 41 }).unwrap();
    expire(&mut coordinator, &pending, 1_040);
    receive(&mut coordinator, vec![50, 0, 2, 0, 0, 1], 1_041);
    assert_eq!(coordinator.snapshot().phase, Phase::Disconnected);
    assert!(coordinator.snapshot().discovery.selected.is_none());
}

#[test]
fn every_supported_vesc_wire_version_accepts_the_exact_route_read() {
    for (major, minor) in [(6, 2), (6, 5), (6, 6), (7, 0)] {
        let mut coordinator = scanning(&[1]);
        let mut identity = motor_payload(42);
        identity[1] = major;
        identity[2] = minor;
        receive_can_motor(&mut coordinator, identity, 1, 40);
        assert_eq!(coordinator.snapshot().discovery.route, Some(Route::Can(1)));
    }
}
