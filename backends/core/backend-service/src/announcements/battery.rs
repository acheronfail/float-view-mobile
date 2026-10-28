//! Board-derived voltage headroom; no chemistry assumptions or speed gate.
use super::Battery;
use float_view_application_runtime::coordinator as c;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Limits {
    low: f64,
    high: f64,
    low_entry: f64,
    high_entry: f64,
    low_recovery: f64,
    high_recovery: f64,
}

impl Limits {
    fn new(low: f64, high: f64) -> Option<Self> {
        let range = high - low;
        let result = Self {
            low,
            high,
            low_entry: low + range * 0.10,
            high_entry: high - range * 0.05,
            low_recovery: low + range * 0.12,
            high_recovery: high - range * 0.07,
        };
        (low > 0.
            && result.low_entry > low
            && result.low_recovery > result.low_entry
            && result.high_recovery > result.low_recovery
            && result.high_entry > result.high_recovery
            && high > result.high_entry
            && [
                low,
                high,
                result.low_entry,
                result.high_entry,
                result.low_recovery,
                result.high_recovery,
            ]
            .into_iter()
            .all(f64::is_finite))
        .then_some(result)
    }
}

/// v1.0.x/1.1.x main.c compares pack settings directly. v1.2.0 through
/// v1.3.0 motor_data.c multiplies values <10 by si_battery_cells (6.05+).
/// Never apply the newer interpretation to older packages or infer cell count.
pub(super) fn resolve(snapshot: &c::Snapshot) -> Option<(u64, Limits)> {
    let config = &snapshot.configuration;
    if config.availability != c::configuration::Availability::Ready
        || matches!(
            config.pending,
            Some(
                c::configuration::PendingOperation::Schema
                    | c::configuration::PendingOperation::ReadCurrent
                    | c::configuration::PendingOperation::Write
            )
        )
    {
        return None;
    }
    let version = config.identity.as_ref()?.package_version;
    let per_cell = match version {
        "1.0.0" | "1.0.1" | "1.1.0" | "1.1.1" | "1.1.2" => false,
        "1.2.0" | "1.2.1" | "1.2.2" | "1.2.3" | "1.3.0" => true,
        _ => return None,
    };
    let current = config.current.as_ref()?;
    if current.source != c::configuration::ValueSource::Current {
        return None;
    }
    let firmware = &snapshot.discovery.selected.as_ref()?.identity;
    let convert = |key: &str| {
        let c::configuration_codec::ConfigurationValue::Decimal(value) =
            &current.values.iter().find(|(name, _)| name == key)?.1
        else {
            return None;
        };
        if per_cell && *value < 10. {
            if (firmware.major, firmware.minor) < (6, 5) {
                return None;
            }
            let count = snapshot
                .current_limits
                .as_ref()?
                .battery_series_cells
                .filter(|v| *v > 0)?;
            Some(value * f64::from(count))
        } else {
            Some(*value)
        }
    };
    Some((
        current.revision,
        Limits::new(convert("tiltback_lv")?, convert("tiltback_hv")?)?,
    ))
}

#[derive(Default)]
struct Gate {
    active: bool,
    entry: Option<u64>,
    recovery: Option<u64>,
}
impl Gate {
    fn observe(&mut self, now: u64, enter: bool, critical: bool, recover: bool, delay: u64) {
        if self.active {
            if recover {
                let start = self.recovery.get_or_insert(now);
                if now.saturating_sub(*start) >= 2000 {
                    *self = Self::default();
                }
            } else {
                self.recovery = None;
            }
        } else if enter {
            let start = self.entry.get_or_insert(now);
            if critical || now.saturating_sub(*start) >= delay {
                self.active = true;
                self.entry = None;
            }
        } else {
            self.entry = None;
        }
    }
}

