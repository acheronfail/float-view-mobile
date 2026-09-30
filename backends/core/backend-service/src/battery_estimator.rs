//! Deterministic cell-voltage estimation and bounded current filtering.
use float_view_product_store::battery_catalog::{CATALOG_VERSION, Curve, profile};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    LoadCompensated,
    LoadClamped,
    VoltageOnly,
    Unavailable,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    NotConfigured,
    SeriesMismatch,
    MissingVoltage,
    NotApplicable,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub level: Option<f64>,
    pub profile_id: Option<String>,
    pub series_count: Option<u8>,
    pub sample_sequence: Option<u64>,
    pub profile_revision: Option<u64>,
    pub catalog_version: String,
    pub mode: Mode,
    pub unavailable_reason: Option<UnavailableReason>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Configuration {
    pub profile_id: Option<String>,
    pub series: Option<u8>,
    pub parallel: Option<u8>,
    pub minimum_v: Option<f64>,
}
fn inverse(volts: &[f64; 11], v: f64) -> f64 {
    if v >= volts[0] {
        return 1.;
    }
    if v <= volts[10] {
        return 0.;
    }
    for i in 0..10 {
        if v >= volts[i + 1] {
            return 1. - (i as f64 + (volts[i] - v) / (volts[i] - volts[i + 1])) / 10.;
        }
    }
    0.
}
fn voltage(volts: &[f64; 11], q: f64) -> f64 {
    let index = ((1. - q).clamp(0., 1.) * 10.).min(10.);
    let lo = (index.floor() as usize).min(9);
    volts[lo] + (volts[lo + 1] - volts[lo]) * (index - lo as f64)
}
fn adjusted(curve: &Curve, minimum: f64) -> [f64; 11] {
    let q_min = inverse(&curve.volts, minimum);
    let mut volts = std::array::from_fn(|i| {
        voltage(&curve.volts, q_min + (1. - q_min) * (1. - i as f64 / 10.))
    });
    volts[0] = 4.2;
    volts[10] = minimum;
    volts
}
impl Estimate {
    pub fn unavailable(reason: UnavailableReason) -> Self {
        Self {
            level: None,
            profile_id: None,
            series_count: None,
            sample_sequence: None,
            profile_revision: None,
            catalog_version: CATALOG_VERSION.into(),
            mode: Mode::Unavailable,
            unavailable_reason: Some(reason),
        }
    }
}
pub fn estimate(
    config: &Configuration,
    revision: Option<u64>,
    sequence: Option<u64>,
    applicable: bool,
    controller_series: Option<u8>,
    pack_voltage: Option<f64>,
    current: Option<f64>,
) -> Estimate {
    let mut result = Estimate::unavailable(UnavailableReason::NotApplicable);
    result.sample_sequence = sequence;
    result.profile_revision = revision;
    if !applicable {
        return result;
    }
    result.profile_id = config.profile_id.clone();
    result.series_count = config.series.filter(|v| (1..=36).contains(v));
    result.unavailable_reason = Some(UnavailableReason::NotConfigured);
    let Some(profile) = config.profile_id.as_deref().and_then(profile) else {
        return result;
    };
    let (Some(series), Some(parallel)) = (
        result.series_count,
        config.parallel.filter(|v| (1..=20).contains(v)),
    ) else {
        return result;
    };
    let minimum = config.minimum_v.unwrap_or(3.0);
    if !minimum.is_finite() || !(2.7..4.2).contains(&minimum) {
        return result;
    }
    if controller_series.is_some_and(|s| s != series) {
        result.series_count = None;
        result.unavailable_reason = Some(UnavailableReason::SeriesMismatch);
        return result;
    }
    let Some(v) = pack_voltage.filter(|v| v.is_finite() && *v > 0.) else {
        result.unavailable_reason = Some(UnavailableReason::MissingVoltage);
        return result;
    };
    let amps = current.filter(|a| a.is_finite() && *a > 0.).unwrap_or(0.) / f64::from(parallel);
    let mut knots = adjusted(&profile.curves[0], minimum);
    result.mode = Mode::VoltageOnly;
    if amps > 0. && profile.curves.len() > 1 {
        let last = profile.curves.last().unwrap();
        result.mode = if amps > last.current_a {
            Mode::LoadClamped
        } else {
            Mode::LoadCompensated
        };
        for pair in profile.curves.windows(2) {
            let fraction = ((amps - pair[0].current_a) / (pair[1].current_a - pair[0].current_a))
                .clamp(0., 1.);
            let low = adjusted(&pair[0], minimum);
            let high = adjusted(&pair[1], minimum);
            knots = std::array::from_fn(|i| low[i] + fraction * (high[i] - low[i]));
            if amps <= pair[1].current_a {
                break;
            }
        }
    }
    result.level = Some(inverse(&knots, v / f64::from(series)).clamp(0., 1.));
    result.unavailable_reason = None;
    result
}

