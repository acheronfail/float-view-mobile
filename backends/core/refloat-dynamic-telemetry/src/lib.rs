//! Strict, bounded decoding of negotiated Refloat command-31/32 telemetry.
//!
//! This safe candidate owns only dynamic layout and wire decoding. Native
//! hosts retain compatibility, routing, polling, layout replacement across
//! reconnect, compact-cache merging, application behavior, and persistence.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

pub const CUSTOM_APP_DATA_COMMAND: u8 = 36;
pub const REFLOAT_PACKAGE_ID: u8 = 101;
pub const REALTIME_DATA_COMMAND: u8 = 31;
pub const REALTIME_DATA_IDS_COMMAND: u8 = 32;

pub const MAX_FIELDS: usize = 64;
pub const MAX_FIELD_BYTES: usize = 64;
pub const MAX_AGGREGATE_FIELD_BYTES: usize = 4_096;
pub const MAX_EXTENSIONS: usize = 32;

const MASK_RUNTIME_VALUES: u8 = 1 << 0;
const MASK_CHARGING_VALUES: u8 = 1 << 1;
const MASK_ACTIVE_ALERTS: u8 = 1 << 2;
const SUPPORTED_MASK: u8 = MASK_RUNTIME_VALUES | MASK_CHARGING_VALUES | MASK_ACTIVE_ALERTS;
const FATAL_ERROR_FLAG: u8 = 1 << 3;
const CHARGING_FLAG: u8 = 1 << 5;
const FIRMWARE_FAULT_ALERT: u32 = 1;

#[derive(Clone, PartialEq, Eq)]
pub struct DynamicLayout {
    always_present: Vec<String>,
    running_only: Vec<String>,
    aggregate_name_bytes: usize,
}

impl DynamicLayout {
    pub fn always_present(&self) -> &[String] {
        &self.always_present
    }

    pub fn running_only(&self) -> &[String] {
        &self.running_only
    }

    pub fn field_count(&self) -> usize {
        self.always_present.len() + self.running_only.len()
    }

    pub fn aggregate_name_bytes(&self) -> usize {
        self.aggregate_name_bytes
    }
}

impl Debug for DynamicLayout {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DynamicLayout")
            .field("always_present_count", &self.always_present.len())
            .field("running_only_count", &self.running_only.len())
            .field("aggregate_name_bytes", &self.aggregate_name_bytes)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageState {
    Disabled,
    Startup,
    Ready,
    Running,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageMode {
    Normal,
    HandTest,
    Flywheel,
    Unknown,
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
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetpointAdjustment {
    None,
    Centering,
    ReverseStop,
    Speed,
    Duty,
    Error,
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
    CellImbalance,
    BmsConnection,
    BmsOverTemperature,
    FirmwareFault,
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

#[derive(Clone, Default, PartialEq)]
pub struct DynamicValues {
    pub speed_kph: Option<f64>,
    pub erpm: Option<f64>,
    pub motor_current_a: Option<f64>,
    pub directional_motor_current_a: Option<f64>,
    pub filtered_motor_current_a: Option<f64>,
    pub duty_cycle: Option<f64>,
    pub pack_voltage_v: Option<f64>,
    pub battery_current_a: Option<f64>,
    pub controller_temp_c: Option<f64>,
    pub motor_temp_c: Option<f64>,
    pub pitch_deg: Option<f64>,
    pub balance_pitch_deg: Option<f64>,
    pub roll_deg: Option<f64>,
    pub footpad_adc1_v: Option<f64>,
    pub footpad_adc2_v: Option<f64>,
    pub remote_input: Option<f64>,
    pub setpoint_deg: Option<f64>,
    pub atr_setpoint_deg: Option<f64>,
    pub brake_tilt_setpoint_deg: Option<f64>,
    pub torque_tilt_setpoint_deg: Option<f64>,
    pub turn_tilt_setpoint_deg: Option<f64>,
    pub remote_setpoint_deg: Option<f64>,
    pub requested_balance_current_a: Option<f64>,
    pub atr_acceleration_difference_erpm_per_second: Option<f64>,
    pub atr_speed_boost: Option<f64>,
    pub booster_current_a: Option<f64>,
}

impl Debug for DynamicValues {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("DynamicValues([redacted])")
    }
}

#[derive(Clone, PartialEq)]
pub struct ExtensionValue {
    pub name: String,
    pub value: f64,
}

impl Debug for ExtensionValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("ExtensionValue([redacted])")
    }
}

#[derive(Clone, PartialEq)]
pub struct DynamicSample {
    pub source_ticks: u32,
    pub package_state: PackageState,
    pub package_mode: PackageMode,
    pub footpad_state: FootpadState,
    pub stop_condition: StopCondition,
    pub setpoint_adjustment: SetpointAdjustment,
    pub wheelslip: bool,
    pub darkride: bool,
    pub charging: bool,
    pub alert_reason: AlertReason,
    pub fatal_error_latched: Option<bool>,
    pub active_alert_words: Option<[u32; 2]>,
    pub firmware_fault_raw_code: Option<u8>,
    pub firmware_fault: Option<FirmwareFault>,
    pub values: DynamicValues,
    pub charging_current_a: Option<f64>,
    pub charging_voltage_v: Option<f64>,
    pub extensions: Vec<ExtensionValue>,
}

impl Debug for DynamicSample {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DynamicSample")
            .field("package_state", &self.package_state)
            .field("package_mode", &self.package_mode)
            .field("extension_count", &self.extensions.len())
            .field("has_charging_values", &self.charging_current_a.is_some())
            .field("has_active_alerts", &self.active_alert_words.is_some())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicTelemetryError {
    TruncatedPayload,
    UnsupportedEnvelope,
    InvalidLayout,
    InvalidTelemetry,
}

impl Display for DynamicTelemetryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedPayload => formatter.write_str("truncated Refloat dynamic telemetry"),
            Self::UnsupportedEnvelope => {
                formatter.write_str("unsupported Refloat dynamic telemetry envelope")
            }
            Self::InvalidLayout => formatter.write_str("invalid Refloat dynamic telemetry layout"),
            Self::InvalidTelemetry => formatter.write_str("invalid Refloat dynamic telemetry"),
        }
    }
}

