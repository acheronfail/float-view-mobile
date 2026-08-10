//! Strict, bounded decoding of Refloat command-10 compact telemetry.
//!
//! This unselected safe candidate owns only the fixed legacy wire layouts.
//! Native hosts retain compatibility, routing, polling, application mapping,
//! compact-cache merging, persistence, and transport.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

pub const CUSTOM_APP_DATA_COMMAND: u8 = 36;
pub const REFLOAT_PACKAGE_ID: u8 = 101;
pub const GET_ALL_DATA_COMMAND: u8 = 10;
pub const ACTIVE_FIRMWARE_FAULT_FORM: u8 = 69;

const DETAIL_LENGTHS: [usize; 4] = [35, 42, 55, 59];
const FIELD_WEAKENING_UNAVAILABLE: u8 = 222;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageState {
    Startup,
    Running,
    Ready,
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageMode {
    Normal,
    HandTest,
    Flywheel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FootpadState {
    None,
    Left,
    Right,
    Both,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopCondition {
    None,
    Pitch,
    Roll,
    SwitchHalf,
    SwitchFull,
    ReverseStop,
    QuickStop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetpointAdjustment {
    Centering,
    ReverseStop,
    None,
    Duty,
    HighVoltage,
    LowVoltage,
    Temperature,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlertReason {
    None,
    LowVoltage,
    HighVoltage,
    ControllerTemperature,
    MotorTemperature,
    Current,
    Duty,
    Sensors,
    LowBattery,
    Idle,
    Error,
    Speed,
    CellUnderTemperature,
    CellOverTemperature,
    CellLowVoltage,
    CellHighVoltage,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirmwareFault {
    OverVoltage,
    UnderVoltage,
    Driver,
    AbsoluteOverCurrent,
    ControllerOverTemperature,
    MotorOverTemperature,
    GateDriverOverVoltage,
    GateDriverUnderVoltage,
    McuUnderVoltage,
    WatchdogReset,
    EncoderSpi,
    EncoderSinCosBelowMinimum,
    EncoderSinCosAboveMaximum,
    FlashCorruption,
    CurrentSensorOffset1,
    CurrentSensorOffset2,
    CurrentSensorOffset3,
    UnbalancedCurrents,
    Brake,
    ResolverLossOfTracking,
    ResolverDegradationOfSignal,
    ResolverLossOfSignal,
    AppConfigFlashCorruption,
    MotorConfigFlashCorruption,
    EncoderNoMagnet,
    EncoderMagnetTooStrong,
    PhaseFilter,
    Unknown,
}

#[derive(Clone, Copy, PartialEq)]
pub struct ActiveFirmwareFault {
    pub raw_code: u8,
    pub fault: FirmwareFault,
}

impl Debug for ActiveFirmwareFault {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ActiveFirmwareFault([redacted])")
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct LegacySample {
    pub detail_level: u8,
    pub package_state: PackageState,
    pub package_mode: PackageMode,
    pub footpad_state: FootpadState,
    pub stop_condition: StopCondition,
    pub setpoint_adjustment: SetpointAdjustment,
    pub wheelslip: bool,
    pub darkride: bool,
    pub charging: bool,
    pub alert_reason: AlertReason,
    pub pack_voltage_v: f64,
    pub duty_cycle: f64,
    pub controller_temp_c: Option<f64>,
    pub motor_temp_c: Option<f64>,
    pub motor_current_a: f64,
    pub battery_current_a: f64,
    pub erpm: f64,
    pub speed_kph: f64,
    pub pitch_deg: f64,
    pub balance_pitch_deg: f64,
    pub roll_deg: f64,
    pub requested_balance_current_a: f64,
    pub booster_current_a: f64,
    pub field_weakening_current_a: Option<f64>,
    pub footpad_adc1_v: f64,
    pub footpad_adc2_v: f64,
    pub setpoint_deg: f64,
    pub atr_setpoint_deg: f64,
    pub brake_tilt_setpoint_deg: f64,
    pub torque_tilt_setpoint_deg: f64,
    pub turn_tilt_setpoint_deg: f64,
    pub remote_setpoint_deg: f64,
    pub distance_m: Option<f64>,
    pub odometer_m: Option<f64>,
    pub amp_hours_drawn_ah: Option<f64>,
    pub amp_hours_charged_ah: Option<f64>,
    pub watt_hours_drawn_wh: Option<f64>,
    pub watt_hours_charged_wh: Option<f64>,
    pub battery_level: Option<f64>,
    pub charging_current_a: Option<f64>,
    pub charging_voltage_v: Option<f64>,
}

impl Debug for LegacySample {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("LegacySample([redacted])")
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
// Boxing the sample would add heap ownership to the future host boundary; the
// fixed enum remains below the audited output-size bound.
#[allow(clippy::large_enum_variant)]
pub enum LegacyTelemetry {
    Sample(LegacySample),
    ActiveFirmwareFault(ActiveFirmwareFault),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyTelemetryError {
    TruncatedPayload,
    UnsupportedEnvelope,
    InvalidPayload,
}

impl Display for LegacyTelemetryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedPayload => formatter.write_str("truncated Refloat legacy telemetry"),
            Self::UnsupportedEnvelope => {
                formatter.write_str("unsupported Refloat legacy telemetry envelope")
            }
            Self::InvalidPayload => formatter.write_str("invalid Refloat legacy telemetry"),
        }
    }
}

impl Error for LegacyTelemetryError {}

pub fn decode(payload: &[u8]) -> Result<LegacyTelemetry, LegacyTelemetryError> {
    if payload.len() < 3 {
        return Err(LegacyTelemetryError::TruncatedPayload);
    }
    if payload[..3]
        != [
            CUSTOM_APP_DATA_COMMAND,
            REFLOAT_PACKAGE_ID,
            GET_ALL_DATA_COMMAND,
        ]
    {
        return Err(LegacyTelemetryError::UnsupportedEnvelope);
    }
    let form = *payload
        .get(3)
        .ok_or(LegacyTelemetryError::TruncatedPayload)?;
    if form == ACTIVE_FIRMWARE_FAULT_FORM {
        return decode_active_fault(payload);
    }

    let expected = DETAIL_LENGTHS
        .get(usize::from(form).wrapping_sub(1))
        .copied()
        .ok_or(LegacyTelemetryError::InvalidPayload)?;
    if payload.len() < expected {
        return Err(LegacyTelemetryError::TruncatedPayload);
    }
    if payload.len() != expected {
        return Err(LegacyTelemetryError::InvalidPayload);
    }

    let legacy_state = payload[10] & 0x0f;
    let (package_state, stop_condition) = semantic_state(legacy_state);
    let adc1 = f64::from(payload[12]) / 50.0;
    let adc2 = f64::from(payload[13]) / 50.0;
    let footpad_state = match payload[11] & 0x07 {
        2 => FootpadState::Both,
        1 if adc1 > adc2 => FootpadState::Left,
        1 => FootpadState::Right,
        _ => FootpadState::None,
    };
    let package_mode = if payload[11] & 0x08 != 0 {
        PackageMode::HandTest
    } else if legacy_state == 5 {
        PackageMode::Flywheel
    } else {
        PackageMode::Normal
    };
    let distance_m = if form >= 2 {
        let value = f32::from_bits(read_u32(payload, 35));
        if !value.is_finite() {
            return Err(LegacyTelemetryError::InvalidPayload);
        }
        Some(f64::from(value))
    } else {
        None
    };
    let field_weakening_current_a =
        (payload[34] != FIELD_WEAKENING_UNAVAILABLE).then(|| f64::from(payload[34]) / 3.0);

    Ok(LegacyTelemetry::Sample(LegacySample {
        detail_level: form,
        package_state,
        package_mode,
        footpad_state,
        stop_condition,
        setpoint_adjustment: setpoint_adjustment(payload[10] >> 4),
        wheelslip: legacy_state == 3,
        darkride: legacy_state == 4,
        charging: legacy_state == 14,
        alert_reason: alert_reason(payload[11] >> 4),
        pack_voltage_v: f64::from(read_i16(payload, 23)) / 10.0,
        duty_cycle: f64::from(i16::from(payload[33]) - 128) / 100.0,
        controller_temp_c: (form >= 2).then(|| f64::from(payload[39]) / 2.0),
        motor_temp_c: (form >= 2).then(|| f64::from(payload[40]) / 2.0),
        motor_current_a: f64::from(read_i16(payload, 29)) / 10.0,
        battery_current_a: f64::from(read_i16(payload, 31)) / 10.0,
        erpm: f64::from(read_i16(payload, 25)),
        speed_kph: f64::from(read_i16(payload, 27)) / 10.0 * 3.6,
        pitch_deg: f64::from(read_i16(payload, 20)) / 10.0,
        balance_pitch_deg: f64::from(read_i16(payload, 6)) / 10.0,
        roll_deg: f64::from(read_i16(payload, 8)) / 10.0,
        requested_balance_current_a: f64::from(read_i16(payload, 4)) / 10.0,
        booster_current_a: f64::from(i16::from(payload[22]) - 128),
        field_weakening_current_a,
        footpad_adc1_v: adc1,
        footpad_adc2_v: adc2,
        setpoint_deg: f64::from(i16::from(payload[14]) - 128) / 5.0,
        atr_setpoint_deg: f64::from(i16::from(payload[15]) - 128) / 5.0,
        brake_tilt_setpoint_deg: f64::from(i16::from(payload[16]) - 128) / 5.0,
        torque_tilt_setpoint_deg: f64::from(i16::from(payload[17]) - 128) / 5.0,
        turn_tilt_setpoint_deg: f64::from(i16::from(payload[18]) - 128) / 5.0,
        remote_setpoint_deg: f64::from(i16::from(payload[19]) - 128) / 5.0,
        distance_m,
        odometer_m: (form >= 3).then(|| f64::from(read_u32(payload, 42))),
        amp_hours_drawn_ah: (form >= 3).then(|| f64::from(read_i16(payload, 46)) / 10.0),
        amp_hours_charged_ah: (form >= 3).then(|| f64::from(read_i16(payload, 48)) / 10.0),
        watt_hours_drawn_wh: (form >= 3).then(|| f64::from(read_i16(payload, 50))),
        watt_hours_charged_wh: (form >= 3).then(|| f64::from(read_i16(payload, 52))),
        battery_level: (form >= 3).then(|| f64::from(payload[54]) / 200.0),
        charging_current_a: (form >= 4).then(|| f64::from(read_i16(payload, 55)) / 10.0),
        charging_voltage_v: (form >= 4).then(|| f64::from(read_i16(payload, 57)) / 10.0),
    }))
}

fn decode_active_fault(payload: &[u8]) -> Result<LegacyTelemetry, LegacyTelemetryError> {
    if payload.len() < 5 {
        return Err(LegacyTelemetryError::TruncatedPayload);
    }
    if payload.len() != 5 || payload[4] == 0 {
        return Err(LegacyTelemetryError::InvalidPayload);
    }
    Ok(LegacyTelemetry::ActiveFirmwareFault(ActiveFirmwareFault {
        raw_code: payload[4],
        fault: firmware_fault(payload[4]),
    }))
}

fn read_i16(payload: &[u8], index: usize) -> i16 {
    i16::from_be_bytes([payload[index], payload[index + 1]])
}

fn read_u32(payload: &[u8], index: usize) -> u32 {
    u32::from_be_bytes([
        payload[index],
        payload[index + 1],
        payload[index + 2],
        payload[index + 3],
    ])
}

const fn semantic_state(value: u8) -> (PackageState, StopCondition) {
    match value {
        0 => (PackageState::Startup, StopCondition::None),
        1..=5 => (PackageState::Running, StopCondition::None),
        6 => (PackageState::Ready, StopCondition::Pitch),
        7 => (PackageState::Ready, StopCondition::Roll),
        8 => (PackageState::Ready, StopCondition::SwitchHalf),
        9 => (PackageState::Ready, StopCondition::SwitchFull),
        12 => (PackageState::Ready, StopCondition::ReverseStop),
        13 => (PackageState::Ready, StopCondition::QuickStop),
        14 => (PackageState::Ready, StopCondition::None),
        15 => (PackageState::Disabled, StopCondition::None),
        _ => (PackageState::Ready, StopCondition::None),
    }
}

const fn setpoint_adjustment(value: u8) -> SetpointAdjustment {
    match value {
        0 => SetpointAdjustment::Centering,
        1 => SetpointAdjustment::ReverseStop,
        2 => SetpointAdjustment::None,
        3 => SetpointAdjustment::Duty,
        4 => SetpointAdjustment::HighVoltage,
        5 => SetpointAdjustment::LowVoltage,
        6 => SetpointAdjustment::Temperature,
        _ => SetpointAdjustment::Unknown,
    }
}

const fn alert_reason(value: u8) -> AlertReason {
    match value {
        0 => AlertReason::None,
        1 => AlertReason::LowVoltage,
        2 => AlertReason::HighVoltage,
        3 => AlertReason::ControllerTemperature,
        4 => AlertReason::MotorTemperature,
        5 => AlertReason::Current,
        6 => AlertReason::Duty,
        7 => AlertReason::Sensors,
        8 => AlertReason::LowBattery,
        9 => AlertReason::Idle,
        10 => AlertReason::Error,
        11 => AlertReason::Speed,
        12 => AlertReason::CellUnderTemperature,
        13 => AlertReason::CellOverTemperature,
        14 => AlertReason::CellLowVoltage,
        15 => AlertReason::CellHighVoltage,
        _ => AlertReason::Unknown,
    }
}

const fn firmware_fault(value: u8) -> FirmwareFault {
    match value {
        1 => FirmwareFault::OverVoltage,
        2 => FirmwareFault::UnderVoltage,
        3 => FirmwareFault::Driver,
        4 => FirmwareFault::AbsoluteOverCurrent,
        5 => FirmwareFault::ControllerOverTemperature,
        6 => FirmwareFault::MotorOverTemperature,
        7 => FirmwareFault::GateDriverOverVoltage,
        8 => FirmwareFault::GateDriverUnderVoltage,
        9 => FirmwareFault::McuUnderVoltage,
        10 => FirmwareFault::WatchdogReset,
        11 => FirmwareFault::EncoderSpi,
        12 => FirmwareFault::EncoderSinCosBelowMinimum,
        13 => FirmwareFault::EncoderSinCosAboveMaximum,
        14 => FirmwareFault::FlashCorruption,
        15 => FirmwareFault::CurrentSensorOffset1,
        16 => FirmwareFault::CurrentSensorOffset2,
        17 => FirmwareFault::CurrentSensorOffset3,
        18 => FirmwareFault::UnbalancedCurrents,
        19 => FirmwareFault::Brake,
        20 => FirmwareFault::ResolverLossOfTracking,
        21 => FirmwareFault::ResolverDegradationOfSignal,
        22 => FirmwareFault::ResolverLossOfSignal,
        23 => FirmwareFault::AppConfigFlashCorruption,
        24 => FirmwareFault::MotorConfigFlashCorruption,
        25 => FirmwareFault::EncoderNoMagnet,
        26 => FirmwareFault::EncoderMagnetTooStrong,
        27 => FirmwareFault::PhaseFilter,
        _ => FirmwareFault::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    const FIXTURES: &str =
        include_str!("../../../fixtures/refloat/detailed-source-provenance.json");

    #[test]
    fn decodes_every_committed_detail_level_and_active_fault() {
        for level in 1..=4 {
            let decoded = decode(&fixture_payload(&format!(
                "1.0.0-command-10-detail-{level}"
            )))
            .unwrap();
            let sample = sample(decoded);
            assert_eq!(sample.detail_level, level);
            assert_base_fixture(sample);
            assert_eq!(sample.distance_m.is_some(), level >= 2);
            assert_eq!(sample.controller_temp_c.is_some(), level >= 2);
            assert_eq!(sample.motor_temp_c.is_some(), level >= 2);
            assert_eq!(sample.odometer_m.is_some(), level >= 3);
            assert_eq!(sample.amp_hours_drawn_ah.is_some(), level >= 3);
            assert_eq!(sample.amp_hours_charged_ah.is_some(), level >= 3);
            assert_eq!(sample.watt_hours_drawn_wh.is_some(), level >= 3);
            assert_eq!(sample.watt_hours_charged_wh.is_some(), level >= 3);
            assert_eq!(sample.battery_level.is_some(), level >= 3);
            assert_eq!(sample.charging_current_a.is_some(), level >= 4);
            assert_eq!(sample.charging_voltage_v.is_some(), level >= 4);
        }

        let detail_four = sample(decode(&fixture_payload("1.0.0-command-10-detail-4")).unwrap());
        assert_eq!(detail_four.distance_m, Some(0.0));
        assert_eq!(detail_four.controller_temp_c, Some(30.0));
        assert_eq!(detail_four.motor_temp_c, Some(28.0));
        assert_eq!(detail_four.odometer_m, Some(12_345.0));
        assert_eq!(detail_four.amp_hours_drawn_ah, Some(18.4));
        assert_eq!(detail_four.amp_hours_charged_ah, Some(2.7));
        assert_eq!(detail_four.watt_hours_drawn_wh, Some(812.0));
        assert_eq!(detail_four.watt_hours_charged_wh, Some(119.0));
        assert_eq!(detail_four.battery_level, Some(0.82));
        assert_eq!(detail_four.charging_current_a, Some(0.0));
        assert_eq!(detail_four.charging_voltage_v, Some(0.0));

        assert_eq!(
            decode(&fixture_payload("1.2.3-command-10-active-firmware-fault")),
            Ok(LegacyTelemetry::ActiveFirmwareFault(ActiveFirmwareFault {
                raw_code: 1,
                fault: FirmwareFault::OverVoltage,
            }))
        );
    }

    #[test]
    fn preserves_field_weakening_sentinel_and_finite_distance() {
        let mut payload = fixture_payload("1.0.0-command-10-detail-2");
        payload[34] = FIELD_WEAKENING_UNAVAILABLE;
        assert_eq!(
            sample(decode(&payload).unwrap()).field_weakening_current_a,
            None
        );
        payload[34] = 221;
        assert_eq!(
            sample(decode(&payload).unwrap()).field_weakening_current_a,
            Some(221.0 / 3.0)
        );

        payload[35..39].copy_from_slice(&f32::INFINITY.to_bits().to_be_bytes());
        assert_eq!(decode(&payload), Err(LegacyTelemetryError::InvalidPayload));
        payload[35..39].copy_from_slice(&f32::NAN.to_bits().to_be_bytes());
        assert_eq!(decode(&payload), Err(LegacyTelemetryError::InvalidPayload));
    }

    #[test]
    fn rejects_every_length_boundary_and_wrong_envelope() {
        for level in 1..=4 {
            let payload = fixture_payload(&format!("1.0.0-command-10-detail-{level}"));
            for length in 0..payload.len() {
                assert!(
                    decode(&payload[..length]).is_err(),
                    "accepted length {length}"
                );
            }
            let mut overlong = payload;
            overlong.push(0);
            assert_eq!(decode(&overlong), Err(LegacyTelemetryError::InvalidPayload));
        }

        let fault = fixture_payload("1.2.3-command-10-active-firmware-fault");
        for length in 0..fault.len() {
            assert!(
                decode(&fault[..length]).is_err(),
                "accepted fault length {length}"
            );
        }
        let mut overlong_fault = fault.clone();
        overlong_fault.push(0);
        assert_eq!(
            decode(&overlong_fault),
            Err(LegacyTelemetryError::InvalidPayload)
        );
        let mut zero_fault = fault;
        zero_fault[4] = 0;
        assert_eq!(
            decode(&zero_fault),
            Err(LegacyTelemetryError::InvalidPayload)
        );

        let normal = fixture_payload("1.0.0-command-10-detail-1");
        for (index, value) in [(0, 35), (1, 100), (2, 11)] {
            let mut wrong = normal.clone();
            wrong[index] = value;
            assert_eq!(
                decode(&wrong),
                Err(LegacyTelemetryError::UnsupportedEnvelope)
            );
        }
        for invalid_level in [0, 5, 68, 70, 255] {
            let mut invalid = normal.clone();
            invalid[3] = invalid_level;
            assert_eq!(decode(&invalid), Err(LegacyTelemetryError::InvalidPayload));
        }
    }

    #[test]
    fn maps_every_legacy_state_mode_and_footpad_semantic() {
        let base = fixture_payload("1.0.0-command-10-detail-1");
        let states = [
            (0, PackageState::Startup, StopCondition::None),
            (1, PackageState::Running, StopCondition::None),
            (3, PackageState::Running, StopCondition::None),
            (4, PackageState::Running, StopCondition::None),
            (5, PackageState::Running, StopCondition::None),
            (6, PackageState::Ready, StopCondition::Pitch),
            (7, PackageState::Ready, StopCondition::Roll),
            (8, PackageState::Ready, StopCondition::SwitchHalf),
            (9, PackageState::Ready, StopCondition::SwitchFull),
            (10, PackageState::Ready, StopCondition::None),
            (11, PackageState::Ready, StopCondition::None),
            (12, PackageState::Ready, StopCondition::ReverseStop),
            (13, PackageState::Ready, StopCondition::QuickStop),
            (14, PackageState::Ready, StopCondition::None),
            (15, PackageState::Disabled, StopCondition::None),
        ];
        for (code, package_state, stop_condition) in states {
            let mut payload = base.clone();
            payload[10] = (payload[10] & 0xf0) | code;
            let decoded = sample(decode(&payload).unwrap());
            assert_eq!(decoded.package_state, package_state, "state {code}");
            assert_eq!(decoded.stop_condition, stop_condition, "state {code}");
            assert_eq!(decoded.wheelslip, code == 3, "state {code}");
            assert_eq!(decoded.darkride, code == 4, "state {code}");
            assert_eq!(decoded.charging, code == 14, "state {code}");
            assert_eq!(
                decoded.package_mode,
                if code == 5 {
                    PackageMode::Flywheel
                } else {
                    PackageMode::Normal
                },
                "state {code}"
            );
        }

        let mut hand_test = base.clone();
        hand_test[10] = (hand_test[10] & 0xf0) | 5;
        hand_test[11] |= 0x08;
        assert_eq!(
            sample(decode(&hand_test).unwrap()).package_mode,
            PackageMode::HandTest
        );

        let mut left = base.clone();
        left[11] = (left[11] & 0xf8) | 1;
        left[12] = 100;
        left[13] = 50;
        assert_eq!(
            sample(decode(&left).unwrap()).footpad_state,
            FootpadState::Left
        );
        left[12] = 50;
        assert_eq!(
            sample(decode(&left).unwrap()).footpad_state,
            FootpadState::Right
        );
        left[11] = (left[11] & 0xf8) | 2;
        assert_eq!(
            sample(decode(&left).unwrap()).footpad_state,
            FootpadState::Both
        );
        left[11] = (left[11] & 0xf8) | 3;
        assert_eq!(
            sample(decode(&left).unwrap()).footpad_state,
            FootpadState::None
        );
    }

    #[test]
    fn maps_every_adjustment_alert_and_firmware_fault_code() {
        let base = fixture_payload("1.0.0-command-10-detail-1");
        let adjustments = [
            SetpointAdjustment::Centering,
            SetpointAdjustment::ReverseStop,
            SetpointAdjustment::None,
            SetpointAdjustment::Duty,
            SetpointAdjustment::HighVoltage,
            SetpointAdjustment::LowVoltage,
            SetpointAdjustment::Temperature,
            SetpointAdjustment::Unknown,
        ];
        for (code, expected) in adjustments.into_iter().enumerate() {
            let mut payload = base.clone();
            payload[10] = (payload[10] & 0x0f) | ((code as u8) << 4);
            assert_eq!(
                sample(decode(&payload).unwrap()).setpoint_adjustment,
                expected
            );
        }

        let alerts = [
            AlertReason::None,
            AlertReason::LowVoltage,
            AlertReason::HighVoltage,
            AlertReason::ControllerTemperature,
            AlertReason::MotorTemperature,
            AlertReason::Current,
            AlertReason::Duty,
            AlertReason::Sensors,
            AlertReason::LowBattery,
            AlertReason::Idle,
            AlertReason::Error,
            AlertReason::Speed,
            AlertReason::CellUnderTemperature,
            AlertReason::CellOverTemperature,
            AlertReason::CellLowVoltage,
            AlertReason::CellHighVoltage,
        ];
        for (code, expected) in alerts.into_iter().enumerate() {
            let mut payload = base.clone();
            payload[11] = (payload[11] & 0x0f) | ((code as u8) << 4);
            assert_eq!(sample(decode(&payload).unwrap()).alert_reason, expected);
        }

        let expected_faults = [
            FirmwareFault::OverVoltage,
            FirmwareFault::UnderVoltage,
            FirmwareFault::Driver,
            FirmwareFault::AbsoluteOverCurrent,
            FirmwareFault::ControllerOverTemperature,
            FirmwareFault::MotorOverTemperature,
            FirmwareFault::GateDriverOverVoltage,
            FirmwareFault::GateDriverUnderVoltage,
            FirmwareFault::McuUnderVoltage,
            FirmwareFault::WatchdogReset,
            FirmwareFault::EncoderSpi,
            FirmwareFault::EncoderSinCosBelowMinimum,
            FirmwareFault::EncoderSinCosAboveMaximum,
            FirmwareFault::FlashCorruption,
            FirmwareFault::CurrentSensorOffset1,
            FirmwareFault::CurrentSensorOffset2,
            FirmwareFault::CurrentSensorOffset3,
            FirmwareFault::UnbalancedCurrents,
            FirmwareFault::Brake,
            FirmwareFault::ResolverLossOfTracking,
            FirmwareFault::ResolverDegradationOfSignal,
            FirmwareFault::ResolverLossOfSignal,
            FirmwareFault::AppConfigFlashCorruption,
            FirmwareFault::MotorConfigFlashCorruption,
            FirmwareFault::EncoderNoMagnet,
            FirmwareFault::EncoderMagnetTooStrong,
            FirmwareFault::PhaseFilter,
        ];
        for (index, expected) in expected_faults.into_iter().enumerate() {
            let code = index as u8 + 1;
            assert_eq!(firmware_fault(code), expected, "fault {code}");
        }
        assert_eq!(firmware_fault(255), FirmwareFault::Unknown);
        assert_eq!(alert_reason(255), AlertReason::Unknown);
    }

    #[test]
    fn preserves_signed_and_scaled_numeric_boundaries() {
        let mut payload = fixture_payload("1.0.0-command-10-detail-4");
        for (index, value) in [
            (4, i16::MIN),
            (6, i16::MAX),
            (8, i16::MIN),
            (20, i16::MAX),
            (23, i16::MIN),
            (25, i16::MIN),
            (27, i16::MAX),
            (29, i16::MIN),
            (31, i16::MAX),
            (46, i16::MIN),
            (48, i16::MAX),
            (50, i16::MIN),
            (52, i16::MAX),
            (55, i16::MIN),
            (57, i16::MAX),
        ] {
            payload[index..index + 2].copy_from_slice(&value.to_be_bytes());
        }
        payload[22] = 0;
        payload[33] = 255;
        payload[34] = 255;
        payload[54] = 255;
        let decoded = sample(decode(&payload).unwrap());
        assert_eq!(
            decoded.requested_balance_current_a,
            f64::from(i16::MIN) / 10.0
        );
        assert_eq!(decoded.balance_pitch_deg, f64::from(i16::MAX) / 10.0);
        assert_eq!(decoded.roll_deg, f64::from(i16::MIN) / 10.0);
        assert_eq!(decoded.pitch_deg, f64::from(i16::MAX) / 10.0);
        assert_eq!(decoded.pack_voltage_v, f64::from(i16::MIN) / 10.0);
        assert_eq!(decoded.erpm, f64::from(i16::MIN));
        assert_eq!(decoded.speed_kph, f64::from(i16::MAX) / 10.0 * 3.6);
        assert_eq!(decoded.motor_current_a, f64::from(i16::MIN) / 10.0);
        assert_eq!(decoded.battery_current_a, f64::from(i16::MAX) / 10.0);
        assert_eq!(decoded.booster_current_a, -128.0);
        assert_eq!(decoded.duty_cycle, 1.27);
        assert_eq!(decoded.field_weakening_current_a, Some(85.0));
        assert_eq!(decoded.amp_hours_drawn_ah, Some(f64::from(i16::MIN) / 10.0));
        assert_eq!(
            decoded.amp_hours_charged_ah,
            Some(f64::from(i16::MAX) / 10.0)
        );
        assert_eq!(decoded.watt_hours_drawn_wh, Some(f64::from(i16::MIN)));
        assert_eq!(decoded.watt_hours_charged_wh, Some(f64::from(i16::MAX)));
        assert_eq!(decoded.battery_level, Some(1.275));
        assert_eq!(decoded.charging_current_a, Some(f64::from(i16::MIN) / 10.0));
        assert_eq!(decoded.charging_voltage_v, Some(f64::from(i16::MAX) / 10.0));
    }

    #[test]
    fn output_is_fixed_size_deterministic_and_redacted() {
        let payload = fixture_payload("1.0.0-command-10-detail-4");
        let first = decode(&payload).unwrap();
        let second = decode(&payload).unwrap();
        assert_eq!(first, second);
        assert!(size_of::<LegacyTelemetry>() < 512);
        assert_eq!(format!("{first:?}"), "Sample(LegacySample([redacted]))");
        assert_eq!(
            LegacyTelemetryError::InvalidPayload.to_string(),
            "invalid Refloat legacy telemetry"
        );
        assert!(!format!("{:?}", LegacyTelemetryError::InvalidPayload).contains("2465"));
    }

    fn assert_base_fixture(sample: LegacySample) {
        assert_eq!(sample.package_state, PackageState::Running);
        assert_eq!(sample.package_mode, PackageMode::Normal);
        assert_eq!(sample.footpad_state, FootpadState::Both);
        assert_eq!(sample.stop_condition, StopCondition::None);
        assert_eq!(sample.setpoint_adjustment, SetpointAdjustment::None);
        assert!(!sample.wheelslip);
        assert!(!sample.darkride);
        assert!(!sample.charging);
        assert_eq!(sample.alert_reason, AlertReason::None);
        assert_eq!(sample.pack_voltage_v, 60.7);
        assert_eq!(sample.duty_cycle, 0.64);
        assert_eq!(sample.motor_current_a, 48.0);
        assert_eq!(sample.battery_current_a, 25.0);
        assert_eq!(sample.erpm, 5_633.0);
        assert!((sample.speed_kph - 23.76).abs() < 1e-12);
        assert_eq!(sample.pitch_deg, 3.2);
        assert_eq!(sample.balance_pitch_deg, 1.1);
        assert_eq!(sample.roll_deg, 0.8);
        assert_eq!(sample.requested_balance_current_a, 52.0);
        assert_eq!(sample.booster_current_a, 6.0);
        assert_eq!(sample.field_weakening_current_a, Some(0.0));
        assert_eq!(sample.footpad_adc1_v, 2.8);
        assert_eq!(sample.footpad_adc2_v, 2.74);
        assert_eq!(sample.setpoint_deg, 1.0);
        assert_eq!(sample.atr_setpoint_deg, 2.4);
        assert_eq!(sample.brake_tilt_setpoint_deg, 0.0);
        assert_eq!(sample.torque_tilt_setpoint_deg, 1.8);
        assert_eq!(sample.turn_tilt_setpoint_deg, 0.0);
        assert_eq!(sample.remote_setpoint_deg, 0.0);
    }

    fn sample(value: LegacyTelemetry) -> LegacySample {
        match value {
            LegacyTelemetry::Sample(sample) => sample,
            LegacyTelemetry::ActiveFirmwareFault(_) => panic!("expected sample"),
        }
    }

    fn fixture_payload(name: &str) -> Vec<u8> {
        let marker = format!("\"name\": \"{name}\"");
        let case = &FIXTURES[FIXTURES.find(&marker).unwrap()..];
        hex_bytes(json_string(case, "payloadHex"))
    }

    fn json_string<'a>(json: &'a str, key: &str) -> &'a str {
        let prefix = format!("\"{key}\": \"");
        let start = json.find(&prefix).unwrap() + prefix.len();
        let remainder = &json[start..];
        &remainder[..remainder.find('"').unwrap()]
    }

    fn hex_bytes(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0);
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let pair = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(pair, 16).unwrap()
            })
            .collect()
    }
}
