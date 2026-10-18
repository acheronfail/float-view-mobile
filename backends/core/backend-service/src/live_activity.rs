//! Small native presentation projection. No transport or ActivityKit ownership.
use crate::{BackendService, battery_estimator::UnavailableReason, os::LinkState};
use float_view_application_runtime::coordinator::Phase;
use float_view_application_runtime::telemetry::{FootpadState, PackageState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveActivity {
    pub generation: u64,
    pub observed_at: Option<i64>,
    pub source: Option<String>,
    pub percentage: Option<f64>,
    pub distance_meters: Option<f64>,
    pub battery_percent: Option<f64>,
    pub battery_voltage: Option<f64>,
    pub highest_temperature_c: Option<f64>,
    pub temperature_source: Option<String>,
    pub imperial: bool,
    pub status: Option<String>,
    pub footpad_left: FootpadIndicator,
    pub footpad_right: FootpadIndicator,
}

/// Shared semantic colours; native hosts select their theme's palette.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FootpadIndicator {
    Neutral,
    Active,
    Warning,
}

fn footpads(
    state: Option<FootpadState>,
    speed: Option<f64>,
) -> (FootpadIndicator, FootpadIndicator) {
    let inactive = if speed.is_some_and(|speed| speed > 2.) {
        FootpadIndicator::Warning
    } else {
        FootpadIndicator::Neutral
    };
    let left = matches!(state, Some(FootpadState::Left | FootpadState::Both));
    let right = matches!(state, Some(FootpadState::Right | FootpadState::Both));
    (
        if left {
            FootpadIndicator::Active
        } else {
            inactive
        },
        if right {
            FootpadIndicator::Active
        } else {
            inactive
        },
    )
}

fn status(state: Option<PackageState>) -> Option<String> {
    match state {
        Some(PackageState::Ready) => Some("Stopped".into()),
        Some(PackageState::Disabled) => Some("Disabled".into()),
        Some(PackageState::Startup) => Some("Starting".into()),
        _ => None,
    }
}

// Match the frontend highest-temperature widget, including stable tie order.
fn highest_temperature(values: [Option<f64>; 5]) -> (Option<&'static str>, Option<f64>) {
    let mut winner = (None, None);
    for (source, value) in ["ESC", "Motor", "Battery", "BMS", "Humidity"]
        .into_iter()
        .zip(values)
    {
        if let Some(value) = value.filter(|v| v.is_finite())
            && winner.1.is_none_or(|previous| value > previous)
        {
            winner = (Some(source), Some(value));
        }
    }
    winner
}

fn fresh_temperature_observation(observed: i64, now: i64) -> bool {
    now.checked_sub(observed)
        .is_some_and(|age| (0..5_000).contains(&age))
}

