//! Canonical board utilisation, shared by capture and every live consumer.
use crate::{coordinator::controller_data::CurrentLimits, telemetry::NormalizedSample};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Load {
    pub duty: Option<f64>,
    pub motor: Option<f64>,
    pub battery: Option<f64>,
    pub winner: Option<Winner>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Winner {
    pub source: Source,
    pub fraction: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Duty,
    Motor,
    Battery,
}
fn magnitude(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite()).map(|v| v.abs().min(1.))
}
fn ratio(value: Option<f64>, signed: Option<f64>, limits: Option<(f64, f64)>) -> Option<f64> {
    let signed = signed.filter(|v| v.is_finite())?;
    let (max, min) = limits?;
    let limit = if signed < 0. { min } else { max };
    if !limit.is_finite() || limit == 0. {
        return None;
    }
    let value = value.filter(|v| v.is_finite())?;
    Some((value.abs() / limit.abs()).min(1.))
}
pub fn calculate(s: &NormalizedSample, limits: Option<&CurrentLimits>) -> Load {
    let duty = magnitude(s.duty_cycle);
    let motor = ratio(
        s.filtered_motor_current_a
            .or(s.directional_motor_current_a)
            .or(s.motor_current_a),
        s.motor_current_a,
        limits.map(|l| (l.motor_current_max_a, l.motor_current_min_a)),
    );
    let battery = ratio(
        s.battery_current_a,
        s.battery_current_a,
        limits.map(|l| (l.battery_current_max_a, l.battery_current_min_a)),
    );
    let mut winner: Option<Winner> = None;
    for (source, value) in [
        (Source::Duty, duty),
        (Source::Motor, motor),
        (Source::Battery, battery),
    ] {
        if let Some(fraction) = value
            && winner.is_none_or(|w| fraction > w.fraction)
        {
            winner = Some(Winner { source, fraction });
        }
    }
    Load {
        duty,
        motor,
        battery,
        winner,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_finite_ratios() {
        assert_eq!(ratio(Some(-60.), Some(-1.), Some((100., -50.))), Some(1.));
        assert_eq!(ratio(Some(60.), Some(1.), Some((100., -50.))), Some(0.6));
        assert_eq!(ratio(Some(60.), Some(1.), Some((0., -50.))), None);
        assert_eq!(ratio(Some(f64::NAN), Some(1.), Some((100., -50.))), None);
        assert_eq!(ratio(Some(1.), Some(f64::NAN), Some((100., -50.))), None);
        assert_eq!(magnitude(Some(-2.)), Some(1.));
    }
}
