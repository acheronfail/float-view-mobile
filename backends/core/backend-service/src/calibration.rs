//! Temporary GPS calibration. Coordinates never enter this owner or its snapshots.
use float_view_application_runtime::coordinator as c;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MIN_SAMPLES: usize = 30;
const MAX_SAMPLES: usize = 180;
const MIN_SAMPLE_SPAN_MS: u64 = 29_000;
const MIN_EFFECTIVE_SAMPLES: f64 = 15.;
const MAX_IN_PROGRESS_PERCENT: u8 = 95;
// Accommodate ordinary BLE/GNSS delivery jitter while keeping alignment bounded.
const BOARD_GAP_MS: u64 = 1_500;
const ALIGNMENT_MS: u64 = 750;
const GPS_AGE_MS: u64 = 3_000;
const MIN_SPEED_MPS: f64 = 2.;
const MAX_GPS_UNCERTAINTY_MPS: f64 = 2.;
const MAX_RELATIVE_UNCERTAINTY: f64 = 0.2;
const MAX_RESULT_UNCERTAINTY: f64 = 0.025;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MeasurementStatus {
    #[default]
    Inactive,
    WaitingGps,
    WaitingBoard,
    LowSpeed,
    GpsUncertain,
    SpeedChanging,
    SpeedMismatch,
    Collecting,
    NeedsMoreMeasurements,
    InconsistentMeasurements,
    Ready,
}
impl MeasurementStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::WaitingGps => "waitingGps",
            Self::WaitingBoard => "waitingBoard",
            Self::LowSpeed => "lowSpeed",
            Self::GpsUncertain => "gpsUncertain",
            Self::SpeedChanging => "speedChanging",
            Self::SpeedMismatch => "speedMismatch",
            Self::Collecting => "collecting",
            Self::NeedsMoreMeasurements => "needsMoreMeasurements",
            Self::InconsistentMeasurements => "inconsistentMeasurements",
            Self::Ready => "ready",
        }
    }
}
pub(crate) const MAX_DURATION_MS: u64 = 3 * 60 * 1000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub status: String,
    pub measurement_status: MeasurementStatus,
    pub current_diameter_mm: Option<f64>,
    pub starting_diameter_mm: Option<f64>,
    pub recommended_diameter_mm: Option<f64>,
    pub accepted_samples: usize,
    pub progress_percent: u8,
    pub minimum_speed_kph: f64,
    pub apply_available: bool,
    pub apply_block_reason: Option<String>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            status: "idle".into(),
            measurement_status: MeasurementStatus::Inactive,
            current_diameter_mm: None,
            starting_diameter_mm: None,
            recommended_diameter_mm: None,
            accepted_samples: 0,
            progress_percent: 0,
            minimum_speed_kph: MIN_SPEED_MPS * 3.6,
            apply_available: false,
            apply_block_reason: None,
        }
    }
}
#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
enum QualityReason {
    #[default]
    WaitingSamples,
    EffectiveSamples,
    Uncertainty,
    Outliers,
    Drift,
    Diameter,
    Ready,
}
#[derive(Serialize)]
pub(crate) struct Diagnostic {
    state: MeasurementStatus,
    algorithm_version: u64,
    duration_ms: u64,
    gps_uncertain: u64,
    gps_accuracy_missing: u64,
    gps_accuracy_absolute: u64,
    gps_accuracy_relative: u64,
    quality: QualityReason,
    effective_samples: usize,
    uncertainty_bps: u64,
    gps_updates: u64,
    accepted: usize,
    board_samples: usize,
    gps_rejected: u64,
    board_rejected: u64,
    low_speed: u64,
    speed_changing: u64,
    speed_mismatch: u64,
}
#[derive(Clone, Copy)]
struct ApprovedEstimate {
    relative_uncertainty: f64,
    effective_samples: usize,
    uncertainty_bps: u64,
}
#[derive(Default)]
pub(crate) struct Calibration {
    best: Option<ApprovedEstimate>,
    pub state: State,
    pub generation: u64,
    pub started: u64,
    last_fix: Option<u64>,
    // Each fix is separated in measurement time and retains its ratio uncertainty.
    samples: Vec<(u64, f64, f64)>,
    last_gps: Option<u64>,
    board_status: MeasurementStatus,
    gps_updates: u64,
    last_diagnostic: Option<u64>,
    finished_diagnostic: bool,
    quality: QualityReason,
    effective_samples: usize,
    uncertainty_bps: u64,
    rejected: [u64; 5],
    gps_uncertain: u64,
    gps_accuracy_missing: u64,
    gps_accuracy_absolute: u64,
    gps_accuracy_relative: u64,
    telemetry: VecDeque<(u64, f64)>,
}
impl Calibration {
    pub fn collecting(&self) -> bool {
        matches!(self.state.status.as_str(), "collecting" | "refining")
    }
    pub fn start(&mut self, now: u64, generation: u64, diameter: f64) {
        *self = Self::default();
        self.started = now;
        self.generation = generation;
        self.state.status = "collecting".into();
        self.state.measurement_status = MeasurementStatus::WaitingGps;
        self.state.starting_diameter_mm = Some(diameter);
    }
    pub fn board(&mut self, now: u64, sample: &c::telemetry::NormalizedSample) {
        if !self.collecting() {
            return;
        }
        self.refresh_status(now);
        let speed = sample
            .speed_kph
            .filter(|s| s.is_finite())
            .map(|s| s.abs() / 3.6);
        self.board_status = if speed.is_none() {
            MeasurementStatus::WaitingBoard
        } else if speed.is_some_and(|s| s < MIN_SPEED_MPS) {
            MeasurementStatus::LowSpeed
        } else if sample.ride_state != c::telemetry::RideState::Riding
            || sample.package_mode != Some(c::telemetry::PackageMode::Normal)
        {
            MeasurementStatus::WaitingBoard
        } else if sample.wheelslip == Some(true) {
            MeasurementStatus::SpeedChanging
        } else {
            MeasurementStatus::Collecting
        };
        if self.board_status != MeasurementStatus::Collecting {
            self.telemetry.clear();
            return;
        }
        if self
            .telemetry
            .back()
            .is_some_and(|(t, _)| now.saturating_sub(*t) > BOARD_GAP_MS)
        {
            self.telemetry.clear();
        }
        self.telemetry
            .push_back((now, speed.expect("validated board speed")));
        while self.telemetry.len() > 128
            || self
                .telemetry
                .front()
                .is_some_and(|(t, _)| now.saturating_sub(*t) > 10_000)
        {
            self.telemetry.pop_front();
        }
    }
    fn refresh_status(&mut self, now: u64) {
        if self.collecting()
            && self
                .last_gps
                .is_none_or(|at| now.saturating_sub(at) > GPS_AGE_MS)
        {
            self.state.measurement_status = MeasurementStatus::WaitingGps;
        }
    }
    pub fn speed(&mut self, now: u64, age: u64, speed: Option<f64>, accuracy: Option<f64>) {
        if !self.collecting() {
            return;
        }
        if let Err(status) = self.observe_speed(now, age, speed, accuracy) {
            self.state.measurement_status = status;
            if status == MeasurementStatus::GpsUncertain {
                self.gps_uncertain = self.gps_uncertain.saturating_add(1).min(1_000_000);
            }
            let index = match status {
                MeasurementStatus::WaitingGps | MeasurementStatus::GpsUncertain => Some(0),
                MeasurementStatus::WaitingBoard => Some(1),
                MeasurementStatus::LowSpeed => Some(2),
                MeasurementStatus::SpeedChanging => Some(3),
                MeasurementStatus::SpeedMismatch => Some(4),
                _ => None,
            };
            if let Some(i) = index {
                self.rejected[i] = self.rejected[i].saturating_add(1).min(1_000_000);
            }
        }
    }
    // A rejected observation carries its own reason. Ignored duplicate callbacks
    // leave the previous display state alone and never repeat its rejection count.
    fn observe_speed(
        &mut self,
        now: u64,
        age: u64,
        speed: Option<f64>,
        accuracy: Option<f64>,
    ) -> Result<(), MeasurementStatus> {
        self.gps_updates = self.gps_updates.saturating_add(1).min(1_000_000);
        self.last_gps = Some(now);
        if now.saturating_sub(self.started) > MAX_DURATION_MS || age > GPS_AGE_MS {
            return Err(MeasurementStatus::WaitingGps);
        }
        let Some(at) = now.checked_sub(age).filter(|at| *at >= self.started) else {
            return Err(MeasurementStatus::WaitingGps);
        };
        if self
            .last_fix
            .is_some_and(|last| at <= last || at - last < 900)
        {
            return Ok(());
        }
        let Some(speed) = speed.filter(|v| v.is_finite() && *v >= 0.) else {
            return Err(MeasurementStatus::WaitingGps);
        };
        if speed < MIN_SPEED_MPS {
            return Err(MeasurementStatus::LowSpeed);
        }
        let Some(accuracy) = accuracy.filter(|v| v.is_finite() && *v >= 0.) else {
            self.gps_accuracy_missing = self.gps_accuracy_missing.saturating_add(1).min(1_000_000);
            return Err(MeasurementStatus::GpsUncertain);
        };
        if accuracy > MAX_GPS_UNCERTAINTY_MPS {
            self.gps_accuracy_absolute =
                self.gps_accuracy_absolute.saturating_add(1).min(1_000_000);
            return Err(MeasurementStatus::GpsUncertain);
        };
        if accuracy / speed > MAX_RELATIVE_UNCERTAINTY {
            self.gps_accuracy_relative =
                self.gps_accuracy_relative.saturating_add(1).min(1_000_000);
            return Err(MeasurementStatus::GpsUncertain);
        }
        if self.telemetry.is_empty() {
            return Err(match self.board_status {
                MeasurementStatus::LowSpeed | MeasurementStatus::SpeedChanging => self.board_status,
                _ => MeasurementStatus::WaitingBoard,
            });
        }
        let window: Vec<_> = self
            .telemetry
            .iter()
            .filter(|(t, _)| *t <= at && at - *t <= 3_000)
            .collect();
        if window.len() < 3 || at.saturating_sub(window[0].0) < 2_000 {
            return Err(MeasurementStatus::WaitingBoard);
        }
        let before = self.telemetry.iter().rev().find(|(t, _)| *t <= at);
        let after = self.telemetry.iter().find(|(t, _)| *t >= at);
        let board = match (before, after) {
            (Some(&(a, x)), Some(&(b, y))) if b - a <= BOARD_GAP_MS => {
                if a == b {
                    x
                } else {
                    x + (y - x) * (at - a) as f64 / (b - a) as f64
                }
            }
            (Some(&(a, x)), None) if at - a <= ALIGNMENT_MS => x,
            _ => return Err(MeasurementStatus::WaitingBoard),
        };
        let (lo, hi) = window
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (_, s)| {
                (lo.min(*s), hi.max(*s))
            });
        if board < MIN_SPEED_MPS || hi.max(board) - lo.min(board) > (board * 0.1).max(0.5) {
            return Err(MeasurementStatus::SpeedChanging);
        }
        let ratio = speed / board;
        if !(0.7..=1.3).contains(&ratio) {
            return Err(MeasurementStatus::SpeedMismatch);
        }
        self.last_fix = Some(at);
        if self.samples.len() == MAX_SAMPLES {
            self.samples.remove(0);
        }
        // A finite floor prevents one nominally perfect OS fix dominating the ride.
        self.samples.push((at, ratio, (accuracy / board).max(0.01)));
        self.state.accepted_samples = self.samples.len();
        self.state.measurement_status = MeasurementStatus::Collecting;
        self.update_progress();
        self.update_recommendation();
        Ok(())
    }
    fn update_progress(&mut self) {
        let (mean, error, effective) = estimate(&self.samples);
        let span = self.samples.last().unwrap().0 - self.samples[0].0;
        // Approximate the fraction of evidence gathered. Variance scales with
        // sample count, so confidence contributes the squared error ratio.
        // This is presentation only: every recommendation gate still runs below.
        let evidence = (self.samples.len() as f64 / MIN_SAMPLES as f64)
            .min(span as f64 / MIN_SAMPLE_SPAN_MS as f64)
            .min(effective / MIN_EFFECTIVE_SAMPLES)
            .min((MAX_RESULT_UNCERTAINTY * mean / error).powi(2))
            .clamp(0., 1.);
        let progress = (evidence * f64::from(MAX_IN_PROGRESS_PERCENT)).floor() as u8;
        self.state.progress_percent = self.state.progress_percent.max(progress);
    }
    fn update_recommendation(&mut self) {
        self.quality = QualityReason::WaitingSamples;
        self.effective_samples = 0;
        self.uncertainty_bps = 0;
        if self.samples.len() < MIN_SAMPLES
            || self.samples.last().unwrap().0 - self.samples[0].0 < MIN_SAMPLE_SPAN_MS
        {
            return;
        }
        self.state.measurement_status = MeasurementStatus::NeedsMoreMeasurements;
        let mut ratios: Vec<_> = self.samples.iter().map(|(_, r, _)| *r).collect();
        ratios.sort_by(f64::total_cmp);
        let median = ratios[ratios.len() / 2];
        let retained: Vec<_> = self
            .samples
            .iter()
            .copied()
            .filter(|(_, r, u)| (r - median).abs() <= (3. * u).max(0.04 * median))
            .collect();
        if retained.len() < MIN_SAMPLES
            || self.samples.len() - retained.len() > self.samples.len() / 10
        {
            self.state.measurement_status = MeasurementStatus::InconsistentMeasurements;
            self.quality = QualityReason::Outliers;
            return;
        }
        let (mean, error, effective) = estimate(&retained);
        self.effective_samples = effective.floor() as usize;
        self.uncertainty_bps = ((error / mean * 10_000.).round() as u64).min(10_000);
        self.quality = QualityReason::EffectiveSamples;
        if effective < MIN_EFFECTIVE_SAMPLES {
            return;
        }
        self.quality = QualityReason::Uncertainty;
        if error / mean > MAX_RESULT_UNCERTAINTY {
            return;
        }
        let mid = retained.len() / 2;
        let (first, first_error, _) = estimate(&retained[..mid]);
        let (second, second_error, _) = estimate(&retained[mid..]);
        if (first - second).abs()
            > (0.02 * mean)
                .max(2. * first_error.hypot(second_error))
                .min(0.03 * mean)
        {
            self.state.measurement_status = MeasurementStatus::InconsistentMeasurements;
            self.quality = QualityReason::Drift;
            return;
        }
        self.quality = QualityReason::Diameter;
        let mm = self.state.starting_diameter_mm.unwrap() * mean;
        if c::controller_data::wheel_diameter::valid(mm) {
            let relative_uncertainty = error / mean;
            if self
                .best
                .is_none_or(|best| relative_uncertainty < best.relative_uncertainty)
            {
                self.best = Some(ApprovedEstimate {
                    relative_uncertainty,
                    effective_samples: self.effective_samples,
                    uncertainty_bps: self.uncertainty_bps,
                });
                self.state.recommended_diameter_mm = Some((mm * 10.).round() / 10.);
            }
            self.state.status = "refining".into();
            self.state.measurement_status = MeasurementStatus::Ready;
            self.state.progress_percent = 100;
            self.quality = QualityReason::Ready;
        }
    }

    /// One bounded, coordinate-free diagnostic summary per five seconds.
    pub fn diagnostic(&mut self, now: u64) -> Option<Diagnostic> {
        self.refresh_status(now);
        let final_summary = matches!(self.state.status.as_str(), "ready" | "insufficient")
            && !self.finished_diagnostic;
        if !final_summary
            && (!self.collecting()
                || self
                    .last_diagnostic
                    .is_some_and(|last| now.saturating_sub(last) < 5_000))
        {
            return None;
        }
        self.finished_diagnostic |= final_summary;
        self.last_diagnostic = Some(now);
        Some(Diagnostic {
            state: self.state.measurement_status,
            algorithm_version: 3,
            duration_ms: now.saturating_sub(self.started).min(MAX_DURATION_MS),
            gps_uncertain: self.gps_uncertain,
            gps_accuracy_missing: self.gps_accuracy_missing,
            gps_accuracy_absolute: self.gps_accuracy_absolute,
            gps_accuracy_relative: self.gps_accuracy_relative,
            quality: self.quality,
            effective_samples: self.effective_samples,
            uncertainty_bps: self.uncertainty_bps,
            gps_updates: self.gps_updates,
            accepted: self.samples.len(),
            board_samples: self.telemetry.len(),
            gps_rejected: self.rejected[0],
            board_rejected: self.rejected[1],
            low_speed: self.rejected[2],
            speed_changing: self.rejected[3],
            speed_mismatch: self.rejected[4],
        })
    }
    pub fn finish(&mut self) {
        if let Some(best) = self.best {
            self.state.status = "ready".into();
            self.state.measurement_status = MeasurementStatus::Ready;
            self.state.progress_percent = 100;
            self.quality = QualityReason::Ready;
            self.effective_samples = best.effective_samples;
            self.uncertainty_bps = best.uncertainty_bps;
        } else {
            self.state.status = "insufficient".into();
        }
        self.telemetry.clear();
    }
    pub fn snapshot(&self, runtime: &c::Snapshot) -> State {
        let mut s = self.state.clone();
        s.current_diameter_mm = runtime.wheel_diameter.current_mm;
        let reason = if runtime.wheel_diameter.pending {
            Some("applying")
        } else if s.status == "applied" {
            Some("applied")
        } else if self.generation != runtime.generation.0 {
            Some("connectionChanged")
        } else if !runtime.wheel_diameter.supported {
            Some("unsupportedSetup")
        } else if runtime.mutations.pending.is_some() {
            Some("busy")
        } else if runtime.telemetry.previous_ride_state != c::telemetry::RideState::Idle {
            Some("stopRiding")
        } else if !runtime.wheel_diameter_write_available {
            Some("boardNotReady")
        } else if s.status != "ready" || s.recommended_diameter_mm.is_none() {
            Some("notReady")
        } else {
            None
        };
        s.apply_available = reason.is_none();
        s.apply_block_reason = reason.map(str::to_owned);
        s
    }
}

