use std::collections::{BTreeMap, BTreeSet};

use domain_types::RideState;

use crate::{PACKAGE_ID, RefloatCommand};

pub const MAX_FIELDS: usize = 64;
pub const MAX_FIELD_BYTES: usize = 64;
pub const MAX_AGGREGATE_FIELD_BYTES: usize = 4096;
pub const MAX_EXTENSIONS: usize = 32;

pub const NORMALIZED_NUMERIC_FIELDS: &[&str] = &[
    "speedKph",
    "erpm",
    "pitchDeg",
    "balancePitchDeg",
    "rollDeg",
    "motorCurrentA",
    "directionalMotorCurrentA",
    "filteredMotorCurrentA",
    "batteryCurrentA",
    "requestedBalanceCurrentA",
    "boosterCurrentA",
    "fieldWeakeningCurrentA",
    "dutyCycle",
    "packVoltageV",
    "controllerTempC",
    "motorTempC",
    "footpadAdc1V",
    "footpadAdc2V",
    "remoteInput",
    "setpointDeg",
    "atrSetpointDeg",
    "brakeTiltSetpointDeg",
    "torqueTiltSetpointDeg",
    "turnTiltSetpointDeg",
    "remoteSetpointDeg",
    "atrAccelerationDifferenceErpmPerSecond",
    "atrSpeedBoost",
    "distanceM",
    "odometerM",
    "ampHoursDrawnAh",
    "ampHoursChargedAh",
    "wattHoursDrawnWh",
    "wattHoursChargedWh",
    "batteryLevel",
    "chargingCurrentA",
    "chargingVoltageV",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageState {
    Disabled,
    Startup,
    Ready,
    Running,
    Unknown,
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
    Unknown,
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

#[derive(Clone, Debug, PartialEq)]
pub struct Telemetry {
    pub ride_state: RideState,
    pub package_state: PackageState,
    pub package_mode: PackageMode,
    pub footpad_state: FootpadState,
    pub stop_condition: StopCondition,
    pub setpoint_adjustment: SetpointAdjustment,
    pub wheelslip: bool,
    pub darkride: bool,
    pub charging: bool,
    pub alert_reason: AlertReason,
    pub firmware_fault: Option<u8>,
    pub fatal_error_latched: Option<bool>,
    pub numeric: BTreeMap<&'static str, f64>,
    pub extensions: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelemetryLayout {
    pub fields: Vec<String>,
    pub runtime_fields: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryError {
    InvalidFrame,
    Truncated,
    InvalidUtf8,
}

/// Decodes a bounded dynamic field layout.
///
/// # Errors
///
/// Rejects truncated, duplicate, empty, oversized, and invalid UTF-8 names.
pub fn decode_layout(payload: &[u8]) -> Result<TelemetryLayout, TelemetryError> {
    if payload.len() < 5
        || payload[..3]
            != [
                36,
                PACKAGE_ID,
                RefloatCommand::RealtimeDataInternalIds as u8,
            ]
    {
        return Err(TelemetryError::Truncated);
    }
    let mut index = 3;
    let mut aggregate = 0;
    let fields = decode_field_list(payload, &mut index, &mut aggregate)?;
    let runtime_fields = decode_field_list(payload, &mut index, &mut aggregate)?;
    let unique: BTreeSet<&str> = fields
        .iter()
        .chain(&runtime_fields)
        .map(String::as_str)
        .collect();
    if index != payload.len()
        || fields.len() + runtime_fields.len() > MAX_FIELDS
        || unique.len() != fields.len() + runtime_fields.len()
    {
        return Err(TelemetryError::InvalidFrame);
    }
    Ok(TelemetryLayout {
        fields,
        runtime_fields,
    })
}

fn decode_field_list(
    payload: &[u8],
    index: &mut usize,
    aggregate: &mut usize,
) -> Result<Vec<String>, TelemetryError> {
    let count = usize::from(*payload.get(*index).ok_or(TelemetryError::Truncated)?);
    *index += 1;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let length = usize::from(*payload.get(*index).ok_or(TelemetryError::Truncated)?);
        *index += 1;
        if length == 0 || length > MAX_FIELD_BYTES {
            return Err(TelemetryError::InvalidFrame);
        }
        let end = index.checked_add(length).ok_or(TelemetryError::Truncated)?;
        let bytes = payload.get(*index..end).ok_or(TelemetryError::Truncated)?;
        let value = std::str::from_utf8(bytes).map_err(|_| TelemetryError::InvalidUtf8)?;
        *aggregate = aggregate
            .checked_add(length)
            .ok_or(TelemetryError::InvalidFrame)?;
        if *aggregate > MAX_AGGREGATE_FIELD_BYTES {
            return Err(TelemetryError::InvalidFrame);
        }
        values.push(value.to_owned());
        *index = end;
    }
    Ok(values)
}

/// Decodes legacy fixed telemetry into normalized fields.
///
/// # Errors
///
/// Rejects invalid command/detail/length shapes.
#[allow(clippy::too_many_lines)]
pub fn decode_legacy(payload: &[u8]) -> Result<Telemetry, TelemetryError> {
    if payload.len() < 4 || payload[..3] != [36, PACKAGE_ID, RefloatCommand::GetAllData as u8] {
        return Err(TelemetryError::Truncated);
    }
    if payload[3] == 69 {
        if payload.len() != 5 {
            return Err(TelemetryError::InvalidFrame);
        }
        return Ok(base(
            PackageState::Unknown,
            RideState::Unknown,
            FootpadState::Unknown,
        ));
    }
    let detail = payload[3];
    if !(1..=4).contains(&detail) {
        return Err(TelemetryError::InvalidFrame);
    }
    let required = if detail >= 4 {
        59
    } else if detail >= 3 {
        55
    } else if detail >= 2 {
        42
    } else {
        35
    };
    if payload.len() != required {
        return Err(TelemetryError::Truncated);
    }
    let signed = |index: usize| f64::from(i16::from_be_bytes([payload[index], payload[index + 1]]));
    let state = payload[10] & 0x0f;
    let package_state = match state {
        0 => PackageState::Startup,
        1..=5 => PackageState::Running,
        15 => PackageState::Disabled,
        _ => PackageState::Ready,
    };
    let adc1 = f64::from(payload[12]) / 50.0;
    let adc2 = f64::from(payload[13]) / 50.0;
    let footpad = match payload[11] & 7 {
        2 => FootpadState::Both,
        1 if adc1 > adc2 => FootpadState::Left,
        1 => FootpadState::Right,
        _ => FootpadState::None,
    };
    let mut value = base(
        package_state,
        if package_state == PackageState::Running {
            RideState::Riding
        } else {
            RideState::Idle
        },
        footpad,
    );
    value.package_mode = if payload[11] & 8 != 0 {
        PackageMode::HandTest
    } else if state == 5 {
        PackageMode::Flywheel
    } else {
        PackageMode::Normal
    };
    value.wheelslip = state == 3;
    value.darkride = state == 4;
    value.charging = state == 14;
    insert(&mut value, "packVoltageV", signed(23) / 10.0);
    insert(
        &mut value,
        "dutyCycle",
        (f64::from(payload[33]) - 128.0) / 100.0,
    );
    insert(&mut value, "motorCurrentA", signed(29) / 10.0);
    insert(&mut value, "batteryCurrentA", signed(31) / 10.0);
    insert(&mut value, "erpm", signed(25));
    insert(&mut value, "speedKph", signed(27) / 10.0 * 3.6);
    insert(&mut value, "pitchDeg", signed(20) / 10.0);
    insert(&mut value, "balancePitchDeg", signed(6) / 10.0);
    insert(&mut value, "rollDeg", signed(8) / 10.0);
    insert(&mut value, "requestedBalanceCurrentA", signed(4) / 10.0);
    insert(
        &mut value,
        "boosterCurrentA",
        f64::from(payload[22]) - 128.0,
    );
    insert(
        &mut value,
        "fieldWeakeningCurrentA",
        f64::from(payload[34]) / 3.0,
    );
    insert(&mut value, "footpadAdc1V", adc1);
    insert(&mut value, "footpadAdc2V", adc2);
    for (name, index) in [
        ("setpointDeg", 14),
        ("atrSetpointDeg", 15),
        ("brakeTiltSetpointDeg", 16),
        ("torqueTiltSetpointDeg", 17),
        ("turnTiltSetpointDeg", 18),
        ("remoteSetpointDeg", 19),
    ] {
        insert(&mut value, name, (f64::from(payload[index]) - 128.0) / 5.0);
    }
    if detail >= 2 {
        insert(
            &mut value,
            "distanceM",
            f64::from(f32::from_bits(u32::from_be_bytes(
                payload[35..39]
                    .try_into()
                    .map_err(|_| TelemetryError::Truncated)?,
            ))),
        );
        insert(&mut value, "controllerTempC", f64::from(payload[39]) / 2.0);
        insert(&mut value, "motorTempC", f64::from(payload[40]) / 2.0);
    }
    if detail >= 3 {
        insert(
            &mut value,
            "odometerM",
            f64::from(u32::from_be_bytes(
                payload[42..46]
                    .try_into()
                    .map_err(|_| TelemetryError::Truncated)?,
            )),
        );
        insert(&mut value, "ampHoursDrawnAh", signed(46) / 10.0);
        insert(&mut value, "ampHoursChargedAh", signed(48) / 10.0);
        insert(&mut value, "wattHoursDrawnWh", signed(50));
        insert(&mut value, "wattHoursChargedWh", signed(52));
        insert(&mut value, "batteryLevel", f64::from(payload[54]) / 200.0);
    }
    if detail >= 4 {
        insert(&mut value, "chargingCurrentA", signed(55) / 10.0);
        insert(&mut value, "chargingVoltageV", signed(57) / 10.0);
    }
    Ok(value)
}

/// Decodes dynamic half-float telemetry with bounded extensions.
///
/// # Errors
///
/// Rejects missing layouts, invalid masks/lengths, non-finite half floats,
/// and excess extension fields.
pub fn decode_dynamic(
    payload: &[u8],
    layout: &TelemetryLayout,
) -> Result<Telemetry, TelemetryError> {
    if payload.len() < 13
        || layout.fields.is_empty()
        || payload[..3] != [36, PACKAGE_ID, RefloatCommand::RealtimeDataInternal as u8]
    {
        return Err(TelemetryError::InvalidFrame);
    }
    let mask = payload[3];
    let runtime = if mask & 1 != 0 {
        &layout.runtime_fields[..]
    } else {
        &[]
    };
    let expected = 13
        + (layout.fields.len() + runtime.len()) * 2
        + if mask & 2 != 0 { 4 } else { 0 }
        + if mask & 4 != 0 { 9 } else { 0 };
    if payload.len() != expected {
        return Err(TelemetryError::Truncated);
    }
    let mut index = 13;
    let mut wire = BTreeMap::new();
    for field in layout.fields.iter().chain(runtime) {
        wire.insert(field.clone(), half(payload[index], payload[index + 1])?);
        index += 2;
    }
    let state = package_state(payload[9] & 3);
    let flags = payload[10];
    let mut value = base(
        state,
        if state == PackageState::Running {
            RideState::Riding
        } else {
            RideState::Idle
        },
        footpad(flags >> 6 & 3),
    );
    value.package_mode = package_mode(payload[9] >> 4 & 3);
    value.wheelslip = flags & 1 != 0;
    value.darkride = flags & 2 != 0;
    value.charging = flags & 0x20 != 0;
    for (wire_name, normalized) in WIRE_FIELDS {
        if let Some(number) = wire.remove(*wire_name) {
            insert(&mut value, normalized, number);
        }
    }
    if mask & 2 != 0 {
        insert(
            &mut value,
            "chargingCurrentA",
            half(payload[index], payload[index + 1])?,
        );
        insert(
            &mut value,
            "chargingVoltageV",
            half(payload[index + 2], payload[index + 3])?,
        );
        index += 4;
    }
    if mask & 4 != 0 {
        let active = u32::from_be_bytes(
            payload[index..index + 4]
                .try_into()
                .map_err(|_| TelemetryError::Truncated)?,
        );
        value.firmware_fault = (active & 1 != 0)
            .then_some(payload[index + 8])
            .filter(|code| *code != 0);
        value.fatal_error_latched = Some(payload[4] & 8 != 0);
    }
    if wire.len() > MAX_EXTENSIONS {
        return Err(TelemetryError::InvalidFrame);
    }
    value.extensions = wire;
    Ok(value)
}

const WIRE_FIELDS: &[(&str, &str)] = &[
    ("motor.speed", "speedKph"),
    ("motor.erpm", "erpm"),
    ("motor.current", "motorCurrentA"),
    ("motor.dir_current", "directionalMotorCurrentA"),
    ("motor.filt_current", "filteredMotorCurrentA"),
    ("motor.duty_cycle", "dutyCycle"),
    ("motor.batt_voltage", "packVoltageV"),
    ("motor.batt_current", "batteryCurrentA"),
    ("motor.mosfet_temp", "controllerTempC"),
    ("motor.motor_temp", "motorTempC"),
    ("imu.pitch", "pitchDeg"),
    ("imu.balance_pitch", "balancePitchDeg"),
    ("imu.roll", "rollDeg"),
    ("footpad.adc1", "footpadAdc1V"),
    ("footpad.adc2", "footpadAdc2V"),
    ("remote.input", "remoteInput"),
    ("setpoint", "setpointDeg"),
    ("atr.setpoint", "atrSetpointDeg"),
    ("brake_tilt.setpoint", "brakeTiltSetpointDeg"),
    ("torque_tilt.setpoint", "torqueTiltSetpointDeg"),
    ("turn_tilt.setpoint", "turnTiltSetpointDeg"),
    ("remote.setpoint", "remoteSetpointDeg"),
    ("balance_current", "requestedBalanceCurrentA"),
    ("atr.accel_diff", "atrAccelerationDifferenceErpmPerSecond"),
    ("atr.speed_boost", "atrSpeedBoost"),
    ("booster.current", "boosterCurrentA"),
];

fn half(high: u8, low: u8) -> Result<f64, TelemetryError> {
    let bits = u16::from_be_bytes([high, low]);
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = u32::from(bits & 0x03ff);
    if exponent == 0x1f {
        return Err(TelemetryError::InvalidFrame);
    }
    Ok(if exponent == 0 {
        sign * f64::from(fraction) * 2_f64.powi(-24)
    } else {
        sign * (1.0 + f64::from(fraction) / 1024.0) * 2_f64.powi(exponent - 15)
    })
}
fn package_state(code: u8) -> PackageState {
    [
        PackageState::Disabled,
        PackageState::Startup,
        PackageState::Ready,
        PackageState::Running,
    ]
    .get(usize::from(code))
    .copied()
    .unwrap_or(PackageState::Unknown)
}
fn package_mode(code: u8) -> PackageMode {
    [
        PackageMode::Normal,
        PackageMode::HandTest,
        PackageMode::Flywheel,
    ]
    .get(usize::from(code))
    .copied()
    .unwrap_or(PackageMode::Unknown)
}
fn footpad(code: u8) -> FootpadState {
    [
        FootpadState::None,
        FootpadState::Left,
        FootpadState::Right,
        FootpadState::Both,
    ]
    .get(usize::from(code))
    .copied()
    .unwrap_or(FootpadState::Unknown)
}
fn base(
    package_state: PackageState,
    ride_state: RideState,
    footpad_state: FootpadState,
) -> Telemetry {
    Telemetry {
        ride_state,
        package_state,
        package_mode: PackageMode::Normal,
        footpad_state,
        stop_condition: StopCondition::None,
        setpoint_adjustment: SetpointAdjustment::None,
        wheelslip: false,
        darkride: false,
        charging: false,
        alert_reason: AlertReason::None,
        firmware_fault: None,
        fatal_error_latched: None,
        numeric: BTreeMap::new(),
        extensions: BTreeMap::new(),
    }
}
fn insert(value: &mut Telemetry, name: &'static str, number: f64) {
    value.numeric.insert(name, number);
}
