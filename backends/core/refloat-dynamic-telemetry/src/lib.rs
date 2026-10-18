//! Strict, bounded decoding of negotiated Refloat command-31/32 telemetry.
//!
//! This safe candidate owns only dynamic layout and wire decoding. Native
//! hosts retain compatibility, routing, polling, layout replacement across
//! reconnect, compact-cache merging, application behavior, and persistence.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

pub mod alerts;

pub const CUSTOM_APP_DATA_COMMAND: u8 = 36;
pub const REFLOAT_PACKAGE_ID: u8 = 101;
pub const REALTIME_DATA_COMMAND: u8 = 31;
pub const REALTIME_DATA_IDS_COMMAND: u8 = 32;
pub const SELECTABLE_REALTIME_DATA_COMMAND: u8 = 33;
pub const SELECTABLE_FLOAT32_FLAG: u8 = 1;

pub const SELECTABLE_EXTRA_FLAGS: u32 = 1 << 0;
pub const SELECTABLE_STATE_FLAGS: u32 = 1 << 1;
pub const SELECTABLE_SPEED: u32 = 1 << 6;
pub const SELECTABLE_ERPM: u32 = 1 << 7;
pub const SELECTABLE_CURRENT: u32 = 1 << 8;
pub const SELECTABLE_DIR_CURRENT: u32 = 1 << 9;
pub const SELECTABLE_FILT_CURRENT: u32 = 1 << 10;
pub const SELECTABLE_DUTY_CYCLE: u32 = 1 << 11;
pub const SELECTABLE_BATTERY_VOLTAGE: u32 = 1 << 12;
pub const SELECTABLE_BATTERY_CURRENT: u32 = 1 << 13;
pub const SELECTABLE_BATTERY_SOC: u32 = 1 << 14;
pub const SELECTABLE_MOSFET_TEMP: u32 = 1 << 15;
pub const SELECTABLE_MOTOR_TEMP: u32 = 1 << 16;
pub const SELECTABLE_PITCH: u32 = 1 << 17;
pub const SELECTABLE_BALANCE_PITCH: u32 = 1 << 18;
pub const SELECTABLE_ROLL: u32 = 1 << 19;
pub const SELECTABLE_ADC_LEFT: u32 = 1 << 20;
pub const SELECTABLE_ADC_RIGHT: u32 = 1 << 21;
pub const SELECTABLE_REMOTE_INPUT: u32 = 1 << 22;
pub const SELECTABLE_SETPOINT: u32 = 1 << 23;
pub const SELECTABLE_ATR_SETPOINT: u32 = 1 << 24;
pub const SELECTABLE_BRAKE_TILT_SETPOINT: u32 = 1 << 25;
pub const SELECTABLE_TORQUE_TILT_SETPOINT: u32 = 1 << 26;
pub const SELECTABLE_TURN_TILT_SETPOINT: u32 = 1 << 27;
pub const SELECTABLE_REMOTE_SETPOINT: u32 = 1 << 28;
pub const SELECTABLE_BALANCE_CURRENT: u32 = 1 << 29;
pub const SELECTABLE_LOOP_FREQUENCY: u32 = 1 << 30;

pub const SELECTABLE_ODOMETER: u32 = 1 << 0;
pub const SELECTABLE_DISTANCE: u32 = 1 << 1;
pub const SELECTABLE_CHARGING_VOLTAGE: u32 = 1 << 2;
pub const SELECTABLE_CHARGING_CURRENT: u32 = 1 << 3;
pub const SELECTABLE_AMP_HOURS: u32 = 1 << 4;
pub const SELECTABLE_AMP_HOURS_CHARGED: u32 = 1 << 5;
pub const SELECTABLE_WATT_HOURS: u32 = 1 << 6;
pub const SELECTABLE_WATT_HOURS_CHARGED: u32 = 1 << 7;
pub const SELECTABLE_MOTOR_ID: u32 = 1 << 8;
pub const SELECTABLE_GNSS_LATITUDE: u32 = 1 << 9;
pub const SELECTABLE_GNSS_LONGITUDE: u32 = 1 << 10;
pub const SELECTABLE_GNSS_ALTITUDE: u32 = 1 << 11;
pub const SELECTABLE_GNSS_SPEED: u32 = 1 << 12;
pub const SELECTABLE_GNSS_ACCURACY: u32 = 1 << 13;
pub const SELECTABLE_GNSS_LAST_UPDATE: u32 = 1 << 14;

pub const SELECTABLE_FAST_MASK1: u32 = SELECTABLE_EXTRA_FLAGS
    | SELECTABLE_STATE_FLAGS
    | SELECTABLE_SPEED
    | SELECTABLE_ERPM
    | SELECTABLE_CURRENT
    | SELECTABLE_DIR_CURRENT
    | SELECTABLE_FILT_CURRENT
    | SELECTABLE_DUTY_CYCLE
    | SELECTABLE_BATTERY_VOLTAGE
    | SELECTABLE_BATTERY_CURRENT
    | SELECTABLE_MOSFET_TEMP
    | SELECTABLE_MOTOR_TEMP
    | SELECTABLE_PITCH
    | SELECTABLE_BALANCE_PITCH
    | SELECTABLE_ROLL
    | SELECTABLE_ADC_LEFT
    | SELECTABLE_ADC_RIGHT
    | SELECTABLE_REMOTE_INPUT
    | SELECTABLE_SETPOINT
    | SELECTABLE_ATR_SETPOINT
    | SELECTABLE_BRAKE_TILT_SETPOINT
    | SELECTABLE_TORQUE_TILT_SETPOINT
    | SELECTABLE_TURN_TILT_SETPOINT
    | SELECTABLE_REMOTE_SETPOINT
    | SELECTABLE_BALANCE_CURRENT
    | SELECTABLE_LOOP_FREQUENCY;
pub const SELECTABLE_SLOW_MASK1: u32 = SELECTABLE_BATTERY_SOC;
pub const SELECTABLE_SLOW_MASK2: u32 = SELECTABLE_ODOMETER
    | SELECTABLE_DISTANCE
    | SELECTABLE_CHARGING_VOLTAGE
    | SELECTABLE_CHARGING_CURRENT
    | SELECTABLE_AMP_HOURS
    | SELECTABLE_AMP_HOURS_CHARGED
    | SELECTABLE_WATT_HOURS
    | SELECTABLE_WATT_HOURS_CHARGED
    | SELECTABLE_MOTOR_ID;
pub const SELECTABLE_GNSS_MASK2: u32 = SELECTABLE_GNSS_LATITUDE
    | SELECTABLE_GNSS_LONGITUDE
    | SELECTABLE_GNSS_ALTITUDE
    | SELECTABLE_GNSS_SPEED
    | SELECTABLE_GNSS_ACCURACY
    | SELECTABLE_GNSS_LAST_UPDATE;

pub const MAX_FIELDS: usize = 64;
pub const MAX_FIELD_BYTES: usize = 64;
pub const MAX_AGGREGATE_FIELD_BYTES: usize = 4_096;
pub const MAX_EXTENSIONS: usize = 32;

pub const RECORD_VERSION: u8 = 1;
pub const LAYOUT_RECORD_KIND: u8 = 1;
pub const SAMPLE_RECORD_KIND: u8 = 2;
pub const SELECTABLE_RECORD_KIND: u8 = 3;
pub const SELECTABLE_RECORD_HEADER_BYTES: usize = 24;
pub const SELECTABLE_FIXED_VALUE_COUNT: usize = 36;
pub const SELECTABLE_FIXED_BYTES: usize =
    SELECTABLE_RECORD_HEADER_BYTES + SELECTABLE_FIXED_VALUE_COUNT * size_of::<f64>();
pub const MAX_SELECTABLE_RECORD_BYTES: usize =
    SELECTABLE_FIXED_BYTES + MAX_EXTENSIONS * (1 + MAX_FIELD_BYTES + size_of::<f64>());
pub const LAYOUT_RECORD_HEADER_BYTES: usize = 8;
pub const SAMPLE_RECORD_HEADER_BYTES: usize = 32;
pub const SAMPLE_FIXED_VALUE_COUNT: usize = 28;
pub const SAMPLE_FIXED_BYTES: usize =
    SAMPLE_RECORD_HEADER_BYTES + SAMPLE_FIXED_VALUE_COUNT * size_of::<f64>();
pub const MAX_LAYOUT_PAYLOAD_BYTES: usize = 5 + MAX_FIELDS + MAX_AGGREGATE_FIELD_BYTES;
pub const MAX_REALTIME_PAYLOAD_BYTES: usize = 13 + MAX_FIELDS * 2 + 4 + 9;
pub const MAX_LAYOUT_RECORD_BYTES: usize =
    LAYOUT_RECORD_HEADER_BYTES + MAX_FIELDS + MAX_AGGREGATE_FIELD_BYTES;
pub const MAX_SAMPLE_RECORD_BYTES: usize =
    SAMPLE_FIXED_BYTES + MAX_EXTENSIONS * (1 + MAX_FIELD_BYTES + size_of::<f64>());