/// Visual qualification is independent of riding/regen and the speech gates.
#[derive(Default)]
pub(super) struct VisualState {
    context: Option<(u64, Option<Limits>, Option<f64>)>,
    observed_at: Option<u64>,
    gates: [Gate; 2],
}
impl VisualState {
    pub(super) fn reset(&mut self) {
        self.observed_at = None;
        self.gates = Default::default();
    }
    pub(super) fn context(&mut self, context: Option<(u64, Option<Limits>, Option<f64>)>) {
        if self.context != context {
            self.context = context;
            self.reset();
        }
    }
    pub(super) fn observe(&mut self, now: u64, volts: Option<f64>) {
        if self
            .observed_at
            .is_some_and(|at| now.saturating_sub(at) >= 2000)
        {
            self.reset();
        }
        let Some((_, limits, full)) = self.context else {
            self.reset();
            return;
        };
        let Some(v) = volts.filter(|v| v.is_finite() && *v > 0.) else {
            self.reset();
            return;
        };
        self.observed_at = Some(now);
        if let Some(limits) = limits {
            let range = limits.high - limits.low;
            self.gates[0].observe(
                now,
                v < limits.low + 0.25 * range,
                v <= limits.low,
                v >= limits.low + 0.27 * range,
                2000,
            );
        } else {
            self.gates[0] = Gate::default();
        }
        if let Some(full) = full.filter(|f| f.is_finite() && *f > 0.) {
            // Entry has no delay or margin. Recovery uses 0.5% headroom and
            // two fresh continuous seconds, so full-boundary noise cannot flicker.
            self.gates[1].observe(now, v > full, true, v <= full * 0.995, 0);
        } else {
            self.gates[1] = Gate::default();
        }
    }
    pub(super) fn pending(&self) -> bool {
        self.gates
            .iter()
            .any(|gate| gate.active || gate.entry.is_some())
    }
    pub(super) fn warning(&self, now: u64) -> Option<Battery> {
        if self
            .observed_at
            .is_none_or(|at| now.saturating_sub(at) >= 2000)
        {
            return None;
        }
        if self.gates[1].active {
            Some(Battery::OverVoltage)
        } else if self.gates[0].active {
            Some(Battery::Low)
        } else {
            None
        }
    }
}

#[derive(Default)]
pub(super) struct State {
    context: Option<(u64, u64, Limits)>,
    observed_at: Option<u64>,
    gates: [Gate; 2],
    last_attempt: [Option<u64>; 2],
}
impl State {
    pub(super) fn reset(&mut self) {
        self.observed_at = None;
        self.gates = Default::default();
    }
    /// Returns true when admitted battery speech no longer has valid settings.
    pub(super) fn context(&mut self, context: Option<(u64, u64, Limits)>) -> bool {
        if self.context == context {
            return false;
        }
        self.context = context;
        self.reset();
        true
    }
    pub(super) fn observe(
        &mut self,
        now: u64,
        running: bool,
        volts: Option<f64>,
        current: Option<f64>,
    ) {
        if self
            .observed_at
            .is_some_and(|at| now.saturating_sub(at) >= 2000)
        {
            self.reset();
        }
        let Some((_, _, limits)) = self.context else {
            self.reset();
            return;
        };
        let Some(v) = volts.filter(|v| v.is_finite() && *v > 0.) else {
            self.reset();
            return;
        };
        if !running {
            self.reset();
            return;
        }
        self.observed_at = Some(now);
        self.gates[0].observe(
            now,
            v < limits.low_entry,
            v <= limits.low,
            v >= limits.low_recovery,
            2000,
        );
        if v >= limits.high || current.is_some_and(|a| a.is_finite() && a < 0.) {
            self.gates[1].observe(
                now,
                v > limits.high_entry,
                v >= limits.high,
                v <= limits.high_recovery,
                500,
            );
        } else {
            self.gates[1] = Gate::default();
        }
    }
    /// Fresh confirmed warning, independent of speech settings and cooldown.
    pub(super) fn warning(&self, now: u64) -> Option<Battery> {
        if self
            .observed_at
            .is_none_or(|at| now.saturating_sub(at) >= 2000)
        {
            return None;
        }
        // A high warning wins conflicting latched evidence even during its cooldown.
        if self.gates[1].active {
            Some(Battery::OverVoltage)
        } else if self.gates[0].active {
            Some(Battery::Low)
        } else {
            None
        }
    }
    pub(super) fn due(&mut self, now: u64) -> Option<Battery> {
        let warning = self.warning(now)?;
        let index = if warning == Battery::Low { 0 } else { 1 };
        self.last_attempt[index]
            .is_none_or(|at| now.saturating_sub(at) >= 60_000)
            .then_some(warning)
    }

