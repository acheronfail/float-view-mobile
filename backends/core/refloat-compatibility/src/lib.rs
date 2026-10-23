//! Strict, bounded Refloat INFO identity and compatibility admission.
//!
//! This unselected safe candidate deliberately knows nothing about VESC
//! framing, CAN routing, request scheduling, connection state, telemetry
//! values, host bridges, persistence, or application behavior.

#![forbid(unsafe_code)]

use std::fmt;

pub use float_view_refloat_dynamic_telemetry::MAX_LAYOUT_RECORD_BYTES;
use float_view_refloat_dynamic_telemetry::parse_layout_record;

/// VESC `COMM_CUSTOM_APP_DATA`.
pub const CUSTOM_APP_DATA_COMMAND: u8 = 36;
/// Refloat custom-app-data package identifier.
pub const REFLOAT_PACKAGE_ID: u8 = 101;
/// Refloat package INFO command.
pub const INFO_COMMAND: u8 = 0;
/// Highest source-defined INFO protocol requested by the hosts.
pub const INFO_PROTOCOL_VERSION: u8 = 2;
/// No optional INFO groups are requested.
pub const INFO_FLAGS: u8 = 0;
/// Existing maximum unframed VESC payload extent.
pub const MAX_PAYLOAD_BYTES: usize = 4_096;
/// Maximum identifiers across both dynamic-layout groups.
pub const MAX_LAYOUT_FIELDS: usize = 64;
/// Maximum encoded UTF-8 bytes in one dynamic identifier.
pub const MAX_IDENTIFIER_BYTES: usize = 64;
/// Maximum aggregate encoded bytes across dynamic identifiers.
pub const MAX_AGGREGATE_IDENTIFIER_BYTES: usize = 4_096;
/// Version of the canonical compatibility bridge record.
pub const RECORD_VERSION: u8 = 2;
/// Record kind for one accepted INFO compatibility decision.
pub const COMPATIBILITY_RECORD_KIND: u8 = 1;
/// Exact extent of the canonical compatibility record.
pub const COMPATIBILITY_RECORD_BYTES: usize = 12 + VERSION_SUFFIX_BYTES;
/// Source-defined width of the zero-padded INFO-v2 version suffix.
pub const VERSION_SUFFIX_BYTES: usize = 20;

const INFO_V2_RESPONSE_BYTES: usize = 61;
const INFO_V2_MAJOR_OFFSET: usize = 25;
const INFO_V2_MINOR_OFFSET: usize = 26;
const INFO_V2_PATCH_OFFSET: usize = 27;
const INFO_V2_CAPABILITIES_OFFSET: usize = 56;
/// Refloat INFO capability bit for command-33 GNSS values.
pub const INFO_CAPABILITY_GNSS: u32 = 1 << 2;

/// Exact unframed INFO request selected by both native hosts.
pub const INFO_REQUEST: [u8; 5] = [
    CUSTOM_APP_DATA_COMMAND,
    REFLOAT_PACKAGE_ID,
    INFO_COMMAND,
    INFO_PROTOCOL_VERSION,
    INFO_FLAGS,
];

/// Required always-present Refloat dynamic telemetry identifiers.
pub const REQUIRED_FIELDS: [&str; 16] = [
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

/// Required running-only Refloat dynamic telemetry identifiers.
pub const REQUIRED_RUNTIME_FIELDS: [&str; 10] = [
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

/// Validated ASCII version suffix, stored exactly as the fixed-width wire field.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct VersionSuffix([u8; VERSION_SUFFIX_BYTES]);

impl VersionSuffix {
    pub const EMPTY: Self = Self([0; VERSION_SUFFIX_BYTES]);

    /// Construct a suffix without padding; an empty suffix denotes a stable release.
    pub const fn new(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() > VERSION_SUFFIX_BYTES {
            return None;
        }
        let mut value = [0; VERSION_SUFFIX_BYTES];
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            if !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b'-' | b'+') {
                return None;
            }
            value[index] = byte;
            index += 1;
        }
        Some(Self(value))
    }

    fn from_wire(bytes: &[u8]) -> Result<Self, CompatibilityError> {
        if bytes.len() != VERSION_SUFFIX_BYTES {
            return Err(CompatibilityError::InvalidIdentifier);
        }
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        if bytes[end..].iter().any(|byte| *byte != 0) {
            return Err(CompatibilityError::InvalidIdentifier);
        }
        let text = std::str::from_utf8(&bytes[..end])
            .map_err(|_| CompatibilityError::InvalidIdentifier)?;
        Self::new(text).ok_or(CompatibilityError::InvalidIdentifier)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        let end = self
            .0
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(VERSION_SUFFIX_BYTES);
        std::str::from_utf8(&self.0[..end]).expect("validated ASCII suffix")
    }
}

/// A source package release represented independently from its wire identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceVersion {
    /// Fixed-width source suffix; empty for a stable release.
    pub suffix: VersionSuffix,
    /// Source major version.
    pub major: u8,
    /// Source minor version.
    pub minor: u8,
    /// Source patch version.
    pub patch: u8,
}

impl SourceVersion {
    /// Preserve the complete source-defined suffix in this identity.
    #[must_use]
    pub const fn with_suffix(mut self, suffix: VersionSuffix) -> Self {
        self.suffix = suffix;
        self
    }

    /// Construct one exact source release.
    #[must_use]
    pub const fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self {
            suffix: VersionSuffix::EMPTY,
            major,
            minor,
            patch,
        }
    }
}

/// A reported Refloat wire identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WireVersion {
    /// Fixed-width source suffix; empty for a stable release.
    pub suffix: VersionSuffix,
    /// Reported major version.
    pub major: u8,
    /// Reported minor version.
    pub minor: u8,
    /// Reported patch, absent from legacy compact identity.
    pub patch: Option<u8>,
}

impl WireVersion {
    /// Preserve the complete source-defined suffix in this identity.
    #[must_use]
    pub const fn with_suffix(mut self, suffix: VersionSuffix) -> Self {
        self.suffix = suffix;
        self
    }

    /// Construct a legacy compact major/minor identity without a patch.
    #[must_use]
    pub const fn legacy(major: u8, minor: u8) -> Self {
        Self {
            suffix: VersionSuffix::EMPTY,
            major,
            minor,
            patch: None,
        }
    }

