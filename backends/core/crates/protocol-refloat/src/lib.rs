//! Refloat telemetry, control, compatibility, and configuration owner.

mod compatibility;
mod configuration;
mod controls;
mod telemetry;

pub use compatibility::*;
pub use configuration::*;
pub use controls::*;
pub use telemetry::*;

pub const PACKAGE_ID: u8 = 101;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum RefloatCommand {
    Info = 0,
    GetAllData = 10,
    PackageLock = 12,
    StableLights = 20,
    Flywheel = 22,
    RealtimeDataInternal = 31,
    RealtimeDataInternalIds = 32,
    AlertsControl = 36,
    LegacyLights = 202,
}

#[must_use]
pub fn request(command: RefloatCommand, parameters: &[u8]) -> Vec<u8> {
    let mut request = vec![
        protocol_vesc::VescCommand::CustomAppData as u8,
        PACKAGE_ID,
        command as u8,
    ];
    request.extend_from_slice(parameters);
    request
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PollKind {
    Layout,
    Realtime,
    Compact,
}

#[must_use]
pub fn poll_payload(
    family: TelemetryFamily,
    kind: PollKind,
    route: protocol_vesc::ControllerRoute,
) -> Vec<u8> {
    let command = match (family, kind) {
        (_, PollKind::Layout) => request(RefloatCommand::RealtimeDataInternalIds, &[]),
        (TelemetryFamily::Dynamic, PollKind::Realtime) => {
            request(RefloatCommand::RealtimeDataInternal, &[])
        }
        (TelemetryFamily::LegacyFixed, PollKind::Realtime | PollKind::Compact)
        | (TelemetryFamily::Dynamic, PollKind::Compact) => {
            request(RefloatCommand::GetAllData, &[4])
        }
    };
    protocol_vesc::route_command(&command, route)
}

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures {
    pub const PROTOCOL_100: &str = include_str!("../../../../fixtures/refloat/protocol-1.0.0.json");
    pub const PROTOCOL_110: &str = include_str!("../../../../fixtures/refloat/protocol-1.1.0.json");
    pub const PROTOCOL_123: &str = include_str!("../../../../fixtures/refloat/protocol-1.2.3.json");
    pub const TELEMETRY_CATALOG: &str =
        include_str!("../../../../fixtures/refloat/telemetry-catalog.json");
    pub const TELEMETRY_LIFECYCLE: &str =
        include_str!("../../../../fixtures/refloat/telemetry-lifecycle.json");
    pub const PRODUCT_CONTROLS: &str =
        include_str!("../../../../fixtures/refloat/product-controls.json");
    pub const CONFIGURATION_MANIFEST: &str =
        include_str!("../../../../fixtures/refloat-configuration/manifest.json");
    pub const CONFIGURATION_SCHEMA_123: &[u8] =
        include_bytes!("../../../../fixtures/refloat-configuration/schemas/1.2.3.qcompress");
    pub const CONFIGURATION_VALUES_123: &[u8] =
        include_bytes!("../../../../fixtures/refloat-configuration/values/1.2.3-current.bin");
}

#[cfg(test)]
mod tests {
    use domain_types::CanId;
    use domain_types::RideState;
    use protocol_vesc::{ControllerRoute, decode_frame};
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ProtocolFixture {
        cases: Vec<ProtocolCase>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ProtocolCase {
        name: String,
        frame_hex: String,
    }

    #[derive(Deserialize)]
    struct ControlFixture {
        cases: Vec<ControlCase>,
    }

    #[derive(Deserialize)]
    struct ControlCase {
        name: String,
        request: Vec<u8>,
        response: Option<Vec<u8>>,
    }

    #[derive(Deserialize)]
    struct ShutdownFixture {
        cases: Vec<ShutdownCase>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ShutdownCase {
        route: String,
        can_id: Option<u8>,
        request: Vec<u8>,
    }

    fn bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn case<'a>(fixture: &'a ProtocolFixture, name: &str) -> &'a ProtocolCase {
        fixture.cases.iter().find(|case| case.name == name).unwrap()
    }

    #[test]
    fn compatibility_catalog_layout_and_polling_are_exact() {
        assert_eq!(COMPATIBILITY.len(), 9);
        assert_eq!(
            compatibility("1.0").unwrap().family(),
            TelemetryFamily::LegacyFixed
        );
        assert_eq!(
            compatibility("1.2.3").unwrap().profile,
            WireProfile::DynamicAlerts
        );
        assert_eq!(compatibility("1.3.0"), None);
        let mut fields: Vec<String> = REQUIRED_FIELDS.iter().map(ToString::to_string).collect();
        fields.push("future.energy".into());
        let runtime: Vec<String> = REQUIRED_RUNTIME_FIELDS
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(supports_dynamic_layout(&fields, &runtime));
        fields.remove(0);
        assert!(!supports_dynamic_layout(&fields, &runtime));

        assert_eq!(
            poll_payload(
                TelemetryFamily::LegacyFixed,
                PollKind::Realtime,
                ControllerRoute::Direct
            ),
            vec![36, 101, 10, 4]
        );
        assert_eq!(
            poll_payload(
                TelemetryFamily::Dynamic,
                PollKind::Layout,
                ControllerRoute::Can(CanId::new(42).unwrap())
            ),
            vec![34, 42, 36, 101, 32]
        );
    }

    #[test]
    fn legacy_golden_vectors_normalize_idle_and_riding() {
        let fixture: ProtocolFixture = serde_json::from_str(fixtures::PROTOCOL_100).unwrap();
        let idle = decode_legacy(
            &decode_frame(&bytes(&case(&fixture, "legacy-idle-all-data").frame_hex)).unwrap(),
        )
        .unwrap();
        assert_eq!(idle.package_state, PackageState::Startup);
        assert_eq!(idle.ride_state, RideState::Idle);
        let riding = decode_legacy(
            &decode_frame(&bytes(&case(&fixture, "legacy-riding-all-data").frame_hex)).unwrap(),
        )
        .unwrap();
        assert_eq!(riding.package_state, PackageState::Running);
        assert_eq!(riding.ride_state, RideState::Riding);
        assert_eq!(riding.footpad_state, FootpadState::Both);
        assert!((riding.numeric["packVoltageV"] - 62.7).abs() < f64::EPSILON);
        assert!((riding.numeric["speedKph"] - 20.88).abs() < f64::EPSILON);
    }

    #[test]
    fn dynamic_golden_layout_and_data_are_bounded_and_normalized() {
        let fixture: ProtocolFixture = serde_json::from_str(fixtures::PROTOCOL_110).unwrap();
        let layout_payload =
            decode_frame(&bytes(&case(&fixture, "dynamic-field-ids").frame_hex)).unwrap();
        let layout = decode_layout(&layout_payload).unwrap();
        assert_eq!(layout.fields.len(), 8);
        assert_eq!(layout.fields[0], "motor.speed");
        assert!(layout.runtime_fields.is_empty());

        let riding_payload =
            decode_frame(&bytes(&case(&fixture, "dynamic-riding").frame_hex)).unwrap();
        let riding = decode_dynamic(&riding_payload, &layout).unwrap();
        assert_eq!(riding.package_state, PackageState::Running);
        assert_eq!(riding.ride_state, RideState::Riding);
        assert!((riding.numeric["speedKph"] - 5.785_156_25).abs() < f64::EPSILON);
        assert_eq!(riding.fatal_error_latched, None);
    }

    #[test]
    fn detailed_catalog_and_decoder_bounds_are_enforced() {
        let catalog: serde_json::Value = serde_json::from_str(fixtures::TELEMETRY_CATALOG).unwrap();
        assert_eq!(catalog["bounds"]["layoutFields"], MAX_FIELDS);
        assert_eq!(catalog["bounds"]["extensionFields"], MAX_EXTENSIONS);
        for field in NORMALIZED_NUMERIC_FIELDS {
            assert!(catalog["numericFields"].get(field).is_some(), "{field}");
        }

        let duplicate = [36, 101, 32, 2, 1, b'a', 1, b'a', 0];
        assert_eq!(decode_layout(&duplicate), Err(TelemetryError::InvalidFrame));
        let nan = [36, 101, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x7c, 0x00];
        assert!(matches!(
            decode_dynamic(
                &nan,
                &TelemetryLayout {
                    fields: vec!["motor.speed".into()],
                    runtime_fields: Vec::new()
                }
            ),
            Err(TelemetryError::InvalidFrame | TelemetryError::Truncated)
        ));
    }

    #[test]
    fn source_derived_controls_lighting_and_shutdown_are_exact() {
        let fixture: ControlFixture = serde_json::from_str(fixtures::PRODUCT_CONTROLS).unwrap();
        for item in fixture.cases {
            let profile = if item.name.contains("stable") || item.name == "fatal-clear" {
                compatibility("1.2.3").unwrap().control_profile()
            } else {
                compatibility("1.0").unwrap().control_profile()
            };
            let payload = match item.name.as_str() {
                "package-enable" => package_request(true),
                "package-disable" => package_request(false),
                "flywheel-enable-defaults-footpad-abort" => flywheel_request(true),
                "flywheel-disable-defaults-footpad-abort" => flywheel_request(false),
                "legacy-lighting-leds-off-headlights-on" => lighting_request(
                    profile,
                    Some(BooleanControl::Leds),
                    false,
                    LightingState {
                        leds_enabled: true,
                        headlights_enabled: true,
                    },
                )
                .unwrap(),
                "stable-lighting-headlights-off-leds-on" => lighting_request(
                    profile,
                    Some(BooleanControl::Headlights),
                    false,
                    LightingState {
                        leds_enabled: true,
                        headlights_enabled: true,
                    },
                )
                .unwrap(),
                "fatal-clear" => fatal_clear_request(profile).unwrap(),
                _ => panic!("unknown fixture case"),
            };
            assert_eq!(payload, item.request, "{}", item.name);
            if let Some(response) = item.response {
                let lighting = decode_lighting(&response, profile).unwrap();
                assert_eq!(lighting.leds_enabled, response[3] & 1 != 0);
                assert_eq!(lighting.headlights_enabled, response[3] & 2 != 0);
            }
        }
        assert!(
            decode_lighting(
                &[36, 101, 20],
                compatibility("1.2.3").unwrap().control_profile()
            )
            .is_err()
        );
        assert!(fatal_clear_request(compatibility("1.1.2").unwrap().control_profile()).is_err());

        let shutdown: ShutdownFixture =
            serde_json::from_str(protocol_vesc::fixtures::CONTROLLER_SHUTDOWN).unwrap();
        for item in shutdown.cases {
            let route = if item.route == "can" {
                ControllerRoute::Can(CanId::new(item.can_id.unwrap()).unwrap())
            } else {
                ControllerRoute::Direct
            };
            assert_eq!(protocol_vesc::shutdown_command(route), item.request);
        }
    }

    #[test]
    fn mutation_owner_enforces_motion_pending_and_flywheel_boundaries() {
        let profile = compatibility("1.2.3").unwrap().control_profile();
        let context = MutationContext {
            generation: 1,
            profile,
            connection_ready: true,
            writable: true,
            ride_state: RideState::Idle,
            package_state: PackageState::Ready,
            package_mode: PackageMode::Normal,
        };
        let mut owner = MutationCoordinator::new(1, true);
        owner.observe(
            PackageState::Ready,
            PackageMode::Normal,
            Some(LightingState {
                leds_enabled: false,
                headlights_enabled: false,
            }),
            Some(false),
            Some(false),
        );
        let authorization = owner
            .authorize_set(BooleanControl::Leds, true, context)
            .unwrap();
        assert!(matches!(
            authorization,
            Authorization::Mutation { one_way: false, .. }
        ));
        assert_eq!(
            owner.authorize_set(BooleanControl::Package, false, context),
            Err(ControlError::InvalidState)
        );

        let mut flywheel = MutationCoordinator::new(1, true);
        flywheel.observe(
            PackageState::Running,
            PackageMode::Flywheel,
            None,
            None,
            None,
        );
        let riding = MutationContext {
            ride_state: RideState::Riding,
            package_state: PackageState::Running,
            package_mode: PackageMode::Flywheel,
            ..context
        };
        assert!(matches!(
            flywheel.authorize_set(BooleanControl::Flywheel, false, riding),
            Ok(Authorization::Mutation { one_way: true, .. })
        ));
        let mut denied = MutationCoordinator::new(1, true);
        denied.observe(
            PackageState::Running,
            PackageMode::Flywheel,
            None,
            None,
            None,
        );
        assert_eq!(
            denied.authorize_set(BooleanControl::Leds, true, riding),
            Err(ControlError::InvalidState)
        );

        let mut shutdown = MutationCoordinator::new(1, true);
        shutdown.observe(PackageState::Ready, PackageMode::Normal, None, None, None);
        assert_eq!(
            shutdown
                .authorize_shutdown(1, true, true, RideState::Idle, PackageMode::Normal)
                .unwrap(),
            vec![156, 0, 0]
        );
        assert!(
            shutdown
                .authorize_shutdown(1, true, true, RideState::Idle, PackageMode::Normal)
                .is_err()
        );
    }

    #[test]
    fn mutation_owner_serializes_shutdown_configuration_and_fatal_clear() {
        let profile = compatibility("1.2.3").unwrap().control_profile();
        let context = MutationContext {
            generation: 1,
            profile,
            connection_ready: true,
            writable: true,
            ride_state: RideState::Idle,
            package_state: PackageState::Ready,
            package_mode: PackageMode::Normal,
        };
        let mut shutdown = MutationCoordinator::new(1, true);
        shutdown.observe(PackageState::Ready, PackageMode::Normal, None, None, None);
        shutdown
            .authorize_shutdown(1, true, true, RideState::Idle, PackageMode::Normal)
            .unwrap();
        assert!(shutdown.clear_shutdown(1));
        shutdown
            .authorize_configuration(ConfigurationMutationContext {
                generation: 1,
                connection_ready: true,
                writable: true,
                ride_state: RideState::Idle,
                footpad_state: FootpadState::None,
                package_state: PackageState::Ready,
                package_mode: PackageMode::Normal,
            })
            .unwrap();
        assert!(
            shutdown
                .authorize_shutdown(1, true, true, RideState::Idle, PackageMode::Normal)
                .is_err()
        );
        assert!(shutdown.finish_configuration(1));

        let mut fatal = MutationCoordinator::new(1, true);
        fatal.observe(
            PackageState::Ready,
            PackageMode::Normal,
            None,
            Some(true),
            Some(false),
        );
        assert_eq!(
            fatal.authorize_fatal_clear(context).unwrap(),
            fatal_clear_request(profile).unwrap()
        );
        fatal.observe(
            PackageState::Ready,
            PackageMode::Normal,
            None,
            Some(false),
            Some(false),
        );
        assert_eq!(fatal.state.pending, None);
    }
}