impl Error for DynamicTelemetryError {}

pub fn decode_layout(payload: &[u8]) -> Result<DynamicLayout, DynamicTelemetryError> {
    validate_envelope(payload, REALTIME_DATA_IDS_COMMAND)?;
    if payload.len() < 5 {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }

    let mut index = 3;
    let mut aggregate_name_bytes = 0;
    let always_count = usize::from(payload[index]);
    index += 1;
    if always_count == 0 || always_count > MAX_FIELDS {
        return Err(DynamicTelemetryError::InvalidLayout);
    }
    let always_present =
        decode_name_list(payload, &mut index, always_count, &mut aggregate_name_bytes)?;
    let running_count = usize::from(
        *payload
            .get(index)
            .ok_or(DynamicTelemetryError::TruncatedPayload)?,
    );
    index += 1;
    if always_count
        .checked_add(running_count)
        .filter(|count| *count <= MAX_FIELDS)
        .is_none()
    {
        return Err(DynamicTelemetryError::InvalidLayout);
    }
    let running_only = decode_name_list(
        payload,
        &mut index,
        running_count,
        &mut aggregate_name_bytes,
    )?;
    if index != payload.len() {
        return Err(DynamicTelemetryError::InvalidLayout);
    }

    let mut all = always_present.iter().chain(&running_only);
    while let Some(name) = all.next() {
        if all.clone().any(|candidate| candidate == name) {
            return Err(DynamicTelemetryError::InvalidLayout);
        }
    }

    Ok(DynamicLayout {
        always_present,
        running_only,
        aggregate_name_bytes,
    })
}

