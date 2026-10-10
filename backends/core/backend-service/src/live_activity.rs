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

// Keep tie order, signed-limit selection and filtered-current preference aligned
// with frontend/src/lib/features/connection/gauge.ts.
fn drive_load(
    duty: Option<f64>,
    motor: Option<f64>,
    battery: Option<f64>,
) -> (Option<&'static str>, Option<f64>) {
    let mut winner = (None, None);
    for (source, value) in [("duty", duty), ("motor", motor), ("battery", battery)] {
        if let Some(value) = value.filter(|v| v.is_finite()) {
            let percent = value.abs().min(1.) * 100.;
            if winner.1.is_none_or(|previous| percent > previous) {
                winner = (Some(source), Some(percent));
            }
        }
    }
    winner
}
fn ratio(
    value: Option<f64>,
    signed_current: Option<f64>,
    limits: Option<(f64, f64)>,
) -> Option<f64> {
    let (max, min) = limits?;
    let limit = if signed_current? < 0. { min } else { max };
    (limit != 0.).then(|| value.map(|v| v / limit)).flatten()
}
impl BackendService {
    pub(crate) fn live_activity(&self) -> Option<LiveActivity> {
        let active = self.policy.active.as_ref()?;
        let runtime = self.runtime.snapshot().application;
        if active.state != LinkState::Ready
            || active.reconnect_started.is_some()
            || runtime.phase != Phase::Ready
            || runtime.standalone_bms
        {
            return None;
        }
        let sample = active.latest.as_ref();
        let limits = runtime.current_limits.as_ref();
        let (source, percentage) = sample
            .map(|s| {
                drive_load(
                    s.duty_cycle,
                    ratio(
                        s.filtered_motor_current_a
                            .or(s.directional_motor_current_a)
                            .or(s.motor_current_a),
                        s.motor_current_a,
                        limits.map(|l| (l.motor_current_max_a, l.motor_current_min_a)),
                    ),
                    ratio(
                        s.battery_current_a,
                        s.battery_current_a,
                        limits.map(|l| (l.battery_current_max_a, l.battery_current_min_a)),
                    ),
                )
            })
            .unwrap_or((None, None));
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
            generation: active.generation,
            observed_at: sample.map(|s| s.observed_at_epoch_ms),
            source: source.map(str::to_owned),
            percentage,
            distance_meters: sample.and_then(|s| s.distance_m),
            battery_percent: level.map(|v| (v * 100.).round()),
            battery_voltage: sample
                .and_then(|s| s.pack_voltage_v)
                .filter(|v| v.is_finite()),
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
    fn gauge_winner_ties_clamping_and_missing_values() {
        assert_eq!(drive_load(None, None, None), (None, None));
        assert_eq!(
            drive_load(Some(-0.5), Some(0.5), Some(0.5)),
            (Some("duty"), Some(50.))
        );
        assert_eq!(
            drive_load(Some(0.1), Some(-0.8), Some(0.2)),
            (Some("motor"), Some(80.))
        );
        assert_eq!(
            drive_load(Some(0.1), Some(0.2), Some(2.)),
            (Some("battery"), Some(100.))
        );
        assert_eq!(ratio(Some(20.), Some(-5.), Some((80., -40.))), Some(-0.5));
        assert_eq!(ratio(Some(20.), None, Some((80., -40.))), None);
        assert_eq!(ratio(Some(20.), Some(5.), Some((0., -40.))), None);
    }
}
