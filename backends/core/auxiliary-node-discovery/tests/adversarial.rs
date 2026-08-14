mod support;

use float_view_auxiliary_node_discovery::{
    AuxiliaryNodeDiscovery, Effect, Generation, InputError, IssueCode, MAX_AUXILIARY_CAN_IDS,
    MAX_BMS_CANDIDATES, Route, State, Status, supports_bms,
};

use support::{effect, local, routed};

#[test]
fn compatibility_catalog_is_exact_and_fail_closed() {
    for value in [
        routed(Route::Direct, 1, 6, 0, "VBMS"),
        routed(Route::Can(42), 2, 6, 6, "VBMS32"),
    ] {
        assert!(supports_bms(&value));
    }
    for value in [
        routed(Route::Direct, 1, 6, 1, "VBMS"),
        routed(Route::Direct, 1, 7, 0, "VBMS"),
        routed(Route::Can(42), 2, 6, 6, "42508004"),
        routed(Route::Can(42), 2, 6, 5, "VBMS32"),
        routed(Route::Can(42), 2, 7, 0, "VBMS32"),
        routed(Route::Can(42), 0, 6, 6, "VBMS32"),
    ] {
        assert!(!supports_bms(&value));
    }
}

#[test]
fn every_can_id_is_normalized_deduplicated_and_bounded() {
    for can_id in u8::MIN..=u8::MAX {
        let mut discovery = scanning();
        let generation = discovery.generation();
        let transition = discovery.accept_can_ids(&[can_id, can_id], generation);
        if can_id <= 253 {
            assert!(matches!(
                effect(&transition),
                Some(Effect::ProbeFirmware {
                    route: Route::Can(value),
                    ..
                }) if value == can_id
            ));
        } else {
            assert!(transition.is_empty());
            assert_eq!(discovery.state().status, Status::NotFound);
        }
    }

    let mut sorted = scanning();
    let generation = sorted.generation();
    let mut transition = sorted.accept_can_ids(&[253, 7, 42, 7, 254, 255, 0], generation);
    for expected in [0, 7, 42, 253] {
        let route = Route::Can(expected);
        assert!(matches!(
            effect(&transition),
            Some(Effect::ProbeFirmware { route: value, .. }) if value == route
        ));
        transition = sorted.probe_timed_out(route, generation);
    }
    assert_eq!(sorted.state().status, Status::NotFound);

    let mut maximum = scanning();
    let generation = maximum.generation();
    let ids = (0..MAX_AUXILIARY_CAN_IDS as u8).rev().collect::<Vec<_>>();
    assert!(matches!(
        effect(&maximum.accept_can_ids(&ids, generation)),
        Some(Effect::ProbeFirmware {
            route: Route::Can(0),
            ..
        })
    ));

    let mut overflow = scanning();
    let generation = overflow.generation();
    let ids = (0..=MAX_AUXILIARY_CAN_IDS as u8).collect::<Vec<_>>();
    assert!(overflow.accept_can_ids(&ids, generation).is_empty());
    assert_issue(&overflow, IssueCode::TooManyCanDevices);
}

