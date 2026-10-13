//! Pure Refloat telemetry application-service decisions for a future Rust runtime.
//!
//! This safe candidate composes decoded telemetry and scheduler facts. It owns
//! no clock, timer, BLE writer, callback, bridge, persistence, transport, or OS
//! behavior and remains unselected by every production host.

#![forbid(unsafe_code)]

use std::fmt::{self, Debug, Formatter};

use float_view_refloat_dynamic_telemetry as dynamic;
use float_view_refloat_legacy_telemetry as legacy;
use float_view_vesc_request_scheduler::{
    ExpectedResponse, Generation, Priority, RequestContext, RequestFacts, Route,
};

pub const RIDING_INTERVAL_MS: u64 = 100;
pub const COMPACT_INTERVAL_MS: u64 = 1_000;
pub const FAULT_PROBE_INTERVAL_MS: u64 = 1_000;
pub const REQUEST_TIMEOUT_MS: u64 = 900;
pub const LAYOUT_RETRY_INTERVAL_MS: u64 = 1_000;
pub const MAX_LAYOUT_ATTEMPTS: u8 = 5;
pub const MIN_IDLE_INTERVAL_MS: u64 = 100;
pub const MAX_IDLE_INTERVAL_MS: u64 = 10_000;
pub const MAX_POST_RIDE_TAIL_MS: u64 = 300_000;
pub const MAX_EXTENSIONS: usize = dynamic::MAX_EXTENSIONS;

