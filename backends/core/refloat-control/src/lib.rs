//! Strict, bounded Refloat live-control wire payloads.
//!
//! This unselected safe candidate deliberately knows nothing about VESC
//! framing, CAN routing, compatibility lookup, mutation policy, host bridges,
//! telemetry, or application state.

#![forbid(unsafe_code)]

use std::fmt;

/// VESC `COMM_CUSTOM_APP_DATA`.
pub const CUSTOM_APP_DATA_COMMAND: u8 = 36;
/// Refloat custom-app-data package identifier.
pub const REFLOAT_PACKAGE_ID: u8 = 101;
/// Longest payload emitted by this codec.
pub const MAX_CONTROL_PAYLOAD_BYTES: usize = 9;

const PACKAGE_LOCK_COMMAND: u8 = 12;
const STABLE_LIGHTS_COMMAND: u8 = 20;
const FLYWHEEL_COMMAND: u8 = 22;
const ALERTS_CONTROL_COMMAND: u8 = 36;
const LEGACY_LIGHTS_COMMAND: u8 = 202;

/// Refloat lighting wire generation selected by native compatibility policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LightingProfile {
    /// Refloat command 202 with mask and flags immediately after the command.
    LegacyCommand202,
    /// Refloat command 20 with three reserved zero bytes before mask and flags.
    StableCommand20,
}

impl LightingProfile {
    const fn command(self) -> u8 {
        match self {
            Self::LegacyCommand202 => LEGACY_LIGHTS_COMMAND,
            Self::StableCommand20 => STABLE_LIGHTS_COMMAND,
        }
    }
}

/// Wire capabilities supplied by native compatibility policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlProfile {
    /// Lighting command generation.
    pub lighting: LightingProfile,
    /// Whether command 36 fatal-latch clear is supported.
    pub fatal_clear_available: bool,
}

/// A single mutable lighting target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LightingTarget {
    /// LED state bit zero.
    Leds,
    /// Headlight state bit one.
    Headlights,
}

impl LightingTarget {
    const fn bit(self) -> u8 {
        match self {
            Self::Leds => 0x01,
            Self::Headlights => 0x02,
        }
    }
}

/// The two defined Refloat lighting state bits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LightingState {
    /// LED state.
    pub leds_enabled: bool,
    /// Headlight state.
    pub headlights_enabled: bool,
}

impl LightingState {
    const fn flags(self) -> u8 {
        (self.leds_enabled as u8) | ((self.headlights_enabled as u8) << 1)
    }
}

/// One statically bounded unframed control payload.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ControlPayload {
    bytes: [u8; MAX_CONTROL_PAYLOAD_BYTES],
    len: u8,
}

impl ControlPayload {
    const fn from_parts(bytes: [u8; MAX_CONTROL_PAYLOAD_BYTES], len: u8) -> Self {
        Self { bytes, len }
    }

    /// Borrow the complete initialized payload.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }

    /// Exact payload length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Control payloads are never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
}

impl fmt::Debug for ControlPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ControlPayload")
            .field("len", &self.len)
            .finish_non_exhaustive()
    }
}

/// Stable bounded codec failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlError {
    /// A response does not have the exact selected structure.
    InvalidPayload,
    /// The supplied compatibility profile does not permit this control.
    UnsupportedCommand(u8),
}

impl fmt::Display for ControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPayload => formatter.write_str("invalid Refloat control payload"),
            Self::UnsupportedCommand(command) => {
                write!(formatter, "unsupported Refloat command {command}")
            }
        }
    }
}

impl std::error::Error for ControlError {}

/// Encode package lock/unlock through Refloat command 12.
#[must_use]
pub const fn package_request(enabled: bool) -> ControlPayload {
    ControlPayload::from_parts(
        [
            CUSTOM_APP_DATA_COMMAND,
            REFLOAT_PACKAGE_ID,
            PACKAGE_LOCK_COMMAND,
            if enabled { 0 } else { 1 },
            0,
            0,
            0,
            0,
            0,
        ],
        4,
    )
}

/// Encode flywheel mode using the established defaults and footpad abort.
#[must_use]
pub const fn flywheel_request(enabled: bool) -> ControlPayload {
    ControlPayload::from_parts(
        [
            CUSTOM_APP_DATA_COMMAND,
            REFLOAT_PACKAGE_ID,
            FLYWHEEL_COMMAND,
            if enabled { 0x81 } else { 0x80 },
            0,
            0,
            0,
            0,
            1,
        ],
        9,
    )
}