const MASK_RUNTIME_VALUES: u8 = 1 << 0;
const MASK_CHARGING_VALUES: u8 = 1 << 1;
const MASK_ACTIVE_ALERTS: u8 = 1 << 2;
const SUPPORTED_MASK: u8 = MASK_RUNTIME_VALUES | MASK_CHARGING_VALUES | MASK_ACTIVE_ALERTS;
const FATAL_ERROR_FLAG: u8 = 1 << 3;
const CHARGING_FLAG: u8 = 1 << 5;
const FIRMWARE_FAULT_ALERT: u32 = 1;
const SAMPLE_BEHAVIOR_FLAGS: u8 = 0x07;
const SAMPLE_OPTION_FATAL_PRESENT: u8 = 0x01;
const SAMPLE_OPTION_FATAL_VALUE: u8 = 0x02;
const SAMPLE_OPTION_ACTIVE_ALERTS: u8 = 0x04;
const SAMPLE_OPTION_FLAGS: u8 =
    SAMPLE_OPTION_FATAL_PRESENT | SAMPLE_OPTION_FATAL_VALUE | SAMPLE_OPTION_ACTIVE_ALERTS;

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

/// One strict command-33 response. Fields absent from the echoed masks remain `None`.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableSample {
    pub source_ticks: u32,
    pub package_state: Option<PackageState>,
    pub package_mode: Option<PackageMode>,
    pub footpad_state: Option<FootpadState>,
    pub stop_condition: Option<StopCondition>,
    pub setpoint_adjustment: Option<SetpointAdjustment>,
    pub wheelslip: Option<bool>,
    pub darkride: Option<bool>,
    pub charging: Option<bool>,
    pub alert_reason: Option<AlertReason>,
    pub fatal_error_latched: Option<bool>,
    pub values: DynamicValues,
    pub battery_level: Option<f64>,
    pub distance_m: Option<f64>,
    pub odometer_m: Option<f64>,
    pub charging_voltage_v: Option<f64>,
    pub charging_current_a: Option<f64>,
    pub amp_hours_drawn_ah: Option<f64>,
    pub amp_hours_charged_ah: Option<f64>,
    pub watt_hours_drawn_wh: Option<f64>,
    pub watt_hours_charged_wh: Option<f64>,
    pub field_weakening_current_a: Option<f64>,
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

#[derive(Clone, PartialEq, Eq)]
pub struct LayoutRecord(Vec<u8>);

