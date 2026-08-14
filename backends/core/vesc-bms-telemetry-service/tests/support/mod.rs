#![allow(dead_code)]

use float_view_auxiliary_node_discovery::{Route, RoutedIdentity};
use float_view_vesc_bms_telemetry_service::{
    AuxiliaryGeneration, Command, Configuration, Effect, RequestKind, RequestOutcome,
    SchedulerGeneration, TimerToken, VescBmsTelemetryService,
};
use float_view_vesc_firmware_identity::FirmwareIdentity;

pub fn routed(
    route: Route,
    hardware_type: u8,
    major: u8,
    minor: u8,
    hardware_name: &str,
) -> RoutedIdentity {
    RoutedIdentity {
        route,
        identity: FirmwareIdentity {
            major,
            minor,
            hardware_name: hardware_name.to_owned(),
            firmware_name: None,
            controller_uuid: [42; 12],
            test_version: 0,
            hardware_type,
            custom_configuration_count: 0,
            hardware_configuration_crc: None,
        },
    }
}

pub fn controller(route: Route) -> RoutedIdentity {
    routed(route, 0, 6, 5, "75_300_R2")
}

pub fn bms(route: Route) -> RoutedIdentity {
    routed(route, 2, 6, 6, "VBMS32")
}

pub fn direct_configuration(service_generation: u64) -> Configuration {
    Configuration {
        service_generation,
        scheduler_generation: SchedulerGeneration(service_generation + 10),
        local_identity: bms(Route::Direct),
        reusable_inventory: None,
        preferred_can_id: None,
    }
}

pub fn can_configuration(service_generation: u64, preferred: bool) -> Configuration {
    let local = controller(Route::Direct);
    Configuration {
        service_generation,
        scheduler_generation: SchedulerGeneration(service_generation + 10),
        local_identity: local.clone(),
        reusable_inventory: Some(vec![local, bms(Route::Can(42))]),
        preferred_can_id: preferred.then_some(42),
    }
}

pub fn begin(service: &mut VescBmsTelemetryService, configuration: Configuration) -> Vec<Effect> {
    service
        .apply(Command::Begin(configuration))
        .expect("valid begin")
}

pub fn pending_identity(
    service: &VescBmsTelemetryService,
    effects: &[Effect],
) -> (u64, AuxiliaryGeneration, RequestKind, Option<Route>) {
    let Effect::Enqueue { kind, route, .. } = effects.last().expect("enqueue effect") else {
        panic!("expected enqueue: {effects:?}");
    };
    let snapshot = service.snapshot();
    (
        snapshot.service_generation.expect("service generation"),
        snapshot.auxiliary_generation,
        *kind,
        *route,
    )
}

pub fn complete(
    service: &mut VescBmsTelemetryService,
    kind: RequestKind,
    route: Option<Route>,
    outcome: RequestOutcome,
) -> Vec<Effect> {
    let snapshot = service.snapshot();
    service
        .apply(Command::RequestCompleted {
            service_generation: snapshot.service_generation.expect("service generation"),
            auxiliary_generation: snapshot.auxiliary_generation,
            kind,
            route,
            outcome,
        })
        .expect("completion")
}

pub fn fire_timer(service: &mut VescBmsTelemetryService) -> Vec<Effect> {
    let token = service.snapshot().timer.expect("poll timer");
    service
        .apply(Command::TimerFired(token))
        .expect("timer fired")
}

pub fn receive_valid(
    service: &mut VescBmsTelemetryService,
    route: Route,
    observed_at_epoch_ms: i64,
) -> Vec<Effect> {
    let snapshot = service.snapshot();
    service
        .apply(Command::ReceiveValues {
            service_generation: snapshot.service_generation.expect("service generation"),
            auxiliary_generation: snapshot.auxiliary_generation,
            route,
            payload: values_payload(
                2,
                2,
                route.can_id().unwrap_or(42),
                Some((7, b"Operational")),
            ),
            observed_at_epoch_ms,
        })
        .expect("values response")
}

pub fn values_payload(
    cells: usize,
    temperatures: usize,
    reported_can_id: u8,
    extended: Option<(u8, &[u8])>,
) -> Vec<u8> {
    assert!(cells <= 50);
    assert!(temperatures <= 50);
    let mut payload = vec![96];
    for value in [
        50_400_000_i32,
        54_600_000,
        -12_345_000,
        -12_300_000,
        1_250,
        62_500,
    ] {
        payload.extend_from_slice(&value.to_be_bytes());
    }
    payload.push(cells as u8);
    for index in 0..cells {
        payload.extend_from_slice(&(4_000_i16 + index as i16).to_be_bytes());
    }
    payload.extend((0..cells).map(|index| (index % 2) as u8));
    payload.push(temperatures as u8);
    for index in 0..temperatures {
        payload.extend_from_slice(&(2_000_i16 + index as i16).to_be_bytes());
    }
    for value in [2_750_i16, 2_475, 5_250, 2_925, 875, 950] {
        payload.extend_from_slice(&value.to_be_bytes());
    }
    payload.push(reported_can_id);
    for raw in [0x4280_0000_u32, 0x4480_0000, 0x4200_0000, 0x4400_0000] {
        payload.extend_from_slice(&raw.to_be_bytes());
    }
    payload.extend_from_slice(&10_132_i16.to_be_bytes());
    if let Some((version, status)) = extended {
        payload.push(version);
        payload.extend_from_slice(status);
        payload.push(0);
    }
    payload
}

pub fn timer(effects: &[Effect]) -> Option<TimerToken> {
    effects.iter().find_map(|effect| match effect {
        Effect::ReplaceTimer { token, .. } => Some(*token),
        _ => None,
    })
}