    pub(super) fn attempted(&mut self, kind: Battery, now: u64) {
        self.last_attempt[if kind == Battery::Low { 0 } else { 1 }] = Some(now);
    }
    pub(super) fn pending(&self) -> bool {
        self.observed_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visual() -> VisualState {
        let mut state = VisualState::default();
        state.context(Some((1, Some(Limits::new(60., 86.).unwrap()), Some(84.))));
        state
    }
    #[test]
    fn visuals_use_earlier_low_margin_and_continuous_recovery() {
        let mut s = visual();
        for t in [0, 1000, 2000] {
            s.observe(t, Some(66.5));
            assert_eq!(s.warning(t), None);
        }
        for t in [2100, 3100, 4099] {
            s.observe(t, Some(66.4));
            assert_eq!(s.warning(t), None);
        }
        s.observe(4100, Some(66.4));
        assert_eq!(s.warning(4100), Some(Battery::Low));
        s.observe(4200, Some(67.02));
        s.observe(5200, Some(66.9)); // Interrupted recovery restarts the timer.
        for t in [5300, 6300, 7299] {
            s.observe(t, Some(67.02));
            assert_eq!(s.warning(t), Some(Battery::Low));
        }
        s.observe(7300, Some(67.02));
        assert_eq!(s.warning(7300), None);
        s.observe(7400, Some(60.));
        assert_eq!(s.warning(7400), Some(Battery::Low));
    }
    #[test]
    fn visual_high_is_immediate_above_full_and_has_recovery_hysteresis() {
        let mut s = visual();
        s.observe(0, Some(84.));
        assert_eq!(s.warning(0), None);
        s.observe(1, Some(84.000001));
        assert_eq!(s.warning(1), Some(Battery::OverVoltage));
        s.observe(2, Some(84.));
        assert_eq!(s.warning(2), Some(Battery::OverVoltage));
        for t in [100, 1100, 2099] {
            s.observe(t, Some(84. * 0.995));
            assert_eq!(s.warning(t), Some(Battery::OverVoltage));
        }
        s.observe(2100, Some(84. * 0.995));
        assert_eq!(s.warning(2100), None);
    }
    #[test]
    fn visual_context_freshness_and_independent_limits_are_required() {
        let mut s = visual();
        s.context(Some((1, None, Some(84.))));
        s.observe(0, Some(59.));
        s.observe(1000, Some(59.));
        assert_eq!(s.warning(1000), None);
        s.observe(1100, Some(85.));
        assert_eq!(s.warning(1100), Some(Battery::OverVoltage));
        s.context(Some((1, Some(Limits::new(60., 86.).unwrap()), None)));
        s.observe(1200, Some(85.));
        assert_eq!(s.warning(1200), None);
        s.observe(1300, Some(59.));
        assert_eq!(s.warning(1300), Some(Battery::Low));
        assert_eq!(s.warning(3300), None);
        s.observe(3300, Some(66.)); // Cannot bridge the gap.
        assert_eq!(s.warning(3300), None);
        s.observe(3400, None);
        s.observe(4400, Some(66.));
        assert_eq!(s.warning(4400), None);
        s.context(Some((2, Some(Limits::new(60., 86.).unwrap()), None)));
        s.observe(5400, Some(66.));
        assert_eq!(s.warning(5400), None);
        s.observe(6400, Some(66.));
        assert_eq!(s.warning(6400), None);
        s.observe(7400, Some(66.));
        assert_eq!(s.warning(7400), Some(Battery::Low));
        for v in [f64::NAN, f64::INFINITY, 0., -1.] {
            s.observe(7500, Some(v));
            assert_eq!(s.warning(7500), None);
        }
        // Proportional boundaries scale, including profile-derived high.
        s.context(Some((
            3,
            Some(Limits::new(120., 172.).unwrap()),
            Some(168.),
        )));
        for t in [8000, 9000, 10000] {
            s.observe(t, Some(132.8));
        }
        assert_eq!(s.warning(10000), Some(Battery::Low));
        s.observe(10001, Some(168.001));
        assert_eq!(s.warning(10001), Some(Battery::OverVoltage));
    }

    fn state() -> State {
        let mut state = State::default();
        state.context(Some((1, 1, Limits::new(60., 86.).unwrap())));
        state
    }
    #[test]
    fn resolution_uses_supported_current_values_and_controller_count_only() {
        let mut snapshot = c::ConnectionApplicationCoordinator::default().snapshot();
        snapshot.configuration.availability = c::configuration::Availability::Ready;
        snapshot.configuration.identity = Some(c::configuration::SchemaIdentity {
            package_name: "refloat",
            package_version: "1.2.0",
            fingerprint: "test",
            wire_signature: 1,
        });
        snapshot.configuration.current = Some(c::configuration::LoadedValues {
            revision: 7,
            source: c::configuration::ValueSource::Current,
            values: vec![
                (
                    "tiltback_lv".into(),
                    c::configuration_codec::ConfigurationValue::Decimal(60.),
                ),
                (
                    "tiltback_hv".into(),
                    c::configuration_codec::ConfigurationValue::Decimal(86.),
                ),
            ],
        });
        snapshot.discovery.selected = Some(c::discovery::RoutedIdentity {
            route: c::discovery::Route::Direct,
            identity: c::firmware::FirmwareIdentity {
                major: 6,
                minor: 5,
                hardware_name: "test".into(),
                firmware_name: None,
                controller_uuid: [1; 12],
                test_version: 0,
                hardware_type: 0,
                custom_configuration_count: 1,
                hardware_configuration_crc: None,
            },
        });
        let expected = Limits::new(60., 86.).unwrap();
        for version in [
            "1.0.0", "1.0.1", "1.1.0", "1.1.1", "1.1.2", "1.2.0", "1.2.1", "1.2.2", "1.2.3",
            "1.3.0",
        ] {
            snapshot
                .configuration
                .identity
                .as_mut()
                .unwrap()
                .package_version = version;
            assert_eq!(resolve(&snapshot), Some((7, expected)), "{version}");
        }
        snapshot.configuration.current.as_mut().unwrap().values = vec![
            (
                "tiltback_lv".into(),
                c::configuration_codec::ConfigurationValue::Decimal(3.),
            ),
            (
                "tiltback_hv".into(),
                c::configuration_codec::ConfigurationValue::Decimal(4.3),
            ),
        ];
        assert!(resolve(&snapshot).is_none());
        snapshot.current_limits = Some(c::controller_data::CurrentLimits {
            motor_current_max_a: 100.,
            motor_current_min_a: -100.,
            battery_current_max_a: 50.,
            battery_current_min_a: -20.,
            battery_series_cells: Some(20),
        });
        assert_eq!(resolve(&snapshot), Some((7, expected)));
        snapshot.discovery.selected.as_mut().unwrap().identity.minor = 2;
        assert!(resolve(&snapshot).is_none());
        snapshot.discovery.selected.as_mut().unwrap().identity.minor = 5;
        snapshot.configuration.pending = Some(c::configuration::PendingOperation::ReadCurrent);
        assert!(resolve(&snapshot).is_none());
        snapshot.configuration.pending = None;
        snapshot.configuration.current.as_mut().unwrap().source =
            c::configuration::ValueSource::Default;
        assert!(resolve(&snapshot).is_none());
        snapshot.configuration.current.as_mut().unwrap().source =
            c::configuration::ValueSource::Current;
        snapshot
            .configuration
            .identity
            .as_mut()
            .unwrap()
            .package_version = "unknown";
        assert!(resolve(&snapshot).is_none());
    }

    #[test]
    fn boundaries_scale_and_invalid_pairs_fail() {
        let l = Limits::new(60., 86.).unwrap();
        assert_eq!(l.low_entry, 62.6);
        assert_eq!(l.high_entry, 84.7);
        assert_eq!(l.low_recovery, 63.12);
        assert_eq!(l.high_recovery, 84.18);
        assert_eq!(Limits::new(120., 172.).unwrap().low_entry, 2. * l.low_entry);
        for (a, b) in [
            (0., 86.),
            (86., 60.),
            (60., 60.),
            (f64::NAN, 86.),
            (60., f64::INFINITY),
        ] {
            assert!(Limits::new(a, b).is_none());
        }
    }
    #[test]
    fn low_confirmation_recovery_and_cooldown_survive_resets() {
        let mut s = state();
        for t in [0, 1000, 1999] {
            s.observe(t, true, Some(62.), None);
            assert_eq!(s.due(t), None);
        }
        s.observe(2000, true, Some(62.), None);
        assert_eq!(s.due(2000), Some(Battery::Low));
        s.attempted(Battery::Low, 2000);
        s.observe(2100, false, Some(59.), None);
        assert_eq!(s.due(2100), None);
        s.observe(61_999, true, Some(59.), None);
        assert_eq!(s.due(61_999), None);
        s.observe(62_000, true, Some(59.), None);
        assert_eq!(s.due(62_000), Some(Battery::Low));
        for t in [62_001, 63_001, 64_001] {
            s.observe(t, true, Some(64.), None);
        }
        assert_eq!(s.due(64_001), None);
    }
    #[test]
    fn high_needs_regen_except_at_critical_boundary() {
        let mut s = state();
        s.observe(0, true, Some(85.), Some(1.));
        assert_eq!(s.due(0), None);
        s.observe(100, true, Some(85.), Some(-1.));
        s.observe(599, true, Some(85.), Some(-1.));
        assert_eq!(s.due(599), None);
        s.observe(600, true, Some(85.), Some(-1.));
        assert_eq!(s.due(600), Some(Battery::OverVoltage));
        s.observe(601, true, Some(85.), None);
        assert_eq!(s.due(601), None);
        s.observe(602, true, Some(86.), None);
        assert_eq!(s.due(602), Some(Battery::OverVoltage));
        s.observe(603, false, Some(87.), Some(-1.));
        assert_eq!(s.due(603), None);
    }
    #[test]
    fn recovery_requires_continuous_evidence_and_high_has_its_own_cooldown() {
        let mut s = state();
        s.observe(0, true, Some(59.), None);
        assert_eq!(s.due(0), Some(Battery::Low));
        s.attempted(Battery::Low, 0);
        s.observe(100, true, Some(86.), None);
        assert_eq!(s.due(100), Some(Battery::OverVoltage));
        s.attempted(Battery::OverVoltage, 100);
        // Resetting settings context retains each independently bounded attempt.
        s.context(None);
        s.context(Some((2, 2, Limits::new(60., 86.).unwrap())));
        s.observe(60_000, true, Some(86.), None);
        assert_eq!(s.due(60_000), None);
        s.observe(60_100, true, Some(86.), None);
        assert_eq!(s.due(60_100), Some(Battery::OverVoltage));
        // A hysteresis-band observation retains a latch; brief recovery cannot clear it.
        let mut s = state();
        s.observe(0, true, Some(59.), None);
        s.observe(100, true, Some(62.8), None);
        assert_eq!(s.due(100), Some(Battery::Low));
        s.observe(200, true, Some(64.), None);
        s.observe(1000, true, Some(62.8), None);
        s.observe(1100, true, Some(64.), None);
        s.observe(2100, true, Some(64.), None);
        assert_eq!(s.due(2100), Some(Battery::Low));
        s.observe(3100, true, Some(64.), None);
        assert_eq!(s.due(3100), None);
    }

    #[test]
    fn high_priority_does_not_fall_back_to_a_conflicting_low_latch() {
        let mut s = state();
        s.observe(0, true, Some(59.), None);
        s.observe(100, true, Some(86.), None);
        assert_eq!(s.due(100), Some(Battery::OverVoltage));
        s.attempted(Battery::OverVoltage, 100);
        assert_eq!(s.due(101), None);
    }

    #[test]
    fn equality_spikes_gaps_and_context_changes_cannot_qualify() {
        let mut s = state();
        for t in [0, 1000, 2000] {
            s.observe(t, true, Some(62.6), None);
        }
        assert_eq!(s.due(2000), None);
        s.observe(2100, true, Some(62.), None);
        s.observe(4100, true, Some(62.), None);
        assert_eq!(s.due(4100), None);
        s.observe(4200, true, Some(59.), None);
        assert_eq!(s.due(6200), None);
        s.context(None);
        s.observe(6300, true, Some(59.), None);
        assert_eq!(s.due(6300), None);
    }
}
