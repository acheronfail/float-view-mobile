mod support;

use float_view_controller_discovery::{
    Continuity, ControllerDiscovery, DiscoveryInputError, Effect, Generation, IssueCode,
    MAX_CAN_IDS, MAX_CANDIDATES, Route, RoutingHint,
};
use float_view_refloat_compatibility::{WireVersion, compatibility};
use float_view_vesc_compatibility::AccessMode;

use support::{effect, identity, probe_route};

#[test]
fn direct_supported_test_and_unsupported_firmware_are_exact() {
    for (minor, test_version, expected_access) in [
        (2, 0, AccessMode::ReadWrite),
        (5, 1, AccessMode::ReadWrite),
        (6, u8::MAX, AccessMode::ReadWrite),
    ] {
        let mut discovery = begun();
        let transition =
            discovery.accept_transport(identity(0, minor, 42, test_version), None, None);
        assert!(matches!(
            effect(&transition),
            Some(Effect::RequestRefloatInfo {
                route: Route::Direct,
                remember_controller: None
            })
        ));
        assert_eq!(discovery.snapshot().selected_access, Some(expected_access));
    }

    for (hardware_type, minor) in [(0, 4), (0, 7)] {
        for test_version in [0, 1, u8::MAX] {
            let mut discovery = begun();
            let transition = discovery.accept_transport(
                identity(hardware_type, minor, 99, test_version),
                None,
                None,
            );
            assert_failed(&transition, IssueCode::UnsupportedController);
            assert_eq!(discovery.snapshot().route, Some(Route::Direct));
        }
    }
}

#[test]
fn preferred_route_is_authoritative_for_identity_timeout_and_hardware() {
    let continuity = Continuity::default();
    let hint = RoutingHint::new(42, [42; 12]).unwrap();

    let mut matched = begun();
    let probe = matched.accept_transport(identity(5, 6, 0, 0), Some(42), Some(hint));
    let generation = probe_generation(&probe);
    let selected = matched.accept_can_identity(42, identity(0, 6, 42, 0), continuity);
    assert!(matches!(
        effect(&selected),
        Some(Effect::RequestRefloatInfo { .. })
    ));

    let mut changed = begun();
    changed.accept_transport(identity(5, 6, 0, 0), Some(42), Some(hint));
    assert_failed(
        &changed.accept_can_identity(42, identity(0, 6, 43, 0), continuity),
        IssueCode::ControllerChanged,
    );

    let mut non_motor = begun();
    non_motor.accept_transport(identity(5, 6, 0, 0), Some(42), None);
    assert_failed(
        &non_motor.accept_can_identity(42, identity(5, 6, 0, 0), continuity),
        IssueCode::PreferredControllerUnavailable,
    );

    let mut timed_out = begun();
    let probe = timed_out.accept_transport(identity(5, 6, 0, 0), Some(42), None);
    assert_failed(
        &timed_out.can_probe_timed_out(42, probe_generation(&probe)),
        IssueCode::PreferredControllerUnavailable,
    );
    assert!(timed_out.can_probe_timed_out(42, generation).is_empty());
}

#[test]
fn learned_hint_clears_before_bounded_fallback() {
    let continuity = Continuity::default();
    let hint = RoutingHint::new(7, [7; 12]).unwrap();
    let mut discovery = begun();
    discovery.accept_transport(identity(5, 6, 0, 0), None, Some(hint));
    let transition = discovery.accept_can_identity(7, identity(5, 6, 8, 0), continuity);
    assert_eq!(
        transition.effects,
        [Effect::ClearRoutingHint, Effect::PingCan]
    );
    assert_eq!(
        probe_route(&discovery.accept_can_ids(&[42], continuity)),
        Some(Route::Can(42))
    );

    let mut timeout = begun();
    let probe = timeout.accept_transport(identity(5, 6, 0, 0), None, Some(hint));
    assert_eq!(
        timeout
            .can_probe_timed_out(7, probe_generation(&probe))
            .effects,
        [Effect::ClearRoutingHint, Effect::PingCan]
    );
}

#[test]
fn can_normalization_is_exhaustive_bounded_and_sorted() {
    for can_id in u8::MIN..=u8::MAX {
        let mut discovery = enumerating();
        let transition = discovery.accept_can_ids(&[can_id, can_id], Continuity::default());
        if can_id <= 253 {
            assert_eq!(probe_route(&transition), Some(Route::Can(can_id)));
        } else {
            assert_failed(&transition, IssueCode::NoControllerFound);
        }
    }

    let mut ordered = enumerating();
    let transition = ordered.accept_can_ids(&[253, 7, 42, 7, 254, 255, 0], Continuity::default());
    assert_eq!(probe_route(&transition), Some(Route::Can(0)));
    let generation = probe_generation(&transition);
    let next = ordered.can_probe_timed_out(0, generation);
    assert_eq!(probe_route(&next), Some(Route::Can(7)));

    let mut maximum = enumerating();
    let ids = (0..MAX_CAN_IDS as u8).rev().collect::<Vec<_>>();
    assert_eq!(
        probe_route(&maximum.accept_can_ids(&ids, Continuity::default())),
        Some(Route::Can(0))
    );

    let mut overflow = enumerating();
    let ids = (0..=MAX_CAN_IDS as u8).collect::<Vec<_>>();
    assert_failed(
        &overflow.accept_can_ids(&ids, Continuity::default()),
        IssueCode::TooManyCanDevices,
    );
}

