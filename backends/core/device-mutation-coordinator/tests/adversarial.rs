mod support;

use float_view_device_mutation_coordinator::{
    BooleanControl, Command, ConfigurationMutation, ControlStatus, CoordinatorError,
    DeviceMutationCoordinator, Diagnostic, Effect, FailureReason, PendingKind,
};
use float_view_refloat_telemetry_service::{
    FirmwareFault, FootpadState, PackageMode, PackageState, RideState,
};
use float_view_vesc_request_scheduler::{Generation, Priority, RequestContext, Route};

use support::*;

#[test]
fn every_request_fact_is_exact_and_route_correct() {
    let cases = [
        (BooleanControl::Package, false, 12, false),
        (BooleanControl::Flywheel, true, 22, false),
        (BooleanControl::Leds, false, 20, true),
        (BooleanControl::Headlights, false, 20, true),
    ];
    for route in [Route::Direct, Route::Can(0), Route::Can(253)] {
        for (control, enabled, refloat_command, expects_response) in cases {
            let mut coordinator = DeviceMutationCoordinator::new();
            ready(&mut coordinator, route);
            let effects = coordinator
                .apply(Command::SetControl {
                    generation: Generation(7),
                    control,
                    enabled,
                })
                .unwrap();
            let (_, request) = submission(&effects);
            assert_eq!(request.priority, Priority::RidingTelemetry);
            assert_eq!(request.context, RequestContext::RefloatControl(route));
            assert_eq!(request.generation, Generation(7));
            assert_eq!(request.timeout_ms, 3_000);
            assert_eq!(request.expected_response.command, 36);
            assert_eq!(
                request.expected_response.refloat_command,
                Some(refloat_command)
            );
            assert_eq!(request.expects_response, expects_response);
            request.validate().unwrap();
            if let Route::Can(can_id) = route {
                assert_eq!(&request.payload[..2], &[34, can_id]);
            }
        }
    }
}

#[test]
fn unsupported_and_read_only_firmware_fail_closed_without_partial_state() {
    let mut unsupported = DeviceMutationCoordinator::new();
    let before = unsupported.snapshot();
    assert_eq!(
        unsupported.apply(Command::Configure(
            float_view_device_mutation_coordinator::ConfigureFacts {
                generation: Generation(7),
                route: Route::Direct,
                firmware: unsupported_firmware(),
                refloat: stable_profile(),
                connection_ready: true,
            }
        )),
        Err(CoordinatorError::UnsupportedController)
    );
    assert_eq!(unsupported.snapshot(), before);

    let mut read_only = DeviceMutationCoordinator::new();
    configure(&mut read_only, Route::Direct, firmware(1), stable_profile());
    observe(
        &mut read_only,
        sample(1, RideState::Idle, PackageMode::Normal),
    );
    observe_lighting(&mut read_only, 1, &[36, 101, 20, 3]);
    assert!(!read_only.snapshot().controls.as_ref().unwrap().writable);
    assert_eq!(
        read_only.apply(Command::SetControl {
            generation: Generation(7),
            control: BooleanControl::Package,
            enabled: false,
        }),
        Err(CoordinatorError::InvalidState)
    );
    assert_eq!(read_only.snapshot().pending, None);
}

#[test]
fn configuration_requires_the_complete_current_safety_state() {
    let invalid = [
        sample_with(
            2,
            RideState::Riding,
            PackageState::Ready,
            PackageMode::Normal,
            FootpadState::None,
            Some(false),
            None,
        ),
        sample_with(
            2,
            RideState::Idle,
            PackageState::Running,
            PackageMode::Normal,
            FootpadState::None,
            Some(false),
            None,
        ),
        sample_with(
            2,
            RideState::Idle,
            PackageState::Ready,
            PackageMode::Flywheel,
            FootpadState::None,
            Some(false),
            None,
        ),
        sample_with(
            2,
            RideState::Idle,
            PackageState::Ready,
            PackageMode::Normal,
            FootpadState::Left,
            Some(false),
            None,
        ),
    ];
    for sample in invalid {
        let mut coordinator = DeviceMutationCoordinator::new();
        ready(&mut coordinator, Route::Direct);
        observe(&mut coordinator, sample);
        assert_eq!(
            coordinator.apply(Command::BeginConfiguration {
                generation: Generation(7),
                mutation: ConfigurationMutation::Write,
            }),
            Err(CoordinatorError::InvalidState)
        );
        assert_eq!(coordinator.snapshot().pending, None);
    }
}