// Bound a reading's weight to four times the weight of the median-uncertainty
// observation. A few nominally precise OS fixes must not starve later evidence.
// Raising the uncertainty floor only reduces influence and increases modeled
// uncertainty; empirical scatter can increase, never reduce, the final error.
fn estimate(samples: &[(u64, f64, f64)]) -> (f64, f64, f64) {
    let mut uncertainties: Vec<_> = samples.iter().map(|(_, _, u)| *u).collect();
    uncertainties.sort_by(f64::total_cmp);
    let floor = uncertainties[uncertainties.len() / 2] / 2.;
    let weight = |u: f64| 1. / u.max(floor).powi(2);
    let total: f64 = samples.iter().map(|(_, _, u)| weight(*u)).sum();
    let mean = samples.iter().map(|(_, r, u)| r * weight(*u)).sum::<f64>() / total;
    let effective = total * total
        / samples
            .iter()
            .map(|(_, _, u)| weight(*u).powi(2))
            .sum::<f64>();
    let scatter = samples
        .iter()
        .map(|(_, r, u)| (r - mean).powi(2) * weight(*u))
        .sum::<f64>()
        / total;
    (
        mean,
        (1. / total).sqrt().max((scatter / effective).sqrt()),
        effective,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn accept_ready(c: &mut Calibration) {
        if c.state.status == "refining" {
            c.finish();
        }
    }
    fn collect(ratio: impl Fn(u64) -> f64) -> Calibration {
        let mut c = Calibration::default();
        c.start(0, 1, 280.);
        for at in (0..=40_000).step_by(100) {
            c.telemetry.push_back((at, 5.));
            while c.telemetry.len() > 50 {
                c.telemetry.pop_front();
            }
            if at % 1000 == 0 {
                c.speed(at, 0, Some(5. * ratio(at)), Some(0.1));
                accept_ready(&mut c);
            }
        }
        c
    }
    #[test]
    fn recommends_known_ratio() {
        let mut c = collect(|_| 0.96);
        c.finish();
        assert_eq!(c.state.status, "ready");
        assert_eq!(c.state.recommended_diameter_mm, Some(268.8));
    }
    #[test]
    fn rejects_inconsistent_halves() {
        let mut c = collect(|t| if t < 20_000 { 0.96 } else { 1.04 });
        c.finish();
        assert_eq!(c.state.status, "insufficient");
    }
    #[test]
    fn rejects_poor_stale_duplicate_and_missing_fixes() {
        let mut c = riding_trace(0.1, &[100], 0, 7_000, false);
        let n = c.samples.len();
        c.speed(7_000, 0, Some(5.), Some(0.1));
        c.speed(8_000, GPS_AGE_MS + 1, Some(5.), Some(0.1));
        c.speed(8_000, 0, Some(5.), None);
        c.speed(8_000, 0, Some(5.), Some(2.));
        assert_eq!(c.samples.len(), n);
        c.start(42_000, 2, 290.);
        assert!(c.samples.is_empty());
        assert!(c.state.recommended_diameter_mm.is_none());
    }
    #[test]
    fn aligns_measurement_time_and_excludes_slip_acceleration_and_gaps() {
        #[allow(dead_code)]
        mod samples {
            use float_view_application_runtime::telemetry as float_view_refloat_telemetry_service;
            include!("../../session-sqlite/tests/support/mod.rs");
        }
        let mut c = Calibration::default();
        c.start(0, 1, 280.);
        let mut sample = samples::sample(1, 0);
        sample.ride_state = c::telemetry::RideState::Riding;
        sample.package_mode = Some(c::telemetry::PackageMode::Normal);
        sample.wheelslip = Some(false);
        sample.speed_kph = Some(18.);
        for at in (0..=4_900).step_by(100) {
            c.board(at, &sample);
        }
        sample.speed_kph = Some(32.4);
        c.board(5_000, &sample);
        c.speed(5_000, 500, Some(4.8), Some(0.1));
        assert_eq!(c.samples.len(), 1);
        assert!((c.samples[0].1 - 0.96).abs() < 0.00001);
        c.speed(5_500, 0, Some(4.8), Some(0.1));
        assert_eq!(c.samples.len(), 1);
        sample.wheelslip = Some(true);
        c.board(5_600, &sample);
        assert!(c.telemetry.is_empty());
        sample.wheelslip = Some(false);
        sample.speed_kph = Some(18.);
        c.board(7_000, &sample);
        c.speed(7_000, 0, Some(4.8), Some(0.1));
        assert_eq!(c.samples.len(), 1);
        sample.ride_state = c::telemetry::RideState::Idle;
        c.board(7_100, &sample);
        assert!(c.telemetry.is_empty());
    }

    #[test]
    fn observation_storage_remains_bounded_within_capture_deadline() {
        let mut c = Calibration::default();
        c.start(0, 1, 280.);
        for at in (0..MAX_DURATION_MS).step_by(100) {
            c.telemetry.push_back((at, 5.));
            while c.telemetry.len() > 50 {
                c.telemetry.pop_front();
            }
            if at % 900 == 0 {
                c.speed(at, 0, Some(if at % 1800 == 0 { 4. } else { 6. }), Some(0.1));
            }
        }
        assert_eq!(c.samples.len(), MAX_SAMPLES);
        assert!(c.state.recommended_diameter_mm.is_none());
        assert_eq!(
            c.state.measurement_status,
            MeasurementStatus::InconsistentMeasurements
        );
    }
    fn riding_trace(
        accuracy: f64,
        gaps: &[u64],
        age: u64,
        duration: u64,
        noisy: bool,
    ) -> Calibration {
        #[allow(dead_code)]
        mod samples {
            use float_view_application_runtime::telemetry as float_view_refloat_telemetry_service;
            include!("../../session-sqlite/tests/support/mod.rs");
        }
        let mut c = Calibration::default();
        c.start(0, 1, 280.);
        let mut sample = samples::sample(1, 0);
        sample.ride_state = c::telemetry::RideState::Riding;
        sample.package_mode = Some(c::telemetry::PackageMode::Normal);
        sample.wheelslip = Some(false);
        let mut next = 0;
        let mut index = 0;
        for now in (0..=duration).step_by(100) {
            if now >= next {
                sample.speed_kph = Some(18.);
                c.board(now, &sample);
                next = now + gaps[index % gaps.len()];
                index += 1;
            }
            if now % 1000 == 0 {
                let noise = if noisy {
                    [0.08, -0.08, 0.04, -0.04, 0.][(now / 1000) as usize % 5]
                } else {
                    0.
                };
                c.speed(now, age, Some(4.8 + noise), Some(accuracy));
                accept_ready(&mut c);
            }
        }
        c
    }
    #[test]
    fn steady_out_of_band_ratio_is_not_reported_as_changing_speed() {
        let mut c = riding_trace(0.1, &[100], 0, 7_000, false);
        let accepted = c.state.accepted_samples;
        c.telemetry.push_back((8_000, 5.));
        c.speed(8_000, 0, Some(7.), Some(0.1));
        assert_eq!(c.state.measurement_status, MeasurementStatus::SpeedMismatch);
        assert_eq!(c.state.accepted_samples, accepted);
        let diagnostic = serde_json::to_value(c.diagnostic(8_000).unwrap()).unwrap();
        assert_eq!(diagnostic["state"], "speedMismatch");
        assert_eq!(diagnostic["speed_mismatch"], 1);
    }
    #[test]
    fn finishing_freezes_result_and_emits_one_final_diagnostic() {
        let mut c = riding_trace(0.1, &[100], 0, 35_000, false);
        assert_eq!(c.state.status, "ready");
        assert!(!c.collecting());
        assert!(c.telemetry.is_empty());
        let count = c.state.accepted_samples;
        let diagnostic = serde_json::to_value(c.diagnostic(35_000).unwrap()).unwrap();
        assert_eq!(diagnostic["state"], "ready");
        assert!(c.diagnostic(35_100).is_none());
        // Late, noisy and stationary observations cannot revoke a completed result.
        c.speed(36_000, 0, Some(6.4), Some(0.1));
        c.speed(37_000, 0, Some(0.), Some(0.1));
        assert_eq!(c.state.accepted_samples, count);
        assert_eq!(c.state.recommended_diameter_mm, Some(268.8));
        assert_eq!(c.state.measurement_status, MeasurementStatus::Ready);
        assert_eq!(c.state.progress_percent, 100);
    }
    fn observe_riding(c: &mut Calibration, at: u64, speed: f64, accuracy: f64) {
        c.telemetry.push_back((at, 5.));
        while c.telemetry.len() > 50 {
            c.telemetry.pop_front();
        }
        if at.is_multiple_of(1000) {
            c.speed(at, 0, Some(speed), Some(accuracy));
        }
    }

    #[test]
    fn approved_result_keeps_collecting_and_can_improve_before_finish() {
        let mut c = Calibration::default();
        c.start(0, 1, 280.);
        for at in (0..=40_000).step_by(100) {
            observe_riding(&mut c, at, 4.8, 0.1);
        }
        assert_eq!(c.state.status, "refining");
        assert!(c.collecting());
        assert_eq!(c.state.progress_percent, 100);
        let before = c.best.unwrap().relative_uncertainty;
        let count = c.state.accepted_samples;
        for at in (40_100..=70_000).step_by(100) {
            observe_riding(&mut c, at, 4.8, 0.1);
        }
        assert!(c.state.accepted_samples > count);
        assert!(c.best.unwrap().relative_uncertainty < before);
        assert_eq!(c.state.status, "refining");
        c.finish();
        assert_eq!(c.state.status, "ready");
        assert!(!c.collecting());
        assert_eq!(c.state.recommended_diameter_mm, Some(268.8));
    }

    #[test]
    fn poorer_approved_estimate_and_later_bad_data_preserve_the_best_result() {
        let mut c = Calibration::default();
        c.start(0, 1, 280.);
        for at in (0..=40_000).step_by(100) {
            observe_riding(&mut c, at, 4.8, 0.05);
        }
        for at in (40_100..=43_000).step_by(100) {
            observe_riding(&mut c, at, 4.985, 0.05);
        }
        let best = c.best.unwrap();
        let diameter = c.state.recommended_diameter_mm;
        for at in (43_100..=44_000).step_by(100) {
            observe_riding(&mut c, at, 4.985, 0.05);
        }
        assert!(matches!(c.quality, QualityReason::Ready));
        let (mean, error, _) = estimate(&c.samples);
        assert!(error / mean > best.relative_uncertainty);
        assert_eq!(
            c.best.unwrap().relative_uncertainty,
            best.relative_uncertainty
        );
        assert_eq!(c.state.recommended_diameter_mm, diameter);
        for at in (45_000..=80_000).step_by(1000) {
            c.speed(at, 0, Some(5.), Some(3.));
        }
        assert_eq!(c.state.recommended_diameter_mm, diameter);
        for at in (80_100..=155_000).step_by(100) {
            observe_riding(&mut c, at, 6., 0.05);
        }
        assert!(!matches!(c.quality, QualityReason::Ready));
        assert_eq!(c.state.status, "refining");
        assert_eq!(c.state.recommended_diameter_mm, diameter);
        c.finish();
        let final_summary = serde_json::to_value(c.diagnostic(155_000).unwrap()).unwrap();
        assert_eq!(final_summary["quality"], "ready");
        assert_eq!(final_summary["uncertainty_bps"], best.uncertainty_bps);
        assert_eq!(final_summary["effective_samples"], best.effective_samples);
        assert_eq!(c.state.recommended_diameter_mm, diameter);
        assert!(c.telemetry.is_empty());
    }

    #[test]
    fn progress_reflects_evidence_and_uncertainty_without_changing_completion() {
        let precise = riding_trace(0.1, &[100], 0, 18_000, false);
        let uncertain = riding_trace(0.9, &[100], 0, 18_000, false);
        assert_eq!(
            precise.state.accepted_samples,
            uncertain.state.accepted_samples
        );
        assert!(precise.state.progress_percent > uncertain.state.progress_percent);
        assert!(uncertain.state.progress_percent > 0);
        assert!(precise.state.progress_percent < 100);
        let blocked = riding_trace(0.9, &[100], 0, 55_000, false);
        assert!(blocked.state.accepted_samples > MIN_SAMPLES);
        assert!(blocked.collecting());
        assert!(blocked.state.progress_percent < 100);
        assert!(blocked.state.recommended_diameter_mm.is_none());
        let inconsistent = collect(|at| if at < 20_000 { 0.96 } else { 1.04 });
        assert!(inconsistent.state.progress_percent <= MAX_IN_PROGRESS_PERCENT);
        assert!(inconsistent.state.recommended_diameter_mm.is_none());
    }

    #[test]
    fn progress_holds_through_rejected_fixes_and_resets_only_for_a_new_attempt() {
        let mut c = riding_trace(0.85, &[100], 0, 44_000, false);
        assert!(c.collecting());
        let progress = c.state.progress_percent;
        let accepted = c.state.accepted_samples;
        for at in (45_000..=150_000).step_by(1000) {
            c.speed(at, 0, Some(5.), Some(3.));
            assert_eq!(c.state.progress_percent, progress);
            assert_eq!(c.state.accepted_samples, accepted);
        }
        // A subsequent accepted but noisier comparison cannot move the bar back.
        for at in (151_000..=154_000).step_by(100) {
            c.telemetry.push_back((at, 5.));
        }
        c.speed(154_000, 0, Some(6.25), Some(0.9));
        assert_eq!(c.state.accepted_samples, accepted + 1);
        assert!(c.state.progress_percent >= progress);
        c.finish();
        assert_eq!(c.state.status, "insufficient");
        assert!(c.state.progress_percent < 100);
        c.start(160_000, 2, 280.);
        assert_eq!(c.state.progress_percent, 0);
    }

    #[test]
    fn count_above_minimum_distinguishes_uncertainty_from_disagreement() {
        let mut uncertain = riding_trace(0.9, &[100], 0, 34_000, false);
        assert!(uncertain.state.accepted_samples > MIN_SAMPLES);
        assert!(uncertain.collecting());
        assert_eq!(
            uncertain.state.measurement_status,
            MeasurementStatus::NeedsMoreMeasurements
        );
        let diagnostic = serde_json::to_value(uncertain.diagnostic(34_000).unwrap()).unwrap();
        assert_eq!(diagnostic["state"], "needsMoreMeasurements");
        let inconsistent = collect(|at| if at < 20_000 { 0.96 } else { 1.04 });
        assert!(inconsistent.state.accepted_samples > MIN_SAMPLES);
        assert!(inconsistent.collecting());
        assert_eq!(
            inconsistent.state.measurement_status,
            MeasurementStatus::InconsistentMeasurements
        );
        assert!(inconsistent.state.recommended_diameter_mm.is_none());
    }
    fn mixed_precision_trace(ratio: impl Fn(u64, usize) -> f64) -> Calibration {
        let mut c = Calibration::default();
        c.start(0, 1, 280.);
        for at in (0..=99_000).step_by(100) {
            c.telemetry.push_back((at, 5.));
            while c.telemetry.len() > 50 {
                c.telemetry.pop_front();
            }
            if at % 1000 == 0 {
                let accuracy = if c.state.accepted_samples < 5 {
                    0.05
                } else {
                    0.9
                };
                let noise = [0.02, -0.02, 0.][(at / 1000) as usize % 3];
                c.speed(
                    at,
                    0,
                    Some(5. * ratio(at, c.state.accepted_samples) + noise),
                    Some(accuracy),
                );
                accept_ready(&mut c);
            }
        }
        c
    }
    #[test]
    fn mixed_gps_precision_does_not_starve_a_consistent_ride() {
        let mut c = mixed_precision_trace(|_, _| 0.96);
        assert_eq!(
            c.state.status,
            "ready",
            "aggregate {:?}",
            estimate(&c.samples)
        );
        assert!(c.state.accepted_samples < 98);
        assert!((c.state.recommended_diameter_mm.unwrap() - 268.8).abs() < 0.5);
        let summary = serde_json::to_value(c.diagnostic(100_000).unwrap()).unwrap();
        assert_eq!(summary["quality"], "ready");
        assert!(summary["effective_samples"].as_u64().unwrap() >= 15);
        assert!(summary["uncertainty_bps"].as_u64().unwrap() <= 250);
    }
    #[test]
    fn mixed_precision_preserves_outlier_and_drift_checks() {
        let c = mixed_precision_trace(|_, accepted| if accepted < 5 { 1.25 } else { 0.96 });
        assert_eq!(c.state.status, "ready");
        assert!((c.state.recommended_diameter_mm.unwrap() - 268.8).abs() < 0.5);
        let mut c = mixed_precision_trace(|at, _| 0.9 + 0.2 * at as f64 / 100_000.);
        assert!(c.collecting());
        assert!(c.state.recommended_diameter_mm.is_none());
        let summary = serde_json::to_value(c.diagnostic(100_000).unwrap()).unwrap();
        assert_eq!(summary["quality"], "drift");
    }
    #[test]
    fn normal_reported_gps_uncertainty_advances_and_noisy_fixes_converge() {
        // Every fix was rejected by the original 0.25 m/s per-fix gate.
        let c = riding_trace(0.5, &[100], 200, 45_000, true);
        assert!(c.state.accepted_samples >= 30);
        assert!((c.state.recommended_diameter_mm.unwrap() - 268.8).abs() < 1.0);
        assert_eq!(c.state.measurement_status, MeasurementStatus::Ready);
    }
    #[test]
    fn bounded_radio_jitter_and_one_second_board_updates_can_calibrate() {
        for gaps in [&[400, 600, 1100, 300, 700, 500][..], &[1000][..]] {
            let c = riding_trace(0.1, gaps, 800, 55_000, false);
            assert!(c.state.accepted_samples >= 30);
            assert_eq!(c.state.recommended_diameter_mm, Some(268.8));
        }
    }
    #[test]
    fn uncertain_observations_need_more_evidence_and_long_gaps_never_count() {
        let c = riding_trace(0.9, &[100], 0, 34_000, false);
        assert!(c.state.accepted_samples >= 30);
        assert!(c.state.recommended_diameter_mm.is_none());
        assert_eq!(
            c.state.measurement_status,
            MeasurementStatus::NeedsMoreMeasurements
        );
        let c = riding_trace(0.9, &[100], 0, 90_000, false);
        assert_eq!(c.state.recommended_diameter_mm, Some(268.8));
        let c = riding_trace(0.1, &[2000], 0, 55_000, false);
        assert_eq!(c.state.accepted_samples, 0);
        assert_eq!(c.state.measurement_status, MeasurementStatus::WaitingBoard);
    }
    #[test]
    fn duplicate_callbacks_do_not_repeat_the_previous_rejection() {
        let mut c = riding_trace(0.1, &[100], 0, 7_000, false);
        c.speed(8_000, 0, Some(5.), None);
        let rejected = c.rejected;
        assert_eq!(c.gps_uncertain, 1);
        // This repeats the last accepted measurement after a rejected fix.
        c.speed(8_100, 1_100, Some(5.), Some(0.1));
        assert_eq!(c.rejected, rejected);
        assert_eq!(c.gps_uncertain, 1);
        assert_eq!(c.state.measurement_status, MeasurementStatus::GpsUncertain);
    }

    #[test]
    fn accuracy_rejections_identify_the_gate_without_raw_observations() {
        let mut c = Calibration::default();
        c.start(0, 1, 280.);
        c.speed(1000, 0, Some(6.), None);
        c.speed(2000, 0, Some(6.), Some(2.1));
        c.speed(3000, 0, Some(6.), Some(1.5));
        let data = serde_json::to_value(c.diagnostic(3000).unwrap()).unwrap();
        assert_eq!(data["gps_uncertain"], 3);
        assert_eq!(data["gps_accuracy_missing"], 1);
        assert_eq!(data["gps_accuracy_absolute"], 1);
        assert_eq!(data["gps_accuracy_relative"], 1);
        for private in ["speed_mps", "accuracy_mps", "latitude", "longitude"] {
            assert!(!data.as_object().unwrap().contains_key(private));
        }
    }
    #[test]
    fn statuses_and_diagnostics_distinguish_missing_uncertain_and_stationary_data() {
        let mut c = Calibration::default();
        c.start(0, 1, 280.);
        assert_eq!(c.state.measurement_status, MeasurementStatus::WaitingGps);
        c.speed(1000, 0, Some(5.), None);
        assert_eq!(c.state.measurement_status, MeasurementStatus::GpsUncertain);
        c.speed(2000, 0, Some(0.), Some(0.5));
        assert_eq!(c.state.measurement_status, MeasurementStatus::LowSpeed);
        c.speed(3000, 0, Some(5.), Some(0.5));
        assert_eq!(c.state.measurement_status, MeasurementStatus::WaitingBoard);
        let data = serde_json::to_value(c.diagnostic(3000).unwrap()).unwrap();
        assert_eq!(data["gps_updates"], 3);
        assert_eq!(data["gps_rejected"], 1);
        assert_eq!(data["gps_uncertain"], 1);
        assert_eq!(data["algorithm_version"], 3);
        assert_eq!(data["duration_ms"], 3000);
        assert_eq!(data["low_speed"], 1);
        assert_eq!(data["board_rejected"], 1);
        assert!(c.diagnostic(4000).is_none());
        let text = serde_json::to_string(&data).unwrap();
        for private in ["latitude", "longitude", "speed_mps", "accuracy_mps"] {
            assert!(!text.contains(private));
        }
        assert_eq!(c.state.accepted_samples, 0);
    }
}