/// Encode a legacy/stable lighting mutation or a zero-mask query.
#[must_use]
pub const fn lighting_request(
    profile: LightingProfile,
    target: Option<LightingTarget>,
    enabled: bool,
    current: LightingState,
) -> ControlPayload {
    let mask = match target {
        Some(target) => target.bit(),
        None => 0,
    };
    let mut flags = current.flags();
    if let Some(target) = target {
        if enabled {
            flags |= target.bit();
        } else {
            flags &= !target.bit();
        }
    }

    match profile {
        LightingProfile::LegacyCommand202 => ControlPayload::from_parts(
            [
                CUSTOM_APP_DATA_COMMAND,
                REFLOAT_PACKAGE_ID,
                LEGACY_LIGHTS_COMMAND,
                mask,
                flags,
                0,
                0,
                0,
                0,
            ],
            5,
        ),
        LightingProfile::StableCommand20 => ControlPayload::from_parts(
            [
                CUSTOM_APP_DATA_COMMAND,
                REFLOAT_PACKAGE_ID,
                STABLE_LIGHTS_COMMAND,
                0,
                0,
                0,
                mask,
                flags,
                0,
            ],
            8,
        ),
    }
}

/// Encode fatal-latch clear when native compatibility policy permits it.
pub const fn fatal_clear_request(profile: ControlProfile) -> Result<ControlPayload, ControlError> {
    if !profile.fatal_clear_available {
        return Err(ControlError::UnsupportedCommand(ALERTS_CONTROL_COMMAND));
    }
    Ok(ControlPayload::from_parts(
        [
            CUSTOM_APP_DATA_COMMAND,
            REFLOAT_PACKAGE_ID,
            ALERTS_CONTROL_COMMAND,
            1,
            0,
            0,
            0,
            0,
            0,
        ],
        4,
    ))
}