#[test]
fn configuration_does_not_wait_for_lower_priority_lighting_observation() {
    let mut coordinator = DeviceMutationCoordinator::new();
    configure(
        &mut coordinator,
        Route::Direct,
        firmware(0),
        stable_profile(),
    );
    observe(
        &mut coordinator,
        sample(1, RideState::Idle, PackageMode::Normal),
    );
    assert_eq!(
        coordinator.snapshot().controls.as_ref().unwrap().status,
        ControlStatus::Loading
    );

    let effects = coordinator
        .apply(Command::BeginConfiguration {
            generation: Generation(7),
            mutation: ConfigurationMutation::Write,
        })
        .unwrap();
    assert_eq!(lease(&effects).kind, PendingKind::ConfigurationWrite);
}

#[test]
fn one_pending_slot_excludes_every_competing_family() {
    let mut coordinator = DeviceMutationCoordinator::new();
    ready(&mut coordinator, Route::Direct);
    let effects = coordinator
        .apply(Command::BeginConfiguration {
            generation: Generation(7),
            mutation: ConfigurationMutation::TuneApply,
        })
        .unwrap();
    let token = lease(&effects);
    for command in [
        Command::SetControl {
            generation: Generation(7),
            control: BooleanControl::Leds,
            enabled: false,
        },
        Command::ClearFatalError {
            generation: Generation(7),
        },
        Command::ShutdownController {
            generation: Generation(7),
        },
        Command::BeginConfiguration {
            generation: Generation(7),
            mutation: ConfigurationMutation::Write,
        },
    ] {
        assert_eq!(
            coordinator.apply(command),
            Err(CoordinatorError::InvalidState)
        );
        assert_eq!(coordinator.snapshot().pending, Some(token));
    }
}

#[test]
fn exact_tokens_own_finish_failure_and_stale_generation_behavior() {
    let mut coordinator = DeviceMutationCoordinator::new();
    ready(&mut coordinator, Route::Direct);
    let token = lease(
        &coordinator
            .apply(Command::BeginConfiguration {
                generation: Generation(7),
                mutation: ConfigurationMutation::Write,
            })
            .unwrap(),
    );
    let wrong = float_view_device_mutation_coordinator::MutationToken {
        sequence: token.sequence.wrapping_add(1),
        ..token
    };
    assert_eq!(
        coordinator
            .apply(Command::CompleteConfiguration { token: wrong })
            .unwrap(),
        [Effect::Diagnostic(Diagnostic::MismatchedToken)]
    );
    let stale = float_view_device_mutation_coordinator::MutationToken {
        generation: Generation(6),
        ..token
    };
    assert_eq!(
        coordinator
            .apply(Command::Fail {
                token: stale,
                reason: FailureReason::RequestFailed,
            })
            .unwrap(),
        [Effect::Diagnostic(Diagnostic::StaleEvent)]
    );
    assert_eq!(coordinator.snapshot().pending, Some(token));
    let complete = coordinator
        .apply(Command::CompleteConfiguration { token })
        .unwrap();
    assert!(complete.contains(&Effect::Confirmed(token)));
    assert_eq!(coordinator.snapshot().pending, None);
}