#[test]
fn candidate_limit_selection_and_private_projection_are_exact() {
    let continuity = Continuity::default();
    let mut maximum = enumerating();
    let ids = (0..MAX_CANDIDATES as u8).collect::<Vec<_>>();
    let mut transition = maximum.accept_can_ids(&ids, continuity);
    for can_id in ids {
        assert_eq!(probe_route(&transition), Some(Route::Can(can_id)));
        transition = maximum.accept_can_identity(can_id, identity(0, 6, can_id + 1, 0), continuity);
    }
    assert!(matches!(
        effect(&transition),
        Some(Effect::SelectingController)
    ));
    assert_eq!(maximum.snapshot().candidates.len(), MAX_CANDIDATES);
    assert!(maximum.snapshot().candidates.iter().all(|candidate| {
        candidate.hardware_name == "THOR300" && candidate.firmware_name.is_none()
    }));

    let before = maximum.snapshot();
    assert_eq!(
        maximum.select_controller(254),
        Err(DiscoveryInputError::InvalidSelection)
    );
    assert_eq!(maximum.snapshot(), before);
    assert!(matches!(
        effect(&maximum.select_controller(7).unwrap()),
        Some(Effect::RequestRefloatInfo {
            route: Route::Can(7),
            remember_controller: Some(_)
        })
    ));
    assert!(maximum.snapshot().candidates.is_empty());

    let mut overflow = enumerating();
    let ids = (0..=MAX_CANDIDATES as u8).collect::<Vec<_>>();
    let mut transition = overflow.accept_can_ids(&ids, continuity);
    for can_id in ids {
        transition =
            overflow.accept_can_identity(can_id, identity(0, 6, can_id + 1, 0), continuity);
        if matches!(effect(&transition), Some(Effect::Failed(_))) {
            break;
        }
    }
    assert_failed(&transition, IssueCode::TooManyControllers);
}

#[test]
fn can_controller_without_meaningful_uuid_is_selected_but_never_remembered() {
    for uuid_byte in [0, u8::MAX] {
        let mut discovery = enumerating();
        assert_eq!(
            probe_route(&discovery.accept_can_ids(&[7], Continuity::default())),
            Some(Route::Can(7))
        );
        assert!(matches!(
            effect(&discovery.accept_can_identity(
                7,
                identity(0, 6, uuid_byte, 0),
                Continuity::default(),
            )),
            Some(Effect::RequestRefloatInfo {
                route: Route::Can(7),
                remember_controller: None,
            })
        ));

        let mut reconnect = begun();
        assert!(matches!(
            effect(&reconnect.accept_transport(identity(5, 6, 0, 0), None, None)),
            Some(Effect::PingCan)
        ));
    }
}

#[test]
fn unsupported_candidate_is_visible_but_never_selectable() {
    let continuity = Continuity::default();
    let mut discovery = enumerating();
    discovery.accept_can_ids(&[7, 42], continuity);
    discovery.accept_can_identity(7, identity(0, 4, 7, 0), continuity);
    let transition = discovery.accept_can_identity(42, identity(0, 6, 42, 0), continuity);
    assert!(matches!(
        effect(&transition),
        Some(Effect::SelectingController)
    ));
    let candidates = discovery.snapshot().candidates;
    assert_eq!(candidates.len(), 2);
    assert!(!candidates[0].vesc_supported);
    assert!(candidates[1].vesc_supported);
    let before = discovery.snapshot();
    assert_eq!(
        discovery.select_controller(7),
        Err(DiscoveryInputError::InvalidSelection)
    );
    assert_eq!(discovery.snapshot(), before);
}

#[test]
fn continuity_prefers_meaningful_uuid_then_supported_prior_route() {
    let mut moved = enumerating();
    let continuity = Continuity::new(true, Some(Route::Can(7)), Some([42; 12]));
    moved.accept_can_ids(&[7, 42], continuity);
    moved.accept_can_identity(7, identity(0, 6, 7, 0), continuity);
    let transition = moved.accept_can_identity(42, identity(0, 6, 42, 0), continuity);
    assert!(matches!(
        effect(&transition),
        Some(Effect::RequestRefloatInfo {
            route: Route::Can(42),
            ..
        })
    ));

    let mut route_only = enumerating();
    let continuity = Continuity::new(true, Some(Route::Can(42)), Some([0; 12]));
    route_only.accept_can_ids(&[7, 42], continuity);
    route_only.accept_can_identity(7, identity(0, 6, 7, 0), continuity);
    let transition = route_only.accept_can_identity(42, identity(0, 6, 42, 0), continuity);
    assert!(matches!(
        effect(&transition),
        Some(Effect::RequestRefloatInfo {
            route: Route::Can(42),
            ..
        })
    ));
}

