#![allow(dead_code)]

use float_view_controller_discovery::{Effect, Route, Transition};
use float_view_vesc_firmware_identity::FirmwareIdentity;

pub fn identity(hardware_type: u8, minor: u8, uuid_byte: u8, test_version: u8) -> FirmwareIdentity {
    FirmwareIdentity {
        major: 6,
        minor,
        hardware_name: if hardware_type == 0 {
            "THOR300".to_owned()
        } else {
            "Gateway".to_owned()
        },
        firmware_name: None,
        controller_uuid: [uuid_byte; 12],
        test_version,
        hardware_type,
        custom_configuration_count: 0,
        hardware_configuration_crc: None,
    }
}

pub fn effect(transition: &Transition) -> Option<&Effect> {
    transition.effects.last()
}

pub fn probe_route(transition: &Transition) -> Option<Route> {
    match effect(transition) {
        Some(Effect::ProbeFirmware { route, .. }) => Some(*route),
        _ => None,
    }
}

pub fn stable_effect(transition: &Transition) -> &'static str {
    match effect(transition) {
        Some(Effect::RequestRefloatInfo { .. }) => "request_refloat",
        Some(Effect::SelectingController) => "selecting",
        Some(Effect::Failed(_)) => "failed",
        Some(Effect::ProbeFirmware { .. }) => "probe",
        Some(Effect::PingCan) => "ping",
        Some(Effect::ClearRoutingHint) => "clear_hint",
        Some(Effect::RequestTelemetryLayout) => "layout",
        Some(Effect::Ready) => "ready",
        None => "none",
    }
}

pub fn stable_route(route: Option<Route>) -> String {
    match route {
        Some(Route::Direct) => "direct".to_owned(),
        Some(Route::Can(can_id)) => format!("can:{can_id}"),
        None => String::new(),
    }
}