/// Constant-size filter. Call only for accepted, persistence-gated telemetry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurrentFilter {
    last: Option<(u64, u64)>,
    current: Option<f64>,
}
impl CurrentFilter {
    pub fn accept(&mut self, sequence: u64, monotonic_ms: u64, current: Option<f64>) -> bool {
        if self
            .last
            .is_some_and(|(s, t)| sequence <= s || monotonic_ms < t)
        {
            return false;
        }
        let value = current.filter(|v| v.is_finite() && *v > 0.);
        self.current = match (self.last, self.current, value) {
            (Some((_, time)), Some(old), Some(new)) if monotonic_ms - time <= 5_000 => {
                Some(old + (1. - (-((monotonic_ms - time) as f64) / 2_000.).exp()) * (new - old))
            }
            (_, _, new) => new,
        };
        self.last = Some((sequence, monotonic_ms));
        true
    }
    pub fn current(&self) -> Option<f64> {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_view_product_store::battery_catalog::PROFILES;
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12, "{a} != {b}");
    }
    fn config() -> Configuration {
        Configuration {
            profile_id: Some("molicel-p45b".into()),
            series: Some(20),
            parallel: Some(2),
            minimum_v: Some(2.7),
        }
    }
    fn calc(c: &Configuration, v: f64, a: Option<f64>) -> Estimate {
        estimate(c, Some(1), Some(1), true, Some(20), Some(v), a)
    }
    #[test]
    fn pinned_catalog_matches_every_source_knot() {
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../product-store/tests/battery-profile-data.json"
        ))
        .unwrap();
        assert_eq!(source["catalogVersion"], CATALOG_VERSION);
        assert_eq!(PROFILES.len(), 7);
        for (p, original) in PROFILES.iter().zip(source["profiles"].as_array().unwrap()) {
            assert_eq!(p.id, original["id"]);
            assert_eq!(p.label, original["label"]);
            assert_eq!(p.curves.len(), original["curves"].as_array().unwrap().len());
            assert_eq!(p.curves[0].current_a, 0.);
            for (c, data) in p.curves.iter().zip(original["curves"].as_array().unwrap()) {
                assert_eq!(c.current_a, data["currentA"].as_f64().unwrap());
                assert!(c.current_a.is_finite());
                for (v, expected) in c.volts.iter().zip(data["volts"].as_array().unwrap()) {
                    assert_eq!(*v, expected.as_f64().unwrap());
                    assert!(v.is_finite());
                }
                assert!(c.volts.windows(2).all(|p| p[0] > p[1]));
            }
            assert!(p.curves.windows(2).all(|c| c[0].current_a < c[1].current_a));
        }
    }
    #[test]
    fn independent_oracles_and_modes() {
        let mut c = config();
        near(calc(&c, 74.6, None).level.unwrap(), 0.5);
        near(calc(&c, 72.1, Some(20.)).level.unwrap(), 0.5);
        c.minimum_v = Some(3.23);
        near(calc(&c, 75.5, Some(0.)).level.unwrap(), 0.5);
        for amps in [None, Some(0.), Some(-30.), Some(f64::NAN)] {
            assert_eq!(calc(&c, 75.5, amps).mode, Mode::VoltageOnly);
        }
        assert_eq!(calc(&c, 75.5, Some(500.)).mode, Mode::LoadClamped);
        for v in [0., -10., f64::NAN, f64::INFINITY] {
            assert_eq!(
                calc(&c, v, None).unavailable_reason,
                Some(UnavailableReason::MissingVoltage)
            );
        }
        assert_eq!(
            estimate(&c, None, None, true, Some(18), Some(75.), None).unavailable_reason,
            Some(UnavailableReason::SeriesMismatch)
        );
        assert_eq!(
            estimate(&c, None, None, false, None, None, None).unavailable_reason,
            Some(UnavailableReason::NotApplicable)
        );
        assert_eq!(
            calc(&Configuration::default(), 75., None).unavailable_reason,
            Some(UnavailableReason::NotConfigured)
        );
    }
    #[test]
    fn all_profiles_knots_midpoints_endpoints_and_load_clamp() {
        for p in PROFILES {
            let c = Configuration {
                profile_id: Some(p.id.into()),
                ..config()
            };
            for curve in p.curves {
                for (i, v) in curve.volts.iter().enumerate() {
                    near(
                        calc(&c, v * 20., Some(curve.current_a * 2.)).level.unwrap(),
                        1. - i as f64 / 10.,
                    );
                }
                for (i, pair) in curve.volts.windows(2).enumerate() {
                    near(
                        calc(&c, (pair[0] + pair[1]) * 10., Some(curve.current_a * 2.))
                            .level
                            .unwrap(),
                        0.95 - i as f64 / 10.,
                    );
                }
            }
            for minimum in [2.7, 3., 3.23, 4.19] {
                let c = Configuration {
                    minimum_v: Some(minimum),
                    ..c.clone()
                };
                assert_eq!(calc(&c, minimum * 20. - 0.01, Some(1000.)).level, Some(0.));
                assert_eq!(calc(&c, 84.1, Some(1000.)).level, Some(1.));
            }
        }
    }
    #[test]
    fn minimum_just_below_full_remains_an_exact_empty_endpoint() {
        let minimum = f64::from_bits(4.2_f64.to_bits() - 1);
        let c = Configuration {
            series: Some(1),
            minimum_v: Some(minimum),
            ..config()
        };
        assert_eq!(
            estimate(&c, Some(1), Some(1), true, None, Some(minimum), None).level,
            Some(0.)
        );
        assert_eq!(
            estimate(&c, Some(1), Some(1), true, None, Some(4.2), None).level,
            Some(1.)
        );
    }
    #[test]
    fn filter_steps_order_gaps_and_regeneration() {
        let mut f = CurrentFilter::default();
        assert!(f.accept(1, 0, Some(10.)));
        near(f.current().unwrap(), 10.);
        assert!(f.accept(2, 2000, Some(30.)));
        near(f.current().unwrap(), 10. + 20. * (1. - (-1_f64).exp()));
        let before = f.clone();
        assert!(!f.accept(2, 3000, Some(50.)));
        assert!(!f.accept(3, 1000, Some(50.)));
        assert_eq!(f, before);
        assert!(f.accept(3, 7001, Some(50.)));
        near(f.current().unwrap(), 50.);
        assert!(f.accept(4, 7002, Some(-5.)));
        assert_eq!(f.current(), None);
        assert!(f.accept(5, 7003, Some(20.)));
        near(f.current().unwrap(), 20.);
        assert!(f.accept(6, 7004, None));
        assert_eq!(f.current(), None);
    }
}