    /// Construct a version-2 major/minor/patch identity.
    #[must_use]
    pub const fn patched(major: u8, minor: u8, patch: u8) -> Self {
        Self {
            suffix: VersionSuffix::EMPTY,
            major,
            minor,
            patch: Some(patch),
        }
    }
}

impl fmt::Display for SourceVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if self.suffix != VersionSuffix::EMPTY {
            write!(formatter, "-{}", self.suffix.as_str())?;
        }
        Ok(())
    }
}

impl fmt::Display for WireVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)?;
        if let Some(patch) = self.patch {
            write!(formatter, ".{patch}")?;
        }
        if self.suffix != VersionSuffix::EMPTY {
            write!(formatter, "-{}", self.suffix.as_str())?;
        }
        Ok(())
    }
}

/// Refloat telemetry wire generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryProfile {
    /// Fixed-layout compact telemetry.
    LegacyFixed,
    /// Negotiated dynamic telemetry without the active-alert tail.
    Dynamic,
    /// Negotiated dynamic telemetry with the active-alert tail.
    DynamicAlerts,
    /// Fixed-mask selectable command-33 telemetry introduced by Refloat 1.3.0.
    Selectable,
}

/// High-level telemetry family selected for discovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryFamily {
    /// Fixed-layout compact telemetry.
    LegacyFixed,
    /// Negotiated dynamic telemetry.
    Dynamic,
    /// Fixed-mask selectable telemetry; no command-32 layout negotiation.
    Selectable,
}

impl TelemetryProfile {
    /// Family used by the controller-discovery state machine.
    #[must_use]
    pub const fn family(self) -> TelemetryFamily {
        match self {
            Self::LegacyFixed => TelemetryFamily::LegacyFixed,
            Self::Dynamic | Self::DynamicAlerts => TelemetryFamily::Dynamic,
            Self::Selectable => TelemetryFamily::Selectable,
        }
    }
}

/// Refloat lighting command generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LightingProfile {
    /// Unstable legacy command 202.
    LegacyCommand202,
    /// Stable command 20.
    StableCommand20,
}

/// Control capabilities derived only from exact compatibility policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlProfile {
    /// Selected lighting command generation.
    pub lighting: LightingProfile,
    /// Whether fatal-latch clear is available.
    pub fatal_clear_available: bool,
}

/// One exact source release and its reported wire/profile mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompatibilityEntry {
    /// Whether this audited release reports IMU timeouts.
    pub imu_timeout_available: bool,
    /// Audited source package release.
    pub source: SourceVersion,
    /// Identity reported by that source release.
    pub wire: WireVersion,
    /// Telemetry wire generation.
    pub telemetry: TelemetryProfile,
    /// Refloat control capabilities.
    pub control: ControlProfile,
}

const LEGACY_CONTROL: ControlProfile = ControlProfile {
    lighting: LightingProfile::LegacyCommand202,
    fatal_clear_available: false,
};
const STABLE_CONTROL: ControlProfile = ControlProfile {
    lighting: LightingProfile::StableCommand20,
    fatal_clear_available: true,
};

/// Exact audited source-release compatibility catalogue.
pub const COMPATIBILITY_ENTRIES: [CompatibilityEntry; 11] = [
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 0, 0),
        wire: WireVersion::legacy(1, 0),
        telemetry: TelemetryProfile::LegacyFixed,
        control: LEGACY_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 0, 1),
        wire: WireVersion::legacy(1, 0),
        telemetry: TelemetryProfile::LegacyFixed,
        control: LEGACY_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 1, 0),
        wire: WireVersion::patched(1, 1, 0),
        telemetry: TelemetryProfile::Dynamic,
        control: LEGACY_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 1, 1),
        wire: WireVersion::patched(1, 1, 1),
        telemetry: TelemetryProfile::Dynamic,
        control: LEGACY_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 1, 2),
        wire: WireVersion::patched(1, 1, 2),
        telemetry: TelemetryProfile::Dynamic,
        control: LEGACY_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 2, 0),
        wire: WireVersion::patched(1, 2, 0),
        telemetry: TelemetryProfile::DynamicAlerts,
        control: STABLE_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 2, 1),
        wire: WireVersion::patched(1, 2, 1),
        telemetry: TelemetryProfile::DynamicAlerts,
        control: STABLE_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 2, 2),
        wire: WireVersion::patched(1, 2, 2),
        telemetry: TelemetryProfile::DynamicAlerts,
        control: STABLE_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 2, 3),
        wire: WireVersion::patched(1, 2, 3),
        telemetry: TelemetryProfile::DynamicAlerts,
        control: STABLE_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: false,
        source: SourceVersion::new(1, 3, 0),
        wire: WireVersion::patched(1, 3, 0),
        telemetry: TelemetryProfile::Selectable,
        control: STABLE_CONTROL,
    },
    CompatibilityEntry {
        imu_timeout_available: true,
        source: SourceVersion::new(1, 3, 1)
            .with_suffix(VersionSuffix::new("beta1").expect("valid catalogue suffix")),
        wire: WireVersion::patched(1, 3, 1)
            .with_suffix(VersionSuffix::new("beta1").expect("valid catalogue suffix")),
        telemetry: TelemetryProfile::Selectable,
        control: STABLE_CONTROL,
    },
];

/// Closed compatibility result for one reported wire identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompatibilityProfile {
    /// Whether guarded writes require confirmed inactive IMU timeout state.
    pub imu_timeout_available: bool,
    /// Telemetry wire generation.
    pub telemetry: TelemetryProfile,
    /// Refloat control capabilities.
    pub control: ControlProfile,
}

