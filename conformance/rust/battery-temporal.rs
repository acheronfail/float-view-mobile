//! Canonical deterministic temporal battery conformance, compiled in the service test module.
use super::*;
fn config(capacity: Option<f64>) -> Configuration {
    Configuration {
        profile_id: Some("generic-linear".into()),
        series: Some(20),
        parallel: Some(2),
        minimum_v: Some(2.7),
        capacity_ah: capacity,
    }
}
fn step(
    t: &mut Temporal,
    c: &Configuration,
    time: u64,
    v: Option<f64>,
    a: Option<f64>,
    charging: bool,
) -> Estimate {
    let (mut e, o) = observe(c, Some(1), Some(time), true, None, v, a);
    t.update(&mut e, o, c, time, a, charging);
    e
}
#[test]
fn independent_canonical_charge_and_load_fixtures() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/battery-temporal.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let mut c = config(case["capacityAh"].as_f64());
        c.minimum_v = case["minimumV"].as_f64();
        let mut temporal = Temporal::default();
        let mut last = Estimate::unavailable(UnavailableReason::MissingVoltage);
        let samples = case["samples"].as_array().unwrap();
        let ah: f64 = samples
            .windows(2)
            .filter(|p| p[1]["timeMs"].as_u64().unwrap() - p[0]["timeMs"].as_u64().unwrap() <= 5000)
            .map(|p| {
                (p[0]["current"].as_f64().unwrap() + p[1]["current"].as_f64().unwrap()) / 2.
                    * (p[1]["timeMs"].as_f64().unwrap() - p[0]["timeMs"].as_f64().unwrap())
                    / 3_600_000.
            })
            .sum();
        assert!((ah - case["expectedAh"].as_f64().unwrap()).abs() < 1e-12);
        for sample in samples {
            last = step(
                &mut temporal,
                &c,
                sample["timeMs"].as_u64().unwrap(),
                sample["voltage"].as_f64(),
                sample["current"].as_f64(),
                sample["charging"].as_bool().unwrap_or(false),
            );
        }
        assert!(
            (last.level.unwrap() - case["expectedLevel"].as_f64().unwrap()).abs() < 1e-12,
            "{}",
            case["name"]
        );
        assert!(
            (temporal.charge.unwrap() - case["expectedCharge"].as_f64().unwrap()).abs() < 1e-12
        );
    }
}
#[test]
fn rest_requires_continuous_stable_low_load_and_convergence() {
    let c = config(Some(9.));
    let mut t = Temporal::default();
    step(&mut t, &c, 0, Some(63.), Some(60.), false);
    for second in 1..31 {
        let e = step(&mut t, &c, second * 1000, Some(78.), Some(0.), false);
        assert!(e.level.unwrap() < 0.301);
        assert!(e.provisional);
    }
    let before = t.charge.unwrap();
    let corrected = step(&mut t, &c, 31_000, Some(78.), Some(0.), false);
    assert!((corrected.level.unwrap() - before - 0.001).abs() < 1e-12);
    let renewed = step(&mut t, &c, 32_000, Some(78.), Some(60.), false);
    assert!(renewed.level < corrected.level);
    assert!(t.settling.is_none());
    let mut t = Temporal::default();
    for second in 0..=30 {
        let e = step(&mut t, &c, second * 1000, Some(78.), Some(0.), false);
        assert_eq!(e.provisional, second < 30);
    }
    // Neither active charging nor unstable voltage can qualify.
    for charging in [true, false] {
        let mut t = Temporal::default();
        for second in 0..=60 {
            let v = if second % 2 == 0 { 78. } else { 79. };
            let e = step(&mut t, &c, second * 1000, Some(v), Some(0.), charging);
            assert!(e.provisional);
        }
    }
}
#[test]
fn broken_pairs_missing_voltage_current_and_reconnect_do_not_integrate() {
    let c = config(Some(9.));
    let mut t = Temporal::default();
    step(&mut t, &c, 0, Some(78.), Some(60.), false);
    let unavailable = step(&mut t, &c, 1000, None, Some(60.), false);
    assert_eq!(unavailable.method, None);
    assert!(!unavailable.provisional);
    let resumed = step(&mut t, &c, 2000, Some(63.), Some(60.), false);
    assert!((resumed.level.unwrap() - 0.8).abs() < 1e-12);
    assert!(resumed.provisional);
    let missing = step(&mut t, &c, 3000, Some(63.), None, false);
    assert_eq!(missing.method, Some(Method::VoltageSmoothed));
    let next = step(&mut t, &c, 4000, Some(63.), Some(60.), false);
    assert_eq!(next.method, Some(Method::VoltageSmoothed));
    t.interrupt();
    let before = t.charge;
    step(&mut t, &c, 5000, Some(63.), Some(-60.), false);
    assert_eq!(t.charge, before);
}
#[test]
fn smoothing_depends_on_elapsed_time_and_integrated_charge_is_not_rate_limited() {
    let c = config(None);
    let mut a = Temporal::default();
    let mut b = Temporal::default();
    step(&mut a, &c, 0, Some(78.), Some(0.), false);
    step(&mut b, &c, 0, Some(78.), Some(0.), false);
    step(&mut a, &c, 1000, Some(63.), Some(0.), false);
    for ms in (100..=1000).step_by(100) {
        step(&mut b, &c, ms, Some(63.), Some(0.), false);
    }
    assert!((a.charge.unwrap() - b.charge.unwrap()).abs() < 1e-12);
    for current in [1000., -1000.] {
        let mut t = Temporal::default();
        let c = config(Some(0.1));
        step(&mut t, &c, 0, Some(78.), Some(current), false);
        let e = step(&mut t, &c, 1000, Some(78.), Some(current), false);
        assert_eq!(e.level, Some(if current > 0. { 0. } else { 1. }));
    }
    for min in [3., 4.19, f64::from_bits(4.2f64.to_bits() - 1)] {
        let c = Configuration {
            minimum_v: Some(min),
            ..config(None)
        };
        let mut t = Temporal::default();
        let old = step(&mut t, &c, 0, Some(84.), Some(0.), false);
        let new = step(&mut t, &c, 1000, Some(54.), Some(0.), false);
        assert!(new.level.unwrap().is_finite());
        assert!(old.level.unwrap() - new.level.unwrap() <= 0.0011);
    }
}
