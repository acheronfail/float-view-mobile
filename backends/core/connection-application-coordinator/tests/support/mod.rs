#![allow(dead_code)]

use float_view_connection_application_coordinator::{
    BeginFacts, Command, ConnectionApplicationCoordinator, Effect, Generation,
};
use float_view_controller_discovery::{Continuity, Route, RoutingHint};
use float_view_refloat_compatibility::{REQUIRED_FIELDS, REQUIRED_RUNTIME_FIELDS};
use float_view_refloat_telemetry_service::Settings;
use float_view_vesc_request_scheduler::{DeadlineToken, ScheduledRequest};

pub fn begin_facts(now_ms: u64) -> BeginFacts {
    BeginFacts {
        now_ms,
        settings: Settings::new(1_000, 5_000).unwrap(),
        preferred_controller_can_id: None,
        routing_hint: None,
        continuity: Continuity::default(),
        preferred_bms_can_id: None,
    }
}

pub fn write(effects: &[Effect]) -> &ScheduledRequest {
    effects
        .iter()
        .rev()
        .find_map(|effect| match effect {
            Effect::Write(request) => Some(request),
            _ => None,
        })
        .expect("write effect")
}

pub fn deadline(effects: &[Effect]) -> DeadlineToken {
    effects
        .iter()
        .rev()
        .find_map(|effect| match effect {
            Effect::ArmDeadline { token, .. } => Some(*token),
            _ => None,
        })
        .expect("deadline effect")
}

pub fn receive(
    coordinator: &mut ConnectionApplicationCoordinator,
    payload: Vec<u8>,
    now_ms: u64,
) -> Vec<Effect> {
    let generation = coordinator.snapshot().generation;
    coordinator
        .apply(Command::Receive {
            generation,
            payload,
            now_ms,
            observed_at_epoch_ms: 1_700_000_000_000 + now_ms as i64,
        })
        .expect("valid response")
}

pub fn activate(coordinator: &mut ConnectionApplicationCoordinator, now_ms: u64) -> Vec<Effect> {
    activate_with_cache(coordinator, now_ms, None)
}

pub fn activate_with_cache(
    coordinator: &mut ConnectionApplicationCoordinator,
    now_ms: u64,
    cached_configuration_schema: Option<Vec<u8>>,
) -> Vec<Effect> {
    coordinator
        .apply(Command::Activate {
            generation: coordinator.snapshot().generation,
            now_ms,
            session_started: true,
            cached_configuration_schema,
        })
        .expect("valid activation")
}

pub fn motor_payload(uuid_byte: u8) -> Vec<u8> {
    firmware_payload(0, 6, b"THOR300", uuid_byte, b"Float")
}

pub fn motor_configuration_payload(uuid_byte: u8) -> Vec<u8> {
    let mut payload = vec![0, 6, 6];
    payload.extend_from_slice(b"THOR300");
    payload.push(0);
    payload.extend_from_slice(&[uuid_byte; 12]);
    payload.extend_from_slice(&[1, 0, 0, 1]);
    payload.extend_from_slice(&[1, 0, 1, 0]);
    payload.extend_from_slice(b"Float");
    payload.push(0);
    payload.extend_from_slice(&123_u32.to_be_bytes());
    payload
}

pub fn gateway_payload(uuid_byte: u8) -> Vec<u8> {
    firmware_payload(5, 6, b"Express", uuid_byte, b"Gateway")
}

fn firmware_payload(
    hardware_type: u8,
    minor: u8,
    hardware_name: &[u8],
    uuid_byte: u8,
    firmware_name: &[u8],
) -> Vec<u8> {
    let mut payload = vec![0, 6, minor];
    payload.extend_from_slice(hardware_name);
    payload.push(0);
    payload.extend_from_slice(&[uuid_byte; 12]);
    payload.extend_from_slice(&[1, 0, hardware_type, 0]);
    payload.extend_from_slice(&[1, 0, 1, 0]);
    payload.extend_from_slice(firmware_name);
    payload.push(0);
    payload
}

pub fn stock_bms_payload() -> Vec<u8> {
    let mut payload = vec![0, 6, 0];
    payload.extend_from_slice(b"VBMS\0");
    payload.extend(0_u8..12);
    payload.extend_from_slice(&[1, 2, 1, 3]);
    payload
}

pub fn legacy_info_payload() -> Vec<u8> {
    vec![36, 101, 0, 10]
}

pub fn dynamic_info_payload() -> Vec<u8> {
    let mut payload = vec![0; 61];
    payload[..5].copy_from_slice(&[36, 101, 0, 2, 0]);
    payload[5..12].copy_from_slice(b"refloat");
    payload[25..28].copy_from_slice(&[1, 2, 3]);
    payload
}

pub fn selectable_info_payload() -> Vec<u8> {
    let mut payload = vec![0; 61];
    payload[..5].copy_from_slice(&[36, 101, 0, 2, 0]);
    payload[5..12].copy_from_slice(b"refloat");
    payload[25..28].copy_from_slice(&[1, 3, 0]);
    payload[59] = 4;
    payload
}

pub fn dynamic_layout_payload() -> Vec<u8> {
    let mut payload = vec![36, 101, 32, REQUIRED_FIELDS.len() as u8];
    for field in REQUIRED_FIELDS {
        payload.push(field.len() as u8);
        payload.extend_from_slice(field.as_bytes());
    }
    payload.push(REQUIRED_RUNTIME_FIELDS.len() as u8);
    for field in REQUIRED_RUNTIME_FIELDS {
        payload.push(field.len() as u8);
        payload.extend_from_slice(field.as_bytes());
    }
    payload
}