/// Stable bounded candidate failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityError {
    /// Input exceeds an established fixed bound.
    InputTooLarge,
    /// INFO input ends before the required source-defined prefix.
    TruncatedInfo,
    /// The VESC/Refloat/INFO envelope is not selected by this component.
    UnsupportedEnvelope,
    /// The INFO response protocol byte has no defined identity form.
    UnsupportedInfoProtocol,
    /// A structurally valid INFO identity is outside the exact catalogue.
    UnsupportedRelease,
    /// A canonical compatibility record is malformed or inconsistent.
    InvalidCompatibilityRecord,
    /// A canonical dynamic-layout record is malformed.
    InvalidLayoutRecord,
    /// Combined layout field count exceeds the fixed bound.
    TooManyIdentifiers,
    /// A dynamic identifier is empty.
    EmptyIdentifier,
    /// A dynamic identifier exceeds its fixed encoded-byte bound.
    IdentifierTooLong,
    /// A dynamic identifier is not valid UTF-8.
    InvalidIdentifier,
    /// Aggregate dynamic identifier bytes exceed the fixed bound.
    TooManyIdentifierBytes,
    /// A dynamic identifier occurs more than once across both groups.
    DuplicateIdentifier,
    /// The always-present group lacks a required identifier.
    MissingRequiredIdentifier,
    /// The running-only group lacks a required identifier.
    MissingRequiredRuntimeIdentifier,
}

impl fmt::Display for CompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InputTooLarge => "Refloat compatibility input is too large",
            Self::TruncatedInfo => "truncated Refloat INFO response",
            Self::UnsupportedEnvelope => "unsupported Refloat INFO envelope",
            Self::UnsupportedInfoProtocol => "unsupported Refloat INFO protocol",
            Self::UnsupportedRelease => "unsupported Refloat release",
            Self::InvalidCompatibilityRecord => "invalid Refloat compatibility record",
            Self::InvalidLayoutRecord => "invalid Refloat layout record",
            Self::TooManyIdentifiers => "too many Refloat layout identifiers",
            Self::EmptyIdentifier => "empty Refloat layout identifier",
            Self::IdentifierTooLong => "Refloat layout identifier is too long",
            Self::InvalidIdentifier => "invalid Refloat layout identifier",
            Self::TooManyIdentifierBytes => "too many Refloat layout identifier bytes",
            Self::DuplicateIdentifier => "duplicate Refloat layout identifier",
            Self::MissingRequiredIdentifier => "missing required Refloat layout identifier",
            Self::MissingRequiredRuntimeIdentifier => {
                "missing required Refloat runtime layout identifier"
            }
        };
        formatter.write_str(message)
    }
}

/// Fixed canonical record for one accepted compatibility decision.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CompatibilityRecord([u8; COMPATIBILITY_RECORD_BYTES]);

impl CompatibilityRecord {
    /// Borrow the complete canonical bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; COMPATIBILITY_RECORD_BYTES] {
        &self.0
    }
}

impl fmt::Debug for CompatibilityRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompatibilityRecord")
            .field("bytes", &COMPATIBILITY_RECORD_BYTES)
            .finish()
    }
}

/// Closed result for atomic INFO decoding and compatibility selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityRecordOutcome {
    /// One complete accepted decision.
    Record(CompatibilityRecord),
    /// Input exceeded the fixed payload bound.
    InputTooLarge,
    /// Input ended before the required identity prefix.
    TruncatedInfo,
    /// The enclosing command/package/operation was not INFO.
    UnsupportedEnvelope,
    /// The INFO protocol byte has no supported structural form.
    UnsupportedInfoProtocol,
    /// The decoded identity is outside the exact supported catalogue.
    UnsupportedRelease,
    /// An impossible internal representation was encountered.
    InternalFailure,
}

impl std::error::Error for CompatibilityError {}

/// Return the exact unframed Refloat INFO request.
#[must_use]
pub const fn info_request() -> [u8; 5] {
    INFO_REQUEST
}

/// Decode one complete bounded INFO identity without admitting the release.
pub fn decode_info_identity(payload: &[u8]) -> Result<WireVersion, CompatibilityError> {
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(CompatibilityError::InputTooLarge);
    }
    if payload.len() < 4 {
        return Err(CompatibilityError::TruncatedInfo);
    }
    if payload[..3] != [CUSTOM_APP_DATA_COMMAND, REFLOAT_PACKAGE_ID, INFO_COMMAND] {
        return Err(CompatibilityError::UnsupportedEnvelope);
    }

    let response_version = payload[3];
    if response_version == 10 {
        return Ok(WireVersion::legacy(1, 0));
    }
    if response_version > 10 {
        return Ok(WireVersion::legacy(
            response_version / 10,
            response_version % 10,
        ));
    }
    if response_version != INFO_PROTOCOL_VERSION {
        return Err(CompatibilityError::UnsupportedInfoProtocol);
    }
    if payload.len() < INFO_V2_RESPONSE_BYTES {
        return Err(CompatibilityError::TruncatedInfo);
    }

    Ok(WireVersion::patched(
        payload[INFO_V2_MAJOR_OFFSET],
        payload[INFO_V2_MINOR_OFFSET],
        payload[INFO_V2_PATCH_OFFSET],
    )
    .with_suffix(VersionSuffix::from_wire(&payload[28..48])?))
}

/// Decode an INFO identity eligible for exact audited-release lookup.
pub fn decode_info(payload: &[u8]) -> Result<WireVersion, CompatibilityError> {
    let version = decode_info_identity(payload)?;
    if version.suffix != VersionSuffix::EMPTY && compatibility(version).is_none() {
        return Err(CompatibilityError::UnsupportedRelease);
    }
    Ok(version)
}

/// Decode the INFO-v2 capability word without interpreting device identity.
pub fn decode_info_capabilities(payload: &[u8]) -> Result<u32, CompatibilityError> {
    let _ = decode_info(payload)?;
    if payload[3] != INFO_PROTOCOL_VERSION {
        return Ok(0);
    }
    Ok(u32::from_be_bytes(
        payload[INFO_V2_CAPABILITIES_OFFSET..INFO_V2_CAPABILITIES_OFFSET + 4]
            .try_into()
            .expect("validated INFO-v2 extent"),
    ))
}

/// Parse only exact canonical wire-identity text, without semantic-version ranges.
#[must_use]
pub fn parse_wire_version(text: &str) -> Option<WireVersion> {
    COMPATIBILITY_ENTRIES
        .iter()
        .find(|entry| entry.wire.to_string() == text)
        .map(|entry| entry.wire)
}