pub fn decode_realtime(
    layout: &DynamicLayout,
    payload: &[u8],
) -> Result<DynamicSample, DynamicTelemetryError> {
    validate_envelope(payload, REALTIME_DATA_COMMAND)?;
    if payload.len() < 13 {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }

    let mask = payload[3];
    if mask & !SUPPORTED_MASK != 0 {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let extra_flags = payload[4];
    if extra_flags & !0x0f != 0 {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let state_and_mode = payload[9];
    if state_and_mode & 0xcc != 0 {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let footpad_and_flags = payload[10];
    if footpad_and_flags & 0x1c != 0 {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }

    let package_state = package_state(state_and_mode & 0x03);
    let includes_runtime = mask & MASK_RUNTIME_VALUES != 0;
    let includes_charging = mask & MASK_CHARGING_VALUES != 0;
    let includes_alerts = mask & MASK_ACTIVE_ALERTS != 0;
    let charging = footpad_and_flags & CHARGING_FLAG != 0;
    if includes_runtime != (package_state == PackageState::Running)
        || includes_charging != charging
        || (!includes_alerts && extra_flags & FATAL_ERROR_FLAG != 0)
    {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }

    let value_count = layout.always_present.len()
        + if includes_runtime {
            layout.running_only.len()
        } else {
            0
        };
    let expected = 13usize
        .checked_add(
            value_count
                .checked_mul(2)
                .ok_or(DynamicTelemetryError::InvalidTelemetry)?,
        )
        .and_then(|value| value.checked_add(if includes_charging { 4 } else { 0 }))
        .and_then(|value| value.checked_add(if includes_alerts { 9 } else { 0 }))
        .ok_or(DynamicTelemetryError::InvalidTelemetry)?;
    if payload.len() < expected {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }
    if payload.len() != expected {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }

    let mut values = DynamicValues::default();
    let mut extensions = Vec::new();
    let names = layout.always_present.iter().chain(
        includes_runtime
            .then_some(layout.running_only.iter())
            .into_iter()
            .flatten(),
    );
    let mut index = 13;
    for name in names {
        let value = decode_float16(u16::from_be_bytes([payload[index], payload[index + 1]]));
        index += 2;
        if !assign_known(&mut values, name, value) {
            if extensions.len() == MAX_EXTENSIONS {
                return Err(DynamicTelemetryError::InvalidTelemetry);
            }
            extensions.push(ExtensionValue {
                name: name.clone(),
                value,
            });
        }
    }

    let (charging_current_a, charging_voltage_v) = if includes_charging {
        let current = decode_float16(u16::from_be_bytes([payload[index], payload[index + 1]]));
        let voltage = decode_float16(u16::from_be_bytes([payload[index + 2], payload[index + 3]]));
        index += 4;
        (Some(current), Some(voltage))
    } else {
        (None, None)
    };

    let (active_alert_words, fatal_error_latched, firmware_fault_raw_code, firmware_fault) =
        if includes_alerts {
            let words = [read_u32(payload, index), read_u32(payload, index + 4)];
            let raw_code = payload[index + 8];
            index += 9;
            let fault_active = words[0] & FIRMWARE_FAULT_ALERT != 0;
            if fault_active != (raw_code != 0) {
                return Err(DynamicTelemetryError::InvalidTelemetry);
            }
            (
                Some(words),
                Some(extra_flags & FATAL_ERROR_FLAG != 0),
                fault_active.then_some(raw_code),
                fault_active.then(|| firmware_fault(raw_code)),
            )
        } else {
            (None, None, None, None)
        };
    debug_assert_eq!(index, payload.len());

    Ok(DynamicSample {
        source_ticks: read_u32(payload, 5),
        package_state,
        package_mode: package_mode((state_and_mode >> 4) & 0x03),
        footpad_state: footpad_state((footpad_and_flags >> 6) & 0x03),
        stop_condition: stop_condition(payload[11] & 0x0f),
        setpoint_adjustment: setpoint_adjustment(payload[11] >> 4),
        wheelslip: footpad_and_flags & 0x01 != 0,
        darkride: footpad_and_flags & 0x02 != 0,
        charging,
        alert_reason: alert_reason(payload[12]),
        fatal_error_latched,
        active_alert_words,
        firmware_fault_raw_code,
        firmware_fault,
        values,
        charging_current_a,
        charging_voltage_v,
        extensions,
    })
}

fn validate_envelope(payload: &[u8], command: u8) -> Result<(), DynamicTelemetryError> {
    if payload.len() < 3 {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }
    if payload[..3] != [CUSTOM_APP_DATA_COMMAND, REFLOAT_PACKAGE_ID, command] {
        return Err(DynamicTelemetryError::UnsupportedEnvelope);
    }
    Ok(())
}

fn decode_name_list(
    payload: &[u8],
    index: &mut usize,
    count: usize,
    aggregate_name_bytes: &mut usize,
) -> Result<Vec<String>, DynamicTelemetryError> {
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        let length = usize::from(
            *payload
                .get(*index)
                .ok_or(DynamicTelemetryError::TruncatedPayload)?,
        );
        *index += 1;
        if length == 0 || length > MAX_FIELD_BYTES {
            return Err(DynamicTelemetryError::InvalidLayout);
        }
        let end = index
            .checked_add(length)
            .ok_or(DynamicTelemetryError::InvalidLayout)?;
        let bytes = payload
            .get(*index..end)
            .ok_or(DynamicTelemetryError::TruncatedPayload)?;
        *aggregate_name_bytes = aggregate_name_bytes
            .checked_add(length)
            .filter(|total| *total <= MAX_AGGREGATE_FIELD_BYTES)
            .ok_or(DynamicTelemetryError::InvalidLayout)?;
        let name = std::str::from_utf8(bytes)
            .map_err(|_| DynamicTelemetryError::InvalidLayout)?
            .to_owned();
        names.push(name);
        *index = end;
    }
    Ok(names)
}

fn assign_known(values: &mut DynamicValues, name: &str, value: f64) -> bool {
    let target = match name {
        "motor.speed" => &mut values.speed_kph,
        "motor.erpm" => &mut values.erpm,
        "motor.current" => &mut values.motor_current_a,
        "motor.dir_current" => &mut values.directional_motor_current_a,
        "motor.filt_current" => &mut values.filtered_motor_current_a,
        "motor.duty_cycle" => &mut values.duty_cycle,
        "motor.batt_voltage" => &mut values.pack_voltage_v,
        "motor.batt_current" => &mut values.battery_current_a,
        "motor.mosfet_temp" => &mut values.controller_temp_c,
        "motor.motor_temp" => &mut values.motor_temp_c,
        "imu.pitch" => &mut values.pitch_deg,
        "imu.balance_pitch" => &mut values.balance_pitch_deg,
        "imu.roll" => &mut values.roll_deg,
        "footpad.adc1" => &mut values.footpad_adc1_v,
        "footpad.adc2" => &mut values.footpad_adc2_v,
        "remote.input" => &mut values.remote_input,
        "setpoint" => &mut values.setpoint_deg,
        "atr.setpoint" => &mut values.atr_setpoint_deg,
        "brake_tilt.setpoint" => &mut values.brake_tilt_setpoint_deg,
        "torque_tilt.setpoint" => &mut values.torque_tilt_setpoint_deg,
        "turn_tilt.setpoint" => &mut values.turn_tilt_setpoint_deg,
        "remote.setpoint" => &mut values.remote_setpoint_deg,
        "balance_current" => &mut values.requested_balance_current_a,
        "atr.accel_diff" => &mut values.atr_acceleration_difference_erpm_per_second,
        "atr.speed_boost" => &mut values.atr_speed_boost,
        "booster.current" => &mut values.booster_current_a,
        _ => return false,
    };
    *target = Some(value);
    true
}

fn decode_float16(bits: u16) -> f64 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f64::from(bits & 0x03ff);
    if exponent == 0 {
        sign * fraction * 2.0_f64.powi(-24)
    } else {
        sign * (1.0 + fraction / 1024.0) * 2.0_f64.powi(exponent - 15)
    }
}

fn read_u32(payload: &[u8], index: usize) -> u32 {
    u32::from_be_bytes([
        payload[index],
        payload[index + 1],
        payload[index + 2],
        payload[index + 3],
    ])
}

const fn package_state(code: u8) -> PackageState {
    match code {
        0 => PackageState::Disabled,
        1 => PackageState::Startup,
        2 => PackageState::Ready,
        _ => PackageState::Running,
    }
}

const fn package_mode(code: u8) -> PackageMode {
    match code {
        0 => PackageMode::Normal,
        1 => PackageMode::HandTest,
        2 => PackageMode::Flywheel,
        _ => PackageMode::Unknown,
    }
}

const fn footpad_state(code: u8) -> FootpadState {
    match code {
        0 => FootpadState::None,
        1 => FootpadState::Left,
        2 => FootpadState::Right,
        _ => FootpadState::Both,
    }
}

const fn stop_condition(code: u8) -> StopCondition {
    match code {
        0 => StopCondition::None,
        1 => StopCondition::Pitch,
        2 => StopCondition::Roll,
        3 => StopCondition::SwitchHalf,
        4 => StopCondition::SwitchFull,
        5 => StopCondition::ReverseStop,
        6 => StopCondition::QuickStop,
        _ => StopCondition::Unknown,
    }
}

const fn setpoint_adjustment(code: u8) -> SetpointAdjustment {
    match code {
        0 => SetpointAdjustment::None,
        1 => SetpointAdjustment::Centering,
        2 => SetpointAdjustment::ReverseStop,
        5 => SetpointAdjustment::Speed,
        6 => SetpointAdjustment::Duty,
        7 => SetpointAdjustment::Error,
        10 => SetpointAdjustment::HighVoltage,
        11 => SetpointAdjustment::LowVoltage,
        12 => SetpointAdjustment::Temperature,
        _ => SetpointAdjustment::Unknown,
    }
}

const fn alert_reason(code: u8) -> AlertReason {
    match code {
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
        16 => AlertReason::CellImbalance,
        17 => AlertReason::BmsConnection,
        18 => AlertReason::BmsOverTemperature,
        19 => AlertReason::FirmwareFault,
        _ => AlertReason::Unknown,
    }
}

const fn firmware_fault(code: u8) -> FirmwareFault {
    match code {
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

    const FIXTURES: &str =
        include_str!("../../../fixtures/refloat/detailed-source-provenance.json");

    #[test]
    fn decodes_committed_layout_and_every_dynamic_fixture() {
        let layout = decode_layout(&fixture_payload("1.1.0-and-1.2.3-command-32-layout")).unwrap();
        assert_eq!(layout.always_present.len(), 16);
        assert_eq!(layout.running_only.len(), 10);
        assert_eq!(
            layout.always_present,
            [
                "motor.speed",
                "motor.erpm",
                "motor.current",
                "motor.dir_current",
                "motor.filt_current",
                "motor.duty_cycle",
                "motor.batt_voltage",
                "motor.batt_current",
                "motor.mosfet_temp",
                "motor.motor_temp",
                "imu.pitch",
                "imu.balance_pitch",
                "imu.roll",
                "footpad.adc1",
                "footpad.adc2",
                "remote.input",
            ]
        );
        assert_eq!(
            layout.running_only,
            [
                "setpoint",
                "atr.setpoint",
                "brake_tilt.setpoint",
                "torque_tilt.setpoint",
                "turn_tilt.setpoint",
                "remote.setpoint",
                "balance_current",
                "atr.accel_diff",
                "atr.speed_boost",
                "booster.current",
            ]
        );

        let running_11 =
            decode_realtime(&layout, &fixture_payload("1.1.0-command-31-running")).unwrap();
        assert_running_sample(&running_11);
        assert_eq!(running_11.active_alert_words, None);
        assert_eq!(running_11.fatal_error_latched, None);

        let running_12 = decode_realtime(
            &layout,
            &fixture_payload("1.2.3-command-31-running-alert-tail"),
        )
        .unwrap();
        assert_running_sample(&running_12);
        assert_eq!(
            known_values(&running_12.values),
            [
                Some(23.96875),
                Some(5_632.0),
                Some(48.0),
                Some(46.0),
                Some(43.0),
                Some(0.64013671875),
                Some(60.75),
                Some(25.0),
                Some(30.0),
                Some(28.0),
                Some(3.19921875),
                Some(1.099609375),
                Some(0.7998046875),
                Some(2.80078125),
                Some(2.75),
                Some(0.0),
                Some(1.0),
                Some(2.400390625),
                Some(0.0),
                Some(1.7998046875),
                Some(0.0),
                Some(0.0),
                Some(52.0),
                Some(1_250.0),
                Some(0.1800537109375),
                Some(6.0),
            ]
        );
        assert_eq!(running_12.active_alert_words, Some([0, 0]));
        assert_eq!(running_12.fatal_error_latched, Some(false));

        let charging =
            decode_realtime(&layout, &fixture_payload("1.2.3-command-31-charging-tail")).unwrap();
        assert_eq!(charging.package_state, PackageState::Ready);
        assert!(charging.charging);
        assert!(charging.charging_current_a.is_some());
        assert!(charging.charging_voltage_v.is_some());
        assert_eq!(charging.charging_current_a, Some(3.5));
        assert_eq!(charging.charging_voltage_v, Some(67.125));
        assert!(charging.values.setpoint_deg.is_none());
        assert_eq!(charging.active_alert_words, Some([0, 0]));

        let fault = decode_realtime(
            &layout,
            &fixture_payload("1.2.3-command-31-active-firmware-fault"),
        )
        .unwrap();
        assert_eq!(fault.alert_reason, AlertReason::FirmwareFault);
        assert_eq!(fault.active_alert_words, Some([1, 0]));
        assert_eq!(fault.firmware_fault_raw_code, Some(1));
        assert_eq!(fault.firmware_fault, Some(FirmwareFault::OverVoltage));

        let mut ticked = fixture_payload("1.1.0-command-31-running");
        ticked[5..9].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            decode_realtime(&layout, &ticked).unwrap().source_ticks,
            u32::MAX
        );
    }

    #[test]
    fn decodes_complete_finite_float16_domain_boundaries() {
        let cases = [
            (0x0000, 0.0),
            (0x8000, -0.0),
            (0x0001, 2.0_f64.powi(-24)),
            (0x8001, -2.0_f64.powi(-24)),
            (0x0400, 2.0_f64.powi(-14)),
            (0x3c00, 1.0),
            (0x7c00, 65_536.0),
            (0x7fff, 131_008.0),
            (0xffff, -131_008.0),
        ];
        for (bits, expected) in cases {
            let actual = decode_float16(bits);
            assert_eq!(actual, expected, "{bits:04x}");
            assert!(actual.is_finite());
        }
        assert!(decode_float16(0x8000).is_sign_negative());
        for bits in 0..=u16::MAX {
            assert!(decode_float16(bits).is_finite(), "{bits:04x}");
        }
    }

    #[test]
    fn rejects_layout_envelope_length_utf8_uniqueness_and_bounds() {
        let valid = fixture_payload("1.1.0-and-1.2.3-command-32-layout");
        for length in 0..valid.len() {
            assert!(decode_layout(&valid[..length]).is_err(), "length {length}");
        }
        let mut overlong = valid.clone();
        overlong.push(0);
        assert_eq!(
            decode_layout(&overlong),
            Err(DynamicTelemetryError::InvalidLayout)
        );
        for (index, value) in [(0, 35), (1, 100), (2, 31)] {
            let mut wrong = valid.clone();
            wrong[index] = value;
            assert_eq!(
                decode_layout(&wrong),
                Err(DynamicTelemetryError::UnsupportedEnvelope)
            );
        }

        assert_eq!(
            decode_layout(&layout_payload(&[], &[])),
            Err(DynamicTelemetryError::InvalidLayout)
        );
        assert_eq!(
            decode_layout(&layout_payload(&[""], &[])),
            Err(DynamicTelemetryError::InvalidLayout)
        );
        assert_eq!(
            decode_layout(&layout_payload(&[&"x".repeat(65)], &[])),
            Err(DynamicTelemetryError::InvalidLayout)
        );
        let mut invalid_utf8 = vec![36, 101, 32, 1, 1, 0xff, 0];
        assert_eq!(
            decode_layout(&invalid_utf8),
            Err(DynamicTelemetryError::InvalidLayout)
        );
        invalid_utf8[5] = b'x';
        assert!(decode_layout(&invalid_utf8).is_ok());
        assert_eq!(
            decode_layout(&layout_payload(&["same"], &["same"])),
            Err(DynamicTelemetryError::InvalidLayout)
        );

        let exact_names: Vec<String> = (0..MAX_FIELDS)
            .map(|index| format!("{index:02}{}", "x".repeat(62)))
            .collect();
        let exact_refs: Vec<&str> = exact_names.iter().map(String::as_str).collect();
        let exact = decode_layout(&layout_payload(&exact_refs, &[])).unwrap();
        assert_eq!(exact.field_count(), MAX_FIELDS);
        assert_eq!(exact.aggregate_name_bytes(), MAX_AGGREGATE_FIELD_BYTES);
        let mut excessive = exact_refs;
        excessive.push("overflow");
        assert_eq!(
            decode_layout(&layout_payload(&excessive[..64], &[excessive[64]])),
            Err(DynamicTelemetryError::InvalidLayout)
        );
    }

    #[test]
    fn rejects_realtime_envelope_masks_extents_reserved_bits_and_tail_relations() {
        let layout = committed_layout();
        let valid = fixture_payload("1.2.3-command-31-running-alert-tail");
        for length in 0..valid.len() {
            assert!(
                decode_realtime(&layout, &valid[..length]).is_err(),
                "length {length}"
            );
        }
        let mut overlong = valid.clone();
        overlong.push(0);
        assert_eq!(
            decode_realtime(&layout, &overlong),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
        for (index, value) in [(0, 35), (1, 100), (2, 32)] {
            let mut wrong = valid.clone();
            wrong[index] = value;
            assert_eq!(
                decode_realtime(&layout, &wrong),
                Err(DynamicTelemetryError::UnsupportedEnvelope)
            );
        }

        for (index, bit) in [(3, 0x08), (4, 0x10), (9, 0x04), (10, 0x04)] {
            let mut invalid = valid.clone();
            invalid[index] |= bit;
            assert_eq!(
                decode_realtime(&layout, &invalid),
                Err(DynamicTelemetryError::InvalidTelemetry),
                "index {index}"
            );
        }
        let mut runtime_mismatch = valid.clone();
        runtime_mismatch[9] = 2;
        assert_eq!(
            decode_realtime(&layout, &runtime_mismatch),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
        let mut charging_mismatch = valid.clone();
        charging_mismatch[10] |= CHARGING_FLAG;
        assert_eq!(
            decode_realtime(&layout, &charging_mismatch),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
        let mut fault_without_code = fixture_payload("1.2.3-command-31-active-firmware-fault");
        *fault_without_code.last_mut().unwrap() = 0;
        assert_eq!(
            decode_realtime(&layout, &fault_without_code),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
        let mut code_without_fault = valid;
        *code_without_fault.last_mut().unwrap() = 1;
        assert_eq!(
            decode_realtime(&layout, &code_without_fault),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
    }

    #[test]
    fn bounds_extensions_and_preserves_order() {
        let names: Vec<String> = (0..MAX_EXTENSIONS)
            .map(|index| format!("future.{index}"))
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let layout = decode_layout(&layout_payload(&refs, &[])).unwrap();
        let sample = decode_realtime(&layout, &realtime_payload(0, refs.len(), 0x3c00)).unwrap();
        assert_eq!(sample.extensions.len(), MAX_EXTENSIONS);
        assert_eq!(sample.extensions[0].name, "future.0");
        assert_eq!(sample.extensions[31].name, "future.31");
        assert!(sample.extensions.iter().all(|item| item.value == 1.0));

        let excessive_names: Vec<String> = (0..=MAX_EXTENSIONS)
            .map(|index| format!("future.{index}"))
            .collect();
        let excessive_refs: Vec<&str> = excessive_names.iter().map(String::as_str).collect();
        let excessive_layout = decode_layout(&layout_payload(&excessive_refs, &[])).unwrap();
        assert_eq!(
            decode_realtime(
                &excessive_layout,
                &realtime_payload(0, excessive_refs.len(), 0x3c00)
            ),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
    }

    #[test]
    fn maps_all_semantics_faults_and_fatal_latch() {
        assert_eq!(package_mode(3), PackageMode::Unknown);
        assert_eq!(stop_condition(15), StopCondition::Unknown);
        for (code, expected) in [
            (0, SetpointAdjustment::None),
            (1, SetpointAdjustment::Centering),
            (2, SetpointAdjustment::ReverseStop),
            (5, SetpointAdjustment::Speed),
            (6, SetpointAdjustment::Duty),
            (7, SetpointAdjustment::Error),
            (10, SetpointAdjustment::HighVoltage),
            (11, SetpointAdjustment::LowVoltage),
            (12, SetpointAdjustment::Temperature),
            (15, SetpointAdjustment::Unknown),
        ] {
            assert_eq!(setpoint_adjustment(code), expected);
        }
        assert_eq!(alert_reason(19), AlertReason::FirmwareFault);
        assert_eq!(alert_reason(255), AlertReason::Unknown);
        assert_eq!(firmware_fault(27), FirmwareFault::PhaseFilter);
        assert_eq!(firmware_fault(255), FirmwareFault::Unknown);

        let layout = committed_layout();
        let mut latched = fixture_payload("1.2.3-command-31-running-alert-tail");
        latched[4] |= FATAL_ERROR_FLAG;
        assert_eq!(
            decode_realtime(&layout, &latched)
                .unwrap()
                .fatal_error_latched,
            Some(true)
        );
    }

    #[test]
    fn output_is_deterministic_and_diagnostics_are_redacted() {
        let layout_payload = fixture_payload("1.1.0-and-1.2.3-command-32-layout");
        let first_layout = decode_layout(&layout_payload).unwrap();
        let second_layout = decode_layout(&layout_payload).unwrap();
        assert_eq!(first_layout, second_layout);
        let payload = fixture_payload("1.2.3-command-31-running-alert-tail");
        let first = decode_realtime(&first_layout, &payload).unwrap();
        let second = decode_realtime(&second_layout, &payload).unwrap();
        assert_eq!(first, second);
        let debug = format!("{first:?}");
        assert!(!debug.contains("motor.speed"));
        assert!(!debug.contains("131008"));
        assert_eq!(
            DynamicTelemetryError::InvalidTelemetry.to_string(),
            "invalid Refloat dynamic telemetry"
        );
    }

    fn assert_running_sample(sample: &DynamicSample) {
        assert_eq!(sample.source_ticks, 0);
        assert_eq!(sample.package_state, PackageState::Running);
        assert_eq!(sample.package_mode, PackageMode::Normal);
        assert_eq!(sample.footpad_state, FootpadState::Both);
        assert_eq!(sample.stop_condition, StopCondition::None);
        assert_eq!(sample.setpoint_adjustment, SetpointAdjustment::None);
        assert!(!sample.wheelslip);
        assert!(!sample.darkride);
        assert!(!sample.charging);
        assert_eq!(sample.alert_reason, AlertReason::None);
        assert!(sample.values.speed_kph.is_some());
        assert!(sample.values.directional_motor_current_a.is_some());
        assert!(sample.values.filtered_motor_current_a.is_some());
        assert!(sample.values.remote_input.is_some());
        assert!(sample.values.setpoint_deg.is_some());
        assert!(
            sample
                .values
                .atr_acceleration_difference_erpm_per_second
                .is_some()
        );
        assert!(sample.values.atr_speed_boost.is_some());
        assert!(sample.values.booster_current_a.is_some());
        assert!(sample.extensions.is_empty());
    }

    fn known_values(values: &DynamicValues) -> [Option<f64>; 26] {
        [
            values.speed_kph,
            values.erpm,
            values.motor_current_a,
            values.directional_motor_current_a,
            values.filtered_motor_current_a,
            values.duty_cycle,
            values.pack_voltage_v,
            values.battery_current_a,
            values.controller_temp_c,
            values.motor_temp_c,
            values.pitch_deg,
            values.balance_pitch_deg,
            values.roll_deg,
            values.footpad_adc1_v,
            values.footpad_adc2_v,
            values.remote_input,
            values.setpoint_deg,
            values.atr_setpoint_deg,
            values.brake_tilt_setpoint_deg,
            values.torque_tilt_setpoint_deg,
            values.turn_tilt_setpoint_deg,
            values.remote_setpoint_deg,
            values.requested_balance_current_a,
            values.atr_acceleration_difference_erpm_per_second,
            values.atr_speed_boost,
            values.booster_current_a,
        ]
    }

    fn committed_layout() -> DynamicLayout {
        decode_layout(&fixture_payload("1.1.0-and-1.2.3-command-32-layout")).unwrap()
    }

    fn layout_payload(always: &[&str], running: &[&str]) -> Vec<u8> {
        let mut payload = vec![36, 101, 32, always.len() as u8];
        for name in always {
            payload.push(name.len() as u8);
            payload.extend_from_slice(name.as_bytes());
        }
        payload.push(running.len() as u8);
        for name in running {
            payload.push(name.len() as u8);
            payload.extend_from_slice(name.as_bytes());
        }
        payload
    }

    fn realtime_payload(mask: u8, value_count: usize, bits: u16) -> Vec<u8> {
        let mut payload = vec![36, 101, 31, mask, 0, 0, 0, 0, 0, 2, 0, 0, 0];
        for _ in 0..value_count {
            payload.extend_from_slice(&bits.to_be_bytes());
        }
        payload
    }

    fn fixture_payload(name: &str) -> Vec<u8> {
        let marker = format!("\"name\": \"{name}\"");
        let case = &FIXTURES[FIXTURES.find(&marker).unwrap()..];
        hex_bytes(json_string(case, "payloadHex"))
    }

    fn json_string<'a>(json: &'a str, key: &str) -> &'a str {
        let prefix = format!("\"{key}\": \"");
        let start = json.find(&prefix).unwrap() + prefix.len();
        let rest = &json[start..];
        &rest[..rest.find('"').unwrap()]
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
}