impl BackendService {
    pub(crate) fn live_activity(&self) -> Option<LiveActivity> {
        let active = self.policy.active.as_ref()?;
        let generation = active.live_activity_generation?;
        let runtime = self.runtime.snapshot().application;
        if runtime.standalone_bms {
            return None;
        }
        if active.state != LinkState::Ready
            || active.reconnect_started.is_some()
            || runtime.phase != Phase::Ready
        {
            // Never present retained telemetry as current while reconnecting.
            return Some(LiveActivity {
                generation,
                observed_at: None,
                source: None,
                percentage: None,
                distance_meters: None,
                battery_percent: None,
                battery_voltage: None,
                highest_temperature_c: None,
                temperature_source: None,
                imperial: self.policy.settings.unit_preference == "imperial",
                status: Some("Reconnecting".into()),
                footpad_left: FootpadIndicator::Neutral,
                footpad_right: FootpadIndicator::Neutral,
            });
        }
        let sample = active.latest.as_ref();
        let bms = active.bms.as_ref().filter(|bms| {
            runtime.bms.status == float_view_application_runtime::coordinator::bms::Status::Attached
                && runtime.bms.route == Some(bms.route)
                && fresh_temperature_observation(
                    bms.observed_at_epoch_ms,
                    self.policy.exchange.epoch,
                )
        });
        let (temperature_source, highest_temperature_c) = highest_temperature([
            sample.and_then(|s| s.controller_temp_c),
            sample.and_then(|s| s.motor_temp_c),
            bms.map(|b| b.highest_cell_temperature_c),
            bms.map(|b| b.highest_circuitry_temperature_c()),
            bms.map(|b| b.humidity_sensor_temperature_c),
        ]);
        let winner = sample.and(active.latest_load.winner);
        let source = winner.map(|w| match w.source {
            float_view_application_runtime::load::Source::Duty => "duty",
            float_view_application_runtime::load::Source::Motor => "motor",
            float_view_application_runtime::load::Source::Battery => "battery",
        });
        let percentage = winner.map(|w| w.fraction * 100.);
        let estimate = &active.battery.estimate;
        let level = estimate.level.or_else(|| {
            if active.saved && estimate.unavailable_reason == Some(UnavailableReason::NotConfigured)
            {
                sample
                    .and_then(|s| s.battery_level)
                    .filter(|v| v.is_finite() && (0. ..=1.).contains(v))
            } else {
                None
            }
        });
        let (footpad_left, footpad_right) = footpads(
            sample.and_then(|s| s.footpad_state),
            sample.and_then(|s| s.speed_kph),
        );
        Some(LiveActivity {
            status: status(sample.and_then(|s| s.package_state)),
            footpad_left,
            footpad_right,
            generation,
            observed_at: sample.map(|s| s.observed_at_epoch_ms),
            source: source.map(str::to_owned),
            percentage,
            distance_meters: sample.and_then(|s| s.distance_m),
            battery_percent: level.map(|v| (v * 100.).round()),
            battery_voltage: sample
                .and_then(|s| s.pack_voltage_v)
                .filter(|v| v.is_finite()),
            highest_temperature_c,
            temperature_source: temperature_source.map(str::to_owned),
            imperial: self.policy.settings.unit_preference == "imperial",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn footpads_match_frontend_colours_and_status_uses_package_state() {
        use FootpadIndicator::*;
        assert_eq!(
            footpads(Some(FootpadState::Both), Some(10.)),
            (Active, Active)
        );
        assert_eq!(
            footpads(Some(FootpadState::Left), Some(2.)),
            (Active, Neutral)
        );
        assert_eq!(
            footpads(Some(FootpadState::Right), Some(2.1)),
            (Warning, Active)
        );
        assert_eq!(
            footpads(Some(FootpadState::None), Some(3.)),
            (Warning, Warning)
        );
        assert_eq!(footpads(None, None), (Neutral, Neutral));
        assert_eq!(
            footpads(Some(FootpadState::Unknown), Some(-3.)),
            (Neutral, Neutral)
        );
        assert_eq!(
            status(Some(PackageState::Ready)).as_deref(),
            Some("Stopped")
        );
        assert_eq!(
            status(Some(PackageState::Disabled)).as_deref(),
            Some("Disabled")
        );
        assert_eq!(
            status(Some(PackageState::Startup)).as_deref(),
            Some("Starting")
        );
        assert_eq!(status(Some(PackageState::Running)), None);
        assert_eq!(status(None), None);
    }

    #[test]
    fn highest_temperature_handles_missing_nonfinite_negative_and_tied_sensors() {
        assert_eq!(highest_temperature([None; 5]), (None, None));
        assert_eq!(
            highest_temperature([Some(f64::NAN), Some(-5.), None, Some(-10.), None]),
            (Some("Motor"), Some(-5.))
        );
        assert_eq!(
            highest_temperature([Some(40.), Some(40.), Some(30.), None, None]),
            (Some("ESC"), Some(40.))
        );
        for (index, source) in ["ESC", "Motor", "Battery", "BMS", "Humidity"]
            .into_iter()
            .enumerate()
        {
            let mut values = [Some(20.); 5];
            values[index] = Some(70.);
            assert_eq!(highest_temperature(values), (Some(source), Some(70.)));
        }
        assert!(fresh_temperature_observation(100, 5099));
        assert!(!fresh_temperature_observation(100, 5100));
        assert!(!fresh_temperature_observation(100, 99));
    }
}