#[test]
fn flywheel_confirmation_order_preserves_only_eligible_disable() {
    let mut enable = DeviceMutationCoordinator::new();
    ready(&mut enable, Route::Direct);
    let enable_token = submission(
        &enable
            .apply(Command::SetControl {
                generation: Generation(7),
                control: BooleanControl::Flywheel,
                enabled: true,
            })
            .unwrap(),
    )
    .0;
    let confirmed = observe(
        &mut enable,
        sample(2, RideState::Riding, PackageMode::Flywheel),
    );
    assert!(confirmed.contains(&Effect::Confirmed(enable_token)));
    assert!(!confirmed.iter().any(|effect| matches!(
        effect,
        Effect::Failed {
            reason: FailureReason::ProhibitedByFlywheel,
            ..
        }
    )));

    let mut disable = DeviceMutationCoordinator::new();
    ready(&mut disable, Route::Direct);
    observe(
        &mut disable,
        sample(2, RideState::Riding, PackageMode::Flywheel),
    );
    let disable_token = submission(
        &disable
            .apply(Command::SetControl {
                generation: Generation(7),
                control: BooleanControl::Flywheel,
                enabled: false,
            })
            .unwrap(),
    )
    .0;
    observe(
        &mut disable,
        sample(3, RideState::Riding, PackageMode::Flywheel),
    );
    assert_eq!(disable.snapshot().pending, Some(disable_token));
    let normal = observe(
        &mut disable,
        sample(4, RideState::Idle, PackageMode::Normal),
    );
    assert!(normal.contains(&Effect::Confirmed(disable_token)));
}

#[test]
fn newly_authoritative_flywheel_mode_fails_other_pending_work() {
    let mut coordinator = DeviceMutationCoordinator::new();
    ready(&mut coordinator, Route::Direct);
    let token = submission(
        &coordinator
            .apply(Command::SetControl {
                generation: Generation(7),
                control: BooleanControl::Leds,
                enabled: false,
            })
            .unwrap(),
    )
    .0;
    let effects = observe(
        &mut coordinator,
        sample(2, RideState::Riding, PackageMode::Flywheel),
    );
    assert!(effects.contains(&Effect::Failed {
        token,
        reason: FailureReason::ProhibitedByFlywheel,
    }));
    assert_eq!(coordinator.snapshot().pending, None);
}

#[test]
fn lighting_and_telemetry_observations_are_exact_monotonic_and_atomic() {
    let mut coordinator = DeviceMutationCoordinator::new();
    ready(&mut coordinator, Route::Direct);
    let before = coordinator.snapshot();
    assert_eq!(
        observe_lighting(&mut coordinator, 1, &[36, 101, 20, 3]),
        [Effect::Diagnostic(Diagnostic::NonMonotonicLighting)]
    );
    assert_eq!(coordinator.snapshot(), before);
    assert_eq!(
        coordinator.apply(Command::ObserveLighting {
            generation: Generation(7),
            observation_sequence: 2,
            payload: vec![36, 101, 202, 3],
        }),
        Err(CoordinatorError::InvalidLightingResponse)
    );
    assert_eq!(coordinator.snapshot(), before);
    assert_eq!(
        observe(
            &mut coordinator,
            sample(1, RideState::Idle, PackageMode::Normal)
        ),
        [Effect::Diagnostic(Diagnostic::NonMonotonicTelemetry)]
    );
    assert_eq!(coordinator.snapshot(), before);
}

#[test]
fn fatal_clear_requires_latched_fault_without_active_firmware_fault() {
    for (latched, fault, allowed) in [
        (Some(false), None, false),
        (Some(true), Some(FirmwareFault::OverVoltage), false),
        (Some(true), None, true),
    ] {
        let mut coordinator = DeviceMutationCoordinator::new();
        ready(&mut coordinator, Route::Direct);
        observe(
            &mut coordinator,
            sample_with(
                2,
                RideState::Idle,
                PackageState::Ready,
                PackageMode::Normal,
                FootpadState::None,
                latched,
                fault,
            ),
        );
        let result = coordinator.apply(Command::ClearFatalError {
            generation: Generation(7),
        });
        assert_eq!(result.is_ok(), allowed);
        assert_eq!(coordinator.snapshot().pending.is_some(), allowed);
    }
}

