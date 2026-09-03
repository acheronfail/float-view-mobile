mod support;

use float_view_auxiliary_node_discovery::{AuxiliaryNodeDiscovery, Effect, Route, RoutedIdentity};

use support::{effect, local, routed, stable_effect, stable_route};

const VECTORS: &str = include_str!("../../../fixtures/connection/auxiliary-node-discovery.tsv");

#[derive(Debug)]
struct Vector {
    name: String,
    local: Identity,
    preferred_can_id: Option<u8>,
    inventory: Option<Vec<Node>>,
    can_ids: Vec<u8>,
    probed: Vec<Node>,
    lifecycle: String,
    expected_status: String,
    expected_effect: String,
    expected_route: String,
    expected_candidates: String,
    expected_issue: String,
    expected_observation: String,
}

#[derive(Clone, Debug)]
struct Identity {
    hardware_type: u8,
    major: u8,
    minor: u8,
    hardware_name: String,
}

#[derive(Clone, Debug)]
struct Node {
    can_id: u8,
    identity: Identity,
}

#[test]
fn shared_native_lifecycle_vectors_match_exactly() {
    let vectors = vectors();
    assert_eq!(
        vectors.len(),
        13,
        "every committed vector must be exercised"
    );
    for vector in vectors {
        let local = local(
            vector.local.hardware_type,
            vector.local.major,
            vector.local.minor,
            &vector.local.hardware_name,
        );
        let inventory = vector.inventory.as_ref().map(|nodes| {
            let mut values = vec![local.clone()];
            values.extend(nodes.iter().map(node));
            values
        });
        let mut discovery = AuxiliaryNodeDiscovery::<&'static str>::new();
        let mut transition = discovery
            .begin(local, inventory.as_deref(), vector.preferred_can_id)
            .unwrap_or_else(|error| panic!("{}: {error:?}", vector.name));

        for _ in 0..128 {
            transition = match effect(&transition) {
                Some(Effect::PingCan { generation }) => {
                    discovery.accept_can_ids(&vector.can_ids, generation)
                }
                Some(Effect::ProbeFirmware { route, generation }) => {
                    let can_id = route.can_id().expect("only CAN routes are probed");
                    if let Some(value) = vector.probed.iter().find(|node| node.can_id == can_id) {
                        discovery.accept_firmware(node(value), generation)
                    } else {
                        discovery.probe_timed_out(route, generation)
                    }
                }
                Some(Effect::RequestValues { .. }) | None => break,
            };
        }

        match vector.lifecycle.as_str() {
            "" => {}
            "observe-unavailable-recover" => {
                let generation = discovery.generation();
                let route = discovery.state().connection.expect("selected route");
                discovery.accept_observation(route, generation, "first");
                discovery.mark_unavailable(route, generation);
                discovery.accept_observation(route, generation, "second");
            }
            "reset" => {
                discovery.reset();
                transition = Default::default();
            }
            value => panic!("{}: unknown lifecycle {value}", vector.name),
        }

        let state = discovery.state();
        let status = state.status.as_str();
        assert_eq!(status, vector.expected_status, "{}", vector.name);
        assert_eq!(
            stable_effect(&transition),
            vector.expected_effect,
            "{}",
            vector.name
        );
        assert_eq!(
            stable_route(state.connection),
            vector.expected_route,
            "{}",
            vector.name
        );
        assert_eq!(
            state
                .candidates
                .iter()
                .map(|candidate| stable_route(Some(candidate.route)))
                .collect::<Vec<_>>()
                .join(","),
            vector.expected_candidates,
            "{}",
            vector.name
        );
        assert_eq!(
            state.issue.map_or("", |issue| issue.code.as_str()),
            vector.expected_issue,
            "{}",
            vector.name
        );
        assert_eq!(
            state.latest_observation.unwrap_or(""),
            vector.expected_observation,
            "{}",
            vector.name
        );
    }
}

fn node(value: &Node) -> RoutedIdentity {
    routed(
        Route::Can(value.can_id),
        value.identity.hardware_type,
        value.identity.major,
        value.identity.minor,
        &value.identity.hardware_name,
    )
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
                local: identity(columns[1]),
                preferred_can_id: optional(columns[2]),
                inventory: inventory(columns[3]),
                can_ids: integers(columns[4]),
                probed: nodes(columns[5]),
                lifecycle: columns[6].to_owned(),
                expected_status: columns[7].to_owned(),
                expected_effect: columns[8].to_owned(),
                expected_route: columns[9].to_owned(),
                expected_candidates: columns[10].to_owned(),
                expected_issue: columns[11].to_owned(),
                expected_observation: columns[12].to_owned(),
            }
        })
        .collect()
}

fn identity(value: &str) -> Identity {
    let fields = value.split(':').collect::<Vec<_>>();
    assert_eq!(fields.len(), 4, "identity: {value}");
    Identity {
        hardware_type: fields[0].parse().expect("hardware type"),
        major: fields[1].parse().expect("major"),
        minor: fields[2].parse().expect("minor"),
        hardware_name: fields[3].to_owned(),
    }
}

fn inventory(value: &str) -> Option<Vec<Node>> {
    match value {
        "none" => None,
        "empty" => Some(Vec::new()),
        _ => Some(nodes(value)),
    }
}

fn nodes(value: &str) -> Vec<Node> {
    if value.is_empty() {
        return Vec::new();
    }
    value
        .split(';')
        .map(|encoded| {
            let mut fields = encoded.splitn(2, ':');
            Node {
                can_id: fields.next().unwrap().parse().expect("CAN ID"),
                identity: identity(fields.next().expect("node identity")),
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