/// Look up the exact supported profile for one structural wire identity.
#[must_use]
pub fn compatibility(version: WireVersion) -> Option<CompatibilityProfile> {
    COMPATIBILITY_ENTRIES
        .iter()
        .find(|entry| entry.wire == version)
        .map(|entry| CompatibilityProfile {
            imu_timeout_available: entry.imu_timeout_available,
            telemetry: entry.telemetry,
            control: entry.control,
        })
}

/// Decode one INFO response and atomically publish its exact supported decision.
#[must_use]
pub fn decode_info_record(payload: &[u8]) -> CompatibilityRecordOutcome {
    let version = match decode_info(payload) {
        Ok(version) => version,
        Err(CompatibilityError::InputTooLarge) => return CompatibilityRecordOutcome::InputTooLarge,
        Err(CompatibilityError::TruncatedInfo) => return CompatibilityRecordOutcome::TruncatedInfo,
        Err(CompatibilityError::UnsupportedEnvelope) => {
            return CompatibilityRecordOutcome::UnsupportedEnvelope;
        }
        Err(CompatibilityError::UnsupportedInfoProtocol) => {
            return CompatibilityRecordOutcome::UnsupportedInfoProtocol;
        }
        Err(CompatibilityError::UnsupportedRelease) => {
            return CompatibilityRecordOutcome::UnsupportedRelease;
        }
        Err(_) => return CompatibilityRecordOutcome::InternalFailure,
    };
    let Some(profile) = compatibility(version) else {
        return CompatibilityRecordOutcome::UnsupportedRelease;
    };
    CompatibilityRecordOutcome::Record(serialize_compatibility(version, profile))
}

/// Parse and fully validate one canonical compatibility record.
pub fn parse_compatibility_record(
    record: &[u8],
) -> Result<(WireVersion, CompatibilityProfile), CompatibilityError> {
    if record.len() != COMPATIBILITY_RECORD_BYTES
        || record[0] != RECORD_VERSION
        || record[1] != COMPATIBILITY_RECORD_KIND
        || record[4] > 1
        || record[9] > 1
        || record[10] > 1
        || record[11] != 0
        || (record[4] == 0 && record[5] != 0)
    {
        return Err(CompatibilityError::InvalidCompatibilityRecord);
    }
    let version = WireVersion {
        suffix: VersionSuffix::from_wire(&record[12..])
            .map_err(|_| CompatibilityError::InvalidCompatibilityRecord)?,
        major: record[2],
        minor: record[3],
        patch: (record[4] == 1).then_some(record[5]),
    };
    let telemetry = match record[6] {
        1 => TelemetryProfile::LegacyFixed,
        2 => TelemetryProfile::Dynamic,
        3 => TelemetryProfile::DynamicAlerts,
        4 => TelemetryProfile::Selectable,
        _ => return Err(CompatibilityError::InvalidCompatibilityRecord),
    };
    let family = match record[7] {
        1 => TelemetryFamily::LegacyFixed,
        2 => TelemetryFamily::Dynamic,
        3 => TelemetryFamily::Selectable,
        _ => return Err(CompatibilityError::InvalidCompatibilityRecord),
    };
    let lighting = match record[8] {
        1 => LightingProfile::LegacyCommand202,
        2 => LightingProfile::StableCommand20,
        _ => return Err(CompatibilityError::InvalidCompatibilityRecord),
    };
    let profile = CompatibilityProfile {
        imu_timeout_available: record[10] == 1,
        telemetry,
        control: ControlProfile {
            lighting,
            fatal_clear_available: record[9] == 1,
        },
    };
    if telemetry.family() != family || compatibility(version) != Some(profile) {
        return Err(CompatibilityError::InvalidCompatibilityRecord);
    }
    Ok((version, profile))
}

/// Admit one canonical Rust dynamic-layout record against compatibility policy.
pub fn admit_dynamic_layout_record(record: &[u8]) -> Result<(), CompatibilityError> {
    let layout =
        parse_layout_record(record).map_err(|_| CompatibilityError::InvalidLayoutRecord)?;
    let fields: Vec<&[u8]> = layout
        .always_present()
        .iter()
        .map(|value| value.as_bytes())
        .collect();
    let runtime_fields: Vec<&[u8]> = layout
        .running_only()
        .iter()
        .map(|value| value.as_bytes())
        .collect();
    admit_dynamic_layout(&fields, &runtime_fields)
}

fn serialize_compatibility(
    version: WireVersion,
    profile: CompatibilityProfile,
) -> CompatibilityRecord {
    let (patch_present, patch) = version.patch.map_or((0, 0), |value| (1, value));
    let telemetry = match profile.telemetry {
        TelemetryProfile::LegacyFixed => 1,
        TelemetryProfile::Dynamic => 2,
        TelemetryProfile::DynamicAlerts => 3,
        TelemetryProfile::Selectable => 4,
    };
    let family = match profile.telemetry.family() {
        TelemetryFamily::LegacyFixed => 1,
        TelemetryFamily::Dynamic => 2,
        TelemetryFamily::Selectable => 3,
    };
    let lighting = match profile.control.lighting {
        LightingProfile::LegacyCommand202 => 1,
        LightingProfile::StableCommand20 => 2,
    };
    let mut record = [0; COMPATIBILITY_RECORD_BYTES];
    record[..12].copy_from_slice(&[
        RECORD_VERSION,
        COMPATIBILITY_RECORD_KIND,
        version.major,
        version.minor,
        patch_present,
        patch,
        telemetry,
        family,
        lighting,
        u8::from(profile.control.fatal_clear_available),
        u8::from(profile.imu_timeout_available),
        0,
    ]);
    record[12..].copy_from_slice(&version.suffix.0);
    CompatibilityRecord(record)
}

