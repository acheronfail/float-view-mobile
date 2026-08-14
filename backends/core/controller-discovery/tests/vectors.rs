mod support;

use float_view_controller_discovery::{
    Continuity, ControllerDiscovery, Effect, Route, RoutingHint,
};

use support::{effect, identity, probe_route, stable_effect, stable_route};

const VECTORS: &str = include_str!("../../../fixtures/connection/controller-discovery.tsv");

#[derive(Debug)]
struct Vector {
    name: String,
    transport_hardware_type: u8,
    preferred_can_id: Option<u8>,
    hint_can_id: Option<u8>,
    hint_uuid_byte: Option<u8>,
    can_ids: Vec<u8>,
    controllers: Vec<VectorController>,
    active_route_can_id: Option<u8>,
    active_uuid_byte: Option<u8>,
    expected_effect: String,
    expected_route: String,
    expected_candidates: String,
    expected_issue: String,
}

#[derive(Debug)]
struct VectorController {
    can_id: u8,
    hardware_type: u8,
    uuid_byte: u8,
    minor: u8,
}

#[test]
fn shared_native_lifecycle_vectors_match_exactly() {
    let vectors = vectors();
    assert_eq!(vectors.len(), 6, "every committed vector must be exercised");
    for vector in vectors {
        let mut discovery = ControllerDiscovery::new();
        let initial = discovery.begin();
        assert_eq!(
            probe_route(&initial),
            Some(Route::Direct),
            "{}",
            vector.name
        );
        let hint = vector.hint_can_id.map(|can_id| {
            RoutingHint::new(can_id, [vector.hint_uuid_byte.unwrap_or_default(); 12])
                .expect("fixture hint is meaningful")
        });
        let mut transition = discovery.accept_transport(
            identity(vector.transport_hardware_type, 6, 0, 0),
            vector.preferred_can_id,
            hint,
        );
        let continuity = Continuity::new(
            vector.active_route_can_id.is_some(),
            vector.active_route_can_id.map(Route::Can),
            vector.active_uuid_byte.map(|byte| [byte; 12]),
        );

        for _ in 0..128 {
            transition = match effect(&transition) {
                Some(Effect::ProbeFirmware {
                    route: Route::Can(can_id),
                    probe_generation: Some(generation),
                    ..
                }) => {
                    if let Some(controller) = vector
                        .controllers
                        .iter()
                        .find(|controller| controller.can_id == *can_id)
                    {
                        discovery.accept_can_identity(
                            *can_id,
                            identity(
                                controller.hardware_type,
                                controller.minor,
                                controller.uuid_byte,
                                0,
                            ),
                            continuity,
                        )
                    } else {
                        discovery.can_probe_timed_out(*can_id, *generation)
                    }
                }
                Some(Effect::PingCan) => discovery.accept_can_ids(&vector.can_ids, continuity),
                Some(
                    Effect::RequestRefloatInfo { .. }
                    | Effect::SelectingController
                    | Effect::Failed(_)
                    | Effect::RequestTelemetryLayout
                    | Effect::Ready,
                ) => break,
                _ => break,
            };
        }

        let snapshot = discovery.snapshot();
        assert_eq!(
            stable_effect(&transition),
            vector.expected_effect,
            "{}",
            vector.name
        );
        assert_eq!(
            stable_route(snapshot.route),
            vector.expected_route,
            "{}",
            vector.name
        );
        assert_eq!(
            snapshot
                .candidates
                .iter()
                .map(|candidate| candidate.can_id.to_string())
                .collect::<Vec<_>>()
                .join(","),
            vector.expected_candidates,
            "{}",
            vector.name
        );
        assert_eq!(
            snapshot.issue.map_or("", |issue| issue.code.as_str()),
            vector.expected_issue,
            "{}",
            vector.name
        );
    }
}

#[test]
fn complete_classified_inventory_preserves_transport_and_mixed_nodes() {
    let mut discovery = ControllerDiscovery::new();
    discovery.begin();
    assert!(matches!(
        effect(&discovery.accept_transport(identity(2, 6, 0, 0), None, None)),
        Some(Effect::PingCan)
    ));
    let continuity = Continuity::default();
    assert_eq!(
        probe_route(&discovery.accept_can_ids(&[7, 4], continuity)),
        Some(Route::Can(4))
    );
    assert_eq!(
        probe_route(&discovery.accept_can_identity(4, identity(1, 0, 4, 0), continuity)),
        Some(Route::Can(7))
    );
    assert!(matches!(
        effect(&discovery.accept_can_identity(7, identity(0, 6, 7, 0), continuity)),
        Some(Effect::RequestRefloatInfo {
            route: Route::Can(7),
            ..
        })
    ));
    let snapshot = discovery.snapshot();
    assert!(snapshot.inventory_complete);
    assert_eq!(
        snapshot
            .inventory
            .iter()
            .map(|value| value.route)
            .collect::<Vec<_>>(),
        [Route::Direct, Route::Can(4), Route::Can(7)]
    );
    assert_eq!(
        snapshot
            .inventory
            .iter()
            .map(|value| value.identity.hardware_type)
            .collect::<Vec<_>>(),
        [2, 1, 0]
    );
}

fn vectors() -> Vec<Vector> {
    VECTORS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let columns = line.split('|').collect::<Vec<_>>();
            assert_eq!(columns.len(), 13, "invalid vector: {line}");
            Vector {
                name: columns[0].to_owned(),
                transport_hardware_type: columns[1].parse().expect("transport type"),
                preferred_can_id: optional(columns[2]),
                hint_can_id: optional(columns[3]),
                hint_uuid_byte: optional(columns[4]),
                can_ids: integers(columns[5]),
                controllers: controllers(columns[6]),
                active_route_can_id: optional(columns[7]),
                active_uuid_byte: optional(columns[8]),
                expected_effect: columns[9].to_owned(),
                expected_route: columns[10].to_owned(),
                expected_candidates: columns[11].to_owned(),
                expected_issue: columns[12].to_owned(),
            }
        })
        .collect()
}

fn optional(value: &str) -> Option<u8> {
    (!value.is_empty()).then(|| value.parse().expect("optional byte"))
}

fn integers(value: &str) -> Vec<u8> {
    if value.is_empty() {
        Vec::new()
    } else {
        value
            .split(',')
            .map(|part| part.parse().expect("CAN ID"))
            .collect()
    }
}

fn controllers(value: &str) -> Vec<VectorController> {
    if value.is_empty() {
        return Vec::new();
    }
    value
        .split(';')
        .map(|encoded| {
            let values = encoded
                .split(':')
                .map(|part| part.parse::<u8>().expect("controller byte"))
                .collect::<Vec<_>>();
            assert_eq!(values.len(), 4);
            VectorController {
                can_id: values[0],
                hardware_type: values[1],
                uuid_byte: values[2],
                minor: values[3],
            }
        })
        .collect()
}
