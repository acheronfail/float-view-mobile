#![allow(dead_code)]

use float_view_auxiliary_node_discovery::{Effect, Route, RoutedIdentity, Transition};
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

pub fn local(hardware_type: u8, major: u8, minor: u8, hardware_name: &str) -> RoutedIdentity {
    routed(Route::Direct, hardware_type, major, minor, hardware_name)
}

pub fn effect(transition: &Transition) -> Option<Effect> {
    transition.effects.last().copied()
}

pub fn stable_effect(transition: &Transition) -> &'static str {
    match effect(transition) {
        Some(Effect::PingCan { .. }) => "ping_can",
        Some(Effect::ProbeFirmware { .. }) => "probe_firmware",
        Some(Effect::RequestValues { .. }) => "request_values",
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