/// Decode one exact lighting response for the supplied wire profile.
pub fn decode_lighting_response(
    payload: &[u8],
    profile: LightingProfile,
) -> Result<LightingState, ControlError> {
    if payload.len() != 4
        || payload[0] != CUSTOM_APP_DATA_COMMAND
        || payload[1] != REFLOAT_PACKAGE_ID
        || payload[2] != profile.command()
        || payload[3] & !0x03 != 0
    {
        return Err(ControlError::InvalidPayload);
    }
    Ok(LightingState {
        leds_enabled: payload[3] & 0x01 != 0,
        headlights_enabled: payload[3] & 0x02 != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../fixtures/refloat/product-controls.json");

    const LEGACY: ControlProfile = ControlProfile {
        lighting: LightingProfile::LegacyCommand202,
        fatal_clear_available: false,
    };
    const STABLE: ControlProfile = ControlProfile {
        lighting: LightingProfile::StableCommand20,
        fatal_clear_available: true,
    };

    #[test]
    fn every_source_derived_request_is_exact() {
        let current = LightingState {
            leds_enabled: true,
            headlights_enabled: true,
        };
        let requests = [
            ("package-enable", package_request(true)),
            ("package-disable", package_request(false)),
            (
                "flywheel-enable-defaults-footpad-abort",
                flywheel_request(true),
            ),
            (
                "flywheel-disable-defaults-footpad-abort",
                flywheel_request(false),
            ),
            (
                "legacy-lighting-leds-off-headlights-on",
                lighting_request(LEGACY.lighting, Some(LightingTarget::Leds), false, current),
            ),
            (
                "stable-lighting-headlights-off-leds-on",
                lighting_request(
                    STABLE.lighting,
                    Some(LightingTarget::Headlights),
                    false,
                    current,
                ),
            ),
            (
                "fatal-clear",
                fatal_clear_request(STABLE).expect("stable profile supports fatal clear"),
            ),
        ];

        for (name, actual) in requests {
            assert_eq!(actual.as_slice(), fixture_array(name, "request"), "{name}");
        }
    }

    #[test]
    fn source_derived_lighting_responses_are_exact() {
        for (name, profile, expected) in [
            (
                "legacy-lighting-leds-off-headlights-on",
                LEGACY.lighting,
                LightingState {
                    leds_enabled: false,
                    headlights_enabled: true,
                },
            ),
            (
                "stable-lighting-headlights-off-leds-on",
                STABLE.lighting,
                LightingState {
                    leds_enabled: true,
                    headlights_enabled: false,
                },
            ),
        ] {
            let response = fixture_array(name, "response");
            assert_eq!(
                decode_lighting_response(&response, profile),
                Ok(expected),
                "{name}"
            );
        }
    }

    #[test]
    fn every_lighting_state_target_and_profile_is_exact() {
        for profile in [LEGACY.lighting, STABLE.lighting] {
            for current_flags in 0_u8..=3 {
                let current = state(current_flags);
                let query = lighting_request(profile, None, false, current);
                assert_lighting_payload(query.as_slice(), profile, 0, current_flags);

                for target in [LightingTarget::Leds, LightingTarget::Headlights] {
                    for enabled in [false, true] {
                        let bit = target.bit();
                        let expected = if enabled {
                            current_flags | bit
                        } else {
                            current_flags & !bit
                        };
                        let request = lighting_request(profile, Some(target), enabled, current);
                        assert_lighting_payload(request.as_slice(), profile, bit, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn payload_extents_are_fixed_and_deterministic() {
        let state = LightingState::default();
        let cases = [
            package_request(true),
            package_request(true),
            flywheel_request(true),
            lighting_request(LEGACY.lighting, None, false, state),
            lighting_request(STABLE.lighting, None, false, state),
            fatal_clear_request(STABLE).expect("supported"),
        ];
        assert_eq!(cases.map(|payload| payload.len()), [4, 4, 9, 5, 8, 4]);
        assert_eq!(cases[0], cases[1]);
        assert!(cases.iter().all(|payload| !payload.is_empty()));
        assert_eq!(
            std::mem::size_of::<ControlPayload>(),
            MAX_CONTROL_PAYLOAD_BYTES + 1
        );
        assert_eq!(format!("{:?}", cases[2]), "ControlPayload { len: 9, .. }");
    }

    #[test]
    fn all_defined_lighting_responses_decode() {
        for profile in [LEGACY.lighting, STABLE.lighting] {
            for flags in 0_u8..=3 {
                assert_eq!(
                    decode_lighting_response(
                        &[
                            CUSTOM_APP_DATA_COMMAND,
                            REFLOAT_PACKAGE_ID,
                            profile.command(),
                            flags
                        ],
                        profile,
                    ),
                    Ok(state(flags)),
                );
            }
        }
    }

    #[test]
    fn unavailable_fatal_clear_and_malformed_responses_fail_closed() {
        assert_eq!(
            fatal_clear_request(LEGACY),
            Err(ControlError::UnsupportedCommand(ALERTS_CONTROL_COMMAND)),
        );

        for payload in [
            vec![],
            vec![36],
            vec![36, 101],
            vec![36, 101, 20],
            vec![36, 101, 20, 0, 0],
            vec![35, 101, 20, 0],
            vec![36, 100, 20, 0],
            vec![36, 101, 202, 0],
            vec![36, 101, 20, 4],
            vec![36, 101, 20, 255],
        ] {
            assert_eq!(
                decode_lighting_response(&payload, STABLE.lighting),
                Err(ControlError::InvalidPayload),
                "payload length {}",
                payload.len(),
            );
        }
        assert_eq!(
            decode_lighting_response(&[36, 101, 20, 0], LEGACY.lighting),
            Err(ControlError::InvalidPayload),
        );
    }

    #[test]
    fn failures_do_not_render_payload_or_state() {
        assert_eq!(
            ControlError::InvalidPayload.to_string(),
            "invalid Refloat control payload",
        );
        assert_eq!(
            ControlError::UnsupportedCommand(36).to_string(),
            "unsupported Refloat command 36",
        );
        let diagnostics = format!("{:?}", ControlError::InvalidPayload);
        assert!(!diagnostics.contains("101"));
        assert!(!diagnostics.contains("led"));
        assert!(!diagnostics.contains("headlight"));
    }

    fn state(flags: u8) -> LightingState {
        LightingState {
            leds_enabled: flags & 1 != 0,
            headlights_enabled: flags & 2 != 0,
        }
    }

    fn assert_lighting_payload(payload: &[u8], profile: LightingProfile, mask: u8, flags: u8) {
        match profile {
            LightingProfile::LegacyCommand202 => {
                assert_eq!(payload, [36, 101, 202, mask, flags]);
            }
            LightingProfile::StableCommand20 => {
                assert_eq!(payload, [36, 101, 20, 0, 0, 0, mask, flags]);
            }
        }
    }

    fn fixture_array(name: &str, field: &str) -> Vec<u8> {
        let case = FIXTURE
            .split(&format!("\"name\": \"{name}\""))
            .nth(1)
            .unwrap_or_else(|| panic!("missing fixture case {name}"));
        let values = case
            .split(&format!("\"{field}\":"))
            .nth(1)
            .unwrap_or_else(|| panic!("missing fixture field {field} for {name}"));
        let values = values
            .split('[')
            .nth(1)
            .unwrap_or_else(|| panic!("fixture field {field} is not an array"))
            .split(']')
            .next()
            .expect("array has a closing bracket");
        values
            .split(',')
            .filter_map(|value| {
                let value = value.trim();
                (!value.is_empty()).then(|| value.parse::<u8>().expect("fixture byte is valid"))
            })
            .collect()
    }
}