/// Validate whether bounded identifier groups satisfy dynamic admission policy.
pub fn admit_dynamic_layout(
    fields: &[&[u8]],
    runtime_fields: &[&[u8]],
) -> Result<(), CompatibilityError> {
    let count = fields
        .len()
        .checked_add(runtime_fields.len())
        .ok_or(CompatibilityError::TooManyIdentifiers)?;
    if count > MAX_LAYOUT_FIELDS {
        return Err(CompatibilityError::TooManyIdentifiers);
    }

    let mut aggregate_bytes = 0_usize;
    for (group_index, group) in [fields, runtime_fields].into_iter().enumerate() {
        for (index, identifier) in group.iter().copied().enumerate() {
            if identifier.is_empty() {
                return Err(CompatibilityError::EmptyIdentifier);
            }
            if identifier.len() > MAX_IDENTIFIER_BYTES {
                return Err(CompatibilityError::IdentifierTooLong);
            }
            std::str::from_utf8(identifier).map_err(|_| CompatibilityError::InvalidIdentifier)?;
            aggregate_bytes = aggregate_bytes
                .checked_add(identifier.len())
                .ok_or(CompatibilityError::TooManyIdentifierBytes)?;
            if aggregate_bytes > MAX_AGGREGATE_IDENTIFIER_BYTES {
                return Err(CompatibilityError::TooManyIdentifierBytes);
            }

            if group[..index].contains(&identifier)
                || (group_index == 1 && fields.contains(&identifier))
            {
                return Err(CompatibilityError::DuplicateIdentifier);
            }
        }
    }

    if !REQUIRED_FIELDS
        .iter()
        .all(|required| fields.contains(&required.as_bytes()))
    {
        return Err(CompatibilityError::MissingRequiredIdentifier);
    }
    if !REQUIRED_RUNTIME_FIELDS
        .iter()
        .all(|required| runtime_fields.contains(&required.as_bytes()))
    {
        return Err(CompatibilityError::MissingRequiredRuntimeIdentifier);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    const BETA_VERSION: WireVersion = WireVersion::patched(1, 3, 1)
        .with_suffix(VersionSuffix::new("beta1").expect("valid test suffix"));

    use super::*;
    use float_view_refloat_dynamic_telemetry::{LayoutRecordOutcome, decode_layout_to_record};

    #[test]
    fn suffix_bounds_and_wire_padding_are_validated() {
        assert_eq!(VersionSuffix::new(""), Some(VersionSuffix::EMPTY));
        for text in [
            "beta1",
            "beta2",
            "rc1",
            "dev.1+abc-2",
            "abcdefghijklmnopqrst",
        ] {
            let suffix = VersionSuffix::new(text).unwrap();
            assert_eq!(suffix.as_str(), text);
            assert_eq!(VersionSuffix::from_wire(&suffix.0), Ok(suffix));
            let version = WireVersion::patched(1, 3, 1).with_suffix(suffix);
            assert_eq!(version.to_string(), format!("1.3.1-{text}"));
            assert_eq!(
                SourceVersion::new(1, 3, 1).with_suffix(suffix).to_string(),
                version.to_string()
            );
        }
        for text in [
            "abcdefghijklmnopqrstu",
            "beta\0x",
            "beta 1",
            "beta\n",
            "béta1",
            "<beta>",
        ] {
            assert_eq!(VersionSuffix::new(text), None, "{text:?}");
        }
        let mut invalid = [0; VERSION_SUFFIX_BYTES];
        invalid[..7].copy_from_slice(b"beta\0\0x");
        assert!(VersionSuffix::from_wire(&invalid).is_err());
        invalid.fill(0xff);
        assert!(VersionSuffix::from_wire(&invalid).is_err());
        assert!(VersionSuffix::from_wire(&invalid[..19]).is_err());
    }

    #[test]
    fn generic_suffix_identity_does_not_broaden_release_admission() {
        for (major, minor, patch, suffix) in [
            (1, 3, 1, "beta2"),
            (1, 3, 1, "rc1"),
            (1, 3, 1, "beta1-extra"),
            (1, 3, 1, "abcdefghijklmnopqrst"),
            (1, 3, 0, "beta1"),
            (2, 0, 0, "beta1"),
        ] {
            let mut payload = info_v2(major, minor, patch);
            payload[28..28 + suffix.len()].copy_from_slice(suffix.as_bytes());
            let identity = decode_info_identity(&payload).unwrap();
            assert_eq!(identity.suffix.as_str(), suffix);
            assert_eq!(compatibility(identity), None);
            assert_eq!(parse_wire_version(&identity.to_string()), None);
            assert_eq!(
                decode_info(&payload),
                Err(CompatibilityError::UnsupportedRelease)
            );
            assert_eq!(
                decode_info_record(&payload),
                CompatibilityRecordOutcome::UnsupportedRelease
            );
        }
        let legacy_with_suffix =
            WireVersion::legacy(1, 0).with_suffix(VersionSuffix::new("beta1").unwrap());
        assert_eq!(compatibility(legacy_with_suffix), None);
        assert_eq!(legacy_with_suffix.to_string(), "1.0-beta1");
    }

    #[test]
    fn catalogue_capabilities_and_complete_identities_round_trip() {
        for entry in COMPATIBILITY_ENTRIES {
            let profile = compatibility(entry.wire).unwrap();
            assert_eq!(profile.imu_timeout_available, entry.imu_timeout_available);
            assert_eq!(
                parse_wire_version(&entry.wire.to_string()),
                Some(entry.wire)
            );
            let record = serialize_compatibility(entry.wire, profile);
            assert_eq!(record.as_bytes()[10], u8::from(entry.imu_timeout_available));
            assert_eq!(record.as_bytes()[12..], entry.wire.suffix.0);
            assert_eq!(
                parse_compatibility_record(record.as_bytes()),
                Ok((entry.wire, profile))
            );
        }
        assert!(compatibility(BETA_VERSION).unwrap().imu_timeout_available);
        assert!(
            !compatibility(WireVersion::patched(1, 3, 0))
                .unwrap()
                .imu_timeout_available
        );
    }

    #[test]
    fn canonical_beta_record_validates_suffix_capability_and_extent_together() {
        let record = serialize_compatibility(BETA_VERSION, compatibility(BETA_VERSION).unwrap());
        let bytes = record.as_bytes();
        assert_eq!(&bytes[12..17], b"beta1");
        assert!(bytes[17..].iter().all(|b| *b == 0));
        for (index, value) in [(0, 1), (10, 0), (12, 0), (16, b'2'), (18, b'x')] {
            let mut invalid = *bytes;
            invalid[index] = value;
            assert_eq!(
                parse_compatibility_record(&invalid),
                Err(CompatibilityError::InvalidCompatibilityRecord)
            );
        }
        for end in 0..COMPATIBILITY_RECORD_BYTES {
            assert!(parse_compatibility_record(&bytes[..end]).is_err());
        }
        let mut trailing = bytes.to_vec();
        trailing.push(0);
        assert!(parse_compatibility_record(&trailing).is_err());
        assert!(parse_compatibility_record(&[1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0]).is_err());
    }

    #[test]
    fn beta_admission_preserves_suffix_and_canonical_identity() {
        for suffix in ["", "beta", "beta2", "beta1-extra"] {
            let mut payload = info_v2(1, 3, 1);
            payload[28..28 + suffix.len()].copy_from_slice(suffix.as_bytes());
            assert!(matches!(
                decode_info_record(&payload),
                CompatibilityRecordOutcome::UnsupportedRelease
            ));
        }
        let mut payload = info_v2(1, 3, 1);
        payload[28..33].copy_from_slice(b"beta1");
        assert_eq!(decode_info(&payload), Ok(BETA_VERSION));
        assert_eq!(
            decode_info_identity(&payload).unwrap().to_string(),
            "1.3.1-beta1"
        );
        assert_eq!(BETA_VERSION.to_string(), "1.3.1-beta1");
        assert_eq!(parse_wire_version("1.3.1-beta1"), Some(BETA_VERSION));
        let CompatibilityRecordOutcome::Record(record) = decode_info_record(&payload) else {
            panic!("beta admitted");
        };
        assert_eq!(
            parse_compatibility_record(record.as_bytes()).unwrap().0,
            BETA_VERSION
        );
        assert_eq!(decode_info_capabilities(&payload), Ok(0));
        payload[27] = 0;
        assert_eq!(
            decode_info(&payload),
            Err(CompatibilityError::UnsupportedRelease)
        );
    }

    #[test]
    fn display_identity_preserves_bounded_suffix_without_admitting_prereleases() {
        let mut payload = info_v2(1, 3, 1);
        payload[28..32].copy_from_slice(b"beta");
        let identity = decode_info_identity(&payload).unwrap();
        assert_eq!(identity.to_string(), "1.3.1-beta");
        assert_eq!(
            decode_info(&payload),
            Err(CompatibilityError::UnsupportedRelease)
        );
        payload[27] = 0; // A beta of an otherwise supported version must also fail closed.
        assert_eq!(
            decode_info(&payload),
            Err(CompatibilityError::UnsupportedRelease)
        );
        for byte in [b'\n', b' ', 0xff, b'<'] {
            payload[28] = byte;
            assert!(decode_info_identity(&payload).is_err());
        }
        payload[28..48].fill(b'a');
        payload[25..28].fill(255);
        assert_eq!(
            decode_info_identity(&payload).unwrap().to_string().len(),
            32
        );
        payload[29] = 0;
        assert!(decode_info_identity(&payload).is_err());
        assert_eq!(
            decode_info_identity(&[36, 101, 0, 10]).unwrap().to_string(),
            "1.0"
        );
    }

    fn required_fields() -> Vec<&'static [u8]> {
        REQUIRED_FIELDS
            .iter()
            .map(|value| value.as_bytes())
            .collect()
    }

    fn required_runtime_fields() -> Vec<&'static [u8]> {
        REQUIRED_RUNTIME_FIELDS
            .iter()
            .map(|value| value.as_bytes())
            .collect()
    }

    fn info_v2(major: u8, minor: u8, patch: u8) -> [u8; INFO_V2_RESPONSE_BYTES] {
        let mut payload = [0_u8; INFO_V2_RESPONSE_BYTES];
        payload[..5].copy_from_slice(&[36, 101, 0, 2, 0]);
        payload[5..12].copy_from_slice(b"refloat");
        payload[INFO_V2_MAJOR_OFFSET] = major;
        payload[INFO_V2_MINOR_OFFSET] = minor;
        payload[INFO_V2_PATCH_OFFSET] = patch;
        payload
    }

    fn layout_record(fields: &[&str], runtime: &[&str]) -> Vec<u8> {
        let mut payload = vec![36, 101, 32, u8::try_from(fields.len()).unwrap()];
        for name in fields {
            payload.push(u8::try_from(name.len()).unwrap());
            payload.extend_from_slice(name.as_bytes());
        }
        payload.push(u8::try_from(runtime.len()).unwrap());
        for name in runtime {
            payload.push(u8::try_from(name.len()).unwrap());
            payload.extend_from_slice(name.as_bytes());
        }
        let LayoutRecordOutcome::Record(record) = decode_layout_to_record(&payload) else {
            panic!("test layout must decode");
        };
        record.as_bytes().to_vec()
    }

    #[test]
    fn exact_info_request_and_source_response_forms() {
        assert_eq!(info_request(), [36, 101, 0, 2, 0]);
        assert_eq!(
            decode_info(&[36, 101, 0, 10]),
            Ok(WireVersion::legacy(1, 0))
        );
        assert_eq!(
            decode_info(&[36, 101, 0, 10, 1, 3]),
            Ok(WireVersion::legacy(1, 0))
        );
        assert_eq!(
            decode_info(&[36, 101, 0, 11]),
            Ok(WireVersion::legacy(1, 1))
        );
        assert_eq!(
            decode_info(&[36, 101, 0, 12]),
            Ok(WireVersion::legacy(1, 2))
        );
        assert_eq!(
            decode_info(&info_v2(1, 2, 3)),
            Ok(WireVersion::patched(1, 2, 3))
        );
        assert_eq!(WireVersion::legacy(1, 0).to_string(), "1.0");
        assert_eq!(WireVersion::patched(1, 2, 3).to_string(), "1.2.3");
        let mut current = info_v2(1, 3, 0);
        current[INFO_V2_CAPABILITIES_OFFSET + 3] = INFO_CAPABILITY_GNSS as u8;
        assert_eq!(decode_info_capabilities(&current), Ok(INFO_CAPABILITY_GNSS));
    }

    #[test]
    fn info_rejects_every_envelope_protocol_and_extent_boundary() {
        assert_eq!(decode_info(&[]), Err(CompatibilityError::TruncatedInfo));
        assert_eq!(
            decode_info(&[36, 101, 0]),
            Err(CompatibilityError::TruncatedInfo)
        );
        for payload in [[35, 101, 0, 10], [36, 100, 0, 10], [36, 101, 1, 10]] {
            assert_eq!(
                decode_info(&payload),
                Err(CompatibilityError::UnsupportedEnvelope)
            );
        }
        for response in [0, 1, 3, 9] {
            assert_eq!(
                decode_info(&[36, 101, 0, response]),
                Err(CompatibilityError::UnsupportedInfoProtocol)
            );
        }
        let complete = info_v2(1, 2, 3);
        assert_eq!(
            decode_info(&complete[..INFO_V2_RESPONSE_BYTES - 1]),
            Err(CompatibilityError::TruncatedInfo)
        );
        let mut maximum = vec![0_u8; MAX_PAYLOAD_BYTES];
        maximum[..INFO_V2_RESPONSE_BYTES].copy_from_slice(&complete);
        assert_eq!(decode_info(&maximum), Ok(WireVersion::patched(1, 2, 3)));
        maximum.push(0);
        assert_eq!(
            decode_info(&maximum),
            Err(CompatibilityError::InputTooLarge)
        );
    }

    #[test]
    fn exact_catalogue_and_profiles_are_closed() {
        let expected_sources = [
            SourceVersion::new(1, 0, 0),
            SourceVersion::new(1, 0, 1),
            SourceVersion::new(1, 1, 0),
            SourceVersion::new(1, 1, 1),
            SourceVersion::new(1, 1, 2),
            SourceVersion::new(1, 2, 0),
            SourceVersion::new(1, 2, 1),
            SourceVersion::new(1, 2, 2),
            SourceVersion::new(1, 2, 3),
            SourceVersion::new(1, 3, 0),
            SourceVersion::new(1, 3, 1).with_suffix(VersionSuffix::new("beta1").unwrap()),
        ];
        assert_eq!(
            COMPATIBILITY_ENTRIES.map(|entry| entry.source),
            expected_sources
        );

        let legacy = compatibility(WireVersion::legacy(1, 0)).unwrap();
        assert_eq!(legacy.telemetry, TelemetryProfile::LegacyFixed);
        assert_eq!(legacy.telemetry.family(), TelemetryFamily::LegacyFixed);
        assert_eq!(legacy.control, LEGACY_CONTROL);

        for patch in 0..=2 {
            let profile = compatibility(WireVersion::patched(1, 1, patch)).unwrap();
            assert_eq!(profile.telemetry, TelemetryProfile::Dynamic);
            assert_eq!(profile.telemetry.family(), TelemetryFamily::Dynamic);
            assert_eq!(profile.control, LEGACY_CONTROL);
        }
        for patch in 0..=3 {
            let profile = compatibility(WireVersion::patched(1, 2, patch)).unwrap();
            assert_eq!(profile.telemetry, TelemetryProfile::DynamicAlerts);
            assert_eq!(profile.telemetry.family(), TelemetryFamily::Dynamic);
            assert_eq!(profile.control, STABLE_CONTROL);
        }
        let selectable = compatibility(WireVersion::patched(1, 3, 0)).unwrap();
        assert_eq!(selectable.telemetry, TelemetryProfile::Selectable);
        assert_eq!(selectable.telemetry.family(), TelemetryFamily::Selectable);
        assert_eq!(selectable.control, STABLE_CONTROL);
    }

    #[test]
    fn exact_wire_text_and_structural_lookup_reject_ranges_and_prereleases() {
        for text in [
            "1.0", "1.1.0", "1.1.1", "1.1.2", "1.2.0", "1.2.1", "1.2.2", "1.2.3", "1.3.0",
        ] {
            let version = parse_wire_version(text).unwrap();
            assert_eq!(version.to_string(), text);
            assert!(compatibility(version).is_some());
        }
        for text in ["1.0.0", "1.2", "1.2.4", "1.2.3-beta1", "^1.2.3", ">=1.1"] {
            assert_eq!(parse_wire_version(text), None);
        }
        for version in [
            WireVersion::legacy(1, 1),
            WireVersion::patched(1, 0, 0),
            WireVersion::patched(1, 2, 4),
            WireVersion::patched(1, 3, 1),
        ] {
            assert_eq!(compatibility(version), None);
        }
    }

    #[test]
    fn complete_layout_accepts_order_changes_and_bounded_future_names() {
        let mut fields = required_fields();
        let mut runtime = required_runtime_fields();
        fields.reverse();
        runtime.reverse();
        fields.push(b"future.energy");
        runtime.push(b"future.runtime");
        assert_eq!(admit_dynamic_layout(&fields, &runtime), Ok(()));
    }

    #[test]
    fn layout_rejects_every_missing_and_wrong_group_required_identifier() {
        for index in 0..REQUIRED_FIELDS.len() {
            let mut fields = required_fields();
            fields.remove(index);
            assert_eq!(
                admit_dynamic_layout(&fields, &required_runtime_fields()),
                Err(CompatibilityError::MissingRequiredIdentifier)
            );
        }
        for index in 0..REQUIRED_RUNTIME_FIELDS.len() {
            let mut runtime = required_runtime_fields();
            runtime.remove(index);
            assert_eq!(
                admit_dynamic_layout(&required_fields(), &runtime),
                Err(CompatibilityError::MissingRequiredRuntimeIdentifier)
            );
        }

        let mut fields = required_fields();
        let mut runtime = required_runtime_fields();
        let moved = fields.remove(0);
        runtime.push(moved);
        assert_eq!(
            admit_dynamic_layout(&fields, &runtime),
            Err(CompatibilityError::MissingRequiredIdentifier)
        );
    }

    #[test]
    fn layout_rejects_invalid_names_duplicates_and_counts() {
        let mut fields = required_fields();
        fields.push(b"");
        assert_eq!(
            admit_dynamic_layout(&fields, &required_runtime_fields()),
            Err(CompatibilityError::EmptyIdentifier)
        );

        let too_long = vec![b'x'; MAX_IDENTIFIER_BYTES + 1];
        let mut fields = required_fields();
        fields.push(&too_long);
        assert_eq!(
            admit_dynamic_layout(&fields, &required_runtime_fields()),
            Err(CompatibilityError::IdentifierTooLong)
        );

        let invalid_utf8 = [0xff_u8];
        let mut fields = required_fields();
        fields.push(&invalid_utf8);
        assert_eq!(
            admit_dynamic_layout(&fields, &required_runtime_fields()),
            Err(CompatibilityError::InvalidIdentifier)
        );

        let mut fields = required_fields();
        fields.push(REQUIRED_FIELDS[0].as_bytes());
        assert_eq!(
            admit_dynamic_layout(&fields, &required_runtime_fields()),
            Err(CompatibilityError::DuplicateIdentifier)
        );

        let mut runtime = required_runtime_fields();
        runtime.push(REQUIRED_FIELDS[0].as_bytes());
        assert_eq!(
            admit_dynamic_layout(&required_fields(), &runtime),
            Err(CompatibilityError::DuplicateIdentifier)
        );

        let future_names: Vec<String> = (0..(MAX_LAYOUT_FIELDS - 25))
            .map(|index| format!("future.{index}"))
            .collect();
        let mut fields = required_fields();
        fields.extend(future_names.iter().map(String::as_bytes));
        assert_eq!(
            admit_dynamic_layout(&fields, &required_runtime_fields()),
            Err(CompatibilityError::TooManyIdentifiers)
        );
    }

    #[test]
    fn layout_accepts_exact_count_and_name_bounds() {
        let future_names: Vec<String> = (0..(MAX_LAYOUT_FIELDS - 26))
            .map(|index| {
                let prefix = format!("future.{index:02}.");
                format!(
                    "{prefix}{}",
                    "x".repeat(MAX_IDENTIFIER_BYTES - prefix.len())
                )
            })
            .collect();
        assert!(
            future_names
                .iter()
                .all(|name| name.len() == MAX_IDENTIFIER_BYTES)
        );
        let mut fields = required_fields();
        fields.extend(future_names.iter().map(String::as_bytes));
        assert_eq!(
            fields.len() + REQUIRED_RUNTIME_FIELDS.len(),
            MAX_LAYOUT_FIELDS
        );
        assert_eq!(
            admit_dynamic_layout(&fields, &required_runtime_fields()),
            Ok(())
        );
    }

    #[test]
    fn failures_and_debug_output_never_include_input() {
        let secret = b"private.controller.payload";
        let mut fields = required_fields();
        fields.push(secret);
        fields.push(secret);
        let error = admit_dynamic_layout(&fields, &required_runtime_fields()).unwrap_err();
        let diagnostic = format!("{error:?}: {error}");
        assert!(!diagnostic.contains("private"));
        assert!(!diagnostic.contains("controller"));
        assert!(!diagnostic.contains("payload"));
    }

    #[test]
    fn atomic_info_records_are_exact_and_round_trip() {
        let CompatibilityRecordOutcome::Record(legacy) = decode_info_record(&[36, 101, 0, 10])
        else {
            panic!("legacy identity must be accepted");
        };
        let mut expected = [0; COMPATIBILITY_RECORD_BYTES];
        expected[..12].copy_from_slice(&[2, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0]);
        assert_eq!(legacy.as_bytes(), &expected);
        assert_eq!(
            parse_compatibility_record(legacy.as_bytes()),
            Ok((
                WireVersion::legacy(1, 0),
                compatibility(WireVersion::legacy(1, 0)).unwrap()
            ))
        );

        let CompatibilityRecordOutcome::Record(stable) = decode_info_record(&info_v2(1, 2, 3))
        else {
            panic!("stable identity must be accepted");
        };
        expected[..12].copy_from_slice(&[2, 1, 1, 2, 1, 3, 3, 2, 2, 1, 0, 0]);
        assert_eq!(stable.as_bytes(), &expected);
        assert_eq!(
            parse_compatibility_record(stable.as_bytes()),
            Ok((
                WireVersion::patched(1, 2, 3),
                compatibility(WireVersion::patched(1, 2, 3)).unwrap()
            ))
        );
    }

    #[test]
    fn atomic_info_records_preserve_closed_failures() {
        assert_eq!(
            decode_info_record(&[]),
            CompatibilityRecordOutcome::TruncatedInfo
        );
        assert_eq!(
            decode_info_record(&[35, 101, 0, 10]),
            CompatibilityRecordOutcome::UnsupportedEnvelope
        );
        assert_eq!(
            decode_info_record(&[36, 101, 0, 1]),
            CompatibilityRecordOutcome::UnsupportedInfoProtocol
        );
        assert_eq!(
            decode_info_record(&info_v2(1, 2, 4)),
            CompatibilityRecordOutcome::UnsupportedRelease
        );
        assert_eq!(
            decode_info_record(&vec![0; MAX_PAYLOAD_BYTES + 1]),
            CompatibilityRecordOutcome::InputTooLarge
        );
    }

    #[test]
    fn compatibility_record_corruption_fails_closed() {
        let CompatibilityRecordOutcome::Record(record) = decode_info_record(&info_v2(1, 2, 3))
        else {
            panic!("stable identity must be accepted");
        };
        for index in 0..COMPATIBILITY_RECORD_BYTES {
            let mut corrupt = *record.as_bytes();
            corrupt[index] ^= if index == 10 || index == 11 { 1 } else { 0xff };
            assert_eq!(
                parse_compatibility_record(&corrupt),
                Err(CompatibilityError::InvalidCompatibilityRecord),
                "byte {index}"
            );
        }
        assert_eq!(
            parse_compatibility_record(&record.as_bytes()[..11]),
            Err(CompatibilityError::InvalidCompatibilityRecord)
        );
    }

    #[test]
    fn canonical_dynamic_layout_record_is_the_only_admission_input() {
        let record = layout_record(&REQUIRED_FIELDS, &REQUIRED_RUNTIME_FIELDS);
        assert_eq!(admit_dynamic_layout_record(&record), Ok(()));

        let mut fields = REQUIRED_FIELDS.to_vec();
        fields.remove(0);
        let incomplete = layout_record(&fields, &REQUIRED_RUNTIME_FIELDS);
        assert_eq!(
            admit_dynamic_layout_record(&incomplete),
            Err(CompatibilityError::MissingRequiredIdentifier)
        );

        let mut corrupt = record;
        corrupt[0] = 2;
        assert_eq!(
            admit_dynamic_layout_record(&corrupt),
            Err(CompatibilityError::InvalidLayoutRecord)
        );
    }
}