#[test]
fn shutdown_is_accepted_only_idle_supported_and_once_forwarded() {
    let mut coordinator = DeviceMutationCoordinator::new();
    ready(&mut coordinator, Route::Can(42));
    let effects = coordinator
        .apply(Command::ShutdownController {
            generation: Generation(7),
        })
        .unwrap();
    let (token, request) = submission(&effects);
    assert_eq!(token.kind, PendingKind::ControllerShutdown);
    assert_eq!(request.payload, [34, 42, 156, 0, 0]);
    assert_eq!(
        request.context,
        RequestContext::ControllerShutdown(Route::Can(42))
    );
    assert!(!request.expects_response);
    let cleared = coordinator.apply(Command::ClearShutdown { token }).unwrap();
    assert!(cleared.contains(&Effect::Released(token)));
}

#[test]
fn shutdown_remains_available_while_independent_controls_are_loading() {
    let mut coordinator = DeviceMutationCoordinator::new();
    configure(
        &mut coordinator,
        Route::Direct,
        firmware(0),
        stable_profile(),
    );
    observe(
        &mut coordinator,
        sample(1, RideState::Idle, PackageMode::Normal),
    );
    assert_eq!(
        coordinator.snapshot().controls.as_ref().unwrap().status,
        ControlStatus::Loading
    );
    assert!(coordinator.snapshot().controller_actions.shutdown_available);
    assert!(
        coordinator
            .apply(Command::ShutdownController {
                generation: Generation(7),
            })
            .is_ok()
    );
}

#[test]
fn reset_clears_all_state_and_invalidates_prior_events() {
    let mut coordinator = DeviceMutationCoordinator::new();
    ready(&mut coordinator, Route::Direct);
    let token = lease(
        &coordinator
            .apply(Command::BeginConfiguration {
                generation: Generation(7),
                mutation: ConfigurationMutation::Write,
            })
            .unwrap(),
    );
    let reset = coordinator.apply(Command::Reset).unwrap();
    assert!(reset.contains(&Effect::Failed {
        token,
        reason: FailureReason::Cancelled,
    }));
    let snapshot = coordinator.snapshot();
    assert!(!snapshot.configured);
    assert_eq!(snapshot.route, None);
    assert_eq!(snapshot.controls, None);
    assert_eq!(snapshot.pending, None);
    assert_eq!(
        coordinator
            .apply(Command::Fail {
                token,
                reason: FailureReason::RequestFailed,
            })
            .unwrap(),
        [Effect::Diagnostic(Diagnostic::StaleEvent)]
    );
}

#[test]
fn diagnostics_and_debug_output_do_not_contain_private_identity_or_telemetry() {
    let mut coordinator = DeviceMutationCoordinator::new();
    let mut identity = firmware(0);
    identity.hardware_name = "private-hardware-name".to_owned();
    identity.firmware_name = Some("private-firmware-name".to_owned());
    identity.controller_uuid = [222; 12];
    configure(&mut coordinator, Route::Direct, identity, stable_profile());
    let debug = format!("{coordinator:?}");
    assert!(!debug.contains("private-hardware-name"));
    assert!(!debug.contains("private-firmware-name"));
    assert!(!debug.contains("222"));
    assert!(!debug.contains("1700000000000"));
}

#[test]
fn legacy_profile_reaches_ready_without_fatal_tail_and_uses_command_202() {
    let mut coordinator = DeviceMutationCoordinator::new();
    configure(
        &mut coordinator,
        Route::Direct,
        firmware(0),
        legacy_profile(),
    );
    observe(
        &mut coordinator,
        sample_with(
            1,
            RideState::Idle,
            PackageState::Ready,
            PackageMode::Normal,
            FootpadState::None,
            None,
            None,
        ),
    );
    observe_lighting(&mut coordinator, 1, &[36, 101, 202, 3]);
    assert_eq!(
        coordinator.snapshot().controls.as_ref().unwrap().status,
        ControlStatus::Ready
    );
    let effects = coordinator
        .apply(Command::SetControl {
            generation: Generation(7),
            control: BooleanControl::Leds,
            enabled: false,
        })
        .unwrap();
    assert_eq!(submission(&effects).1.payload, [36, 101, 202, 1, 2]);
}
