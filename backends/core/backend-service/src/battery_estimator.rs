//! Coherent voltage observations and bounded, process-owned temporal charge tracking.
use float_view_product_store::battery_catalog::{CATALOG_VERSION, profile};
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
    pub method: Option<Method>,
    pub provisional: bool,
    pub unavailable_reason: Option<UnavailableReason>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Configuration {
    pub profile_id: Option<String>,
    pub series: Option<u8>,
    pub parallel: Option<u8>,
    pub minimum_v: Option<f64>,
    pub capacity_ah: Option<f64>,
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
            method: None,
            provisional: false,
            unavailable_reason: Some(reason),
        }
    }
}
pub fn observe(
    config: &Configuration,
    revision: Option<u64>,
    sequence: Option<u64>,
    applicable: bool,
    controller_series: Option<u8>,
    pack_voltage: Option<f64>,
    current: Option<f64>,
) -> (Estimate, Option<Observation>) {
    let mut result = Estimate::unavailable(UnavailableReason::NotApplicable);
    result.sample_sequence = sequence;
    result.profile_revision = revision;
    if !applicable {
        return (result, None);
    }
    result.profile_id = config.profile_id.clone();
    result.series_count = config.series.filter(|v| (1..=36).contains(v));
    result.unavailable_reason = Some(UnavailableReason::NotConfigured);
    let Some(profile) = config.profile_id.as_deref().and_then(profile) else {
        return (result, None);
    };
    let (Some(series), Some(parallel)) = (
        result.series_count,
        config.parallel.filter(|v| (1..=20).contains(v)),
    ) else {
        return (result, None);
    };
    let minimum = config.minimum_v.unwrap_or(3.0);
    if !minimum.is_finite() || !(2.7..4.2).contains(&minimum) {
        return (result, None);
    }
    if controller_series.is_some_and(|s| s != series) {
        result.series_count = None;
        result.unavailable_reason = Some(UnavailableReason::SeriesMismatch);
        return (result, None);
    }
    let Some(v) = pack_voltage.filter(|v| v.is_finite() && *v > 0.) else {
        result.unavailable_reason = Some(UnavailableReason::MissingVoltage);
        return (result, None);
    };
    let amps = current.filter(|a| a.is_finite() && *a > 0.).unwrap_or(0.) / f64::from(parallel);
    let mut knots = profile.curves[0].volts;
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
            let low = pair[0].volts;
            let high = pair[1].volts;
            knots = std::array::from_fn(|i| low[i] + fraction * (high[i] - low[i]));
            if amps <= pair[1].current_a {
                break;
            }
        }
    }
    let cell_v = v / f64::from(series);
    // Keep a positive representable reserve denominator even at the largest valid minimum.
    let empty = inverse(&profile.curves[0].volts, minimum).min(1. - f64::EPSILON);
    let mut charge = inverse(&knots, cell_v);
    if amps == 0. && cell_v <= minimum {
        charge = charge.min(empty);
    }
    result.level = Some(display(charge, empty));
    result.method = Some(Method::VoltageSmoothed);
    result.provisional = true;
    result.unavailable_reason = None;
    (
        result,
        Some(Observation {
            charge,
            empty,
            cell_v,
            single_curve: profile.curves.len() == 1,
        }),
    )
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    VoltageSmoothed,
    ChargeIntegrated,
}
#[derive(Clone, Copy, Debug)]
pub struct Observation {
    pub charge: f64,
    pub empty: f64,
    pub cell_v: f64,
    pub single_curve: bool,
}
fn display(q: f64, empty: f64) -> f64 {
    ((q - empty) / (1. - empty)).clamp(0., 1.)
}
#[derive(Clone, Debug, PartialEq)]
struct Settling {
    since: u64,
    min_v: f64,
    max_v: f64,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Temporal {
    charge: Option<f64>,
    last: Option<(u64, Option<f64>)>,
    settling: Option<Settling>,
    provisional: bool,
}
impl Temporal {
    /// Retain charge, never integrate or smooth over an unobserved interval.
    pub fn interrupt(&mut self) {
        self.last = None;
        self.settling = None;
        self.provisional = true;
    }
    pub fn update(
        &mut self,
        result: &mut Estimate,
        observation: Option<Observation>,
        config: &Configuration,
        time: u64,
        current: Option<f64>,
        charging: bool,
    ) {
        let Some(o) = observation else {
            self.interrupt();
            return;
        };
        let current = current.filter(|v| v.is_finite());
        let interval = self.last.filter(|(t, _)| time >= *t && time - *t <= 5_000);
        if interval.is_none() {
            self.interrupt();
        }
        let dt = interval.map_or(0., |(t, _)| (time - t) as f64 / 1000.);
        let mut q = self.charge.unwrap_or(o.charge);
        if self.charge.is_none() {
            self.provisional = true;
        }
        let capacity = config
            .capacity_ah
            .filter(|a| a.is_finite() && (0.1..=200.).contains(a));
        let integrated = match (capacity, interval, current) {
            (Some(ah), Some((_, Some(previous))), Some(amps)) if dt > 0. => {
                q = (q - (previous + amps) * dt / (2. * 3600. * ah)).clamp(0., 1.);
                true
            }
            _ => false,
        };
        let low = !charging
            && current.is_some_and(|a| a.abs() <= 0.1 * f64::from(config.parallel.unwrap_or(1)));
        if low {
            match self.settling.as_mut() {
                Some(window)
                    if (window.max_v.max(o.cell_v) - window.min_v.min(o.cell_v)) <= 0.01 =>
                {
                    window.min_v = window.min_v.min(o.cell_v);
                    window.max_v = window.max_v.max(o.cell_v);
                }
                _ => {
                    self.settling = Some(Settling {
                        since: time,
                        min_v: o.cell_v,
                        max_v: o.cell_v,
                    })
                }
            }
        } else {
            self.settling = None;
        }
        let settled = self
            .settling
            .as_ref()
            .is_some_and(|w| time.saturating_sub(w.since) >= 30_000);
        if !integrated || settled {
            let tau = if integrated { 120. } else { 30. };
            let max_step = 0.001 * dt * (1. - o.empty);
            let correction = (o.charge - q) * (1. - (-dt / tau).exp());
            q = (q + correction.clamp(-max_step, max_step)).clamp(0., 1.);
        }
        if current.is_none()
            || current.is_some_and(|a| a < 0.)
            || result.mode == Mode::LoadClamped
            || (o.single_curve && current.is_some_and(|a| a > 0.))
        {
            self.provisional = true;
        }
        if settled && (display(q, o.empty) - display(o.charge, o.empty)).abs() <= 0.02 {
            self.provisional = false;
        }
        self.charge = Some(q);
        self.last = Some((time, current));
        result.level = Some(display(q, o.empty));
        result.method = Some(if integrated {
            Method::ChargeIntegrated
        } else {
            Method::VoltageSmoothed
        });
        result.provisional = self.provisional;
    }
}
#[cfg(test)]
fn estimate(
    config: &Configuration,
    revision: Option<u64>,
    sequence: Option<u64>,
    applicable: bool,
    controller_series: Option<u8>,
    voltage: Option<f64>,
    current: Option<f64>,
) -> Estimate {
    observe(
        config,
        revision,
        sequence,
        applicable,
        controller_series,
        voltage,
        current,
    )
    .0
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
            capacity_ah: None,
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
                assert_eq!(calc(&c, minimum * 20. - 0.01, Some(0.)).level, Some(0.));
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
}

#[cfg(test)]
#[path = "../../../../conformance/rust/battery-temporal.rs"]
mod temporal_tests;