const CUSTOM_APP_DATA_COMMAND: u8 = 36;
const REFLOAT_PACKAGE_ID: u8 = 101;
const GET_ALL_DATA_COMMAND: u8 = 10;
const REALTIME_DATA_COMMAND: u8 = 31;
const REALTIME_DATA_IDS_COMMAND: u8 = 32;
const FORWARD_CAN_COMMAND: u8 = 34;
const DETAIL_LEVEL_FOUR: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryFamily {
    LegacyFixed,
    Dynamic,
    Selectable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Settings {
    pub idle_interval_ms: u64,
    pub post_ride_tail_ms: u64,
}

impl Settings {
    pub fn new(idle_interval_ms: u64, post_ride_tail_ms: u64) -> Result<Self, ServiceError> {
        let value = Self {
            idle_interval_ms,
            post_ride_tail_ms,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(self) -> Result<(), ServiceError> {
        if !(MIN_IDLE_INTERVAL_MS..=MAX_IDLE_INTERVAL_MS).contains(&self.idle_interval_ms) {
            return Err(ServiceError::InvalidIdleInterval);
        }
        if self.post_ride_tail_ms > MAX_POST_RIDE_TAIL_MS {
            return Err(ServiceError::InvalidPostRideTail);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Configuration {
    pub service_generation: u64,
    pub scheduler_generation: Generation,
    pub route: Route,
    pub family: TelemetryFamily,
    pub selectable_gnss: bool,
    pub settings: Settings,
}

impl Configuration {
    pub fn validate(self) -> Result<(), ServiceError> {
        self.settings.validate()?;
        validate_route(self.route)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerKind {
    LayoutRetry,
    Poll,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerToken {
    pub kind: TimerKind,
    pub service_generation: u64,
    pub nonce: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestKind {
    Layout,
    Legacy,
    Realtime,
    Compact,
    SelectableFast,
    SelectableSlow,
    SelectableFault,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestOutcome {
    Response,
    Sent,
    TimedOut,
    QueueFull,
    Cancelled,
    StaleGeneration,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DecodedTelemetry {
    Legacy(legacy::LegacyTelemetry),
    Dynamic(dynamic::DynamicSample),
    Selectable(dynamic::SelectableSample),
    SelectableFault(Option<legacy::ActiveFirmwareFault>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RideState {
    Idle,
    Riding,
    Unknown,
}

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
pub struct NormalizedSample {
    pub sequence: u64,
    pub observed_at_epoch_ms: i64,
    pub slow_observed_at_epoch_ms: Option<i64>,
    pub elapsed_ms: u64,
    pub riding_elapsed_ms: u64,
    pub ride_state: RideState,
    pub package_state: Option<PackageState>,
    pub package_mode: Option<PackageMode>,
    pub footpad_state: Option<FootpadState>,
    pub stop_condition: Option<StopCondition>,
    pub setpoint_adjustment: Option<SetpointAdjustment>,
    pub wheelslip: Option<bool>,
    pub darkride: Option<bool>,
    pub charging: Option<bool>,
    pub alert_reason: Option<AlertReason>,
    pub firmware_fault: Option<FirmwareFault>,
    pub fatal_error_latched: Option<bool>,
    pub pack_voltage_v: Option<f64>,
    pub duty_cycle: Option<f64>,
    pub controller_temp_c: Option<f64>,
    pub motor_temp_c: Option<f64>,
    pub motor_current_a: Option<f64>,
    pub battery_current_a: Option<f64>,
    pub erpm: Option<f64>,
    pub speed_kph: Option<f64>,
    pub pitch_deg: Option<f64>,
    pub balance_pitch_deg: Option<f64>,
    pub roll_deg: Option<f64>,
    pub directional_motor_current_a: Option<f64>,
    pub filtered_motor_current_a: Option<f64>,
    pub requested_balance_current_a: Option<f64>,
    pub booster_current_a: Option<f64>,
    pub field_weakening_current_a: Option<f64>,
    pub footpad_adc1_v: Option<f64>,
    pub footpad_adc2_v: Option<f64>,
    pub remote_input: Option<f64>,
    pub setpoint_deg: Option<f64>,
    pub atr_setpoint_deg: Option<f64>,
    pub brake_tilt_setpoint_deg: Option<f64>,
    pub torque_tilt_setpoint_deg: Option<f64>,
    pub turn_tilt_setpoint_deg: Option<f64>,
    pub remote_setpoint_deg: Option<f64>,
    pub atr_acceleration_difference_erpm_per_second: Option<f64>,
    pub atr_speed_boost: Option<f64>,
    pub distance_m: Option<f64>,
    pub odometer_m: Option<f64>,
    pub amp_hours_drawn_ah: Option<f64>,
    pub amp_hours_charged_ah: Option<f64>,
    pub watt_hours_drawn_wh: Option<f64>,
    pub watt_hours_charged_wh: Option<f64>,
    pub battery_level: Option<f64>,
    pub charging_current_a: Option<f64>,
    pub charging_voltage_v: Option<f64>,
    pub extensions: Vec<ExtensionValue>,
}

impl Debug for NormalizedSample {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NormalizedSample")
            .field("sequence", &self.sequence)
            .field("ride_state", &self.ride_state)
            .field("extension_count", &self.extensions.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    LayoutTimeout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Diagnostic {
    LayoutRequestTimeout,
    TelemetryRequestTimeout,
    MalformedTelemetry,
    UnexpectedTelemetryResponse,
    StaleEvent,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    ReplaceTimer {
        token: TimerToken,
        interval_ms: u64,
    },
    CancelTimer(TimerToken),
    Enqueue {
        kind: RequestKind,
        facts: RequestFacts,
    },
    Publish(Box<NormalizedSample>),
    Failure(Failure),
    Diagnostic(Diagnostic),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    BeginLayout {
        service_generation: u64,
        scheduler_generation: Generation,
        route: Route,
    },
    LayoutEstablished {
        service_generation: u64,
    },
    Start {
        configuration: Configuration,
        now_ms: u64,
    },
    UpdateSettings {
        service_generation: u64,
        settings: Settings,
        now_ms: u64,
    },
    MarkSessionStarted {
        service_generation: u64,
        now_ms: u64,
    },
    Refresh {
        service_generation: u64,
        now_ms: u64,
    },
    TimerFired {
        token: TimerToken,
        now_ms: u64,
    },
    RequestCompleted {
        service_generation: u64,
        kind: RequestKind,
        outcome: RequestOutcome,
    },
    DecodeFailed {
        service_generation: u64,
        kind: RequestKind,
    },
    Receive {
        service_generation: u64,
        telemetry: Box<DecodedTelemetry>,
        now_ms: u64,
        observed_at_epoch_ms: i64,
    },
    Stop {
        service_generation: u64,
    },
    /// Clear connection work while retaining the active ride's clock and sequence.
    ResetConnection,
    Reset,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub service_generation: Option<u64>,
    pub scheduler_generation: Option<Generation>,
    pub route: Option<Route>,
    pub family: TelemetryFamily,
    pub active: bool,
    pub layout_attempts: u8,
    pub pending_request: Option<RequestKind>,
    pub polling_interval_ms: Option<u64>,
    pub next_compact_due_ms: Option<u64>,
    pub next_fault_due_ms: Option<u64>,
    pub has_compact_cache: bool,
    pub sequence: u64,
    pub previous_ride_state: RideState,
    pub post_ride_probe_until_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceError {
    InvalidIdleInterval,
    InvalidPostRideTail,
    InvalidRoute,
    MonotonicOverflow,
    InvalidTelemetry,
}

#[derive(Clone, PartialEq)]
struct CommonRecord {
    ride_state: RideState,
    package_state: Option<PackageState>,
    package_mode: Option<PackageMode>,
    footpad_state: Option<FootpadState>,
    stop_condition: Option<StopCondition>,
    setpoint_adjustment: Option<SetpointAdjustment>,
    wheelslip: Option<bool>,
    darkride: Option<bool>,
    charging: Option<bool>,
    alert_reason: Option<AlertReason>,
    firmware_fault: Option<FirmwareFault>,
    fatal_error_latched: Option<bool>,
    pack_voltage_v: Option<f64>,
    duty_cycle: Option<f64>,
    controller_temp_c: Option<f64>,
    motor_temp_c: Option<f64>,
    motor_current_a: Option<f64>,
    battery_current_a: Option<f64>,
    erpm: Option<f64>,
    speed_kph: Option<f64>,
    pitch_deg: Option<f64>,
    balance_pitch_deg: Option<f64>,
    roll_deg: Option<f64>,
    directional_motor_current_a: Option<f64>,
    filtered_motor_current_a: Option<f64>,
    requested_balance_current_a: Option<f64>,
    booster_current_a: Option<f64>,
    field_weakening_current_a: Option<f64>,
    footpad_adc1_v: Option<f64>,
    footpad_adc2_v: Option<f64>,
    remote_input: Option<f64>,
    setpoint_deg: Option<f64>,
    atr_setpoint_deg: Option<f64>,
    brake_tilt_setpoint_deg: Option<f64>,
    torque_tilt_setpoint_deg: Option<f64>,
    turn_tilt_setpoint_deg: Option<f64>,
    remote_setpoint_deg: Option<f64>,
    atr_acceleration_difference_erpm_per_second: Option<f64>,
    atr_speed_boost: Option<f64>,
    distance_m: Option<f64>,
    odometer_m: Option<f64>,
    amp_hours_drawn_ah: Option<f64>,
    amp_hours_charged_ah: Option<f64>,
    watt_hours_drawn_wh: Option<f64>,
    watt_hours_charged_wh: Option<f64>,
    battery_level: Option<f64>,
    charging_current_a: Option<f64>,
    charging_voltage_v: Option<f64>,
    extensions: Vec<ExtensionValue>,
}

#[derive(Clone, PartialEq)]
struct SlowCache {
    observed_at_epoch_ms: i64,
    field_weakening_current_a: Option<f64>,
    distance_m: Option<f64>,
    odometer_m: Option<f64>,
    amp_hours_drawn_ah: Option<f64>,
    amp_hours_charged_ah: Option<f64>,
    watt_hours_drawn_wh: Option<f64>,
    watt_hours_charged_wh: Option<f64>,
    battery_level: Option<f64>,
    charging_current_a: Option<f64>,
    charging_voltage_v: Option<f64>,
    extensions: Vec<ExtensionValue>,
}

impl SlowCache {
    fn from_record(value: &CommonRecord, observed_at_epoch_ms: i64) -> Self {
        Self {
            observed_at_epoch_ms,
            field_weakening_current_a: value.field_weakening_current_a,
            distance_m: value.distance_m,
            odometer_m: value.odometer_m,
            amp_hours_drawn_ah: value.amp_hours_drawn_ah,
            amp_hours_charged_ah: value.amp_hours_charged_ah,
            watt_hours_drawn_wh: value.watt_hours_drawn_wh,
            watt_hours_charged_wh: value.watt_hours_charged_wh,
            battery_level: value.battery_level,
            charging_current_a: value.charging_current_a,
            charging_voltage_v: value.charging_voltage_v,
            extensions: value.extensions.clone(),
        }
    }
}

pub struct RefloatTelemetryService {
    service_generation: Option<u64>,
    scheduler_generation: Option<Generation>,
    route: Option<Route>,
    family: TelemetryFamily,
    settings: Settings,
    layout_timer: Option<TimerToken>,
    layout_attempts: u8,
    poll_timer: Option<TimerToken>,
    polling_interval_ms: Option<u64>,
    started_at_ms: Option<u64>,
    last_sample_at_ms: Option<u64>,
    riding_elapsed_ms: u64,
    sequence: u64,
    previous_ride_state: RideState,
    post_ride_probe_until_ms: Option<u64>,
    pending_request: Option<RequestKind>,
    next_compact_due_ms: Option<u64>,
    next_fault_due_ms: Option<u64>,
    latest_compact: Option<SlowCache>,
    selectable_gnss: bool,
    latest_firmware_fault: Option<FirmwareFault>,
    active: bool,
    next_timer_nonce: u64,
}

impl Debug for RefloatTelemetryService {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RefloatTelemetryService")
            .field("snapshot", &self.snapshot())
            .finish_non_exhaustive()
    }
}

impl Default for RefloatTelemetryService {
    fn default() -> Self {
        Self {
            service_generation: None,
            scheduler_generation: None,
            route: None,
            family: TelemetryFamily::Dynamic,
            settings: Settings {
                idle_interval_ms: 1_000,
                post_ride_tail_ms: 5_000,
            },
            layout_timer: None,
            layout_attempts: 0,
            poll_timer: None,
            polling_interval_ms: None,
            started_at_ms: None,
            last_sample_at_ms: None,
            riding_elapsed_ms: 0,
            sequence: 0,
            previous_ride_state: RideState::Unknown,
            post_ride_probe_until_ms: None,
            pending_request: None,
            next_compact_due_ms: None,
            next_fault_due_ms: None,
            latest_compact: None,
            selectable_gnss: false,
            latest_firmware_fault: None,
            active: false,
            next_timer_nonce: 0,
        }
    }
}

impl RefloatTelemetryService {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            service_generation: self.service_generation,
            scheduler_generation: self.scheduler_generation,
            route: self.route,
            family: self.family,
            active: self.active,
            layout_attempts: self.layout_attempts,
            pending_request: self.pending_request,
            polling_interval_ms: self.polling_interval_ms,
            next_compact_due_ms: self.next_compact_due_ms,
            next_fault_due_ms: self.next_fault_due_ms,
            has_compact_cache: self.latest_compact.is_some(),
            sequence: self.sequence,
            previous_ride_state: self.previous_ride_state,
            post_ride_probe_until_ms: self.post_ride_probe_until_ms,
        }
    }

    pub fn apply(&mut self, command: Command) -> Result<Vec<Effect>, ServiceError> {
        match command {
            Command::BeginLayout {
                service_generation,
                scheduler_generation,
                route,
            } => self.begin_layout(service_generation, scheduler_generation, route),
            Command::LayoutEstablished { service_generation } => {
                Ok(self.layout_established(service_generation))
            }
            Command::Start {
                configuration,
                now_ms,
            } => self.start(configuration, now_ms),
            Command::UpdateSettings {
                service_generation,
                settings,
                now_ms,
            } => self.update_settings(service_generation, settings, now_ms),
            Command::MarkSessionStarted {
                service_generation,
                now_ms,
            } => Ok(self.mark_session_started(service_generation, now_ms)),
            Command::Refresh {
                service_generation,
                now_ms,
            } => self.refresh(service_generation, now_ms),
            Command::TimerFired { token, now_ms } => self.timer_fired(token, now_ms),
            Command::RequestCompleted {
                service_generation,
                kind,
                outcome,
            } => Ok(self.request_completed(service_generation, kind, outcome)),
            Command::DecodeFailed {
                service_generation,
                kind,
            } => Ok(self.decode_failed(service_generation, kind)),
            Command::Receive {
                service_generation,
                telemetry,
                now_ms,
                observed_at_epoch_ms,
            } => self.receive(service_generation, *telemetry, now_ms, observed_at_epoch_ms),
            Command::Stop { service_generation } => Ok(self.stop(service_generation)),
            Command::ResetConnection => Ok(self.reset_connection()),
            Command::Reset => Ok(self.reset()),
        }
    }

    fn begin_layout(
        &mut self,
        service_generation: u64,
        scheduler_generation: Generation,
        route: Route,
    ) -> Result<Vec<Effect>, ServiceError> {
        validate_route(route)?;
        if self.layout_timer.is_some() {
            return Ok(Vec::new());
        }
        self.service_generation = Some(service_generation);
        self.scheduler_generation = Some(scheduler_generation);
        self.route = Some(route);
        self.layout_attempts = 0;
        let request = self.layout_request()?;
        let token = self.new_timer(TimerKind::LayoutRetry, service_generation);
        self.layout_timer = Some(token);
        Ok(vec![
            request,
            Effect::ReplaceTimer {
                token,
                interval_ms: LAYOUT_RETRY_INTERVAL_MS,
            },
        ])
    }

    fn layout_established(&mut self, service_generation: u64) -> Vec<Effect> {
        if self.service_generation != Some(service_generation) {
            return vec![Effect::Diagnostic(Diagnostic::StaleEvent)];
        }
        self.cancel_layout()
    }

    fn start(
        &mut self,
        configuration: Configuration,
        now_ms: u64,
    ) -> Result<Vec<Effect>, ServiceError> {
        configuration.validate()?;
        let next_compact_due = match configuration.family {
            TelemetryFamily::Dynamic | TelemetryFamily::Selectable => Some(
                now_ms
                    .checked_add(COMPACT_INTERVAL_MS)
                    .ok_or(ServiceError::MonotonicOverflow)?,
            ),
            TelemetryFamily::LegacyFixed => None,
        };
        let next_fault_due = if configuration.family == TelemetryFamily::Selectable {
            Some(
                now_ms
                    .checked_add(FAULT_PROBE_INTERVAL_MS)
                    .ok_or(ServiceError::MonotonicOverflow)?,
            )
        } else {
            None
        };
        self.service_generation = Some(configuration.service_generation);
        self.scheduler_generation = Some(configuration.scheduler_generation);
        self.route = Some(configuration.route);
        self.family = configuration.family;
        self.settings = configuration.settings;
        self.pending_request = None;
        self.latest_compact = None;
        self.next_compact_due_ms = next_compact_due;
        self.next_fault_due_ms = next_fault_due;
        self.selectable_gnss = configuration.selectable_gnss;
        self.latest_firmware_fault = None;
        self.active = true;
        let interval = if self.previous_ride_state == RideState::Riding {
            RIDING_INTERVAL_MS
        } else {
            configuration.settings.idle_interval_ms
        };
        self.restart_polling(interval, now_ms)
    }

    fn update_settings(
        &mut self,
        service_generation: u64,
        settings: Settings,
        now_ms: u64,
    ) -> Result<Vec<Effect>, ServiceError> {
        settings.validate()?;
        if self.service_generation != Some(service_generation) {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        self.preflight_immediate_request(now_ms, false)?;
        self.settings = settings;
        if !self.active {
            return Ok(Vec::new());
        }
        let interval = if self.previous_ride_state == RideState::Riding {
            RIDING_INTERVAL_MS
        } else {
            settings.idle_interval_ms
        };
        self.restart_polling(interval, now_ms)
    }

    fn mark_session_started(&mut self, service_generation: u64, now_ms: u64) -> Vec<Effect> {
        if self.service_generation != Some(service_generation) {
            return vec![Effect::Diagnostic(Diagnostic::StaleEvent)];
        }
        self.started_at_ms = Some(now_ms);
        Vec::new()
    }

    fn refresh(
        &mut self,
        service_generation: u64,
        now_ms: u64,
    ) -> Result<Vec<Effect>, ServiceError> {
        if self.service_generation != Some(service_generation) {
            return Ok(vec![Effect::Diagnostic(Diagnostic::StaleEvent)]);
        }
        self.request_sample(now_ms)
    }

    fn timer_fired(&mut self, token: TimerToken, now_ms: u64) -> Result<Vec<Effect>, ServiceError> {
        if self.service_generation != Some(token.service_generation) {
            return Ok(Vec::new());
        }
        match token.kind {
            TimerKind::LayoutRetry if self.layout_timer == Some(token) => {
                if self.layout_attempts >= MAX_LAYOUT_ATTEMPTS {
                    self.layout_timer = None;
                    self.layout_attempts = 0;
                    Ok(vec![
                        Effect::CancelTimer(token),
                        Effect::Failure(Failure::LayoutTimeout),
                    ])
                } else {
                    Ok(vec![self.layout_request()?])
                }
            }
            TimerKind::Poll if self.poll_timer == Some(token) => self.request_sample(now_ms),
            _ => Ok(Vec::new()),
        }
    }

    fn request_completed(
        &mut self,
        service_generation: u64,
        kind: RequestKind,
        outcome: RequestOutcome,
    ) -> Vec<Effect> {
        if self.service_generation != Some(service_generation) {
            return Vec::new();
        }
        if outcome == RequestOutcome::TimedOut {
            if kind == RequestKind::Layout {
                return vec![Effect::Diagnostic(Diagnostic::LayoutRequestTimeout)];
            }
            if self.pending_request == Some(kind) {
                self.pending_request = None;
                return vec![Effect::Diagnostic(Diagnostic::TelemetryRequestTimeout)];
            }
        }
        Vec::new()
    }

    fn decode_failed(&mut self, service_generation: u64, kind: RequestKind) -> Vec<Effect> {
        if self.service_generation != Some(service_generation) {
            return Vec::new();
        }
        if kind == RequestKind::Layout {
            return vec![Effect::Diagnostic(Diagnostic::MalformedTelemetry)];
        }
        if self.pending_request != Some(kind) {
            return vec![Effect::Diagnostic(Diagnostic::UnexpectedTelemetryResponse)];
        }
        self.pending_request = None;
        vec![Effect::Diagnostic(Diagnostic::MalformedTelemetry)]
    }

    fn receive(
        &mut self,
        service_generation: u64,
        telemetry: DecodedTelemetry,
        now_ms: u64,
        observed_at_epoch_ms: i64,
    ) -> Result<Vec<Effect>, ServiceError> {
        if self.service_generation != Some(service_generation) || !self.active {
            return Ok(Vec::new());
        }
        let pending = self.pending_request;
        let matched = matches!(
            (&telemetry, self.family, pending),
            (
                DecodedTelemetry::Legacy(_),
                TelemetryFamily::LegacyFixed,
                Some(RequestKind::Legacy)
            ) | (
                DecodedTelemetry::Legacy(_),
                TelemetryFamily::Dynamic,
                Some(RequestKind::Compact)
            ) | (
                DecodedTelemetry::Dynamic(_),
                TelemetryFamily::Dynamic,
                Some(RequestKind::Realtime)
            ) | (
                DecodedTelemetry::Selectable(_),
                TelemetryFamily::Selectable,
                Some(RequestKind::SelectableFast | RequestKind::SelectableSlow)
            ) | (
                DecodedTelemetry::SelectableFault(_),
                TelemetryFamily::Selectable,
                Some(RequestKind::SelectableFault)
            )
        );
        if !matched {
            return Ok(vec![Effect::Diagnostic(
                Diagnostic::UnexpectedTelemetryResponse,
            )]);
        }
        if let DecodedTelemetry::SelectableFault(fault) = telemetry {
            self.pending_request = None;
            self.latest_firmware_fault = fault.map(|value| map_legacy_firmware_fault(value.fault));
            return Ok(Vec::new());
        }
        let mut record = common_record(telemetry)?;
        if matches!(
            pending,
            Some(RequestKind::Compact | RequestKind::SelectableSlow)
        ) {
            self.pending_request = None;
            self.latest_compact = Some(SlowCache::from_record(&record, observed_at_epoch_ms));
            return self.selectable_followup(now_ms, false);
        }
        if self.family == TelemetryFamily::Selectable {
            record.firmware_fault = self.latest_firmware_fault;
            if record.firmware_fault.is_some() {
                record.alert_reason = Some(AlertReason::FirmwareFault);
            }
        }

        let started_at_ms = self.started_at_ms.unwrap_or(now_ms);
        let next_sequence = self.sequence.wrapping_add(1);
        let next_tail = if record.ride_state == RideState::Riding {
            None
        } else if self.previous_ride_state == RideState::Riding {
            Some(
                now_ms
                    .checked_add(self.settings.post_ride_tail_ms)
                    .ok_or(ServiceError::MonotonicOverflow)?,
            )
        } else {
            self.post_ride_probe_until_ms
        };
        let in_post_ride_tail = next_tail.is_some_and(|until| now_ms <= until);
        let target_interval = if record.ride_state == RideState::Riding || in_post_ride_tail {
            RIDING_INTERVAL_MS
        } else {
            self.settings.idle_interval_ms
        };
        if self.polling_interval_ms != Some(target_interval) {
            self.preflight_immediate_request(now_ms, true)?;
        }
        // Only continuous, observed riding intervals advance the live ride clock.
        // Starting/reconnecting cannot attribute the preceding idle/offline gap to riding.
        let riding_delta = if self.previous_ride_state == RideState::Riding
            && record.ride_state == RideState::Riding
        {
            self.last_sample_at_ms
                .map_or(0, |last| now_ms.saturating_sub(last))
        } else {
            0
        };
        let next_riding_elapsed_ms = self.riding_elapsed_ms.saturating_add(riding_delta);
        let mut sample = self.normalized_sample(
            record,
            next_sequence,
            observed_at_epoch_ms,
            now_ms.saturating_sub(started_at_ms),
        );

        sample.riding_elapsed_ms = next_riding_elapsed_ms;
        self.riding_elapsed_ms = next_riding_elapsed_ms;
        self.last_sample_at_ms = Some(now_ms);
        self.pending_request = None;
        self.started_at_ms = Some(started_at_ms);
        self.sequence = next_sequence;
        self.post_ride_probe_until_ms = next_tail;
        self.previous_ride_state = sample.ride_state;
        let mut effects = vec![Effect::Publish(Box::new(sample))];
        if self.polling_interval_ms != Some(target_interval) {
            effects.extend(self.restart_polling(target_interval, now_ms)?);
        }
        effects.extend(self.selectable_followup(now_ms, true)?);
        Ok(effects)
    }

    fn stop(&mut self, service_generation: u64) -> Vec<Effect> {
        if self.service_generation != Some(service_generation) {
            return Vec::new();
        }
        self.stop_active()
    }

    fn reset(&mut self) -> Vec<Effect> {
        self.started_at_ms = None;
        self.riding_elapsed_ms = 0;
        self.sequence = 0;
        self.reset_connection()
    }

    fn reset_connection(&mut self) -> Vec<Effect> {
        let mut effects = self.stop_active();
        effects.extend(self.cancel_layout());
        self.service_generation = None;
        self.scheduler_generation = None;
        self.route = None;
        self.family = TelemetryFamily::Dynamic;
        effects
    }

    fn stop_active(&mut self) -> Vec<Effect> {
        self.active = false;
        self.last_sample_at_ms = None;
        let mut effects = Vec::new();
        if let Some(token) = self.poll_timer.take() {
            effects.push(Effect::CancelTimer(token));
        }
        self.polling_interval_ms = None;
        self.previous_ride_state = RideState::Unknown;
        self.post_ride_probe_until_ms = None;
        self.pending_request = None;
        self.next_compact_due_ms = None;
        self.next_fault_due_ms = None;
        self.latest_compact = None;
        self.selectable_gnss = false;
        self.latest_firmware_fault = None;
        effects
    }

    fn cancel_layout(&mut self) -> Vec<Effect> {
        self.layout_attempts = 0;
        self.layout_timer
            .take()
            .map(Effect::CancelTimer)
            .into_iter()
            .collect()
    }

    fn restart_polling(
        &mut self,
        interval_ms: u64,
        now_ms: u64,
    ) -> Result<Vec<Effect>, ServiceError> {
        let mut effects = Vec::new();
        if let Some(token) = self.poll_timer.take() {
            effects.push(Effect::CancelTimer(token));
        }
        self.polling_interval_ms = Some(interval_ms);
        effects.extend(self.request_sample(now_ms)?);
        let generation = self
            .service_generation
            .ok_or(ServiceError::InvalidTelemetry)?;
        let token = self.new_timer(TimerKind::Poll, generation);
        self.poll_timer = Some(token);
        effects.push(Effect::ReplaceTimer { token, interval_ms });
        Ok(effects)
    }

    fn request_sample(&mut self, now_ms: u64) -> Result<Vec<Effect>, ServiceError> {
        if !self.active || self.pending_request.is_some() {
            return Ok(Vec::new());
        }
        let kind = match self.family {
            TelemetryFamily::LegacyFixed => RequestKind::Legacy,
            TelemetryFamily::Dynamic
                if self.next_compact_due_ms.is_some_and(|due| now_ms >= due) =>
            {
                let next = now_ms
                    .checked_add(COMPACT_INTERVAL_MS)
                    .ok_or(ServiceError::MonotonicOverflow)?;
                self.next_compact_due_ms = Some(next);
                RequestKind::Compact
            }
            TelemetryFamily::Dynamic => RequestKind::Realtime,
            TelemetryFamily::Selectable => RequestKind::SelectableFast,
        };
        let riding_priority = self.previous_ride_state == RideState::Riding
            || self
                .post_ride_probe_until_ms
                .is_some_and(|until| now_ms <= until);
        let facts = self.request_facts(
            kind,
            if riding_priority {
                Priority::RidingTelemetry
            } else {
                Priority::ControllerWork
            },
        )?;
        self.pending_request = Some(kind);
        Ok(vec![Effect::Enqueue { kind, facts }])
    }

    // Successful fast samples may be followed by the due slow/fault probes on
    // the same scheduler lane. Timer ticks always start with fast state so an
    // idle interval at or above one second cannot starve control confirmation.
    fn selectable_followup(
        &mut self,
        now_ms: u64,
        allow_slow: bool,
    ) -> Result<Vec<Effect>, ServiceError> {
        if self.family != TelemetryFamily::Selectable || self.pending_request.is_some() {
            return Ok(Vec::new());
        }
        let kind = if allow_slow && self.next_compact_due_ms.is_some_and(|due| now_ms >= due) {
            self.next_compact_due_ms = Some(
                now_ms
                    .checked_add(COMPACT_INTERVAL_MS)
                    .ok_or(ServiceError::MonotonicOverflow)?,
            );
            RequestKind::SelectableSlow
        } else if self.next_fault_due_ms.is_some_and(|due| now_ms >= due) {
            self.next_fault_due_ms = Some(
                now_ms
                    .checked_add(FAULT_PROBE_INTERVAL_MS)
                    .ok_or(ServiceError::MonotonicOverflow)?,
            );
            RequestKind::SelectableFault
        } else {
            return Ok(Vec::new());
        };
        let priority = if self.previous_ride_state == RideState::Riding
            || self
                .post_ride_probe_until_ms
                .is_some_and(|until| now_ms <= until)
        {
            Priority::RidingTelemetry
        } else {
            Priority::ControllerWork
        };
        let facts = self.request_facts(kind, priority)?;
        self.pending_request = Some(kind);
        Ok(vec![Effect::Enqueue { kind, facts }])
    }

    fn layout_request(&mut self) -> Result<Effect, ServiceError> {
        let facts = self.request_facts(RequestKind::Layout, Priority::ControllerWork)?;
        self.layout_attempts = self.layout_attempts.saturating_add(1);
        Ok(Effect::Enqueue {
            kind: RequestKind::Layout,
            facts,
        })
    }

    fn request_facts(
        &self,
        kind: RequestKind,
        priority: Priority,
    ) -> Result<RequestFacts, ServiceError> {
        let route = self.route.ok_or(ServiceError::InvalidTelemetry)?;
        validate_route(route)?;
        let generation = self
            .scheduler_generation
            .ok_or(ServiceError::InvalidTelemetry)?;
        let (command, context) = match kind {
            RequestKind::Layout => (
                REALTIME_DATA_IDS_COMMAND,
                RequestContext::RefloatLayout(route),
            ),
            RequestKind::Legacy | RequestKind::Compact => (
                GET_ALL_DATA_COMMAND,
                RequestContext::RefloatTelemetry(route),
            ),
            RequestKind::Realtime => (
                REALTIME_DATA_COMMAND,
                RequestContext::RefloatTelemetry(route),
            ),
            RequestKind::SelectableFast | RequestKind::SelectableSlow => (
                dynamic::SELECTABLE_REALTIME_DATA_COMMAND,
                RequestContext::RefloatTelemetry(route),
            ),
            RequestKind::SelectableFault => (
                GET_ALL_DATA_COMMAND,
                RequestContext::RefloatTelemetry(route),
            ),
        };
        let mut payload = match kind {
            RequestKind::SelectableFast => dynamic::selectable_fast_request().to_vec(),
            RequestKind::SelectableSlow => {
                dynamic::selectable_slow_request(self.selectable_gnss).to_vec()
            }
            RequestKind::SelectableFault => legacy::FAULT_ONLY_REQUEST.to_vec(),
            _ => vec![CUSTOM_APP_DATA_COMMAND, REFLOAT_PACKAGE_ID, command],
        };
        if matches!(kind, RequestKind::Legacy | RequestKind::Compact) {
            payload.push(DETAIL_LEVEL_FOUR);
        }
        if let Route::Can(can_id) = route {
            let mut routed = Vec::with_capacity(payload.len() + 2);
            routed.extend([FORWARD_CAN_COMMAND, can_id]);
            routed.extend(payload);
            payload = routed;
        }
        Ok(RequestFacts {
            payload,
            expected_response: ExpectedResponse {
                command: CUSTOM_APP_DATA_COMMAND,
                refloat_command: Some(command),
            },
            priority,
            context,
            timeout_ms: REQUEST_TIMEOUT_MS,
            generation,
            expects_response: true,
        })
    }

    fn new_timer(&mut self, kind: TimerKind, service_generation: u64) -> TimerToken {
        self.next_timer_nonce = self.next_timer_nonce.wrapping_add(1);
        TimerToken {
            kind,
            service_generation,
            nonce: self.next_timer_nonce,
        }
    }

    fn preflight_immediate_request(
        &self,
        now_ms: u64,
        pending_will_clear: bool,
    ) -> Result<(), ServiceError> {
        if self.active
            && (self.pending_request.is_none() || pending_will_clear)
            && matches!(
                self.family,
                TelemetryFamily::Dynamic | TelemetryFamily::Selectable
            )
            && self.next_compact_due_ms.is_some_and(|due| now_ms >= due)
        {
            now_ms
                .checked_add(COMPACT_INTERVAL_MS)
                .ok_or(ServiceError::MonotonicOverflow)?;
        }
        Ok(())
    }

    fn normalized_sample(
        &self,
        value: CommonRecord,
        sequence: u64,
        observed_at_epoch_ms: i64,
        elapsed_ms: u64,
    ) -> NormalizedSample {
        let slow = match self.family {
            TelemetryFamily::Dynamic | TelemetryFamily::Selectable => self.latest_compact.clone(),
            TelemetryFamily::LegacyFixed => {
                Some(SlowCache::from_record(&value, observed_at_epoch_ms))
            }
        };
        NormalizedSample {
            sequence,
            observed_at_epoch_ms,
            slow_observed_at_epoch_ms: slow.as_ref().map(|value| value.observed_at_epoch_ms),
            elapsed_ms,
            riding_elapsed_ms: self.riding_elapsed_ms,
            ride_state: value.ride_state,
            package_state: value.package_state,
            package_mode: value.package_mode,
            footpad_state: value.footpad_state,
            stop_condition: value.stop_condition,
            setpoint_adjustment: value.setpoint_adjustment,
            wheelslip: value.wheelslip,
            darkride: value.darkride,
            charging: value.charging,
            alert_reason: value.alert_reason,
            firmware_fault: value.firmware_fault,
            fatal_error_latched: value.fatal_error_latched,
            pack_voltage_v: value.pack_voltage_v,
            duty_cycle: value.duty_cycle,
            controller_temp_c: value.controller_temp_c,
            motor_temp_c: value.motor_temp_c,
            motor_current_a: value.motor_current_a,
            battery_current_a: value.battery_current_a,
            erpm: value.erpm,
            speed_kph: value.speed_kph,
            pitch_deg: value.pitch_deg,
            balance_pitch_deg: value.balance_pitch_deg,
            roll_deg: value.roll_deg,
            directional_motor_current_a: value.directional_motor_current_a,
            filtered_motor_current_a: value.filtered_motor_current_a,
            requested_balance_current_a: value.requested_balance_current_a,
            booster_current_a: value.booster_current_a,
            field_weakening_current_a: slow
                .as_ref()
                .and_then(|value| value.field_weakening_current_a),
            footpad_adc1_v: value.footpad_adc1_v,
            footpad_adc2_v: value.footpad_adc2_v,
            remote_input: value.remote_input,
            setpoint_deg: value.setpoint_deg,
            atr_setpoint_deg: value.atr_setpoint_deg,
            brake_tilt_setpoint_deg: value.brake_tilt_setpoint_deg,
            torque_tilt_setpoint_deg: value.torque_tilt_setpoint_deg,
            turn_tilt_setpoint_deg: value.turn_tilt_setpoint_deg,
            remote_setpoint_deg: value.remote_setpoint_deg,
            atr_acceleration_difference_erpm_per_second: value
                .atr_acceleration_difference_erpm_per_second,
            atr_speed_boost: value.atr_speed_boost,
            distance_m: slow.as_ref().and_then(|value| value.distance_m),
            odometer_m: slow.as_ref().and_then(|value| value.odometer_m),
            amp_hours_drawn_ah: slow.as_ref().and_then(|value| value.amp_hours_drawn_ah),
            amp_hours_charged_ah: slow.as_ref().and_then(|value| value.amp_hours_charged_ah),
            watt_hours_drawn_wh: slow.as_ref().and_then(|value| value.watt_hours_drawn_wh),
            watt_hours_charged_wh: slow.as_ref().and_then(|value| value.watt_hours_charged_wh),
            battery_level: slow.as_ref().and_then(|value| value.battery_level),
            charging_current_a: slow.as_ref().and_then(|value| value.charging_current_a),
            charging_voltage_v: slow.as_ref().and_then(|value| value.charging_voltage_v),
            extensions: {
                let mut extensions = value.extensions;
                if let Some(slow) = &slow {
                    for extension in &slow.extensions {
                        if extensions.len() == MAX_EXTENSIONS {
                            break;
                        }
                        if !extensions.iter().any(|value| value.name == extension.name) {
                            extensions.push(extension.clone());
                        }
                    }
                }
                extensions
            },
        }
    }
}

fn common_record(value: DecodedTelemetry) -> Result<CommonRecord, ServiceError> {
    let record = match value {
        DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(value)) => {
            common_legacy_sample(value)
        }
        DecodedTelemetry::Legacy(legacy::LegacyTelemetry::ActiveFirmwareFault(value)) => {
            common_active_fault(value)
        }
        DecodedTelemetry::Dynamic(value) => common_dynamic_sample(value)?,
        DecodedTelemetry::Selectable(value) => common_selectable_sample(value)?,
        DecodedTelemetry::SelectableFault(_) => return Err(ServiceError::InvalidTelemetry),
    };
    validate_common_record(&record)?;
    Ok(record)
}

fn common_selectable_sample(
    value: dynamic::SelectableSample,
) -> Result<CommonRecord, ServiceError> {
    if value.extensions.len() > MAX_EXTENSIONS
        || value.extensions.iter().any(|item| {
            item.name.is_empty()
                || item.name.len() > dynamic::MAX_FIELD_BYTES
                || !item.value.is_finite()
        })
    {
        return Err(ServiceError::InvalidTelemetry);
    }
    let package_state = value.package_state.map(map_dynamic_package_state);
    Ok(CommonRecord {
        ride_state: package_state.map_or(RideState::Unknown, ride_state),
        package_state,
        package_mode: value.package_mode.map(map_dynamic_package_mode),
        footpad_state: value.footpad_state.map(map_dynamic_footpad_state),
        stop_condition: value.stop_condition.map(map_dynamic_stop_condition),
        setpoint_adjustment: value
            .setpoint_adjustment
            .map(map_dynamic_setpoint_adjustment),
        wheelslip: value.wheelslip,
        darkride: value.darkride,
        charging: value.charging,
        alert_reason: value.alert_reason.map(map_dynamic_alert_reason),
        firmware_fault: None,
        fatal_error_latched: value.fatal_error_latched,
        pack_voltage_v: value.values.pack_voltage_v,
        duty_cycle: value.values.duty_cycle,
        controller_temp_c: value.values.controller_temp_c,
        motor_temp_c: value.values.motor_temp_c,
        motor_current_a: value.values.motor_current_a,
        battery_current_a: value.values.battery_current_a,
        erpm: value.values.erpm,
        speed_kph: value.values.speed_kph,
        pitch_deg: value.values.pitch_deg,
        balance_pitch_deg: value.values.balance_pitch_deg,
        roll_deg: value.values.roll_deg,
        directional_motor_current_a: value.values.directional_motor_current_a,
        filtered_motor_current_a: value.values.filtered_motor_current_a,
        requested_balance_current_a: value.values.requested_balance_current_a,
        booster_current_a: None,
        field_weakening_current_a: value.field_weakening_current_a,
        footpad_adc1_v: value.values.footpad_adc1_v,
        footpad_adc2_v: value.values.footpad_adc2_v,
        remote_input: value.values.remote_input,
        setpoint_deg: value.values.setpoint_deg,
        atr_setpoint_deg: value.values.atr_setpoint_deg,
        brake_tilt_setpoint_deg: value.values.brake_tilt_setpoint_deg,
        torque_tilt_setpoint_deg: value.values.torque_tilt_setpoint_deg,
        turn_tilt_setpoint_deg: value.values.turn_tilt_setpoint_deg,
        remote_setpoint_deg: value.values.remote_setpoint_deg,
        atr_acceleration_difference_erpm_per_second: None,
        atr_speed_boost: None,
        distance_m: value.distance_m,
        odometer_m: value.odometer_m,
        amp_hours_drawn_ah: value.amp_hours_drawn_ah,
        amp_hours_charged_ah: value.amp_hours_charged_ah,
        watt_hours_drawn_wh: value.watt_hours_drawn_wh,
        watt_hours_charged_wh: value.watt_hours_charged_wh,
        battery_level: value.battery_level,
        charging_current_a: value.charging_current_a,
        charging_voltage_v: value.charging_voltage_v,
        extensions: value
            .extensions
            .into_iter()
            .map(|item| ExtensionValue {
                name: item.name,
                value: item.value,
            })
            .collect(),
    })
}

fn common_legacy_sample(value: legacy::LegacySample) -> CommonRecord {
    let package_state = map_legacy_package_state(value.package_state);
    CommonRecord {
        ride_state: ride_state(package_state),
        package_state: Some(package_state),
        package_mode: Some(map_legacy_package_mode(value.package_mode)),
        footpad_state: Some(map_legacy_footpad_state(value.footpad_state)),
        stop_condition: Some(map_legacy_stop_condition(value.stop_condition)),
        setpoint_adjustment: Some(map_legacy_setpoint_adjustment(value.setpoint_adjustment)),
        wheelslip: Some(value.wheelslip),
        darkride: Some(value.darkride),
        charging: Some(value.charging),
        alert_reason: Some(map_legacy_alert_reason(value.alert_reason)),
        firmware_fault: None,
        fatal_error_latched: None,
        pack_voltage_v: Some(value.pack_voltage_v),
        duty_cycle: Some(value.duty_cycle),
        controller_temp_c: value.controller_temp_c,
        motor_temp_c: value.motor_temp_c,
        motor_current_a: Some(value.motor_current_a),
        battery_current_a: Some(value.battery_current_a),
        erpm: Some(value.erpm),
        speed_kph: Some(value.speed_kph),
        pitch_deg: Some(value.pitch_deg),
        balance_pitch_deg: Some(value.balance_pitch_deg),
        roll_deg: Some(value.roll_deg),
        directional_motor_current_a: None,
        filtered_motor_current_a: None,
        requested_balance_current_a: Some(value.requested_balance_current_a),
        booster_current_a: Some(value.booster_current_a),
        field_weakening_current_a: value.field_weakening_current_a,
        footpad_adc1_v: Some(value.footpad_adc1_v),
        footpad_adc2_v: Some(value.footpad_adc2_v),
        remote_input: None,
        setpoint_deg: Some(value.setpoint_deg),
        atr_setpoint_deg: Some(value.atr_setpoint_deg),
        brake_tilt_setpoint_deg: Some(value.brake_tilt_setpoint_deg),
        torque_tilt_setpoint_deg: Some(value.torque_tilt_setpoint_deg),
        turn_tilt_setpoint_deg: Some(value.turn_tilt_setpoint_deg),
        remote_setpoint_deg: Some(value.remote_setpoint_deg),
        atr_acceleration_difference_erpm_per_second: None,
        atr_speed_boost: None,
        distance_m: value.distance_m,
        odometer_m: value.odometer_m,
        amp_hours_drawn_ah: value.amp_hours_drawn_ah,
        amp_hours_charged_ah: value.amp_hours_charged_ah,
        watt_hours_drawn_wh: value.watt_hours_drawn_wh,
        watt_hours_charged_wh: value.watt_hours_charged_wh,
        battery_level: value.battery_level,
        charging_current_a: value.charging_current_a,
        charging_voltage_v: value.charging_voltage_v,
        extensions: Vec::new(),
    }
}

fn common_active_fault(value: legacy::ActiveFirmwareFault) -> CommonRecord {
    CommonRecord {
        ride_state: RideState::Unknown,
        package_state: None,
        package_mode: None,
        footpad_state: None,
        stop_condition: None,
        setpoint_adjustment: None,
        wheelslip: None,
        darkride: None,
        charging: None,
        alert_reason: Some(AlertReason::FirmwareFault),
        firmware_fault: Some(map_legacy_firmware_fault(value.fault)),
        fatal_error_latched: None,
        pack_voltage_v: None,
        duty_cycle: None,
        controller_temp_c: None,
        motor_temp_c: None,
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
        extensions: Vec::new(),
    }
}

fn common_dynamic_sample(value: dynamic::DynamicSample) -> Result<CommonRecord, ServiceError> {
    if value.extensions.len() > MAX_EXTENSIONS
        || value.extensions.iter().any(|item| {
            item.name.is_empty()
                || item.name.len() > dynamic::MAX_FIELD_BYTES
                || !item.value.is_finite()
        })
        || value
            .extensions
            .iter()
            .try_fold(0usize, |total, item| total.checked_add(item.name.len()))
            .is_none_or(|total| total > dynamic::MAX_AGGREGATE_FIELD_BYTES)
    {
        return Err(ServiceError::InvalidTelemetry);
    }
    let package_state = map_dynamic_package_state(value.package_state);
    Ok(CommonRecord {
        ride_state: ride_state(package_state),
        package_state: Some(package_state),
        package_mode: Some(map_dynamic_package_mode(value.package_mode)),
        footpad_state: Some(map_dynamic_footpad_state(value.footpad_state)),
        stop_condition: Some(map_dynamic_stop_condition(value.stop_condition)),
        setpoint_adjustment: Some(map_dynamic_setpoint_adjustment(value.setpoint_adjustment)),
        wheelslip: Some(value.wheelslip),
        darkride: Some(value.darkride),
        charging: Some(value.charging),
        alert_reason: Some(map_dynamic_alert_reason(value.alert_reason)),
        firmware_fault: value.firmware_fault.map(map_dynamic_firmware_fault),
        fatal_error_latched: value.fatal_error_latched,
        pack_voltage_v: value.values.pack_voltage_v,
        duty_cycle: value.values.duty_cycle,
        controller_temp_c: value.values.controller_temp_c,
        motor_temp_c: value.values.motor_temp_c,
        motor_current_a: value.values.motor_current_a,
        battery_current_a: value.values.battery_current_a,
        erpm: value.values.erpm,
        speed_kph: value.values.speed_kph,
        pitch_deg: value.values.pitch_deg,
        balance_pitch_deg: value.values.balance_pitch_deg,
        roll_deg: value.values.roll_deg,
        directional_motor_current_a: value.values.directional_motor_current_a,
        filtered_motor_current_a: value.values.filtered_motor_current_a,
        requested_balance_current_a: value.values.requested_balance_current_a,
        booster_current_a: value.values.booster_current_a,
        field_weakening_current_a: None,
        footpad_adc1_v: value.values.footpad_adc1_v,
        footpad_adc2_v: value.values.footpad_adc2_v,
        remote_input: value.values.remote_input,
        setpoint_deg: value.values.setpoint_deg,
        atr_setpoint_deg: value.values.atr_setpoint_deg,
        brake_tilt_setpoint_deg: value.values.brake_tilt_setpoint_deg,
        torque_tilt_setpoint_deg: value.values.torque_tilt_setpoint_deg,
        turn_tilt_setpoint_deg: value.values.turn_tilt_setpoint_deg,
        remote_setpoint_deg: value.values.remote_setpoint_deg,
        atr_acceleration_difference_erpm_per_second: value
            .values
            .atr_acceleration_difference_erpm_per_second,
        atr_speed_boost: value.values.atr_speed_boost,
        distance_m: None,
        odometer_m: None,
        amp_hours_drawn_ah: None,
        amp_hours_charged_ah: None,
        watt_hours_drawn_wh: None,
        watt_hours_charged_wh: None,
        battery_level: None,
        charging_current_a: value.charging_current_a,
        charging_voltage_v: value.charging_voltage_v,
        extensions: value
            .extensions
            .into_iter()
            .map(|item| ExtensionValue {
                name: item.name,
                value: item.value,
            })
            .collect(),
    })
}

fn validate_route(route: Route) -> Result<(), ServiceError> {
    match route {
        Route::Direct | Route::Can(0..=253) => Ok(()),
        Route::Can(_) => Err(ServiceError::InvalidRoute),
    }
}

fn validate_common_record(value: &CommonRecord) -> Result<(), ServiceError> {
    let values = [
        value.pack_voltage_v,
        value.duty_cycle,
        value.controller_temp_c,
        value.motor_temp_c,
        value.motor_current_a,
        value.battery_current_a,
        value.erpm,
        value.speed_kph,
        value.pitch_deg,
        value.balance_pitch_deg,
        value.roll_deg,
        value.directional_motor_current_a,
        value.filtered_motor_current_a,
        value.requested_balance_current_a,
        value.booster_current_a,
        value.field_weakening_current_a,
        value.footpad_adc1_v,
        value.footpad_adc2_v,
        value.remote_input,
        value.setpoint_deg,
        value.atr_setpoint_deg,
        value.brake_tilt_setpoint_deg,
        value.torque_tilt_setpoint_deg,
        value.turn_tilt_setpoint_deg,
        value.remote_setpoint_deg,
        value.atr_acceleration_difference_erpm_per_second,
        value.atr_speed_boost,
        value.distance_m,
        value.odometer_m,
        value.amp_hours_drawn_ah,
        value.amp_hours_charged_ah,
        value.watt_hours_drawn_wh,
        value.watt_hours_charged_wh,
        value.battery_level,
        value.charging_current_a,
        value.charging_voltage_v,
    ];
    if values.into_iter().flatten().all(f64::is_finite) {
        Ok(())
    } else {
        Err(ServiceError::InvalidTelemetry)
    }
}

fn ride_state(value: PackageState) -> RideState {
    if value == PackageState::Running {
        RideState::Riding
    } else {
        RideState::Idle
    }
}

macro_rules! map_same_variants {
    ($function:ident, $source:path, $target:ident, [$($variant:ident),+ $(,)?]) => {
        fn $function(value: $source) -> $target {
            match value {
                $(<$source>::$variant => $target::$variant,)+
            }
        }
    };
}

map_same_variants!(
    map_legacy_package_state,
    legacy::PackageState,
    PackageState,
    [Startup, Running, Ready, Disabled]
);
map_same_variants!(
    map_dynamic_package_state,
    dynamic::PackageState,
    PackageState,
    [Disabled, Startup, Ready, Running]
);
map_same_variants!(
    map_legacy_package_mode,
    legacy::PackageMode,
    PackageMode,
    [Normal, HandTest, Flywheel]
);
map_same_variants!(
    map_dynamic_package_mode,
    dynamic::PackageMode,
    PackageMode,
    [Normal, HandTest, Flywheel, Unknown]
);
map_same_variants!(
    map_legacy_footpad_state,
    legacy::FootpadState,
    FootpadState,
    [None, Left, Right, Both]
);
map_same_variants!(
    map_dynamic_footpad_state,
    dynamic::FootpadState,
    FootpadState,
    [None, Left, Right, Both]
);
map_same_variants!(
    map_legacy_stop_condition,
    legacy::StopCondition,
    StopCondition,
    [
        None,
        Pitch,
        Roll,
        SwitchHalf,
        SwitchFull,
        ReverseStop,
        QuickStop
    ]
);

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings::new(1_000, 5_000).expect("valid settings")
    }

    fn configuration(family: TelemetryFamily, route: Route) -> Configuration {
        Configuration {
            service_generation: 7,
            scheduler_generation: Generation(11),
            route,
            family,
            selectable_gnss: false,
            settings: settings(),
        }
    }

    fn start(
        service: &mut RefloatTelemetryService,
        family: TelemetryFamily,
        route: Route,
        now_ms: u64,
    ) -> Vec<Effect> {
        service
            .apply(Command::Start {
                configuration: configuration(family, route),
                now_ms,
            })
            .expect("start")
    }

    fn enqueue(effects: &[Effect]) -> (RequestKind, &RequestFacts) {
        effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Enqueue { kind, facts } => Some((*kind, facts)),
                _ => None,
            })
            .expect("enqueue effect")
    }

    fn timer(effects: &[Effect], kind: TimerKind) -> TimerToken {
        effects
            .iter()
            .find_map(|effect| match effect {
                Effect::ReplaceTimer { token, .. } if token.kind == kind => Some(*token),
                _ => None,
            })
            .expect("timer effect")
    }

    fn published(effects: &[Effect]) -> &NormalizedSample {
        effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Publish(sample) => Some(sample.as_ref()),
                _ => None,
            })
            .expect("publish effect")
    }

    fn legacy_sample(state: legacy::PackageState, base: f64) -> legacy::LegacySample {
        legacy::LegacySample {
            detail_level: 4,
            package_state: state,
            package_mode: legacy::PackageMode::Flywheel,
            footpad_state: legacy::FootpadState::Both,
            stop_condition: legacy::StopCondition::QuickStop,
            setpoint_adjustment: legacy::SetpointAdjustment::Temperature,
            wheelslip: true,
            darkride: true,
            charging: true,
            alert_reason: legacy::AlertReason::CellHighVoltage,
            pack_voltage_v: base,
            duty_cycle: base + 1.0,
            controller_temp_c: Some(base + 2.0),
            motor_temp_c: Some(base + 3.0),
            motor_current_a: base + 4.0,
            battery_current_a: base + 5.0,
            erpm: base + 6.0,
            speed_kph: base + 7.0,
            pitch_deg: base + 8.0,
            balance_pitch_deg: base + 9.0,
            roll_deg: base + 10.0,
            requested_balance_current_a: base + 11.0,
            booster_current_a: base + 12.0,
            field_weakening_current_a: Some(base + 13.0),
            footpad_adc1_v: base + 14.0,
            footpad_adc2_v: base + 15.0,
            setpoint_deg: base + 16.0,
            atr_setpoint_deg: base + 17.0,
            brake_tilt_setpoint_deg: base + 18.0,
            torque_tilt_setpoint_deg: base + 19.0,
            turn_tilt_setpoint_deg: base + 20.0,
            remote_setpoint_deg: base + 21.0,
            distance_m: Some(base + 22.0),
            odometer_m: Some(base + 23.0),
            amp_hours_drawn_ah: Some(base + 24.0),
            amp_hours_charged_ah: Some(base + 25.0),
            watt_hours_drawn_wh: Some(base + 26.0),
            watt_hours_charged_wh: Some(base + 27.0),
            battery_level: Some(base + 28.0),
            charging_current_a: Some(base + 29.0),
            charging_voltage_v: Some(base + 30.0),
        }
    }

    fn dynamic_sample(state: dynamic::PackageState, base: f64) -> dynamic::DynamicSample {
        dynamic::DynamicSample {
            source_ticks: 42,
            package_state: state,
            package_mode: dynamic::PackageMode::Unknown,
            footpad_state: dynamic::FootpadState::Left,
            stop_condition: dynamic::StopCondition::Unknown,
            setpoint_adjustment: dynamic::SetpointAdjustment::Error,
            wheelslip: true,
            darkride: false,
            charging: true,
            alert_reason: dynamic::AlertReason::BmsOverTemperature,
            fatal_error_latched: Some(true),
            active_alert_words: Some([1, 2]),
            firmware_fault_raw_code: Some(27),
            firmware_fault: Some(dynamic::FirmwareFault::PhaseFilter),
            values: dynamic::DynamicValues {
                speed_kph: Some(base),
                erpm: Some(base + 1.0),
                motor_current_a: Some(base + 2.0),
                directional_motor_current_a: Some(base + 3.0),
                filtered_motor_current_a: Some(base + 4.0),
                duty_cycle: Some(base + 5.0),
                pack_voltage_v: Some(base + 6.0),
                battery_current_a: Some(base + 7.0),
                controller_temp_c: Some(base + 8.0),
                motor_temp_c: Some(base + 9.0),
                pitch_deg: Some(base + 10.0),
                balance_pitch_deg: Some(base + 11.0),
                roll_deg: Some(base + 12.0),
                footpad_adc1_v: Some(base + 13.0),
                footpad_adc2_v: Some(base + 14.0),
                remote_input: Some(base + 15.0),
                setpoint_deg: Some(base + 16.0),
                atr_setpoint_deg: Some(base + 17.0),
                brake_tilt_setpoint_deg: Some(base + 18.0),
                torque_tilt_setpoint_deg: Some(base + 19.0),
                turn_tilt_setpoint_deg: Some(base + 20.0),
                remote_setpoint_deg: Some(base + 21.0),
                requested_balance_current_a: Some(base + 22.0),
                atr_acceleration_difference_erpm_per_second: Some(base + 23.0),
                atr_speed_boost: Some(base + 24.0),
                booster_current_a: Some(base + 25.0),
            },
            charging_current_a: Some(base + 26.0),
            charging_voltage_v: Some(base + 27.0),
            extensions: vec![dynamic::ExtensionValue {
                name: "future".into(),
                value: base + 28.0,
            }],
        }
    }

    fn selectable_sample(
        state: Option<dynamic::PackageState>,
        base: f64,
    ) -> dynamic::SelectableSample {
        dynamic::SelectableSample {
            source_ticks: 43,
            package_state: state,
            package_mode: state.map(|_| dynamic::PackageMode::Normal),
            footpad_state: state.map(|_| dynamic::FootpadState::Both),
            stop_condition: state.map(|_| dynamic::StopCondition::None),
            setpoint_adjustment: state.map(|_| dynamic::SetpointAdjustment::None),
            wheelslip: state.map(|_| false),
            darkride: state.map(|_| false),
            charging: state.map(|_| false),
            alert_reason: state.map(|_| dynamic::AlertReason::None),
            fatal_error_latched: state.map(|_| false),
            values: dynamic::DynamicValues {
                speed_kph: state.map(|_| base),
                pack_voltage_v: state.map(|_| base + 1.0),
                requested_balance_current_a: state.map(|_| base + 2.0),
                ..dynamic::DynamicValues::default()
            },
            battery_level: state.is_none().then_some(base + 3.0),
            distance_m: state.is_none().then_some(base + 4.0),
            odometer_m: state.is_none().then_some(base + 5.0),
            charging_voltage_v: None,
            charging_current_a: None,
            amp_hours_drawn_ah: None,
            amp_hours_charged_ah: None,
            watt_hours_drawn_wh: None,
            watt_hours_charged_wh: None,
            field_weakening_current_a: state.is_none().then_some(base + 6.0),
            extensions: vec![dynamic::ExtensionValue {
                name: if state.is_some() {
                    "refloat.loop_frequency_hz"
                } else {
                    "refloat.gnss.latitude"
                }
                .into(),
                value: base + 7.0,
            }],
        }
    }

    fn receive(
        service: &mut RefloatTelemetryService,
        telemetry: DecodedTelemetry,
        now_ms: u64,
        observed_at_epoch_ms: i64,
    ) -> Vec<Effect> {
        service
            .apply(Command::Receive {
                service_generation: 7,
                telemetry: Box::new(telemetry),
                now_ms,
                observed_at_epoch_ms,
            })
            .expect("receive")
    }

    #[test]
    fn selectable_cycles_fast_slow_and_fault_without_publishing_probe_payloads() {
        let mut service = RefloatTelemetryService::default();
        let mut config = configuration(TelemetryFamily::Selectable, Route::Direct);
        config.selectable_gnss = true;
        let effects = service
            .apply(Command::Start {
                configuration: config,
                now_ms: 0,
            })
            .unwrap();
        let (kind, facts) = enqueue(&effects);
        assert_eq!(kind, RequestKind::SelectableFast);
        assert_eq!(facts.payload, dynamic::selectable_fast_request());
        assert!(
            receive(
                &mut service,
                DecodedTelemetry::Selectable(selectable_sample(
                    Some(dynamic::PackageState::Ready),
                    1.0
                )),
                1,
                1_001,
            )
            .iter()
            .any(|effect| matches!(effect, Effect::Publish(_)))
        );

        let effects = service
            .apply(Command::Refresh {
                service_generation: 7,
                now_ms: COMPACT_INTERVAL_MS,
            })
            .unwrap();
        assert_eq!(enqueue(&effects).0, RequestKind::SelectableFast);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Selectable(selectable_sample(
                Some(dynamic::PackageState::Ready),
                2.0,
            )),
            1_000,
            2_000,
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Publish(_)))
        );
        let (kind, facts) = enqueue(&effects);
        assert_eq!(kind, RequestKind::SelectableSlow);
        assert_eq!(facts.payload, dynamic::selectable_slow_request(true));
        let effects = receive(
            &mut service,
            DecodedTelemetry::Selectable(selectable_sample(None, 10.0)),
            1_001,
            2_001,
        );
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, Effect::Publish(_)))
        );
        let (kind, facts) = enqueue(&effects);
        assert_eq!(kind, RequestKind::SelectableFault);
        assert_eq!(facts.payload, legacy::FAULT_ONLY_REQUEST);
        assert!(
            receive(
                &mut service,
                DecodedTelemetry::SelectableFault(Some(legacy::ActiveFirmwareFault {
                    raw_code: 1,
                    fault: legacy::FirmwareFault::OverVoltage,
                })),
                1_001,
                2_001,
            )
            .is_empty()
        );

        let effects = service
            .apply(Command::Refresh {
                service_generation: 7,
                now_ms: 1_002,
            })
            .unwrap();
        assert_eq!(enqueue(&effects).0, RequestKind::SelectableFast);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Selectable(selectable_sample(
                Some(dynamic::PackageState::Ready),
                20.0,
            )),
            1_002,
            2_002,
        );
        let sample = published(&effects);
        assert_eq!(sample.firmware_fault, Some(FirmwareFault::OverVoltage));
        assert_eq!(sample.alert_reason, Some(AlertReason::FirmwareFault));
        assert_eq!(sample.battery_level, Some(13.0));
        assert_eq!(sample.field_weakening_current_a, Some(16.0));
        assert_eq!(sample.booster_current_a, None);
        assert_eq!(sample.extensions.len(), 2);
    }

    #[test]
    fn validates_settings_without_mutation() {
        assert_eq!(Settings::new(99, 0), Err(ServiceError::InvalidIdleInterval));
        assert_eq!(
            Settings::new(10_001, 0),
            Err(ServiceError::InvalidIdleInterval)
        );
        assert_eq!(
            Settings::new(100, 300_001),
            Err(ServiceError::InvalidPostRideTail)
        );
        assert!(Settings::new(100, 300_000).is_ok());

        let mut service = RefloatTelemetryService::default();
        let before = service.snapshot();
        let result = service.apply(Command::Start {
            configuration: Configuration {
                settings: Settings {
                    idle_interval_ms: 0,
                    post_ride_tail_ms: 0,
                },
                ..configuration(TelemetryFamily::Dynamic, Route::Direct)
            },
            now_ms: 1,
        });
        assert_eq!(result, Err(ServiceError::InvalidIdleInterval));
        assert_eq!(service.snapshot(), before);
    }

    #[test]
    fn rejects_unchecked_invalid_route_atomically() {
        let mut service = RefloatTelemetryService::default();
        let before = service.snapshot();
        assert_eq!(
            service.apply(Command::BeginLayout {
                service_generation: 7,
                scheduler_generation: Generation(11),
                route: Route::Can(254),
            }),
            Err(ServiceError::InvalidRoute)
        );
        assert_eq!(service.snapshot(), before);
        assert_eq!(
            service.apply(Command::Start {
                configuration: configuration(TelemetryFamily::Dynamic, Route::Can(255)),
                now_ms: 0,
            }),
            Err(ServiceError::InvalidRoute)
        );
        assert_eq!(service.snapshot(), before);
    }

    #[test]
    fn layout_retries_exactly_five_times_then_fails() {
        let mut service = RefloatTelemetryService::default();
        let effects = service
            .apply(Command::BeginLayout {
                service_generation: 7,
                scheduler_generation: Generation(11),
                route: Route::Direct,
            })
            .unwrap();
        let (kind, facts) = enqueue(&effects);
        assert_eq!(kind, RequestKind::Layout);
        assert_eq!(facts.payload, [36, 101, 32]);
        assert_eq!(facts.expected_response.refloat_command, Some(32));
        assert_eq!(facts.context, RequestContext::RefloatLayout(Route::Direct));
        assert_eq!(facts.timeout_ms, 900);
        let token = timer(&effects, TimerKind::LayoutRetry);
        assert_eq!(service.snapshot().layout_attempts, 1);

        for expected_attempt in 2..=5 {
            let effects = service
                .apply(Command::TimerFired {
                    token,
                    now_ms: u64::from(expected_attempt) * 1_000,
                })
                .unwrap();
            assert_eq!(enqueue(&effects).0, RequestKind::Layout);
            assert_eq!(service.snapshot().layout_attempts, expected_attempt);
        }
        assert_eq!(
            service
                .apply(Command::TimerFired {
                    token,
                    now_ms: 6_000,
                })
                .unwrap(),
            [
                Effect::CancelTimer(token),
                Effect::Failure(Failure::LayoutTimeout)
            ]
        );
        assert_eq!(service.snapshot().layout_attempts, 0);
    }

    #[test]
    fn can_layout_is_wrapped_once_and_establishment_cancels_timer() {
        let mut service = RefloatTelemetryService::default();
        let effects = service
            .apply(Command::BeginLayout {
                service_generation: 7,
                scheduler_generation: Generation(11),
                route: Route::can(253).unwrap(),
            })
            .unwrap();
        assert_eq!(enqueue(&effects).1.payload, [34, 253, 36, 101, 32]);
        let token = timer(&effects, TimerKind::LayoutRetry);
        assert_eq!(
            service
                .apply(Command::LayoutEstablished {
                    service_generation: 7,
                })
                .unwrap(),
            [Effect::CancelTimer(token)]
        );
        assert!(
            service
                .apply(Command::TimerFired {
                    token,
                    now_ms: 2_000,
                })
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn dynamic_start_enqueues_realtime_then_arms_idle_timer() {
        let mut service = RefloatTelemetryService::default();
        let effects = start(&mut service, TelemetryFamily::Dynamic, Route::Direct, 50);
        assert_eq!(effects.len(), 2);
        let (kind, facts) = enqueue(&effects);
        assert_eq!(kind, RequestKind::Realtime);
        assert_eq!(facts.payload, [36, 101, 31]);
        assert_eq!(facts.priority, Priority::ControllerWork);
        assert_eq!(facts.generation, Generation(11));
        assert_eq!(
            facts.context,
            RequestContext::RefloatTelemetry(Route::Direct)
        );
        assert!(matches!(
            effects[1],
            Effect::ReplaceTimer {
                interval_ms: 1_000,
                ..
            }
        ));
        assert_eq!(service.snapshot().next_compact_due_ms, Some(1_050));
    }

    #[test]
    fn legacy_start_uses_detail_four_and_can_route() {
        let mut service = RefloatTelemetryService::default();
        let effects = start(
            &mut service,
            TelemetryFamily::LegacyFixed,
            Route::can(42).unwrap(),
            0,
        );
        let (kind, facts) = enqueue(&effects);
        assert_eq!(kind, RequestKind::Legacy);
        assert_eq!(facts.payload, [34, 42, 36, 101, 10, 4]);
        assert_eq!(facts.expected_response.refloat_command, Some(10));
        assert_eq!(service.snapshot().next_compact_due_ms, None);
    }

    #[test]
    fn one_pending_request_blocks_ticks_and_timeout_does_not_retry() {
        let mut service = RefloatTelemetryService::default();
        let effects = start(&mut service, TelemetryFamily::Dynamic, Route::Direct, 0);
        let poll = timer(&effects, TimerKind::Poll);
        assert!(
            service
                .apply(Command::TimerFired {
                    token: poll,
                    now_ms: 1_000,
                })
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            service.snapshot().pending_request,
            Some(RequestKind::Realtime)
        );

        let effects = service
            .apply(Command::RequestCompleted {
                service_generation: 7,
                kind: RequestKind::Realtime,
                outcome: RequestOutcome::TimedOut,
            })
            .unwrap();
        assert_eq!(
            effects,
            [Effect::Diagnostic(Diagnostic::TelemetryRequestTimeout)]
        );
        assert_eq!(service.snapshot().pending_request, None);
        assert!(
            service
                .apply(Command::RequestCompleted {
                    service_generation: 7,
                    kind: RequestKind::Realtime,
                    outcome: RequestOutcome::TimedOut,
                })
                .unwrap()
                .is_empty()
        );

        let effects = service
            .apply(Command::TimerFired {
                token: poll,
                now_ms: 1_000,
            })
            .unwrap();
        let (kind, facts) = enqueue(&effects);
        assert_eq!(kind, RequestKind::Compact);
        assert_eq!(facts.payload, [36, 101, 10, 4]);
    }

    #[test]
    fn dynamic_compact_replaces_cache_and_next_realtime_merges_it() {
        let mut service = RefloatTelemetryService::default();
        let effects = start(&mut service, TelemetryFamily::Dynamic, Route::Direct, 0);
        let poll = timer(&effects, TimerKind::Poll);
        service
            .apply(Command::RequestCompleted {
                service_generation: 7,
                kind: RequestKind::Realtime,
                outcome: RequestOutcome::TimedOut,
            })
            .unwrap();
        let effects = service
            .apply(Command::TimerFired {
                token: poll,
                now_ms: 1_000,
            })
            .unwrap();
        assert_eq!(enqueue(&effects).0, RequestKind::Compact);
        assert!(
            receive(
                &mut service,
                DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                    legacy::PackageState::Ready,
                    100.0,
                ))),
                1_010,
                8_000,
            )
            .is_empty()
        );
        assert!(service.snapshot().has_compact_cache);

        let effects = service
            .apply(Command::TimerFired {
                token: poll,
                now_ms: 1_100,
            })
            .unwrap();
        assert_eq!(enqueue(&effects).0, RequestKind::Realtime);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Dynamic(dynamic_sample(dynamic::PackageState::Ready, 1.0)),
            1_120,
            8_120,
        );
        let sample = published(&effects);
        assert_eq!(sample.slow_observed_at_epoch_ms, Some(8_000));
        assert_eq!(sample.field_weakening_current_a, Some(113.0));
        assert_eq!(sample.distance_m, Some(122.0));
        assert_eq!(sample.odometer_m, Some(123.0));
        assert_eq!(sample.amp_hours_drawn_ah, Some(124.0));
        assert_eq!(sample.amp_hours_charged_ah, Some(125.0));
        assert_eq!(sample.watt_hours_drawn_wh, Some(126.0));
        assert_eq!(sample.watt_hours_charged_wh, Some(127.0));
        assert_eq!(sample.battery_level, Some(128.0));
        assert_eq!(sample.charging_current_a, Some(129.0));
        assert_eq!(sample.charging_voltage_v, Some(130.0));

        service.pending_request = Some(RequestKind::Compact);
        receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                legacy::PackageState::Ready,
                10.0,
            ))),
            2_000,
            9_000,
        );
        service.pending_request = Some(RequestKind::Realtime);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Dynamic(dynamic_sample(dynamic::PackageState::Ready, 2.0)),
            2_010,
            9_010,
        );
        let sample = published(&effects);
        assert_eq!(sample.odometer_m, Some(33.0));
        assert_eq!(sample.slow_observed_at_epoch_ms, Some(9_000));
    }

    #[test]
    fn ride_clock_pauses_for_idle_unknown_and_reconnect_and_resets_for_new_session() {
        let mut service = RefloatTelemetryService::default();
        start(&mut service, TelemetryFamily::LegacyFixed, Route::Direct, 0);
        let observe = |service: &mut RefloatTelemetryService, at, state| {
            service
                .apply(Command::Refresh {
                    service_generation: 7,
                    now_ms: at,
                })
                .unwrap();
            let effects = receive(
                service,
                DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                    state, 1.0,
                ))),
                at,
                at as i64,
            );
            published(&effects).clone()
        };
        use legacy::PackageState::{Ready, Running};
        assert_eq!(observe(&mut service, 100, Ready).riding_elapsed_ms, 0);
        assert_eq!(observe(&mut service, 10_000, Running).riding_elapsed_ms, 0);
        assert_eq!(
            observe(&mut service, 11_000, Running).riding_elapsed_ms,
            1_000
        );
        assert_eq!(
            observe(&mut service, 12_000, Ready).riding_elapsed_ms,
            1_000
        );
        assert_eq!(
            observe(&mut service, 20_000, Running).riding_elapsed_ms,
            1_000
        );
        assert_eq!(
            observe(&mut service, 21_000, Running).riding_elapsed_ms,
            2_000
        );
        service
            .apply(Command::Refresh {
                service_generation: 7,
                now_ms: 22_000,
            })
            .unwrap();
        let effects = receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::ActiveFirmwareFault(
                legacy::ActiveFirmwareFault {
                    raw_code: 255,
                    fault: legacy::FirmwareFault::Unknown,
                },
            )),
            22_000,
            22_000,
        );
        assert_eq!(published(&effects).ride_state, RideState::Unknown);
        assert_eq!(published(&effects).riding_elapsed_ms, 2_000);
        assert_eq!(
            observe(&mut service, 30_000, Running).riding_elapsed_ms,
            2_000
        );
        // Losing the connection preserves the total, never integrating the offline gap.
        service.apply(Command::ResetConnection).unwrap();
        start(
            &mut service,
            TelemetryFamily::LegacyFixed,
            Route::Direct,
            50_000,
        );
        let sample = observe(&mut service, 50_000, Running);
        assert_eq!(sample.riding_elapsed_ms, 2_000);
        assert_eq!(sample.elapsed_ms, 49_900);
        assert_eq!(
            observe(&mut service, 51_000, Running).riding_elapsed_ms,
            3_000
        );
        service.apply(Command::Reset).unwrap();
        start(
            &mut service,
            TelemetryFamily::LegacyFixed,
            Route::Direct,
            60_000,
        );
        assert_eq!(observe(&mut service, 60_000, Running).riding_elapsed_ms, 0);
    }

    #[test]
    fn dynamic_publication_preserves_every_realtime_field_and_extension() {
        let mut service = RefloatTelemetryService::default();
        start(&mut service, TelemetryFamily::Dynamic, Route::Direct, 10);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Dynamic(dynamic_sample(dynamic::PackageState::Running, 20.0)),
            30,
            1_234,
        );
        let sample = published(&effects);
        assert_eq!(sample.sequence, 1);
        assert_eq!(sample.observed_at_epoch_ms, 1_234);
        assert_eq!(sample.elapsed_ms, 0);
        assert_eq!(sample.ride_state, RideState::Riding);
        assert_eq!(sample.package_state, Some(PackageState::Running));
        assert_eq!(sample.package_mode, Some(PackageMode::Unknown));
        assert_eq!(sample.footpad_state, Some(FootpadState::Left));
        assert_eq!(sample.stop_condition, Some(StopCondition::Unknown));
        assert_eq!(sample.setpoint_adjustment, Some(SetpointAdjustment::Error));
        assert_eq!(sample.wheelslip, Some(true));
        assert_eq!(sample.darkride, Some(false));
        assert_eq!(sample.charging, Some(true));
        assert_eq!(sample.alert_reason, Some(AlertReason::BmsOverTemperature));
        assert_eq!(sample.firmware_fault, Some(FirmwareFault::PhaseFilter));
        assert_eq!(sample.fatal_error_latched, Some(true));
        assert_eq!(sample.speed_kph, Some(20.0));
        assert_eq!(sample.erpm, Some(21.0));
        assert_eq!(sample.motor_current_a, Some(22.0));
        assert_eq!(sample.directional_motor_current_a, Some(23.0));
        assert_eq!(sample.filtered_motor_current_a, Some(24.0));
        assert_eq!(sample.duty_cycle, Some(25.0));
        assert_eq!(sample.pack_voltage_v, Some(26.0));
        assert_eq!(sample.battery_current_a, Some(27.0));
        assert_eq!(sample.controller_temp_c, Some(28.0));
        assert_eq!(sample.motor_temp_c, Some(29.0));
        assert_eq!(sample.pitch_deg, Some(30.0));
        assert_eq!(sample.balance_pitch_deg, Some(31.0));
        assert_eq!(sample.roll_deg, Some(32.0));
        assert_eq!(sample.footpad_adc1_v, Some(33.0));
        assert_eq!(sample.footpad_adc2_v, Some(34.0));
        assert_eq!(sample.remote_input, Some(35.0));
        assert_eq!(sample.setpoint_deg, Some(36.0));
        assert_eq!(sample.atr_setpoint_deg, Some(37.0));
        assert_eq!(sample.brake_tilt_setpoint_deg, Some(38.0));
        assert_eq!(sample.torque_tilt_setpoint_deg, Some(39.0));
        assert_eq!(sample.turn_tilt_setpoint_deg, Some(40.0));
        assert_eq!(sample.remote_setpoint_deg, Some(41.0));
        assert_eq!(sample.requested_balance_current_a, Some(42.0));
        assert_eq!(
            sample.atr_acceleration_difference_erpm_per_second,
            Some(43.0)
        );
        assert_eq!(sample.atr_speed_boost, Some(44.0));
        assert_eq!(sample.booster_current_a, Some(45.0));
        assert_eq!(sample.extensions.len(), 1);
        assert_eq!(sample.extensions[0].name, "future");
        assert_eq!(sample.extensions[0].value, 48.0);
        assert!(matches!(
            effects.as_slice(),
            [
                Effect::Publish(_),
                Effect::CancelTimer(_),
                Effect::Enqueue {
                    kind: RequestKind::Realtime,
                    ..
                },
                Effect::ReplaceTimer {
                    interval_ms: 100,
                    ..
                }
            ]
        ));
    }

    #[test]
    fn ride_to_idle_uses_inclusive_post_ride_tail_then_idle() {
        let mut service = RefloatTelemetryService::default();
        let effects = start(&mut service, TelemetryFamily::LegacyFixed, Route::Direct, 0);
        let initial_poll = timer(&effects, TimerKind::Poll);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                legacy::PackageState::Running,
                1.0,
            ))),
            100,
            100,
        );
        assert!(matches!(effects[0], Effect::Publish(_)));
        assert_eq!(effects[1], Effect::CancelTimer(initial_poll));
        let fast_poll = timer(&effects, TimerKind::Poll);
        assert_eq!(service.snapshot().polling_interval_ms, Some(100));

        service.pending_request = Some(RequestKind::Legacy);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                legacy::PackageState::Ready,
                2.0,
            ))),
            200,
            200,
        );
        assert_eq!(service.snapshot().post_ride_probe_until_ms, Some(5_200));
        assert_eq!(service.snapshot().polling_interval_ms, Some(100));
        assert_eq!(effects.len(), 1);

        service.pending_request = Some(RequestKind::Legacy);
        receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                legacy::PackageState::Ready,
                3.0,
            ))),
            5_200,
            5_200,
        );
        assert_eq!(service.snapshot().polling_interval_ms, Some(100));

        service.pending_request = Some(RequestKind::Legacy);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                legacy::PackageState::Ready,
                4.0,
            ))),
            5_201,
            5_201,
        );
        assert_eq!(effects[1], Effect::CancelTimer(fast_poll));
        assert_eq!(service.snapshot().polling_interval_ms, Some(1_000));
    }

    #[test]
    fn mismatched_and_stale_input_do_not_advance_state() {
        let mut service = RefloatTelemetryService::default();
        start(&mut service, TelemetryFamily::Dynamic, Route::Direct, 0);
        let before = service.snapshot();
        assert_eq!(
            receive(
                &mut service,
                DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                    legacy::PackageState::Ready,
                    1.0,
                ))),
                10,
                10,
            ),
            [Effect::Diagnostic(Diagnostic::UnexpectedTelemetryResponse)]
        );
        assert_eq!(service.snapshot(), before);
        assert!(
            service
                .apply(Command::Receive {
                    service_generation: 6,
                    telemetry: Box::new(DecodedTelemetry::Dynamic(dynamic_sample(
                        dynamic::PackageState::Ready,
                        1.0,
                    ))),
                    now_ms: 10,
                    observed_at_epoch_ms: 10,
                })
                .unwrap()
                .is_empty()
        );
        assert_eq!(service.snapshot(), before);

        let stale_timer = TimerToken {
            kind: TimerKind::Poll,
            service_generation: 7,
            nonce: 999,
        };
        assert!(
            service
                .apply(Command::TimerFired {
                    token: stale_timer,
                    now_ms: 1_000,
                })
                .unwrap()
                .is_empty()
        );
        assert_eq!(service.snapshot(), before);
    }

    #[test]
    fn active_fault_publishes_fault_only_legacy_sample() {
        let mut service = RefloatTelemetryService::default();
        start(&mut service, TelemetryFamily::LegacyFixed, Route::Direct, 0);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::ActiveFirmwareFault(
                legacy::ActiveFirmwareFault {
                    raw_code: 25,
                    fault: legacy::FirmwareFault::EncoderNoMagnet,
                },
            )),
            1,
            2,
        );
        let sample = published(&effects);
        assert_eq!(sample.ride_state, RideState::Unknown);
        assert_eq!(sample.alert_reason, Some(AlertReason::FirmwareFault));
        assert_eq!(sample.firmware_fault, Some(FirmwareFault::EncoderNoMagnet));
        assert_eq!(sample.pack_voltage_v, None);
        assert_eq!(sample.slow_observed_at_epoch_ms, Some(2));
    }

    #[test]
    fn update_settings_restarts_in_native_effect_order() {
        let mut service = RefloatTelemetryService::default();
        let effects = start(&mut service, TelemetryFamily::LegacyFixed, Route::Direct, 0);
        let old_timer = timer(&effects, TimerKind::Poll);
        service
            .apply(Command::RequestCompleted {
                service_generation: 7,
                kind: RequestKind::Legacy,
                outcome: RequestOutcome::TimedOut,
            })
            .unwrap();
        let effects = service
            .apply(Command::UpdateSettings {
                service_generation: 7,
                settings: Settings::new(2_000, 10_000).unwrap(),
                now_ms: 100,
            })
            .unwrap();
        assert_eq!(effects[0], Effect::CancelTimer(old_timer));
        assert_eq!(enqueue(&effects).0, RequestKind::Legacy);
        assert!(matches!(
            effects[2],
            Effect::ReplaceTimer {
                interval_ms: 2_000,
                ..
            }
        ));
    }

    #[test]
    fn reset_cancels_poll_then_layout_and_clears_route_state() {
        let mut service = RefloatTelemetryService::default();
        let layout = service
            .apply(Command::BeginLayout {
                service_generation: 7,
                scheduler_generation: Generation(11),
                route: Route::Direct,
            })
            .unwrap();
        let layout_timer = timer(&layout, TimerKind::LayoutRetry);
        let polling = start(&mut service, TelemetryFamily::Dynamic, Route::Direct, 0);
        let poll_timer = timer(&polling, TimerKind::Poll);
        service.started_at_ms = Some(100);
        service.sequence = 510;
        assert_eq!(
            service.apply(Command::ResetConnection).unwrap(),
            [
                Effect::CancelTimer(poll_timer),
                Effect::CancelTimer(layout_timer)
            ]
        );
        let mut retained = RefloatTelemetryService::default().snapshot();
        retained.sequence = 510;
        assert_eq!(service.snapshot(), retained);
        assert_eq!(service.started_at_ms, Some(100));
        assert!(service.apply(Command::Reset).unwrap().is_empty());
        assert_eq!(service.started_at_ms, None);
        assert_eq!(
            service.snapshot(),
            RefloatTelemetryService::default().snapshot()
        );
    }

    #[test]
    fn time_overflow_is_atomic_and_elapsed_clamps_to_zero() {
        let mut service = RefloatTelemetryService::default();
        let before = service.snapshot();
        assert_eq!(
            service.apply(Command::Start {
                configuration: configuration(TelemetryFamily::Dynamic, Route::Direct),
                now_ms: u64::MAX,
            }),
            Err(ServiceError::MonotonicOverflow)
        );
        assert_eq!(service.snapshot(), before);

        start(
            &mut service,
            TelemetryFamily::LegacyFixed,
            Route::Direct,
            100,
        );
        service
            .apply(Command::MarkSessionStarted {
                service_generation: 7,
                now_ms: 500,
            })
            .unwrap();
        let effects = receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                legacy::PackageState::Ready,
                1.0,
            ))),
            400,
            1,
        );
        assert_eq!(published(&effects).elapsed_ms, 0);
    }

    #[test]
    fn post_ride_overflow_rejects_receive_atomically() {
        let mut service = RefloatTelemetryService::default();
        start(&mut service, TelemetryFamily::LegacyFixed, Route::Direct, 0);
        receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                legacy::PackageState::Running,
                1.0,
            ))),
            10,
            10,
        );
        service.pending_request = Some(RequestKind::Legacy);
        let before = service.snapshot();
        assert_eq!(
            service.apply(Command::Receive {
                service_generation: 7,
                telemetry: Box::new(DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(
                    legacy_sample(legacy::PackageState::Ready, 2.0),
                ))),
                now_ms: u64::MAX,
                observed_at_epoch_ms: 11,
            }),
            Err(ServiceError::MonotonicOverflow)
        );
        assert_eq!(service.snapshot(), before);
    }

    #[test]
    fn malformed_decoded_values_are_rejected_atomically() {
        let mut service = RefloatTelemetryService::default();
        start(&mut service, TelemetryFamily::Dynamic, Route::Direct, 0);
        let before = service.snapshot();
        let mut sample = dynamic_sample(dynamic::PackageState::Ready, 1.0);
        sample.values.speed_kph = Some(f64::NAN);
        assert_eq!(
            service.apply(Command::Receive {
                service_generation: 7,
                telemetry: Box::new(DecodedTelemetry::Dynamic(sample)),
                now_ms: 1,
                observed_at_epoch_ms: 1,
            }),
            Err(ServiceError::InvalidTelemetry)
        );
        assert_eq!(service.snapshot(), before);
    }

    #[test]
    fn cadence_restart_overflow_is_atomic() {
        let mut service = RefloatTelemetryService::default();
        start(&mut service, TelemetryFamily::Dynamic, Route::Direct, 0);
        service.pending_request = None;
        service.next_compact_due_ms = Some(1);
        let before = service.snapshot();
        assert_eq!(
            service.apply(Command::UpdateSettings {
                service_generation: 7,
                settings: Settings::new(2_000, 5_000).unwrap(),
                now_ms: u64::MAX,
            }),
            Err(ServiceError::MonotonicOverflow)
        );
        assert_eq!(service.snapshot(), before);
        assert_eq!(service.settings, settings());
    }

    #[test]
    fn counters_wrap_without_reusing_timer_identity() {
        let mut service = RefloatTelemetryService {
            sequence: u64::MAX,
            next_timer_nonce: u64::MAX,
            ..RefloatTelemetryService::default()
        };
        let effects = start(&mut service, TelemetryFamily::LegacyFixed, Route::Direct, 0);
        let first = timer(&effects, TimerKind::Poll);
        assert_eq!(first.nonce, 0);
        let effects = receive(
            &mut service,
            DecodedTelemetry::Legacy(legacy::LegacyTelemetry::Sample(legacy_sample(
                legacy::PackageState::Ready,
                1.0,
            ))),
            1,
            1,
        );
        assert_eq!(published(&effects).sequence, 0);
    }

    #[test]
    fn debug_output_redacts_telemetry_values() {
        let record = dynamic_sample(dynamic::PackageState::Ready, 123_456.0);
        assert!(!format!("{record:?}").contains("123456"));
        let common = common_dynamic_sample(record).unwrap();
        let service = RefloatTelemetryService::default();
        let sample = service.normalized_sample(common, 1, 1, 1);
        assert!(!format!("{sample:?}").contains("123456"));
    }

    #[test]
    fn semantic_maps_cover_every_source_variant() {
        assert_eq!(
            map_legacy_setpoint_adjustment(legacy::SetpointAdjustment::Centering),
            SetpointAdjustment::Centering
        );
        assert_eq!(
            map_dynamic_setpoint_adjustment(dynamic::SetpointAdjustment::Speed),
            SetpointAdjustment::Speed
        );
        assert_eq!(
            map_dynamic_alert_reason(dynamic::AlertReason::CellImbalance),
            AlertReason::CellImbalance
        );
        assert_eq!(
            map_dynamic_alert_reason(dynamic::AlertReason::BmsConnection),
            AlertReason::BmsConnection
        );
        assert_eq!(
            map_legacy_firmware_fault(legacy::FirmwareFault::Unknown),
            FirmwareFault::Unknown
        );
        assert_eq!(
            map_dynamic_firmware_fault(dynamic::FirmwareFault::Unknown),
            FirmwareFault::Unknown
        );
    }
}
map_same_variants!(
    map_dynamic_stop_condition,
    dynamic::StopCondition,
    StopCondition,
    [
        None,
        Pitch,
        Roll,
        SwitchHalf,
        SwitchFull,
        ReverseStop,
        QuickStop,
        Unknown
    ]
);
map_same_variants!(
    map_legacy_setpoint_adjustment,
    legacy::SetpointAdjustment,
    SetpointAdjustment,
    [
        Centering,
        ReverseStop,
        None,
        Duty,
        HighVoltage,
        LowVoltage,
        Temperature,
        Unknown
    ]
);
map_same_variants!(
    map_dynamic_setpoint_adjustment,
    dynamic::SetpointAdjustment,
    SetpointAdjustment,
    [
        None,
        Centering,
        ReverseStop,
        Speed,
        Duty,
        Error,
        HighVoltage,
        LowVoltage,
        Temperature,
        Unknown
    ]
);
map_same_variants!(
    map_legacy_alert_reason,
    legacy::AlertReason,
    AlertReason,
    [
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
        Unknown
    ]
);
map_same_variants!(
    map_dynamic_alert_reason,
    dynamic::AlertReason,
    AlertReason,
    [
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
        Unknown
    ]
);
map_same_variants!(
    map_legacy_firmware_fault,
    legacy::FirmwareFault,
    FirmwareFault,
    [
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
        Unknown
    ]
);
map_same_variants!(
    map_dynamic_firmware_fault,
    dynamic::FirmwareFault,
    FirmwareFault,
    [
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
        Unknown
    ]
);