#[test]
fn refloat_readiness_is_family_exact_and_fail_closed() {
    let legacy = compatibility(WireVersion::legacy(1, 0)).unwrap();
    let dynamic = compatibility(WireVersion::patched(1, 2, 3)).unwrap();
    let selectable = compatibility(WireVersion::patched(1, 3, 0)).unwrap();

    let mut legacy_discovery = selected_direct();
    assert!(matches!(
        effect(&legacy_discovery.accept_refloat_profile(legacy)),
        Some(Effect::Ready)
    ));

    let mut dynamic_discovery = selected_direct();
    assert!(matches!(
        effect(&dynamic_discovery.accept_refloat_profile(dynamic)),
        Some(Effect::RequestTelemetryLayout)
    ));
    assert!(matches!(
        effect(&dynamic_discovery.accept_telemetry_layout(true)),
        Some(Effect::Ready)
    ));

    let mut selectable_discovery = selected_direct();
    assert!(matches!(
        effect(&selectable_discovery.accept_refloat_profile(selectable)),
        Some(Effect::Ready)
    ));
    assert_eq!(
        selectable_discovery.snapshot().refloat_profile,
        Some(selectable)
    );

    let mut rejected = selected_direct();
    rejected.accept_refloat_profile(dynamic);
    assert_failed(
        &rejected.accept_telemetry_layout(false),
        IssueCode::UnsupportedRefloat,
    );
}

#[test]
fn stale_repeated_and_reset_events_do_not_mutate_current_state() {
    let mut discovery = enumerating();
    let transition = discovery.accept_can_ids(&[7], Continuity::default());
    let generation = probe_generation(&transition);
    let before = discovery.snapshot();
    assert!(discovery.can_probe_timed_out(8, generation).is_empty());
    assert!(
        discovery
            .can_probe_timed_out(7, Generation(generation.0.wrapping_add(1)))
            .is_empty()
    );
    assert_eq!(discovery.snapshot(), before);

    let identification = discovery.snapshot().identification_generation;
    discovery.reset();
    let reset = discovery.snapshot();
    assert!(
        discovery
            .identification_timed_out(identification)
            .is_empty()
    );
    assert_eq!(discovery.snapshot(), reset);
    assert!(
        discovery
            .accept_can_ids(&[7], Continuity::default())
            .is_empty()
    );
}

#[test]
fn diagnostics_redact_uuid_names_and_never_embed_private_values() {
    assert_eq!(
        RoutingHint::new(42, [0; 12]),
        Err(DiscoveryInputError::InvalidControllerUuid)
    );
    assert_eq!(
        RoutingHint::new(42, [u8::MAX; 12]),
        Err(DiscoveryInputError::InvalidControllerUuid)
    );
    assert_eq!(
        RoutingHint::new(254, [42; 12]),
        Err(DiscoveryInputError::InvalidCanId)
    );
    let hint = RoutingHint::new(42, [222; 12]).unwrap();
    let mut discovery = begun();
    discovery.accept_transport(
        float_view_vesc_firmware_identity::FirmwareIdentity {
            hardware_name: "private-hardware-name".to_owned(),
            firmware_name: Some("private-firmware-name".to_owned()),
            ..identity(5, 6, 222, 0)
        },
        None,
        Some(hint),
    );
    let debug = format!("{hint:?} {discovery:?}");
    assert!(debug.contains("[redacted]"));
    assert!(!debug.contains("222"));
    assert!(!debug.contains("private-hardware-name"));
    assert!(!debug.contains("private-firmware-name"));
}

fn begun() -> ControllerDiscovery {
    let mut discovery = ControllerDiscovery::new();
    discovery.begin();
    discovery
}

fn enumerating() -> ControllerDiscovery {
    let mut discovery = begun();
    assert!(matches!(
        effect(&discovery.accept_transport(identity(5, 6, 0, 0), None, None)),
        Some(Effect::PingCan)
    ));
    discovery
}

fn selected_direct() -> ControllerDiscovery {
    let mut discovery = begun();
    discovery.accept_transport(identity(0, 6, 42, 0), None, None);
    discovery
}

fn probe_generation(transition: &float_view_controller_discovery::Transition) -> Generation {
    match effect(transition) {
        Some(Effect::ProbeFirmware {
            probe_generation: Some(generation),
            ..
        }) => *generation,
        value => panic!("expected probe generation, got {value:?}"),
    }
}

fn assert_failed(transition: &float_view_controller_discovery::Transition, expected: IssueCode) {
    match effect(transition) {
        Some(Effect::Failed(issue)) => assert_eq!(issue.code, expected),
        value => panic!("expected {expected:?}, got {value:?}"),
    }
}
