mod support;

use float_view_auxiliary_node_discovery::{Route, Status};
use float_view_vesc_bms_telemetry_service::{
    Command, Effect, RequestKind, RequestOutcome, VescBmsTelemetryService,
};

use support::{
    begin, bms, can_configuration, complete, controller, direct_configuration, fire_timer,
    pending_identity, receive_valid,
};

const VECTORS: &str = include_str!("../../../fixtures/connection/vesc-bms-telemetry-service.tsv");

#[test]
fn shared_native_bms_lifecycle_vectors_match_exactly() {
    let vectors = VECTORS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();
    assert_eq!(vectors.len(), 12, "every committed vector must run");

    for line in vectors {
        let columns = line.split('|').collect::<Vec<_>>();
        assert_eq!(columns.len(), 7, "invalid vector: {line}");
        let name = columns[0];
        let mut service = VescBmsTelemetryService::new();
        let mut effects = match columns[1] {
            "direct" => begin(&mut service, direct_configuration(1)),
            "reused-can" => begin(&mut service, can_configuration(1, false)),
            "preferred-can" => begin(&mut service, can_configuration(1, true)),
            "enumerated-can" | "enumerated-none" => {
                let local = controller(Route::Direct);
                let effects = begin(
                    &mut service,
                    float_view_vesc_bms_telemetry_service::Configuration {
                        service_generation: 1,
                        scheduler_generation:
                            float_view_vesc_bms_telemetry_service::SchedulerGeneration(11),
                        local_identity: local,
                        reusable_inventory: None,
                        preferred_can_id: None,
                    },
                );
                let (_, generation, RequestKind::CanScan, None) =
                    pending_identity(&service, &effects)
                else {
                    panic!("{name}: expected scan");
                };
                complete(
                    &mut service,
                    RequestKind::CanScan,
                    None,
                    RequestOutcome::Response,
                );
                let ids = if columns[1] == "enumerated-can" {
                    vec![42]
                } else {
                    vec![]
                };
                let probe = service
                    .apply(Command::AcceptCanIds {
                        service_generation: 1,
                        auxiliary_generation: generation,
                        can_ids: ids,
                    })
                    .expect("CAN IDs");
                if columns[1] == "enumerated-none" {
                    probe
                } else {
                    complete(
                        &mut service,
                        RequestKind::Firmware,
                        Some(Route::Can(42)),
                        RequestOutcome::Response,
                    );
                    service
                        .apply(Command::AcceptFirmware {
                            service_generation: 1,
                            auxiliary_generation: generation,
                            identity: bms(Route::Can(42)),
                        })
                        .expect("firmware")
                }
            }
            setup => panic!("{name}: unknown setup {setup}"),
        };

        for (index, action) in columns[2]
            .split(',')
            .filter(|value| !value.is_empty())
            .enumerate()
        {
            if index > 0 && action != "reset" {
                fire_timer(&mut service);
            }
            let route = service.snapshot().route.expect("selected route");
            effects = match action {
                "valid" => {
                    complete(
                        &mut service,
                        RequestKind::Values,
                        Some(route),
                        RequestOutcome::Response,
                    );
                    receive_valid(&mut service, route, 1_700_000_000_000 + index as i64)
                }
                "malformed" => {
                    complete(
                        &mut service,
                        RequestKind::Values,
                        Some(route),
                        RequestOutcome::Response,
                    );
                    let snapshot = service.snapshot();
                    service
                        .apply(Command::ReceiveValues {
                            service_generation: snapshot.service_generation.unwrap(),
                            auxiliary_generation: snapshot.auxiliary_generation,
                            route,
                            payload: vec![96],
                            observed_at_epoch_ms: 1_700_000_000_000,
                        })
                        .unwrap()
                }
                "timeout" => complete(
                    &mut service,
                    RequestKind::Values,
                    Some(route),
                    RequestOutcome::TimedOut,
                ),
                "queue_full" => complete(
                    &mut service,
                    RequestKind::Values,
                    Some(route),
                    RequestOutcome::QueueFull,
                ),
                "cancelled" => complete(
                    &mut service,
                    RequestKind::Values,
                    Some(route),
                    RequestOutcome::Cancelled,
                ),
                "reset" => service.apply(Command::Reset).unwrap(),
                value => panic!("{name}: unknown lifecycle {value}"),
            };
        }

        let snapshot = service.snapshot();
        let expected_status = match columns[3] {
            "discovering" => Status::Discovering,
            "not_found" => Status::NotFound,
            "attached" => Status::Attached,
            "unavailable" => Status::Unavailable,
            value => panic!("{name}: status {value}"),
        };
        assert_eq!(snapshot.status, expected_status, "{name}");
        assert_eq!(
            snapshot.consecutive_failures,
            columns[4].parse::<u8>().unwrap(),
            "{name}"
        );
        assert_eq!(snapshot.has_observation, columns[5] == "yes", "{name}");
        assert_eq!(snapshot.timer.is_some(), columns[6] == "yes", "{name}");
        assert!(
            effects.len() <= 3,
            "{name}: effects remain bounded: {effects:?}"
        );
        assert!(effects.iter().all(|effect| !matches!(
            effect,
            Effect::Enqueue { facts, .. }
                if facts
                    .payload
                    .first()
                    .is_some_and(|command| ![0, 34, 62, 96].contains(command))
        )));
    }
}
