#![allow(dead_code)]

use float_view_device_mutation_coordinator::{
    Command, ConfigureFacts, DeviceMutationCoordinator, Effect, MutationToken,
};
use float_view_refloat_compatibility::{self as compatibility, CompatibilityProfile};
use float_view_refloat_telemetry_service::{
    ExtensionValue, FirmwareFault, FootpadState, NormalizedSample, PackageMode, PackageState,
    RideState,
};
use float_view_vesc_firmware_identity::{FirmwareIdentity, MOTOR_HARDWARE_TYPE};
use float_view_vesc_request_scheduler::{Generation, RequestFacts, Route};

pub fn firmware(test_version: u8) -> FirmwareIdentity {
    FirmwareIdentity {
        major: 6,
        minor: 6,
        hardware_name: "THOR300".to_owned(),
        firmware_name: None,
        controller_uuid: [42; 12],
        test_version,
        hardware_type: MOTOR_HARDWARE_TYPE,
        custom_configuration_count: 1,
        hardware_configuration_crc: Some(0x1234_5678),
    }
}

pub fn unsupported_firmware() -> FirmwareIdentity {
    FirmwareIdentity {
        major: 5,
        minor: 3,
        ..firmware(0)
    }
}

pub fn stable_profile() -> CompatibilityProfile {
    compatibility::compatibility(compatibility::parse_wire_version("1.2.3").unwrap()).unwrap()
}

pub fn legacy_profile() -> CompatibilityProfile {
    compatibility::compatibility(compatibility::parse_wire_version("1.0").unwrap()).unwrap()
}

pub fn configure(
    coordinator: &mut DeviceMutationCoordinator,
    route: Route,
    identity: FirmwareIdentity,
    profile: CompatibilityProfile,
) -> Vec<Effect> {
    coordinator
        .apply(Command::Configure(ConfigureFacts {
            generation: Generation(7),
            route,
            firmware: identity,
            refloat: profile,
            connection_ready: true,
        }))
        .unwrap()
}

pub fn ready(coordinator: &mut DeviceMutationCoordinator, route: Route) {
    configure(coordinator, route, firmware(0), stable_profile());
    observe(coordinator, sample(1, RideState::Idle, PackageMode::Normal));
    observe_lighting(coordinator, 1, &[36, 101, 20, 3]);
    assert_eq!(
        coordinator.snapshot().controls.unwrap().status,
        float_view_device_mutation_coordinator::ControlStatus::Ready
    );
}

pub fn observe(
    coordinator: &mut DeviceMutationCoordinator,
    sample: NormalizedSample,
) -> Vec<Effect> {
    coordinator
        .apply(Command::ObserveTelemetry {
            generation: Generation(7),
            sample: Box::new(sample),
        })
        .unwrap()
}

pub fn observe_lighting(
    coordinator: &mut DeviceMutationCoordinator,
    observation_sequence: u64,
    payload: &[u8],
) -> Vec<Effect> {
    coordinator
        .apply(Command::ObserveLighting {
            generation: Generation(7),
            observation_sequence,
            payload: payload.to_vec(),
        })
        .unwrap()
}

pub fn sample(sequence: u64, ride_state: RideState, package_mode: PackageMode) -> NormalizedSample {
    sample_with(
        sequence,
        ride_state,
        PackageState::Ready,
        package_mode,
        FootpadState::None,
        Some(false),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn sample_with(
    sequence: u64,
    ride_state: RideState,
    package_state: PackageState,
    package_mode: PackageMode,
    footpad_state: FootpadState,
    fatal_error_latched: Option<bool>,
    firmware_fault: Option<FirmwareFault>,
) -> NormalizedSample {
    NormalizedSample {
        sequence,
        observed_at_epoch_ms: 1_700_000_000_000 + i64::try_from(sequence).unwrap(),
        slow_observed_at_epoch_ms: None,
        elapsed_ms: sequence,
        riding_elapsed_ms: 0,
        ride_state,
        package_state: Some(package_state),
        package_mode: Some(package_mode),
        footpad_state: Some(footpad_state),
        stop_condition: None,
        setpoint_adjustment: None,
        wheelslip: None,
        darkride: None,
        charging: None,
        alert_reason: None,
        firmware_fault,
        fatal_error_latched,
        pack_voltage_v: None,
        duty_cycle: None,
        controller_temp_c: None,
        motor_temp_c: None,
        bms_circuitry_temp_c: None,
        bms_highest_cell_temp_c: None,
        motor_current_a: None,
        battery_current_a: None,
        erpm: None,
        speed_kph: None,
        pitch_deg: None,
        balance_pitch_deg: None,
        roll_deg: None,
        directional_motor_current_a: None,
        filtered_motor_current_a: None,
        requested_balance_current_a: None,
        booster_current_a: None,
        field_weakening_current_a: None,
        footpad_adc1_v: None,
        footpad_adc2_v: None,
        remote_input: None,
        setpoint_deg: None,
        atr_setpoint_deg: None,
        brake_tilt_setpoint_deg: None,
        torque_tilt_setpoint_deg: None,
        turn_tilt_setpoint_deg: None,
        remote_setpoint_deg: None,
        atr_acceleration_difference_erpm_per_second: None,
        atr_speed_boost: None,
        distance_m: None,
        odometer_m: None,
        amp_hours_drawn_ah: None,
        amp_hours_charged_ah: None,
        watt_hours_drawn_wh: None,
        watt_hours_charged_wh: None,
        battery_level: None,
        charging_current_a: None,
        charging_voltage_v: None,
        extensions: Vec::<ExtensionValue>::new(),
    }
}

pub fn submission(effects: &[Effect]) -> (MutationToken, &RequestFacts) {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Submit { token, request } => Some((*token, request)),
            _ => None,
        })
        .expect("submit effect")
}

pub fn lease(effects: &[Effect]) -> MutationToken {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Lease(token) => Some(*token),
            _ => None,
        })
        .expect("lease effect")
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