pub fn legacy_idle_payload() -> Vec<u8> {
    fixture_payload(
        include_str!("../../../../fixtures/refloat/protocol-1.0.0.json"),
        "legacy-idle-all-data",
    )
}

pub fn dynamic_idle_payload() -> Vec<u8> {
    fixture_payload(
        include_str!("../../../../fixtures/refloat/rtdata-1.2.3-android-hardware.json"),
        "idle-realtime-data",
    )
}

pub fn selectable_fast_payload() -> Vec<u8> {
    selectable_fixture("1.3.0-command-33-fast-float32")
}

pub fn selectable_fixture(name: &str) -> Vec<u8> {
    let document = include_str!("../../../../fixtures/refloat/detailed-source-provenance.json");
    let marker = format!("\"name\": \"{name}\"");
    let case = document.split_once(&marker).expect("selectable fixture").1;
    hex_bytes(string_field(case, "payloadHex"))
}

pub fn configuration_schema_1_3_0() -> Vec<u8> {
    include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.3.0.qcompress").to_vec()
}

pub fn configuration_current_response_1_3_0() -> Vec<u8> {
    configuration_values_response(
        93,
        include_bytes!("../../../../fixtures/refloat-configuration/values/1.3.0-current.bin"),
    )
}

pub fn configuration_schema_1_2_3() -> Vec<u8> {
    include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.2.3.qcompress").to_vec()
}

pub fn configuration_current_response_1_2_3() -> Vec<u8> {
    configuration_values_response(
        93,
        include_bytes!("../../../../fixtures/refloat-configuration/values/1.2.3-current.bin"),
    )
}

pub fn configuration_default_response_1_2_3() -> Vec<u8> {
    configuration_values_response(
        94,
        include_bytes!("../../../../fixtures/refloat-configuration/values/1.2.3-default.bin"),
    )
}

pub fn configuration_readback_response_1_2_3() -> Vec<u8> {
    configuration_values_response(
        93,
        include_bytes!("../../../../fixtures/refloat-configuration/values/1.2.3-readback.bin"),
    )
}

pub fn configuration_write_values_1_2_3() -> &'static [u8] {
    include_bytes!("../../../../fixtures/refloat-configuration/values/1.2.3-write.bin")
}

pub fn current_limits_payload(signature: u32, battery_series_cells: Option<u8>) -> Vec<u8> {
    let mut payload = vec![14];
    payload.extend_from_slice(&signature.to_be_bytes());
    payload.extend_from_slice(&[0; 4]);
    for raw in [
        0x42C8_0000_u32,
        0xC2C8_0000_u32,
        0x4248_0000_u32,
        0xC1A0_0000_u32,
    ] {
        payload.extend_from_slice(&raw.to_be_bytes());
    }
    if let Some(cells) = battery_series_cells {
        payload.resize(459, 0);
        payload[458] = cells;
    }
    payload
}

fn configuration_values_response(command: u8, bytes: &[u8]) -> Vec<u8> {
    let mut response = vec![command, 0];
    response.extend_from_slice(bytes);
    response
}

fn fixture_payload(document: &str, name: &str) -> Vec<u8> {
    let marker = format!("\"name\": \"{name}\"");
    let case = document.split_once(&marker).expect("fixture case").1;
    let frame = string_field(case, "frameHex");
    let bytes = hex_bytes(frame);
    match bytes.first() {
        Some(2) => {
            let length = usize::from(bytes[1]);
            bytes[2..2 + length].to_vec()
        }
        Some(3) => {
            let length = usize::from(u16::from_be_bytes([bytes[1], bytes[2]]));
            bytes[3..3 + length].to_vec()
        }
        value => panic!("unsupported fixture frame: {value:?}"),
    }
}

fn string_field<'a>(document: &'a str, field: &str) -> &'a str {
    let marker = format!("\"{field}\": \"");
    document
        .split_once(&marker)
        .and_then(|(_, suffix)| suffix.split_once('"'))
        .map(|(value, _)| value)
        .expect("fixture string field")
}

fn hex_bytes(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}

pub fn bms_values_payload() -> Vec<u8> {
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
    payload.push(2);
    payload.extend_from_slice(&4_000_i16.to_be_bytes());
    payload.extend_from_slice(&4_001_i16.to_be_bytes());
    payload.extend([0, 1]);
    payload.push(2);
    payload.extend_from_slice(&2_000_i16.to_be_bytes());
    payload.extend_from_slice(&2_001_i16.to_be_bytes());
    for value in [2_750_i16, 2_475, 5_250, 2_925, 875, 950] {
        payload.extend_from_slice(&value.to_be_bytes());
    }
    payload.push(42);
    for raw in [0x4280_0000_u32, 0x4480_0000, 0x4200_0000, 0x4400_0000] {
        payload.extend_from_slice(&raw.to_be_bytes());
    }
    payload.extend_from_slice(&10_132_i16.to_be_bytes());
    payload.extend_from_slice(&[7]);
    payload.extend_from_slice(b"Operational\0");
    payload
}

pub fn route_text(route: Option<Route>) -> String {
    match route {
        Some(Route::Direct) => "direct".to_owned(),
        Some(Route::Can(can_id)) => format!("can:{can_id}"),
        None => String::new(),
    }
}

pub fn hint(can_id: u8, uuid_byte: u8) -> RoutingHint {
    RoutingHint::new(can_id, [uuid_byte; 12]).unwrap()
}

pub fn stale_generation(current: Generation) -> Generation {
    Generation(current.0.wrapping_sub(1))
}
