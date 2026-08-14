mod support;

use float_view_device_mutation_coordinator::{
    BooleanControl, Command, ConfigurationMutation, ConfigureFacts, DeviceMutationCoordinator,
    Effect, FailureReason, PendingKind,
};
use float_view_refloat_telemetry_service::{FootpadState, PackageMode, PackageState, RideState};
use float_view_vesc_request_scheduler::{Generation, Route};

use support::*;

const VECTORS: &str = include_str!("../../../fixtures/connection/device-mutation-coordinator.tsv");

#[test]
fn shared_native_mutation_lifecycle_vectors_match_exactly() {
    let vectors = VECTORS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();
    assert_eq!(vectors.len(), 16);
    for line in vectors {
        let columns = line.split('|').collect::<Vec<_>>();
        assert_eq!(columns.len(), 5, "invalid vector: {line}");
        let effects = run(columns[0], columns[1]);
        assert_eq!(outcome(&effects), columns[2], "{}", columns[0]);
        if !columns[3].is_empty() {
            assert_eq!(kind(&effects), columns[3], "{}", columns[0]);
        }
        if !columns[4].is_empty() {
            let (_, request) = submission(&effects);
            assert_eq!(hex(&request.payload), columns[4], "{}", columns[0]);
        }
    }
}

fn run(name: &str, route: &str) -> Vec<Effect> {
    let route = match route {
        "direct" => Route::Direct,
        "can:42" => Route::Can(42),
        _ => panic!("unknown route {route}"),
    };
    let mut coordinator = DeviceMutationCoordinator::new();
    ready(&mut coordinator, route);
    match name {
        "package-idempotent" => coordinator
            .apply(Command::SetControl {
                generation: Generation(7),
                control: BooleanControl::Package,
                enabled: true,
            })
            .unwrap(),
        "package-disable" => set(&mut coordinator, BooleanControl::Package, false),
        "flywheel-enable" => set(&mut coordinator, BooleanControl::Flywheel, true),
        "flywheel-disable-active" => {
            observe(
                &mut coordinator,
                sample(2, RideState::Riding, PackageMode::Flywheel),
            );
            set(&mut coordinator, BooleanControl::Flywheel, false)
        }
        "leds-disable" => set(&mut coordinator, BooleanControl::Leds, false),
        "headlights-disable" => set(&mut coordinator, BooleanControl::Headlights, false),
        "fatal-clear" => {
            observe(
                &mut coordinator,
                sample_with(
                    2,
                    RideState::Idle,
                    float_view_refloat_telemetry_service::PackageState::Ready,
                    PackageMode::Normal,
                    float_view_refloat_telemetry_service::FootpadState::None,
                    Some(true),
                    None,
                ),
            );
            coordinator
                .apply(Command::ClearFatalError {
                    generation: Generation(7),
                })
                .unwrap()
        }
        "configuration-write" => configuration(&mut coordinator, ConfigurationMutation::Write),
        "tune-apply" => configuration(&mut coordinator, ConfigurationMutation::TuneApply),
        "shutdown-direct" | "shutdown-can" => coordinator
            .apply(Command::ShutdownController {
                generation: Generation(7),
            })
            .unwrap(),
        "reset-pending" => {
            configuration(&mut coordinator, ConfigurationMutation::Write);
            coordinator.apply(Command::Reset).unwrap()
        }
        "package-confirmed" => {
            set(&mut coordinator, BooleanControl::Package, false);
            observe(
                &mut coordinator,
                sample_with(
                    2,
                    RideState::Idle,
                    PackageState::Disabled,
                    PackageMode::Normal,
                    FootpadState::None,
                    Some(false),
                    None,
                ),
            )
        }
        "configuration-failed" => {
            let token = lease(&configuration(
                &mut coordinator,
                ConfigurationMutation::Write,
            ));
            coordinator
                .apply(Command::Fail {
                    token,
                    reason: FailureReason::RequestFailed,
                })
                .unwrap()
        }
        "configuration-cancelled" => {
            configuration(&mut coordinator, ConfigurationMutation::Write);
            coordinator
                .apply(Command::Cancel {
                    generation: Generation(7),
                })
                .unwrap()
        }
        "reconnect-replaced" => {
            configuration(&mut coordinator, ConfigurationMutation::Write);
            coordinator
                .apply(Command::Configure(ConfigureFacts {
                    generation: Generation(8),
                    route,
                    firmware: firmware(0),
                    refloat: stable_profile(),
                    connection_ready: true,
                }))
                .unwrap()
        }
        _ => panic!("unknown vector {name}"),
    }
}

fn set(
    coordinator: &mut DeviceMutationCoordinator,
    control: BooleanControl,
    enabled: bool,
) -> Vec<Effect> {
    coordinator
        .apply(Command::SetControl {
            generation: Generation(7),
            control,
            enabled,
        })
        .unwrap()
}

fn configuration(
    coordinator: &mut DeviceMutationCoordinator,
    mutation: ConfigurationMutation,
) -> Vec<Effect> {
    coordinator
        .apply(Command::BeginConfiguration {
            generation: Generation(7),
            mutation,
        })
        .unwrap()
}

fn outcome(effects: &[Effect]) -> &'static str {
    if effects
        .iter()
        .any(|effect| matches!(effect, Effect::Idempotent(_)))
    {
        "idempotent"
    } else if effects
        .iter()
        .any(|effect| matches!(effect, Effect::Submit { .. }))
    {
        "submit"
    } else if effects
        .iter()
        .any(|effect| matches!(effect, Effect::Lease(_)))
    {
        "lease"
    } else if effects.iter().any(|effect| {
        matches!(
            effect,
            Effect::Failed {
                reason: FailureReason::Cancelled,
                ..
            }
        )
    }) {
        "cancelled"
    } else if effects
        .iter()
        .any(|effect| matches!(effect, Effect::Confirmed(_)))
    {
        "confirmed"
    } else if effects.iter().any(|effect| {
        matches!(
            effect,
            Effect::Failed {
                reason: FailureReason::Replaced,
                ..
            }
        )
    }) {
        "replaced"
    } else if effects
        .iter()
        .any(|effect| matches!(effect, Effect::Failed { .. }))
    {
        "failed"
    } else {
        "unknown"
    }
}

fn kind(effects: &[Effect]) -> &'static str {
    let token = effects.iter().find_map(|effect| match effect {
        Effect::Submit { token, .. }
        | Effect::Lease(token)
        | Effect::Confirmed(token)
        | Effect::Failed { token, .. } => Some(*token),
        _ => None,
    });
    match token.expect("mutation token").kind {
        PendingKind::Package => "package",
        PendingKind::Flywheel => "flywheel",
        PendingKind::Leds => "leds",
        PendingKind::Headlights => "headlights",
        PendingKind::ClearFatalError => "clear-fatal-error",
        PendingKind::ControllerShutdown => "controller-shutdown",
        PendingKind::ConfigurationWrite => "configuration-write",
        PendingKind::TuneApply => "tune-apply",
    }
}
