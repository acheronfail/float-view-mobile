use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryFamily {
    LegacyFixed,
    Dynamic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireProfile {
    LegacyFixed,
    Dynamic,
    DynamicAlerts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LightingProfile {
    LegacyCommand202,
    StableCommand20,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlProfile {
    pub lighting: LightingProfile,
    pub fatal_clear_available: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompatibilityEntry {
    pub source_version: &'static str,
    pub wire_version: &'static str,
    pub profile: WireProfile,
}

impl CompatibilityEntry {
    #[must_use]
    pub const fn family(self) -> TelemetryFamily {
        match self.profile {
            WireProfile::LegacyFixed => TelemetryFamily::LegacyFixed,
            WireProfile::Dynamic | WireProfile::DynamicAlerts => TelemetryFamily::Dynamic,
        }
    }

    #[must_use]
    pub const fn control_profile(self) -> ControlProfile {
        match self.profile {
            WireProfile::LegacyFixed | WireProfile::Dynamic => ControlProfile {
                lighting: LightingProfile::LegacyCommand202,
                fatal_clear_available: false,
            },
            WireProfile::DynamicAlerts => ControlProfile {
                lighting: LightingProfile::StableCommand20,
                fatal_clear_available: true,
            },
        }
    }
}

pub const COMPATIBILITY: &[CompatibilityEntry] = &[
    CompatibilityEntry {
        source_version: "1.0.0",
        wire_version: "1.0",
        profile: WireProfile::LegacyFixed,
    },
    CompatibilityEntry {
        source_version: "1.0.1",
        wire_version: "1.0",
        profile: WireProfile::LegacyFixed,
    },
    CompatibilityEntry {
        source_version: "1.1.0",
        wire_version: "1.1.0",
        profile: WireProfile::Dynamic,
    },
    CompatibilityEntry {
        source_version: "1.1.1",
        wire_version: "1.1.1",
        profile: WireProfile::Dynamic,
    },
    CompatibilityEntry {
        source_version: "1.1.2",
        wire_version: "1.1.2",
        profile: WireProfile::Dynamic,
    },
    CompatibilityEntry {
        source_version: "1.2.0",
        wire_version: "1.2.0",
        profile: WireProfile::DynamicAlerts,
    },
    CompatibilityEntry {
        source_version: "1.2.1",
        wire_version: "1.2.1",
        profile: WireProfile::DynamicAlerts,
    },
    CompatibilityEntry {
        source_version: "1.2.2",
        wire_version: "1.2.2",
        profile: WireProfile::DynamicAlerts,
    },
    CompatibilityEntry {
        source_version: "1.2.3",
        wire_version: "1.2.3",
        profile: WireProfile::DynamicAlerts,
    },
];

pub const REQUIRED_FIELDS: &[&str] = &[
    "motor.speed",
    "motor.erpm",
    "motor.current",
    "motor.dir_current",
    "motor.filt_current",
    "motor.duty_cycle",
    "motor.batt_voltage",
    "motor.batt_current",
    "motor.mosfet_temp",
    "motor.motor_temp",
    "imu.pitch",
    "imu.balance_pitch",
    "imu.roll",
    "footpad.adc1",
    "footpad.adc2",
    "remote.input",
];

pub const REQUIRED_RUNTIME_FIELDS: &[&str] = &[
    "setpoint",
    "atr.setpoint",
    "brake_tilt.setpoint",
    "torque_tilt.setpoint",
    "turn_tilt.setpoint",
    "remote.setpoint",
    "balance_current",
    "atr.accel_diff",
    "atr.speed_boost",
    "booster.current",
];

#[must_use]
pub fn compatibility(wire_version: &str) -> Option<CompatibilityEntry> {
    COMPATIBILITY
        .iter()
        .copied()
        .find(|entry| entry.wire_version == wire_version)
}

#[must_use]
pub fn supports_dynamic_layout(fields: &[String], runtime_fields: &[String]) -> bool {
    let fields: BTreeSet<&str> = fields.iter().map(String::as_str).collect();
    let runtime: BTreeSet<&str> = runtime_fields.iter().map(String::as_str).collect();
    REQUIRED_FIELDS
        .iter()
        .all(|required| fields.contains(required))
        && REQUIRED_RUNTIME_FIELDS
            .iter()
            .all(|required| runtime.contains(required))
}