impl LayoutRecord {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Debug for LayoutRecord {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LayoutRecord")
            .field("length", &self.0.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SampleRecord(Vec<u8>);

impl SampleRecord {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SelectableRecord(Vec<u8>);

impl SelectableRecord {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Debug for SelectableRecord {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SelectableRecord")
            .field("length", &self.0.len())
            .finish_non_exhaustive()
    }
}

impl Debug for SampleRecord {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SampleRecord")
            .field("length", &self.0.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutRecordOutcome {
    Record(LayoutRecord),
    UnsupportedEnvelope,
    TruncatedPayload,
    InvalidLayout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SampleRecordOutcome {
    Record(SampleRecord),
    InvalidLayoutRecord,
    UnsupportedEnvelope,
    TruncatedPayload,
    InvalidTelemetry,
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
    // The mask is the sole wire authority for optional value groups. Keep the
    // semantic state independent: existing hosts accept bounded snapshots taken
    // across state transitions where the two can momentarily differ.
    if includes_charging != charging || (!includes_alerts && extra_flags & FATAL_ERROR_FLAG != 0) {
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

/// Encode an exact float32 command-33 request for the supplied masks.
#[must_use]
pub fn selectable_request(mask1: u32, mask2: u32) -> [u8; 12] {
    let mut request = [0_u8; 12];
    request[..4].copy_from_slice(&[
        CUSTOM_APP_DATA_COMMAND,
        REFLOAT_PACKAGE_ID,
        SELECTABLE_REALTIME_DATA_COMMAND,
        SELECTABLE_FLOAT32_FLAG,
    ]);
    request[4..8].copy_from_slice(&mask1.to_be_bytes());
    request[8..12].copy_from_slice(&mask2.to_be_bytes());
    request
}

#[must_use]
pub fn selectable_fast_request() -> [u8; 12] {
    selectable_request(SELECTABLE_FAST_MASK1, 0)
}

#[must_use]
pub fn selectable_slow_request(include_gnss: bool) -> [u8; 12] {
    selectable_request(
        SELECTABLE_SLOW_MASK1,
        SELECTABLE_SLOW_MASK2
            | if include_gnss {
                SELECTABLE_GNSS_MASK2
            } else {
                0
            },
    )
}

/// Decode a command-33 response only when it exactly echoes the requested masks.
#[allow(clippy::field_reassign_with_default)] // Assignments must retain upstream wire order.
pub fn decode_selectable(
    expected_mask1: u32,
    expected_mask2: u32,
    payload: &[u8],
) -> Result<SelectableSample, DynamicTelemetryError> {
    validate_envelope(payload, SELECTABLE_REALTIME_DATA_COMMAND)?;
    if payload.len() < 16 {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }
    if payload[3] != SELECTABLE_FLOAT32_FLAG {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let mask1 = read_u32(payload, 4);
    let mask2 = read_u32(payload, 8);
    let supported1 = SELECTABLE_EXTRA_FLAGS | SELECTABLE_STATE_FLAGS | (0x01ff_ffff << 6);
    let supported2 = (1 << 15) - 1;
    if mask1 != expected_mask1
        || mask2 != expected_mask2
        || mask1 & !supported1 != 0
        || mask2 & !supported2 != 0
    {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }

    let numeric1 = mask1 & !(SELECTABLE_EXTRA_FLAGS | SELECTABLE_STATE_FLAGS);
    let regular2 = mask2
        & !(SELECTABLE_ODOMETER
            | SELECTABLE_GNSS_LATITUDE
            | SELECTABLE_GNSS_LONGITUDE
            | SELECTABLE_GNSS_LAST_UPDATE);
    let expected = 16usize
        + usize::from(mask1 & SELECTABLE_EXTRA_FLAGS != 0)
        + 4 * usize::try_from((mask1 & SELECTABLE_STATE_FLAGS != 0) as u32).unwrap()
        + 4 * usize::try_from(numeric1.count_ones()).unwrap()
        + 4 * usize::try_from(regular2.count_ones()).unwrap()
        + 4 * usize::from(mask2 & SELECTABLE_ODOMETER != 0)
        + 8 * usize::from(mask2 & SELECTABLE_GNSS_LATITUDE != 0)
        + 8 * usize::from(mask2 & SELECTABLE_GNSS_LONGITUDE != 0)
        + 4 * usize::from(mask2 & SELECTABLE_GNSS_LAST_UPDATE != 0);
    if payload.len() < expected {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }
    if payload.len() != expected {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }

    let mut index = 16;
    let mut extensions = Vec::new();
    if mask1 & SELECTABLE_EXTRA_FLAGS != 0 {
        extensions.push(ExtensionValue {
            name: "refloat.data_recording".to_owned(),
            value: f64::from(payload[index]),
        });
        index += 1;
    }
    let mut package_state_value = None;
    let mut package_mode_value = None;
    let mut footpad_state_value = None;
    let mut stop_condition_value = None;
    let mut setpoint_adjustment_value = None;
    let mut wheelslip = None;
    let mut darkride = None;
    let mut charging = None;
    let mut alert_reason_value = None;
    let mut fatal_error_latched = None;
    if mask1 & SELECTABLE_STATE_FLAGS != 0 {
        let flags = read_u32(payload, index);
        index += 4;
        if flags & 0x0c0c_0000 != 0 {
            return Err(DynamicTelemetryError::InvalidTelemetry);
        }
        package_mode_value = Some(package_mode(((flags >> 28) & 0x03) as u8));
        package_state_value = Some(package_state(((flags >> 24) & 0x03) as u8));
        footpad_state_value = Some(footpad_state(((flags >> 22) & 0x03) as u8));
        charging = Some(flags & (1 << 21) != 0);
        fatal_error_latched = Some(flags & (1 << 20) != 0);
        darkride = Some(flags & (1 << 17) != 0);
        wheelslip = Some(flags & (1 << 16) != 0);
        setpoint_adjustment_value = Some(setpoint_adjustment(((flags >> 12) & 0x0f) as u8));
        stop_condition_value = Some(stop_condition(((flags >> 8) & 0x0f) as u8));
        alert_reason_value = Some(alert_reason((flags & 0xff) as u8));
    }

    macro_rules! take_f32 {
        ($mask:expr, $bit:expr) => {{
            if $mask & $bit != 0 {
                let value = f32::from_bits(read_u32(payload, index)) as f64;
                index += 4;
                if !value.is_finite() {
                    return Err(DynamicTelemetryError::InvalidTelemetry);
                }
                Some(value)
            } else {
                None
            }
        }};
    }
    let mut values = DynamicValues::default();
    values.speed_kph = take_f32!(mask1, SELECTABLE_SPEED);
    values.erpm = take_f32!(mask1, SELECTABLE_ERPM);
    values.motor_current_a = take_f32!(mask1, SELECTABLE_CURRENT);
    values.directional_motor_current_a = take_f32!(mask1, SELECTABLE_DIR_CURRENT);
    values.filtered_motor_current_a = take_f32!(mask1, SELECTABLE_FILT_CURRENT);
    values.duty_cycle = take_f32!(mask1, SELECTABLE_DUTY_CYCLE);
    values.pack_voltage_v = take_f32!(mask1, SELECTABLE_BATTERY_VOLTAGE);
    values.battery_current_a = take_f32!(mask1, SELECTABLE_BATTERY_CURRENT);
    let battery_level = take_f32!(mask1, SELECTABLE_BATTERY_SOC);
    values.controller_temp_c = take_f32!(mask1, SELECTABLE_MOSFET_TEMP);
    values.motor_temp_c = take_f32!(mask1, SELECTABLE_MOTOR_TEMP);
    values.pitch_deg = take_f32!(mask1, SELECTABLE_PITCH);
    values.balance_pitch_deg = take_f32!(mask1, SELECTABLE_BALANCE_PITCH);
    values.roll_deg = take_f32!(mask1, SELECTABLE_ROLL);
    values.footpad_adc1_v = take_f32!(mask1, SELECTABLE_ADC_LEFT);
    values.footpad_adc2_v = take_f32!(mask1, SELECTABLE_ADC_RIGHT);
    values.remote_input = take_f32!(mask1, SELECTABLE_REMOTE_INPUT);
    values.setpoint_deg = take_f32!(mask1, SELECTABLE_SETPOINT);
    values.atr_setpoint_deg = take_f32!(mask1, SELECTABLE_ATR_SETPOINT);
    values.brake_tilt_setpoint_deg = take_f32!(mask1, SELECTABLE_BRAKE_TILT_SETPOINT);
    values.torque_tilt_setpoint_deg = take_f32!(mask1, SELECTABLE_TORQUE_TILT_SETPOINT);
    values.turn_tilt_setpoint_deg = take_f32!(mask1, SELECTABLE_TURN_TILT_SETPOINT);
    values.remote_setpoint_deg = take_f32!(mask1, SELECTABLE_REMOTE_SETPOINT);
    values.requested_balance_current_a = take_f32!(mask1, SELECTABLE_BALANCE_CURRENT);
    if let Some(value) = take_f32!(mask1, SELECTABLE_LOOP_FREQUENCY) {
        extensions.push(ExtensionValue {
            name: "refloat.loop_frequency_hz".to_owned(),
            value,
        });
    }

    let odometer_m = if mask2 & SELECTABLE_ODOMETER != 0 {
        let value = f64::from(read_u32(payload, index));
        index += 4;
        Some(value)
    } else {
        None
    };
    let distance_m = take_f32!(mask2, SELECTABLE_DISTANCE);
    let charging_voltage_v = take_f32!(mask2, SELECTABLE_CHARGING_VOLTAGE);
    let charging_current_a = take_f32!(mask2, SELECTABLE_CHARGING_CURRENT);
    let amp_hours_drawn_ah = take_f32!(mask2, SELECTABLE_AMP_HOURS);
    let amp_hours_charged_ah = take_f32!(mask2, SELECTABLE_AMP_HOURS_CHARGED);
    let watt_hours_drawn_wh = take_f32!(mask2, SELECTABLE_WATT_HOURS);
    let watt_hours_charged_wh = take_f32!(mask2, SELECTABLE_WATT_HOURS_CHARGED);
    let field_weakening_current_a = take_f32!(mask2, SELECTABLE_MOTOR_ID);
    for (bit, name) in [
        (SELECTABLE_GNSS_LATITUDE, "refloat.gnss.latitude"),
        (SELECTABLE_GNSS_LONGITUDE, "refloat.gnss.longitude"),
    ] {
        if mask2 & bit != 0 {
            let value = f64::from_bits(read_u64_checked(payload, index)?);
            index += 8;
            if !value.is_finite() {
                return Err(DynamicTelemetryError::InvalidTelemetry);
            }
            extensions.push(ExtensionValue {
                name: name.to_owned(),
                value,
            });
        }
    }
    for (bit, name) in [
        (SELECTABLE_GNSS_ALTITUDE, "refloat.gnss.altitude_m"),
        (SELECTABLE_GNSS_SPEED, "refloat.gnss.speed_kph"),
        (SELECTABLE_GNSS_ACCURACY, "refloat.gnss.accuracy"),
    ] {
        if let Some(value) = take_f32!(mask2, bit) {
            extensions.push(ExtensionValue {
                name: name.to_owned(),
                value,
            });
        }
    }
    if mask2 & SELECTABLE_GNSS_LAST_UPDATE != 0 {
        extensions.push(ExtensionValue {
            name: "refloat.gnss.last_update_ms".to_owned(),
            value: f64::from(read_u32(payload, index)),
        });
        index += 4;
    }
    debug_assert_eq!(index, payload.len());

    Ok(SelectableSample {
        source_ticks: read_u32(payload, 12),
        package_state: package_state_value,
        package_mode: package_mode_value,
        footpad_state: footpad_state_value,
        stop_condition: stop_condition_value,
        setpoint_adjustment: setpoint_adjustment_value,
        wheelslip,
        darkride,
        charging,
        alert_reason: alert_reason_value,
        fatal_error_latched,
        values,
        battery_level,
        distance_m,
        odometer_m,
        charging_voltage_v,
        charging_current_a,
        amp_hours_drawn_ah,
        amp_hours_charged_ah,
        watt_hours_drawn_wh,
        watt_hours_charged_wh,
        field_weakening_current_a,
        extensions,
    })
}

pub fn serialize_selectable_record(
    sample: &SelectableSample,
) -> Result<SelectableRecord, DynamicTelemetryError> {
    if sample.extensions.len() > MAX_EXTENSIONS {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let mut record = vec![0_u8; SELECTABLE_RECORD_HEADER_BYTES];
    record[0] = RECORD_VERSION;
    record[1] = SELECTABLE_RECORD_KIND;
    record[2] = sample.package_state.map_or(0, package_state_record_code);
    record[3] = sample.package_mode.map_or(0, package_mode_record_code);
    record[4] = sample.footpad_state.map_or(0, footpad_state_record_code);
    record[5] = sample.stop_condition.map_or(0, stop_condition_record_code);
    record[6] = sample
        .setpoint_adjustment
        .map_or(0, setpoint_adjustment_record_code);
    for (index, value) in [
        sample.wheelslip,
        sample.darkride,
        sample.charging,
        sample.fatal_error_latched,
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(value) = value {
            record[7] |= 1 << index;
            record[8] |= u8::from(value) << index;
        }
    }
    record[9] = sample.alert_reason.map_or(0, alert_reason_record_code);
    record[10] = sample.extensions.len() as u8;
    record[12..16].copy_from_slice(&sample.source_ticks.to_be_bytes());
    let fixed = selectable_fixed_values(sample);
    let mut presence = 0_u64;
    for (index, value) in fixed.into_iter().enumerate() {
        let bits = if let Some(value) = value {
            if !value.is_finite() {
                return Err(DynamicTelemetryError::InvalidTelemetry);
            }
            presence |= 1 << index;
            value.to_bits()
        } else {
            0
        };
        record.extend_from_slice(&bits.to_be_bytes());
    }
    record[16..24].copy_from_slice(&presence.to_be_bytes());
    let mut names = 0usize;
    for extension in &sample.extensions {
        if extension.name.is_empty()
            || extension.name.len() > MAX_FIELD_BYTES
            || !extension.value.is_finite()
            || sample
                .extensions
                .iter()
                .filter(|candidate| candidate.name == extension.name)
                .count()
                != 1
        {
            return Err(DynamicTelemetryError::InvalidTelemetry);
        }
        names = names
            .checked_add(extension.name.len())
            .filter(|value| *value <= MAX_AGGREGATE_FIELD_BYTES)
            .ok_or(DynamicTelemetryError::InvalidTelemetry)?;
        record.push(extension.name.len() as u8);
        record.extend_from_slice(extension.name.as_bytes());
        record.extend_from_slice(&extension.value.to_bits().to_be_bytes());
    }
    Ok(SelectableRecord(record))
}

pub fn parse_selectable_record(record: &[u8]) -> Result<SelectableSample, DynamicTelemetryError> {
    if record.len() < SELECTABLE_FIXED_BYTES {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }
    if record.len() > MAX_SELECTABLE_RECORD_BYTES
        || record[0] != RECORD_VERSION
        || record[1] != SELECTABLE_RECORD_KIND
        || record[7] & !0x0f != 0
        || record[8] & !record[7] != 0
        || record[11] != 0
    {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let package_state = (record[2] != 0)
        .then(|| record_package_state(record[2]))
        .transpose()?;
    let package_mode = (record[3] != 0)
        .then(|| record_package_mode(record[3]))
        .transpose()?;
    let footpad_state = (record[4] != 0)
        .then(|| record_footpad_state(record[4]))
        .transpose()?;
    let stop_condition = (record[5] != 0)
        .then(|| record_stop_condition(record[5]))
        .transpose()?;
    let setpoint_adjustment = (record[6] != 0)
        .then(|| record_setpoint_adjustment(record[6]))
        .transpose()?;
    let alert_reason = (record[9] != 0)
        .then(|| record_alert_reason(record[9]))
        .transpose()?;
    let optional_bool =
        |index: usize| (record[7] & (1 << index) != 0).then_some(record[8] & (1 << index) != 0);
    let presence = read_u64_checked(record, 16)?;
    if presence >> SELECTABLE_FIXED_VALUE_COUNT != 0 {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let mut fixed = [None; SELECTABLE_FIXED_VALUE_COUNT];
    let mut index = SELECTABLE_RECORD_HEADER_BYTES;
    for (value_index, slot) in fixed.iter_mut().enumerate() {
        let bits = read_u64_checked(record, index)?;
        index += 8;
        if presence & (1 << value_index) == 0 {
            if bits != 0 {
                return Err(DynamicTelemetryError::InvalidTelemetry);
            }
        } else {
            let value = f64::from_bits(bits);
            if !value.is_finite() {
                return Err(DynamicTelemetryError::InvalidTelemetry);
            }
            *slot = Some(value);
        }
    }
    let extension_count = usize::from(record[10]);
    if extension_count > MAX_EXTENSIONS {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let mut extensions = Vec::with_capacity(extension_count);
    let mut name_bytes = 0usize;
    for _ in 0..extension_count {
        let length = usize::from(
            *record
                .get(index)
                .ok_or(DynamicTelemetryError::TruncatedPayload)?,
        );
        index += 1;
        if length == 0 || length > MAX_FIELD_BYTES {
            return Err(DynamicTelemetryError::InvalidTelemetry);
        }
        let end = index
            .checked_add(length)
            .ok_or(DynamicTelemetryError::InvalidTelemetry)?;
        let name = std::str::from_utf8(
            record
                .get(index..end)
                .ok_or(DynamicTelemetryError::TruncatedPayload)?,
        )
        .map_err(|_| DynamicTelemetryError::InvalidTelemetry)?
        .to_owned();
        index = end;
        name_bytes = name_bytes
            .checked_add(length)
            .filter(|value| *value <= MAX_AGGREGATE_FIELD_BYTES)
            .ok_or(DynamicTelemetryError::InvalidTelemetry)?;
        if extensions
            .iter()
            .any(|value: &ExtensionValue| value.name == name)
        {
            return Err(DynamicTelemetryError::InvalidTelemetry);
        }
        let value = f64::from_bits(read_u64_checked(record, index)?);
        index += 8;
        if !value.is_finite() {
            return Err(DynamicTelemetryError::InvalidTelemetry);
        }
        extensions.push(ExtensionValue { name, value });
    }
    if index != record.len() {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    Ok(selectable_from_fixed(
        read_u32(record, 12),
        package_state,
        package_mode,
        footpad_state,
        stop_condition,
        setpoint_adjustment,
        optional_bool(0),
        optional_bool(1),
        optional_bool(2),
        alert_reason,
        optional_bool(3),
        fixed,
        extensions,
    ))
}

pub fn decode_layout_to_record(payload: &[u8]) -> LayoutRecordOutcome {
    match decode_layout(payload) {
        Ok(layout) => LayoutRecordOutcome::Record(serialize_layout(&layout)),
        Err(DynamicTelemetryError::UnsupportedEnvelope) => LayoutRecordOutcome::UnsupportedEnvelope,
        Err(DynamicTelemetryError::TruncatedPayload) => LayoutRecordOutcome::TruncatedPayload,
        Err(DynamicTelemetryError::InvalidLayout | DynamicTelemetryError::InvalidTelemetry) => {
            LayoutRecordOutcome::InvalidLayout
        }
    }
}

pub fn decode_realtime_to_record(layout_record: &[u8], payload: &[u8]) -> SampleRecordOutcome {
    let layout = match parse_layout_record(layout_record) {
        Ok(layout) => layout,
        Err(_) => return SampleRecordOutcome::InvalidLayoutRecord,
    };
    match decode_realtime(&layout, payload) {
        Ok(sample) => SampleRecordOutcome::Record(serialize_sample(&sample)),
        Err(DynamicTelemetryError::UnsupportedEnvelope) => SampleRecordOutcome::UnsupportedEnvelope,
        Err(DynamicTelemetryError::TruncatedPayload) => SampleRecordOutcome::TruncatedPayload,
        Err(DynamicTelemetryError::InvalidTelemetry | DynamicTelemetryError::InvalidLayout) => {
            SampleRecordOutcome::InvalidTelemetry
        }
    }
}

pub fn parse_layout_record(record: &[u8]) -> Result<DynamicLayout, DynamicTelemetryError> {
    if record.len() < LAYOUT_RECORD_HEADER_BYTES {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }
    if record.len() > MAX_LAYOUT_RECORD_BYTES
        || record[0] != RECORD_VERSION
        || record[1] != LAYOUT_RECORD_KIND
        || record[6] != 0
        || record[7] != 0
    {
        return Err(DynamicTelemetryError::InvalidLayout);
    }
    let always_count = usize::from(record[2]);
    let running_count = usize::from(record[3]);
    if always_count == 0
        || always_count
            .checked_add(running_count)
            .filter(|count| *count <= MAX_FIELDS)
            .is_none()
    {
        return Err(DynamicTelemetryError::InvalidLayout);
    }
    let expected_aggregate = usize::from(u16::from_be_bytes([record[4], record[5]]));
    if expected_aggregate > MAX_AGGREGATE_FIELD_BYTES {
        return Err(DynamicTelemetryError::InvalidLayout);
    }
    let mut index = LAYOUT_RECORD_HEADER_BYTES;
    let mut aggregate_name_bytes = 0;
    let always_present =
        decode_record_name_list(record, &mut index, always_count, &mut aggregate_name_bytes)?;
    let running_only =
        decode_record_name_list(record, &mut index, running_count, &mut aggregate_name_bytes)?;
    if index != record.len() || aggregate_name_bytes != expected_aggregate {
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

pub fn parse_sample_record(record: &[u8]) -> Result<DynamicSample, DynamicTelemetryError> {
    if record.len() < SAMPLE_FIXED_BYTES {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }
    if record.len() > MAX_SAMPLE_RECORD_BYTES
        || record[0] != RECORD_VERSION
        || record[1] != SAMPLE_RECORD_KIND
        || record[7] & !SAMPLE_BEHAVIOR_FLAGS != 0
        || record[11] & !SAMPLE_OPTION_FLAGS != 0
        || record[13..16] != [0, 0, 0]
    {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let package_state = record_package_state(record[2])?;
    let package_mode = record_package_mode(record[3])?;
    let footpad_state = record_footpad_state(record[4])?;
    let stop_condition = record_stop_condition(record[5])?;
    let setpoint_adjustment = record_setpoint_adjustment(record[6])?;
    let alert_reason = record_alert_reason(record[8])?;
    let extension_count = usize::from(record[12]);
    if extension_count > MAX_EXTENSIONS {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let option_flags = record[11];
    let has_alerts = option_flags & SAMPLE_OPTION_ACTIVE_ALERTS != 0;
    let has_fatal = option_flags & SAMPLE_OPTION_FATAL_PRESENT != 0;
    let fatal_value = option_flags & SAMPLE_OPTION_FATAL_VALUE != 0;
    if has_alerts != has_fatal || (!has_fatal && fatal_value) {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let words = [read_u32(record, 24), read_u32(record, 28)];
    let raw_fault = record[10];
    let fault_code = record[9];
    if !has_alerts && words != [0, 0]
        || (raw_fault == 0) != (fault_code == 0)
        || (raw_fault != 0) != (words[0] & FIRMWARE_FAULT_ALERT != 0)
        || raw_fault != 0 && fault_code != if raw_fault <= 27 { raw_fault } else { 28 }
    {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let firmware_fault = if fault_code == 0 {
        None
    } else {
        Some(record_firmware_fault(fault_code)?)
    };

    let presence = read_u32(record, 20);
    if presence >> SAMPLE_FIXED_VALUE_COUNT != 0 {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }
    let mut index = SAMPLE_RECORD_HEADER_BYTES;
    let mut fixed = [None; SAMPLE_FIXED_VALUE_COUNT];
    for (value_index, slot) in fixed.iter_mut().enumerate() {
        let bits = read_u64(record, index);
        index += size_of::<f64>();
        let present = presence & (1 << value_index) != 0;
        if !present {
            if bits != 0 {
                return Err(DynamicTelemetryError::InvalidTelemetry);
            }
            continue;
        }
        let value = f64::from_bits(bits);
        if !value.is_finite() {
            return Err(DynamicTelemetryError::InvalidTelemetry);
        }
        *slot = Some(value);
    }

    let mut extensions = Vec::with_capacity(extension_count);
    let mut extension_name_bytes = 0usize;
    for _ in 0..extension_count {
        let length = usize::from(
            *record
                .get(index)
                .ok_or(DynamicTelemetryError::TruncatedPayload)?,
        );
        index += 1;
        if length == 0 || length > MAX_FIELD_BYTES {
            return Err(DynamicTelemetryError::InvalidTelemetry);
        }
        let end = index
            .checked_add(length)
            .ok_or(DynamicTelemetryError::InvalidTelemetry)?;
        let name = std::str::from_utf8(
            record
                .get(index..end)
                .ok_or(DynamicTelemetryError::TruncatedPayload)?,
        )
        .map_err(|_| DynamicTelemetryError::InvalidTelemetry)?
        .to_owned();
        index = end;
        extension_name_bytes = extension_name_bytes
            .checked_add(length)
            .filter(|total| *total <= MAX_AGGREGATE_FIELD_BYTES)
            .ok_or(DynamicTelemetryError::InvalidTelemetry)?;
        if is_known_name(&name)
            || extensions
                .iter()
                .any(|item: &ExtensionValue| item.name == name)
        {
            return Err(DynamicTelemetryError::InvalidTelemetry);
        }
        let bits = read_u64_checked(record, index)?;
        index += size_of::<f64>();
        let value = f64::from_bits(bits);
        if !value.is_finite() {
            return Err(DynamicTelemetryError::InvalidTelemetry);
        }
        extensions.push(ExtensionValue { name, value });
    }
    if index != record.len() {
        return Err(DynamicTelemetryError::InvalidTelemetry);
    }

    Ok(DynamicSample {
        source_ticks: read_u32(record, 16),
        package_state,
        package_mode,
        footpad_state,
        stop_condition,
        setpoint_adjustment,
        wheelslip: record[7] & 0x01 != 0,
        darkride: record[7] & 0x02 != 0,
        charging: record[7] & 0x04 != 0,
        alert_reason,
        fatal_error_latched: has_fatal.then_some(fatal_value),
        active_alert_words: has_alerts.then_some(words),
        firmware_fault_raw_code: (raw_fault != 0).then_some(raw_fault),
        firmware_fault,
        values: DynamicValues {
            speed_kph: fixed[0],
            erpm: fixed[1],
            motor_current_a: fixed[2],
            directional_motor_current_a: fixed[3],
            filtered_motor_current_a: fixed[4],
            duty_cycle: fixed[5],
            pack_voltage_v: fixed[6],
            battery_current_a: fixed[7],
            controller_temp_c: fixed[8],
            motor_temp_c: fixed[9],
            pitch_deg: fixed[10],
            balance_pitch_deg: fixed[11],
            roll_deg: fixed[12],
            footpad_adc1_v: fixed[13],
            footpad_adc2_v: fixed[14],
            remote_input: fixed[15],
            setpoint_deg: fixed[16],
            atr_setpoint_deg: fixed[17],
            brake_tilt_setpoint_deg: fixed[18],
            torque_tilt_setpoint_deg: fixed[19],
            turn_tilt_setpoint_deg: fixed[20],
            remote_setpoint_deg: fixed[21],
            requested_balance_current_a: fixed[22],
            atr_acceleration_difference_erpm_per_second: fixed[23],
            atr_speed_boost: fixed[24],
            booster_current_a: fixed[25],
        },
        charging_current_a: fixed[26],
        charging_voltage_v: fixed[27],
        extensions,
    })
}

fn serialize_layout(layout: &DynamicLayout) -> LayoutRecord {
    let mut record = Vec::with_capacity(
        LAYOUT_RECORD_HEADER_BYTES + layout.field_count() + layout.aggregate_name_bytes,
    );
    record.extend_from_slice(&[
        RECORD_VERSION,
        LAYOUT_RECORD_KIND,
        layout.always_present.len() as u8,
        layout.running_only.len() as u8,
    ]);
    record.extend_from_slice(&(layout.aggregate_name_bytes as u16).to_be_bytes());
    record.extend_from_slice(&[0, 0]);
    for name in layout.always_present.iter().chain(&layout.running_only) {
        record.push(name.len() as u8);
        record.extend_from_slice(name.as_bytes());
    }
    debug_assert!(record.len() <= MAX_LAYOUT_RECORD_BYTES);
    LayoutRecord(record)
}

fn serialize_sample(sample: &DynamicSample) -> SampleRecord {
    let mut record = vec![0; SAMPLE_RECORD_HEADER_BYTES];
    record[0] = RECORD_VERSION;
    record[1] = SAMPLE_RECORD_KIND;
    record[2] = package_state_record_code(sample.package_state);
    record[3] = package_mode_record_code(sample.package_mode);
    record[4] = footpad_state_record_code(sample.footpad_state);
    record[5] = stop_condition_record_code(sample.stop_condition);
    record[6] = setpoint_adjustment_record_code(sample.setpoint_adjustment);
    record[7] = u8::from(sample.wheelslip)
        | u8::from(sample.darkride) << 1
        | u8::from(sample.charging) << 2;
    record[8] = alert_reason_record_code(sample.alert_reason);
    if let (Some(raw), Some(fault)) = (sample.firmware_fault_raw_code, sample.firmware_fault) {
        record[9] = firmware_fault_record_code(fault);
        record[10] = raw;
    }
    if let Some(fatal) = sample.fatal_error_latched {
        record[11] |= SAMPLE_OPTION_FATAL_PRESENT | u8::from(fatal) << 1;
    }
    if let Some(words) = sample.active_alert_words {
        record[11] |= SAMPLE_OPTION_ACTIVE_ALERTS;
        record[24..28].copy_from_slice(&words[0].to_be_bytes());
        record[28..32].copy_from_slice(&words[1].to_be_bytes());
    }
    record[12] = sample.extensions.len() as u8;
    record[16..20].copy_from_slice(&sample.source_ticks.to_be_bytes());
    let fixed = sample_fixed_values(sample);
    let mut presence = 0u32;
    for (index, value) in fixed.into_iter().enumerate() {
        let bits = if let Some(value) = value {
            presence |= 1 << index;
            value.to_bits()
        } else {
            0
        };
        record.extend_from_slice(&bits.to_be_bytes());
    }
    record[20..24].copy_from_slice(&presence.to_be_bytes());
    for extension in &sample.extensions {
        record.push(extension.name.len() as u8);
        record.extend_from_slice(extension.name.as_bytes());
        record.extend_from_slice(&extension.value.to_bits().to_be_bytes());
    }
    debug_assert!(record.len() <= MAX_SAMPLE_RECORD_BYTES);
    SampleRecord(record)
}

fn decode_record_name_list(
    record: &[u8],
    index: &mut usize,
    count: usize,
    aggregate_name_bytes: &mut usize,
) -> Result<Vec<String>, DynamicTelemetryError> {
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        let length = usize::from(
            *record
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
        let bytes = record
            .get(*index..end)
            .ok_or(DynamicTelemetryError::TruncatedPayload)?;
        *aggregate_name_bytes = aggregate_name_bytes
            .checked_add(length)
            .filter(|total| *total <= MAX_AGGREGATE_FIELD_BYTES)
            .ok_or(DynamicTelemetryError::InvalidLayout)?;
        names.push(
            std::str::from_utf8(bytes)
                .map_err(|_| DynamicTelemetryError::InvalidLayout)?
                .to_owned(),
        );
        *index = end;
    }
    Ok(names)
}

fn sample_fixed_values(sample: &DynamicSample) -> [Option<f64>; SAMPLE_FIXED_VALUE_COUNT] {
    [
        sample.values.speed_kph,
        sample.values.erpm,
        sample.values.motor_current_a,
        sample.values.directional_motor_current_a,
        sample.values.filtered_motor_current_a,
        sample.values.duty_cycle,
        sample.values.pack_voltage_v,
        sample.values.battery_current_a,
        sample.values.controller_temp_c,
        sample.values.motor_temp_c,
        sample.values.pitch_deg,
        sample.values.balance_pitch_deg,
        sample.values.roll_deg,
        sample.values.footpad_adc1_v,
        sample.values.footpad_adc2_v,
        sample.values.remote_input,
        sample.values.setpoint_deg,
        sample.values.atr_setpoint_deg,
        sample.values.brake_tilt_setpoint_deg,
        sample.values.torque_tilt_setpoint_deg,
        sample.values.turn_tilt_setpoint_deg,
        sample.values.remote_setpoint_deg,
        sample.values.requested_balance_current_a,
        sample.values.atr_acceleration_difference_erpm_per_second,
        sample.values.atr_speed_boost,
        sample.values.booster_current_a,
        sample.charging_current_a,
        sample.charging_voltage_v,
    ]
}

fn selectable_fixed_values(
    sample: &SelectableSample,
) -> [Option<f64>; SELECTABLE_FIXED_VALUE_COUNT] {
    let dynamic = &sample.values;
    [
        dynamic.speed_kph,
        dynamic.erpm,
        dynamic.motor_current_a,
        dynamic.directional_motor_current_a,
        dynamic.filtered_motor_current_a,
        dynamic.duty_cycle,
        dynamic.pack_voltage_v,
        dynamic.battery_current_a,
        dynamic.controller_temp_c,
        dynamic.motor_temp_c,
        dynamic.pitch_deg,
        dynamic.balance_pitch_deg,
        dynamic.roll_deg,
        dynamic.footpad_adc1_v,
        dynamic.footpad_adc2_v,
        dynamic.remote_input,
        dynamic.setpoint_deg,
        dynamic.atr_setpoint_deg,
        dynamic.brake_tilt_setpoint_deg,
        dynamic.torque_tilt_setpoint_deg,
        dynamic.turn_tilt_setpoint_deg,
        dynamic.remote_setpoint_deg,
        dynamic.requested_balance_current_a,
        dynamic.atr_acceleration_difference_erpm_per_second,
        dynamic.atr_speed_boost,
        dynamic.booster_current_a,
        sample.battery_level,
        sample.distance_m,
        sample.odometer_m,
        sample.charging_voltage_v,
        sample.charging_current_a,
        sample.amp_hours_drawn_ah,
        sample.amp_hours_charged_ah,
        sample.watt_hours_drawn_wh,
        sample.watt_hours_charged_wh,
        sample.field_weakening_current_a,
    ]
}

#[allow(clippy::too_many_arguments)]
fn selectable_from_fixed(
    source_ticks: u32,
    package_state: Option<PackageState>,
    package_mode: Option<PackageMode>,
    footpad_state: Option<FootpadState>,
    stop_condition: Option<StopCondition>,
    setpoint_adjustment: Option<SetpointAdjustment>,
    wheelslip: Option<bool>,
    darkride: Option<bool>,
    charging: Option<bool>,
    alert_reason: Option<AlertReason>,
    fatal_error_latched: Option<bool>,
    fixed: [Option<f64>; SELECTABLE_FIXED_VALUE_COUNT],
    extensions: Vec<ExtensionValue>,
) -> SelectableSample {
    SelectableSample {
        source_ticks,
        package_state,
        package_mode,
        footpad_state,
        stop_condition,
        setpoint_adjustment,
        wheelslip,
        darkride,
        charging,
        alert_reason,
        fatal_error_latched,
        values: DynamicValues {
            speed_kph: fixed[0],
            erpm: fixed[1],
            motor_current_a: fixed[2],
            directional_motor_current_a: fixed[3],
            filtered_motor_current_a: fixed[4],
            duty_cycle: fixed[5],
            pack_voltage_v: fixed[6],
            battery_current_a: fixed[7],
            controller_temp_c: fixed[8],
            motor_temp_c: fixed[9],
            pitch_deg: fixed[10],
            balance_pitch_deg: fixed[11],
            roll_deg: fixed[12],
            footpad_adc1_v: fixed[13],
            footpad_adc2_v: fixed[14],
            remote_input: fixed[15],
            setpoint_deg: fixed[16],
            atr_setpoint_deg: fixed[17],
            brake_tilt_setpoint_deg: fixed[18],
            torque_tilt_setpoint_deg: fixed[19],
            turn_tilt_setpoint_deg: fixed[20],
            remote_setpoint_deg: fixed[21],
            requested_balance_current_a: fixed[22],
            atr_acceleration_difference_erpm_per_second: fixed[23],
            atr_speed_boost: fixed[24],
            booster_current_a: fixed[25],
        },
        battery_level: fixed[26],
        distance_m: fixed[27],
        odometer_m: fixed[28],
        charging_voltage_v: fixed[29],
        charging_current_a: fixed[30],
        amp_hours_drawn_ah: fixed[31],
        amp_hours_charged_ah: fixed[32],
        watt_hours_drawn_wh: fixed[33],
        watt_hours_charged_wh: fixed[34],
        field_weakening_current_a: fixed[35],
        extensions,
    }
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

fn read_u64(payload: &[u8], index: usize) -> u64 {
    u64::from_be_bytes([
        payload[index],
        payload[index + 1],
        payload[index + 2],
        payload[index + 3],
        payload[index + 4],
        payload[index + 5],
        payload[index + 6],
        payload[index + 7],
    ])
}

fn read_u64_checked(payload: &[u8], index: usize) -> Result<u64, DynamicTelemetryError> {
    if index > payload.len().saturating_sub(size_of::<u64>()) {
        return Err(DynamicTelemetryError::TruncatedPayload);
    }
    Ok(read_u64(payload, index))
}

fn is_known_name(name: &str) -> bool {
    matches!(
        name,
        "motor.speed"
            | "motor.erpm"
            | "motor.current"
            | "motor.dir_current"
            | "motor.filt_current"
            | "motor.duty_cycle"
            | "motor.batt_voltage"
            | "motor.batt_current"
            | "motor.mosfet_temp"
            | "motor.motor_temp"
            | "imu.pitch"
            | "imu.balance_pitch"
            | "imu.roll"
            | "footpad.adc1"
            | "footpad.adc2"
            | "remote.input"
            | "setpoint"
            | "atr.setpoint"
            | "brake_tilt.setpoint"
            | "torque_tilt.setpoint"
            | "turn_tilt.setpoint"
            | "remote.setpoint"
            | "balance_current"
            | "atr.accel_diff"
            | "atr.speed_boost"
            | "booster.current"
    )
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

const fn package_state_record_code(value: PackageState) -> u8 {
    match value {
        PackageState::Disabled => 1,
        PackageState::Startup => 2,
        PackageState::Ready => 3,
        PackageState::Running => 4,
    }
}

const fn record_package_state(code: u8) -> Result<PackageState, DynamicTelemetryError> {
    match code {
        1 => Ok(PackageState::Disabled),
        2 => Ok(PackageState::Startup),
        3 => Ok(PackageState::Ready),
        4 => Ok(PackageState::Running),
        _ => Err(DynamicTelemetryError::InvalidTelemetry),
    }
}

const fn package_mode_record_code(value: PackageMode) -> u8 {
    match value {
        PackageMode::Normal => 1,
        PackageMode::HandTest => 2,
        PackageMode::Flywheel => 3,
        PackageMode::Unknown => 4,
    }
}

const fn record_package_mode(code: u8) -> Result<PackageMode, DynamicTelemetryError> {
    match code {
        1 => Ok(PackageMode::Normal),
        2 => Ok(PackageMode::HandTest),
        3 => Ok(PackageMode::Flywheel),
        4 => Ok(PackageMode::Unknown),
        _ => Err(DynamicTelemetryError::InvalidTelemetry),
    }
}

const fn footpad_state_record_code(value: FootpadState) -> u8 {
    match value {
        FootpadState::None => 1,
        FootpadState::Left => 2,
        FootpadState::Right => 3,
        FootpadState::Both => 4,
    }
}

const fn record_footpad_state(code: u8) -> Result<FootpadState, DynamicTelemetryError> {
    match code {
        1 => Ok(FootpadState::None),
        2 => Ok(FootpadState::Left),
        3 => Ok(FootpadState::Right),
        4 => Ok(FootpadState::Both),
        _ => Err(DynamicTelemetryError::InvalidTelemetry),
    }
}

const fn stop_condition_record_code(value: StopCondition) -> u8 {
    match value {
        StopCondition::None => 1,
        StopCondition::Pitch => 2,
        StopCondition::Roll => 3,
        StopCondition::SwitchHalf => 4,
        StopCondition::SwitchFull => 5,
        StopCondition::ReverseStop => 6,
        StopCondition::QuickStop => 7,
        StopCondition::Unknown => 8,
    }
}

const fn record_stop_condition(code: u8) -> Result<StopCondition, DynamicTelemetryError> {
    match code {
        1 => Ok(StopCondition::None),
        2 => Ok(StopCondition::Pitch),
        3 => Ok(StopCondition::Roll),
        4 => Ok(StopCondition::SwitchHalf),
        5 => Ok(StopCondition::SwitchFull),
        6 => Ok(StopCondition::ReverseStop),
        7 => Ok(StopCondition::QuickStop),
        8 => Ok(StopCondition::Unknown),
        _ => Err(DynamicTelemetryError::InvalidTelemetry),
    }
}

const fn setpoint_adjustment_record_code(value: SetpointAdjustment) -> u8 {
    match value {
        SetpointAdjustment::None => 1,
        SetpointAdjustment::Centering => 2,
        SetpointAdjustment::ReverseStop => 3,
        SetpointAdjustment::Speed => 4,
        SetpointAdjustment::Duty => 5,
        SetpointAdjustment::Error => 6,
        SetpointAdjustment::HighVoltage => 7,
        SetpointAdjustment::LowVoltage => 8,
        SetpointAdjustment::Temperature => 9,
        SetpointAdjustment::Unknown => 10,
    }
}

const fn record_setpoint_adjustment(code: u8) -> Result<SetpointAdjustment, DynamicTelemetryError> {
    match code {
        1 => Ok(SetpointAdjustment::None),
        2 => Ok(SetpointAdjustment::Centering),
        3 => Ok(SetpointAdjustment::ReverseStop),
        4 => Ok(SetpointAdjustment::Speed),
        5 => Ok(SetpointAdjustment::Duty),
        6 => Ok(SetpointAdjustment::Error),
        7 => Ok(SetpointAdjustment::HighVoltage),
        8 => Ok(SetpointAdjustment::LowVoltage),
        9 => Ok(SetpointAdjustment::Temperature),
        10 => Ok(SetpointAdjustment::Unknown),
        _ => Err(DynamicTelemetryError::InvalidTelemetry),
    }
}

const fn alert_reason_record_code(value: AlertReason) -> u8 {
    match value {
        AlertReason::None => 1,
        AlertReason::LowVoltage => 2,
        AlertReason::HighVoltage => 3,
        AlertReason::ControllerTemperature => 4,
        AlertReason::MotorTemperature => 5,
        AlertReason::Current => 6,
        AlertReason::Duty => 7,
        AlertReason::Sensors => 8,
        AlertReason::LowBattery => 9,
        AlertReason::Idle => 10,
        AlertReason::Error => 11,
        AlertReason::Speed => 12,
        AlertReason::CellUnderTemperature => 13,
        AlertReason::CellOverTemperature => 14,
        AlertReason::CellLowVoltage => 15,
        AlertReason::CellHighVoltage => 16,
        AlertReason::CellImbalance => 17,
        AlertReason::BmsConnection => 18,
        AlertReason::BmsOverTemperature => 19,
        AlertReason::FirmwareFault => 20,
        AlertReason::Unknown => 21,
    }
}

const fn record_alert_reason(code: u8) -> Result<AlertReason, DynamicTelemetryError> {
    match code {
        1 => Ok(AlertReason::None),
        2 => Ok(AlertReason::LowVoltage),
        3 => Ok(AlertReason::HighVoltage),
        4 => Ok(AlertReason::ControllerTemperature),
        5 => Ok(AlertReason::MotorTemperature),
        6 => Ok(AlertReason::Current),
        7 => Ok(AlertReason::Duty),
        8 => Ok(AlertReason::Sensors),
        9 => Ok(AlertReason::LowBattery),
        10 => Ok(AlertReason::Idle),
        11 => Ok(AlertReason::Error),
        12 => Ok(AlertReason::Speed),
        13 => Ok(AlertReason::CellUnderTemperature),
        14 => Ok(AlertReason::CellOverTemperature),
        15 => Ok(AlertReason::CellLowVoltage),
        16 => Ok(AlertReason::CellHighVoltage),
        17 => Ok(AlertReason::CellImbalance),
        18 => Ok(AlertReason::BmsConnection),
        19 => Ok(AlertReason::BmsOverTemperature),
        20 => Ok(AlertReason::FirmwareFault),
        21 => Ok(AlertReason::Unknown),
        _ => Err(DynamicTelemetryError::InvalidTelemetry),
    }
}

const fn firmware_fault_record_code(value: FirmwareFault) -> u8 {
    match value {
        FirmwareFault::OverVoltage => 1,
        FirmwareFault::UnderVoltage => 2,
        FirmwareFault::Driver => 3,
        FirmwareFault::AbsoluteOverCurrent => 4,
        FirmwareFault::ControllerOverTemperature => 5,
        FirmwareFault::MotorOverTemperature => 6,
        FirmwareFault::GateDriverOverVoltage => 7,
        FirmwareFault::GateDriverUnderVoltage => 8,
        FirmwareFault::McuUnderVoltage => 9,
        FirmwareFault::WatchdogReset => 10,
        FirmwareFault::EncoderSpi => 11,
        FirmwareFault::EncoderSinCosBelowMinimum => 12,
        FirmwareFault::EncoderSinCosAboveMaximum => 13,
        FirmwareFault::FlashCorruption => 14,
        FirmwareFault::CurrentSensorOffset1 => 15,
        FirmwareFault::CurrentSensorOffset2 => 16,
        FirmwareFault::CurrentSensorOffset3 => 17,
        FirmwareFault::UnbalancedCurrents => 18,
        FirmwareFault::Brake => 19,
        FirmwareFault::ResolverLossOfTracking => 20,
        FirmwareFault::ResolverDegradationOfSignal => 21,
        FirmwareFault::ResolverLossOfSignal => 22,
        FirmwareFault::AppConfigFlashCorruption => 23,
        FirmwareFault::MotorConfigFlashCorruption => 24,
        FirmwareFault::EncoderNoMagnet => 25,
        FirmwareFault::EncoderMagnetTooStrong => 26,
        FirmwareFault::PhaseFilter => 27,
        FirmwareFault::Unknown => 28,
    }
}

const fn record_firmware_fault(code: u8) -> Result<FirmwareFault, DynamicTelemetryError> {
    if code == 0 || code > 28 {
        Err(DynamicTelemetryError::InvalidTelemetry)
    } else if code == 28 {
        Ok(FirmwareFault::Unknown)
    } else {
        Ok(firmware_fault(code))
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
        let mut state_without_runtime_values = valid.clone();
        state_without_runtime_values[3] &= !MASK_RUNTIME_VALUES;
        state_without_runtime_values[9] = 3;
        state_without_runtime_values.drain(
            13 + layout.always_present.len() * 2
                ..13 + (layout.always_present.len() + layout.running_only.len()) * 2,
        );
        let transitional = decode_realtime(&layout, &state_without_runtime_values).unwrap();
        assert_eq!(transitional.package_state, PackageState::Running);
        assert_eq!(transitional.values.setpoint_deg, None);
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
    fn canonical_records_round_trip_every_fixture_and_float16_value() {
        let source_layout_payload = fixture_payload("1.1.0-and-1.2.3-command-32-layout");
        let layout_record = match decode_layout_to_record(&source_layout_payload) {
            LayoutRecordOutcome::Record(record) => record,
            outcome => panic!("unexpected layout outcome: {outcome:?}"),
        };
        assert_eq!(
            &layout_record.as_bytes()[..8],
            &[1, 1, 16, 10, 1, 119, 0, 0]
        );
        assert_eq!(
            parse_layout_record(layout_record.as_bytes()).unwrap(),
            committed_layout()
        );

        for fixture in [
            "1.1.0-command-31-running",
            "1.2.3-command-31-running-alert-tail",
            "1.2.3-command-31-charging-tail",
            "1.2.3-command-31-active-firmware-fault",
        ] {
            let payload = fixture_payload(fixture);
            let expected = decode_realtime(&committed_layout(), &payload).unwrap();
            let record = match decode_realtime_to_record(layout_record.as_bytes(), &payload) {
                SampleRecordOutcome::Record(record) => record,
                outcome => panic!("unexpected sample outcome: {outcome:?}"),
            };
            assert_eq!(record.as_bytes()[..2], [1, 2]);
            assert_eq!(parse_sample_record(record.as_bytes()).unwrap(), expected);
        }

        let single_layout = decode_layout_to_record(&layout_payload(&["motor.speed"], &[]));
        let LayoutRecordOutcome::Record(single_layout) = single_layout else {
            panic!("single layout failed")
        };
        for bits in 0..=u16::MAX {
            let payload = realtime_payload(0, 1, bits);
            let SampleRecordOutcome::Record(record) =
                decode_realtime_to_record(single_layout.as_bytes(), &payload)
            else {
                panic!("record failed for {bits:04x}")
            };
            assert_eq!(
                parse_sample_record(record.as_bytes())
                    .unwrap()
                    .values
                    .speed_kph,
                Some(decode_float16(bits)),
                "{bits:04x}"
            );
        }
    }

    #[test]
    fn canonical_record_parsers_reject_corruption_and_noncanonical_absence() {
        let payload = fixture_payload("1.1.0-and-1.2.3-command-32-layout");
        let LayoutRecordOutcome::Record(layout_record) = decode_layout_to_record(&payload) else {
            panic!("layout record failed")
        };
        for (index, value) in [(0, 2), (1, 2), (2, 0), (6, 1), (7, 1)] {
            let mut invalid = layout_record.as_bytes().to_vec();
            invalid[index] = value;
            assert!(
                parse_layout_record(&invalid).is_err(),
                "layout byte {index}"
            );
        }
        let mut wrong_aggregate = layout_record.as_bytes().to_vec();
        wrong_aggregate[5] -= 1;
        assert!(parse_layout_record(&wrong_aggregate).is_err());
        let mut trailing_layout = layout_record.as_bytes().to_vec();
        trailing_layout.push(0);
        assert!(parse_layout_record(&trailing_layout).is_err());

        let sample_payload = fixture_payload("1.2.3-command-31-running-alert-tail");
        let SampleRecordOutcome::Record(sample_record) =
            decode_realtime_to_record(layout_record.as_bytes(), &sample_payload)
        else {
            panic!("sample record failed")
        };
        for (index, value) in [
            (0, 2),
            (1, 1),
            (2, 0),
            (3, 0),
            (4, 0),
            (5, 0),
            (6, 0),
            (7, 0x80),
            (8, 0),
            (11, 0x80),
            (13, 1),
        ] {
            let mut invalid = sample_record.as_bytes().to_vec();
            invalid[index] = value;
            assert!(
                parse_sample_record(&invalid).is_err(),
                "sample byte {index}"
            );
        }
        let mut absent_nonzero = sample_record.as_bytes().to_vec();
        absent_nonzero[20..24].copy_from_slice(&0_u32.to_be_bytes());
        absent_nonzero[32..40].copy_from_slice(&1_f64.to_bits().to_be_bytes());
        assert!(parse_sample_record(&absent_nonzero).is_err());
        let mut nonfinite = sample_record.as_bytes().to_vec();
        nonfinite[32..40].copy_from_slice(&f64::INFINITY.to_bits().to_be_bytes());
        assert!(parse_sample_record(&nonfinite).is_err());
        let mut invalid_options = sample_record.as_bytes().to_vec();
        invalid_options[11] = SAMPLE_OPTION_FATAL_VALUE;
        assert!(parse_sample_record(&invalid_options).is_err());
        let mut trailing_sample = sample_record.as_bytes().to_vec();
        trailing_sample.push(0);
        assert!(parse_sample_record(&trailing_sample).is_err());
    }

    #[test]
    fn canonical_codes_are_closed_and_extension_records_are_exactly_bounded() {
        assert!((1..=4).all(|code| record_package_state(code).is_ok()));
        assert!((1..=4).all(|code| record_package_mode(code).is_ok()));
        assert!((1..=4).all(|code| record_footpad_state(code).is_ok()));
        assert!((1..=8).all(|code| record_stop_condition(code).is_ok()));
        assert!((1..=10).all(|code| record_setpoint_adjustment(code).is_ok()));
        assert!((1..=21).all(|code| record_alert_reason(code).is_ok()));
        assert!((1..=28).all(|code| record_firmware_fault(code).is_ok()));
        for invalid in [0, 29, u8::MAX] {
            assert!(record_firmware_fault(invalid).is_err());
        }

        let names: Vec<String> = (0..MAX_EXTENSIONS)
            .map(|index| format!("{index:02}{}", "x".repeat(62)))
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let layout_payload = layout_payload(&refs, &[]);
        let LayoutRecordOutcome::Record(layout_record) = decode_layout_to_record(&layout_payload)
        else {
            panic!("maximum extension layout failed")
        };
        let SampleRecordOutcome::Record(sample_record) = decode_realtime_to_record(
            layout_record.as_bytes(),
            &realtime_payload(0, MAX_EXTENSIONS, 0x3c00),
        ) else {
            panic!("maximum extension sample failed")
        };
        assert_eq!(sample_record.as_bytes().len(), MAX_SAMPLE_RECORD_BYTES);
        let sample = parse_sample_record(sample_record.as_bytes()).unwrap();
        assert_eq!(sample.extensions.len(), MAX_EXTENSIONS);
        assert_eq!(sample.extensions[0].name, names[0]);
        assert_eq!(sample.extensions[31].name, names[31]);
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
        let LayoutRecordOutcome::Record(first_record) = decode_layout_to_record(&layout_payload)
        else {
            panic!("layout record failed")
        };
        let LayoutRecordOutcome::Record(second_record) = decode_layout_to_record(&layout_payload)
        else {
            panic!("layout record failed")
        };
        assert_eq!(first_record, second_record);
        let debug = format!("{first:?}");
        assert!(!debug.contains("motor.speed"));
        assert!(!debug.contains("131008"));
        assert!(!format!("{first_record:?}").contains("motor.speed"));
        assert_eq!(
            DynamicTelemetryError::InvalidTelemetry.to_string(),
            "invalid Refloat dynamic telemetry"
        );
    }

    #[test]
    fn command_33_requests_and_float32_responses_are_exact() {
        assert_eq!(
            selectable_fast_request(),
            selectable_request(SELECTABLE_FAST_MASK1, 0)
        );
        assert_eq!(
            selectable_slow_request(false),
            selectable_request(SELECTABLE_SLOW_MASK1, SELECTABLE_SLOW_MASK2)
        );
        assert_eq!(
            selectable_slow_request(true),
            selectable_request(
                SELECTABLE_SLOW_MASK1,
                SELECTABLE_SLOW_MASK2 | SELECTABLE_GNSS_MASK2
            )
        );

        let mut fast = vec![36, 101, 33, 1];
        fast.extend_from_slice(&SELECTABLE_FAST_MASK1.to_be_bytes());
        fast.extend_from_slice(&0_u32.to_be_bytes());
        fast.extend_from_slice(&1234_u32.to_be_bytes());
        fast.push(1);
        // normal/running/both, fatal latch, darkride and wheelslip, no alert.
        fast.extend_from_slice(&0x03f3_0000_u32.to_be_bytes());
        for value in 1..=24 {
            fast.extend_from_slice(&(value as f32).to_bits().to_be_bytes());
        }
        let sample = decode_selectable(SELECTABLE_FAST_MASK1, 0, &fast).unwrap();
        assert_eq!(sample.source_ticks, 1234);
        assert_eq!(sample.package_state, Some(PackageState::Running));
        assert_eq!(sample.footpad_state, Some(FootpadState::Both));
        assert_eq!(sample.fatal_error_latched, Some(true));
        assert_eq!(sample.values.speed_kph, Some(1.0));
        assert_eq!(sample.values.requested_balance_current_a, Some(23.0));
        assert_eq!(sample.values.booster_current_a, None);
        assert_eq!(sample.extensions.len(), 2);
        assert_eq!(sample.extensions[1].name, "refloat.loop_frequency_hz");
        let record = serialize_selectable_record(&sample).unwrap();
        assert_eq!(
            parse_selectable_record(record.as_bytes()),
            Ok(sample.clone())
        );

        let mask2 = SELECTABLE_SLOW_MASK2 | SELECTABLE_GNSS_MASK2;
        let mut slow = vec![36, 101, 33, 1];
        slow.extend_from_slice(&SELECTABLE_SLOW_MASK1.to_be_bytes());
        slow.extend_from_slice(&mask2.to_be_bytes());
        slow.extend_from_slice(&4321_u32.to_be_bytes());
        slow.extend_from_slice(&0.75_f32.to_bits().to_be_bytes());
        slow.extend_from_slice(&42_u32.to_be_bytes());
        for value in 2..=9 {
            slow.extend_from_slice(&(value as f32).to_bits().to_be_bytes());
        }
        slow.extend_from_slice(&(-33.86_f64).to_bits().to_be_bytes());
        slow.extend_from_slice(&(151.21_f64).to_bits().to_be_bytes());
        for value in 10..=12 {
            slow.extend_from_slice(&(value as f32).to_bits().to_be_bytes());
        }
        slow.extend_from_slice(&999_u32.to_be_bytes());
        let sample = decode_selectable(SELECTABLE_SLOW_MASK1, mask2, &slow).unwrap();
        assert_eq!(sample.battery_level, Some(0.75));
        assert_eq!(sample.odometer_m, Some(42.0));
        assert_eq!(sample.field_weakening_current_a, Some(9.0));
        assert_eq!(sample.extensions.len(), 6);
        let record = serialize_selectable_record(&sample).unwrap();
        assert_eq!(parse_selectable_record(record.as_bytes()), Ok(sample));

        let mut wrong_mask = fast.clone();
        wrong_mask[7] ^= 1;
        assert_eq!(
            decode_selectable(SELECTABLE_FAST_MASK1, 0, &wrong_mask),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
        let mut wrong_flag = fast.clone();
        wrong_flag[3] = 0;
        assert_eq!(
            decode_selectable(SELECTABLE_FAST_MASK1, 0, &wrong_flag),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
        let mut nonfinite = fast.clone();
        nonfinite[21..25].copy_from_slice(&f32::INFINITY.to_bits().to_be_bytes());
        assert_eq!(
            decode_selectable(SELECTABLE_FAST_MASK1, 0, &nonfinite),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
        let reserved = SELECTABLE_FAST_MASK1 | (1 << 2);
        let mut reserved_payload = fast.clone();
        reserved_payload[4..8].copy_from_slice(&reserved.to_be_bytes());
        assert_eq!(
            decode_selectable(reserved, 0, &reserved_payload),
            Err(DynamicTelemetryError::InvalidTelemetry)
        );
        assert_eq!(
            decode_selectable(SELECTABLE_FAST_MASK1, 0, &fast[..fast.len() - 1]),
            Err(DynamicTelemetryError::TruncatedPayload)
        );
        fast.push(0);
        assert_eq!(
            decode_selectable(SELECTABLE_FAST_MASK1, 0, &fast),
            Err(DynamicTelemetryError::InvalidTelemetry)
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