#[test]
fn candidate_limit_is_sorted_and_seventeenth_fails_closed() {
    let local = gateway();
    let sixteen = (0..MAX_BMS_CANDIDATES as u8)
        .rev()
        .map(|can_id| routed(Route::Can(can_id), 1, 6, 0, "VBMS"))
        .collect::<Vec<_>>();
    let mut maximum = AuxiliaryNodeDiscovery::<u8>::new();
    assert!(
        maximum
            .begin(local.clone(), Some(&sixteen), None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(maximum.state().status, Status::Ambiguous);
    assert_eq!(maximum.state().candidates.len(), MAX_BMS_CANDIDATES);
    assert_eq!(maximum.state().candidates[0].route, Route::Can(0));
    assert_eq!(
        maximum.state().candidates[MAX_BMS_CANDIDATES - 1].route,
        Route::Can(15)
    );

    let seventeen = (0..=MAX_BMS_CANDIDATES as u8)
        .map(|can_id| routed(Route::Can(can_id), 1, 6, 0, "VBMS"))
        .collect::<Vec<_>>();
    let mut overflow = AuxiliaryNodeDiscovery::<u8>::new();
    assert!(
        overflow
            .begin(local, Some(&seventeen), None)
            .unwrap()
            .is_empty()
    );
    assert_issue(&overflow, IssueCode::TooManyBmsCandidates);
    assert!(overflow.state().candidates.is_empty());
}

#[test]
fn preferred_route_is_authoritative_for_reuse_probe_timeout_and_identity() {
    let local = gateway();
    let bms = routed(Route::Can(42), 1, 6, 0, "VBMS");
    let mut reused = AuxiliaryNodeDiscovery::<u8>::new();
    assert!(matches!(
        effect(&reused.begin(local.clone(), Some(&[bms]), Some(42)).unwrap()),
        Some(Effect::RequestValues {
            route: Route::Can(42),
            ..
        })
    ));

    let mut probed = AuxiliaryNodeDiscovery::<u8>::new();
    let transition = probed.begin(local.clone(), Some(&[]), Some(42)).unwrap();
    let generation = probed.generation();
    assert!(matches!(
        effect(&transition),
        Some(Effect::ProbeFirmware {
            route: Route::Can(42),
            ..
        })
    ));
    assert!(matches!(
        effect(&probed.accept_firmware(routed(Route::Can(42), 2, 6, 6, "VBMS32"), generation)),
        Some(Effect::RequestValues {
            route: Route::Can(42),
            ..
        })
    ));

    let mut wrong = AuxiliaryNodeDiscovery::<u8>::new();
    wrong.begin(local.clone(), None, Some(42)).unwrap();
    let generation = wrong.generation();
    wrong.accept_firmware(routed(Route::Can(42), 2, 6, 6, "Gateway"), generation);
    assert_issue(&wrong, IssueCode::PreferredBmsUnavailable);

    let mut timeout = AuxiliaryNodeDiscovery::<u8>::new();
    timeout.begin(local, None, Some(42)).unwrap();
    let generation = timeout.generation();
    timeout.probe_timed_out(Route::Can(42), generation);
    assert_issue(&timeout, IssueCode::PreferredBmsUnavailable);
}

#[test]
fn current_observations_attach_retain_recover_and_reset() {
    let mut discovery = AuxiliaryNodeDiscovery::<String>::new();
    let transition = discovery.begin(local(1, 6, 0, "VBMS"), None, None).unwrap();
    let generation = discovery.generation();
    assert!(matches!(
        effect(&transition),
        Some(Effect::RequestValues {
            route: Route::Direct,
            ..
        })
    ));
    discovery.accept_observation(Route::Direct, generation, "first-private".to_owned());
    assert_eq!(discovery.state().status, Status::Attached);
    assert_eq!(
        discovery.state().latest_observation.as_deref(),
        Some("first-private")
    );
    discovery.mark_unavailable(Route::Direct, generation);
    assert_eq!(discovery.state().status, Status::Unavailable);
    assert_eq!(
        discovery.state().latest_observation.as_deref(),
        Some("first-private")
    );
    discovery.accept_observation(Route::Direct, generation, "second-private".to_owned());
    assert_eq!(discovery.state().status, Status::Attached);
    assert_eq!(
        discovery.state().latest_observation.as_deref(),
        Some("second-private")
    );
    let debug = format!("{discovery:?}");
    assert!(debug.contains("[redacted]"));
    assert!(!debug.contains("first-private"));
    assert!(!debug.contains("second-private"));

    discovery.reset();
    assert_eq!(discovery.state(), &State::default());
}

#[test]
fn stale_mismatched_repeated_and_failed_work_does_not_mutate_current_state() {
    let mut discovery = scanning();
    let generation = discovery.generation();
    let transition = discovery.accept_can_ids(&[7], generation);
    assert!(matches!(
        effect(&transition),
        Some(Effect::ProbeFirmware { .. })
    ));
    let before = format!("{discovery:?}");
    assert!(
        discovery
            .accept_firmware(routed(Route::Can(8), 1, 6, 0, "VBMS"), generation)
            .is_empty()
    );
    assert!(
        discovery
            .probe_timed_out(Route::Can(7), Generation(generation.0.wrapping_add(1)))
            .is_empty()
    );
    assert_eq!(format!("{discovery:?}"), before);

    discovery.reset();
    let reset_generation = discovery.generation();
    assert!(discovery.accept_can_ids(&[7], generation).is_empty());
    assert!(discovery.scan_failed(generation).is_empty());
    assert_eq!(discovery.generation(), reset_generation);
    assert_eq!(discovery.state(), &State::default());
}

#[test]
fn invalid_begin_inputs_are_total_and_nonmutating() {
    let mut discovery = AuxiliaryNodeDiscovery::<u8>::new();
    let before = format!("{discovery:?}");
    assert_eq!(
        discovery.begin(routed(Route::Can(42), 1, 6, 0, "VBMS"), None, None),
        Err(InputError::InvalidLocalRoute)
    );
    assert_eq!(
        discovery.begin(gateway(), None, Some(254)),
        Err(InputError::InvalidPreferredCanId)
    );
    assert_eq!(
        discovery.begin(
            gateway(),
            Some(&[routed(Route::Can(255), 1, 6, 0, "VBMS")]),
            None
        ),
        Err(InputError::InvalidInventoryRoute)
    );
    assert_eq!(
        discovery.begin(
            gateway(),
            Some(&[
                routed(Route::Can(7), 1, 6, 0, "VBMS"),
                routed(Route::Can(7), 1, 6, 0, "VBMS"),
            ]),
            None
        ),
        Err(InputError::InvalidInventoryRoute)
    );
    let oversized = (0..=MAX_AUXILIARY_CAN_IDS as u8)
        .map(|can_id| routed(Route::Can(can_id), 0, 6, 6, "node"))
        .collect::<Vec<_>>();
    assert_eq!(
        discovery.begin(gateway(), Some(&oversized), None),
        Err(InputError::InvalidInventorySize)
    );
    assert_eq!(format!("{discovery:?}"), before);
}

#[test]
fn discovery_scan_failure_is_generation_qualified() {
    let mut discovery = scanning();
    let generation = discovery.generation();
    assert!(
        discovery
            .scan_failed(Generation(generation.0 + 1))
            .is_empty()
    );
    assert_eq!(discovery.state().status, Status::Discovering);
    assert!(discovery.scan_failed(generation).is_empty());
    assert_issue(&discovery, IssueCode::DiscoveryUnavailable);
}

#[test]
fn diagnostics_redact_candidates_identities_and_observations() {
    let private_name = "private-hardware-name".repeat(10);
    let inventory = [routed(Route::Can(42), 1, 6, 0, &private_name)];
    let mut discovery = AuxiliaryNodeDiscovery::<String>::new();
    discovery.begin(gateway(), Some(&inventory), None).unwrap();
    let generation = discovery.generation();
    discovery.accept_observation(Route::Can(42), generation, "private-observation".to_owned());
    assert_eq!(discovery.state().connection, Some(Route::Can(42)));
    let debug = format!("{discovery:?} {:?}", discovery.state().candidates);
    assert!(!debug.contains("private-hardware-name"));
    assert!(!debug.contains("private-observation"));
    assert!(debug.contains("[redacted]"));
}

fn gateway() -> float_view_auxiliary_node_discovery::RoutedIdentity {
    local(0, 6, 6, "THOR300")
}

fn scanning() -> AuxiliaryNodeDiscovery<u8> {
    let mut discovery = AuxiliaryNodeDiscovery::new();
    let transition = discovery.begin(gateway(), None, None).unwrap();
    assert!(matches!(effect(&transition), Some(Effect::PingCan { .. })));
    discovery
}

fn assert_issue<O>(discovery: &AuxiliaryNodeDiscovery<O>, expected: IssueCode) {
    assert_eq!(discovery.state().status, Status::Unavailable);
    assert_eq!(
        discovery.state().issue.map(|issue| issue.code),
        Some(expected)
    );
}
