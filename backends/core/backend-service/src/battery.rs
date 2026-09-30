//! Estimation belongs to the process-owned connection, never a view or reader.
use crate::battery_estimator::{
    Configuration, CurrentFilter, Estimate, UnavailableReason, estimate,
};
use crate::{BackendService, operations::Failure};
#[derive(Clone, PartialEq)]
pub(crate) struct State {
    pub estimate: Estimate,
    config: Configuration,
    filter: CurrentFilter,
    sample_time: Option<u64>,
    dirty: bool,
    reset_pending: bool,
    applicable: bool,
    controller_series: Option<u8>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            estimate: Estimate::unavailable(UnavailableReason::NotApplicable),
            config: Configuration::default(),
            filter: CurrentFilter::default(),
            sample_time: None,
            dirty: true,
            reset_pending: true,
            applicable: false,
            controller_series: None,
        }
    }
}
impl State {
    pub fn reset_filter(&mut self) {
        self.reset_pending = true;
    }
    pub fn accept(&mut self, sequence: u64, time: u64, current: Option<f64>) -> bool {
        if self.reset_pending {
            self.filter = CurrentFilter::default();
        }
        if !self.filter.accept(sequence, time, current) {
            return false;
        }
        self.reset_pending = false;
        self.sample_time = Some(time);
        self.dirty = true;
        true
    }
}
impl BackendService {
    pub(crate) fn accept_battery_sample(
        &mut self,
        sample: float_view_application_runtime::telemetry::NormalizedSample,
    ) {
        if let Some(active) = self.policy.active.as_mut()
            && active.battery.accept(
                sample.sequence,
                self.policy.exchange.now,
                sample.battery_current_a,
            )
        {
            active.latest = Some(sample);
        }
    }
    pub(crate) fn refresh_battery(&mut self) -> Result<(), Failure> {
        let Some(active) = self.policy.active.as_mut() else {
            return Ok(());
        };
        let profile = active
            .board
            .as_ref()
            .map(|b| self.store.profile(&b.saved_device_id))
            .transpose()
            .map_err(crate::store_failure)?;
        let config = profile
            .as_ref()
            .map(|p| {
                let count = |key: &str| {
                    p.values
                        .get(key)
                        .and_then(|v| v.as_f64())
                        .filter(|v| v.is_finite() && v.fract() == 0. && (0.0..=255.0).contains(v))
                        .map(|v| v as u8)
                };
                Configuration {
                    profile_id: p
                        .values
                        .get("batteryCellProfileId")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
                    series: count("seriesCount"),
                    parallel: count("parallelCount"),
                    minimum_v: p
                        .values
                        .get("batteryMinimumCellVoltageV")
                        .and_then(|v| v.as_f64()),
                }
            })
            .unwrap_or_default();
        let runtime = self.runtime.snapshot().application;
        let battery = &mut active.battery;
        let applicable = active.saved && !runtime.standalone_bms;
        if applicable != battery.applicable {
            battery.applicable = applicable;
            battery.dirty = true;
        }
        // Retain the last known controller count while reconnecting, just like
        // the retained sample. A new ready controller supplies its own facts.
        if runtime.phase == float_view_application_runtime::coordinator::Phase::Ready {
            let series = runtime
                .current_limits
                .as_ref()
                .and_then(|l| l.battery_series_cells);
            if series != battery.controller_series {
                battery.controller_series = series;
                battery.dirty = true;
            }
        }
        if config != battery.config {
            battery.dirty = true;
            battery.filter = CurrentFilter::default();
            if let (Some(sample), Some(time)) = (&active.latest, battery.sample_time) {
                battery
                    .filter
                    .accept(sample.sequence, time, sample.battery_current_a);
            }
            battery.config = config;
        }
        if !battery.dirty
            && active.latest.is_some()
            && battery.estimate.profile_revision == profile.as_ref().map(|p| p.revision)
        {
            return Ok(());
        }
        battery.dirty = false;
        battery.estimate = estimate(
            &battery.config,
            profile.as_ref().map(|p| p.revision),
            active.latest.as_ref().map(|s| s.sequence),
            battery.applicable,
            battery.controller_series,
            active.latest.as_ref().and_then(|s| s.pack_voltage_v),
            battery.filter.current(),
        );
        Ok(())
    }
}
