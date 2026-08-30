use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use float_view_refloat_telemetry_service::{
    AlertReason, ExtensionValue, FirmwareFault, FootpadState, NormalizedSample, PackageMode,
    PackageState, RideState, SetpointAdjustment, StopCondition,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct TestDirectory(PathBuf);

impl TestDirectory {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "float-view-session-writer-{}-{}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create isolated test directory");
        Self(path)
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove isolated test directory");
    }
}

pub fn sample(sequence: u64, elapsed_ms: u64) -> NormalizedSample {
    NormalizedSample {
        sequence,
        observed_at_epoch_ms: 1_000_000 + elapsed_ms as i64,
        slow_observed_at_epoch_ms: None,
        elapsed_ms,
        ride_state: RideState::Idle,
        package_state: None,
        package_mode: None,
        footpad_state: None,
        stop_condition: None,
        setpoint_adjustment: None,
        wheelslip: None,
        darkride: None,
        charging: None,
        alert_reason: None,
        firmware_fault: None,
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

#[allow(dead_code)]
fn keep_domain_variants_linked() {
    let _ = (
        AlertReason::None,
        FirmwareFault::Unknown,
        FootpadState::None,
        PackageMode::Normal,
        PackageState::Ready,
        SetpointAdjustment::None,
        StopCondition::None,
        ExtensionValue {
            name: String::new(),
            value: 0.0,
        },
    );
}
