//! Strict, bounded decoding for VESC `COMM_FW_VERSION` identity payloads.
//!
//! This crate is an isolated candidate. Native Apple and Android decoders
//! remain the selected production owners until separate host cutovers.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

/// VESC command byte for `COMM_FW_VERSION`.
pub const COMM_FW_VERSION: u8 = 0;

/// Motor-controller hardware type emitted by VESC firmware.
pub const MOTOR_HARDWARE_TYPE: u8 = 0;

/// Stock VESC BMS hardware type.
pub const BMS_HARDWARE_TYPE: u8 = 1;

/// Maximum accepted byte length for either terminated identity name.
pub const MAX_NAME_BYTES: usize = 64;

const UUID_BYTES: usize = 12;
const COMMON_BYTES_AFTER_UUID: usize = 4;
const CAPABILITY_BYTES: usize = 4;

/// Structurally decoded firmware identity.
///
/// Names and the controller UUID are intentionally redacted by [`Debug`].
#[derive(Clone, Eq, PartialEq)]
pub struct FirmwareIdentity {
    /// Firmware major version byte.
    pub major: u8,
    /// Firmware minor version byte.
    pub minor: u8,
    /// Bounded hardware name decoded with UTF-8 replacement.
    pub hardware_name: String,
    /// Optional bounded firmware name decoded with UTF-8 replacement.
    pub firmware_name: Option<String>,
    /// Twelve-byte private controller identity.
    pub controller_uuid: [u8; UUID_BYTES],
    /// Nonzero for a VESC test build.
    pub test_version: u8,
    /// VESC hardware type discriminator.
    pub hardware_type: u8,
    /// Number of custom configuration slots advertised by the firmware.
    pub custom_configuration_count: u8,
    /// Optional big-endian hardware configuration CRC.
    pub hardware_configuration_crc: Option<u32>,
}

impl Debug for FirmwareIdentity {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FirmwareIdentity")
            .field("major", &self.major)
            .field("minor", &self.minor)
            .field("hardware_name", &"[redacted]")
            .field(
                "firmware_name",
                &self.firmware_name.as_ref().map(|_| "[redacted]"),
            )
            .field("controller_uuid", &"[redacted]")
            .field("test_version", &self.test_version)
            .field("hardware_type", &self.hardware_type)
            .field(
                "custom_configuration_count",
                &self.custom_configuration_count,
            )
            .field(
                "hardware_configuration_crc",
                &self.hardware_configuration_crc,
            )
            .finish()
    }
}

/// Closed failure classes matching the selected native decoders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirmwareIdentityError {
    /// The payload contains a command other than `COMM_FW_VERSION`.
    UnsupportedCommand(u8),
    /// The payload is missing bytes or violates the strict established layout.
    TruncatedPayload,
}

impl Display for FirmwareIdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCommand(command) => {
                write!(formatter, "unsupported VESC command {command}")
            }
            Self::TruncatedPayload => formatter.write_str("truncated VESC firmware identity"),
        }
    }
}

impl Error for FirmwareIdentityError {}

/// Decode exactly one strict `COMM_FW_VERSION` payload.
///
/// Pairing and capability bytes are consumed to validate layout but are not
/// exposed. Higher-level firmware compatibility policy remains outside this
/// structural decoder.
pub fn decode(payload: &[u8]) -> Result<FirmwareIdentity, FirmwareIdentityError> {
    let command = payload
        .first()
        .copied()
        .ok_or(FirmwareIdentityError::TruncatedPayload)?;
    if command != COMM_FW_VERSION {
        return Err(FirmwareIdentityError::UnsupportedCommand(command));
    }
    let (&major, remainder) = payload
        .get(1..)
        .and_then(|bytes| bytes.split_first())
        .ok_or(FirmwareIdentityError::TruncatedPayload)?;
    let (&minor, mut remainder) = remainder
        .split_first()
        .ok_or(FirmwareIdentityError::TruncatedPayload)?;

    let (hardware_name, after_hardware_name) = terminated_name(remainder)?;
    remainder = after_hardware_name;
    let common_bytes = UUID_BYTES + COMMON_BYTES_AFTER_UUID;
    if remainder.len() < common_bytes {
        return Err(FirmwareIdentityError::TruncatedPayload);
    }

    let mut controller_uuid = [0_u8; UUID_BYTES];
    controller_uuid.copy_from_slice(&remainder[..UUID_BYTES]);
    let test_version = remainder[UUID_BYTES + 1];
    let hardware_type = remainder[UUID_BYTES + 2];
    let custom_configuration_count = remainder[UUID_BYTES + 3];
    remainder = &remainder[common_bytes..];

    let (firmware_name, hardware_configuration_crc) = if hardware_type == BMS_HARDWARE_TYPE {
        if major != 6 || minor != 0 || !remainder.is_empty() {
            return Err(FirmwareIdentityError::TruncatedPayload);
        }
        (None, None)
    } else {
        let extended = remainder
            .get(CAPABILITY_BYTES..)
            .ok_or(FirmwareIdentityError::TruncatedPayload)?;
        let (name, tail) = terminated_name(extended)?;
        let crc = match tail {
            [] => None,
            [a, b, c, d] => Some(u32::from_be_bytes([*a, *b, *c, *d])),
            _ => return Err(FirmwareIdentityError::TruncatedPayload),
        };
        ((!name.is_empty()).then_some(name), crc)
    };

    Ok(FirmwareIdentity {
        major,
        minor,
        hardware_name,
        firmware_name,
        controller_uuid,
        test_version,
        hardware_type,
        custom_configuration_count,
        hardware_configuration_crc,
    })
}

fn terminated_name(bytes: &[u8]) -> Result<(String, &[u8]), FirmwareIdentityError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .filter(|end| *end <= MAX_NAME_BYTES)
        .ok_or(FirmwareIdentityError::TruncatedPayload)?;
    let name = String::from_utf8_lossy(&bytes[..end]).into_owned();
    Ok((name, &bytes[end + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const THOR_602: &str = include_str!("../../../fixtures/vesc/firmware-thor300-6.02.json");
    const THOR_605: &str = include_str!("../../../fixtures/vesc/firmware-thor300-6.05.json");
    const THOR_606: &str = include_str!("../../../fixtures/vesc/firmware-thor300-6.06.json");
    const THOR_700: &str = include_str!("../../../fixtures/vesc/firmware-thor300-7.00.json");
    const EXPRESS_605: &str = include_str!("../../../fixtures/vesc/firmware-express-6.05.json");
    const BMS_IDENTITIES: &str = include_str!("../../../fixtures/vesc-bms/source-derived.json");

    #[test]
    fn decodes_every_audited_thor_identity() {
        for (fixture, major, minor, crc) in [
            (THOR_602, 6, 2, None),
            (THOR_605, 6, 5, Some(0)),
            (THOR_606, 6, 6, Some(0)),
            (THOR_700, 7, 0, Some(0)),
        ] {
            let identity = decode(&framed_payload(fixture)).expect("audited THOR identity");
            assert_eq!(identity.major, major);
            assert_eq!(identity.minor, minor);
            assert_eq!(identity.hardware_name, "THOR300");
            assert_eq!(identity.firmware_name, None);
            assert_eq!(identity.hardware_type, MOTOR_HARDWARE_TYPE);
            assert_eq!(identity.controller_uuid, [0; UUID_BYTES]);
            assert_eq!(identity.test_version, 0);
            assert_eq!(identity.custom_configuration_count, 0);
            assert_eq!(identity.hardware_configuration_crc, crc);
        }
    }

    #[test]
    fn decodes_audited_express_identity() {
        let identity = decode(&framed_payload(EXPRESS_605)).expect("audited Express identity");
        assert_eq!(identity.major, 6);
        assert_eq!(identity.minor, 5);
        assert_eq!(identity.hardware_name, "VESC Express T");
        assert_eq!(identity.firmware_name, None);
        assert_eq!(identity.hardware_type, 2);
        assert_eq!(identity.controller_uuid, [0; UUID_BYTES]);
        assert_eq!(identity.test_version, 0);
        assert_eq!(identity.custom_configuration_count, 0);
        assert_eq!(identity.hardware_configuration_crc, None);
    }

    #[test]
    fn decodes_audited_stock_and_custom_bms_identities() {
        let stock = decode(&bms_payload("vbms-firmware-6.00")).expect("audited stock VBMS");
        assert_eq!(stock.major, 6);
        assert_eq!(stock.minor, 0);
        assert_eq!(stock.hardware_name, "VBMS");
        assert_eq!(stock.firmware_name, None);
        assert_eq!(stock.hardware_type, BMS_HARDWARE_TYPE);
        assert_eq!(
            stock.controller_uuid,
            [
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb
            ]
        );
        assert_eq!(stock.test_version, 0);
        assert_eq!(stock.custom_configuration_count, 1);
        assert_eq!(stock.hardware_configuration_crc, None);

        let custom = decode(&bms_payload("vbms32-firmware-6.06")).expect("audited VBMS32");
        assert_eq!(custom.major, 6);
        assert_eq!(custom.minor, 6);
        assert_eq!(custom.hardware_name, "VBMS32");
        assert_eq!(custom.firmware_name, None);
        assert_eq!(custom.hardware_type, 2);
        assert_eq!(
            custom.controller_uuid,
            [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(custom.test_version, 0);
        assert_eq!(custom.custom_configuration_count, 1);
        assert_eq!(custom.hardware_configuration_crc, Some(0));
    }

    #[test]
    fn preserves_markers_counts_names_uuid_and_big_endian_crc() {
        let uuid = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let payload = custom_payload(
            &[b'H'; MAX_NAME_BYTES],
            &[b'F'; MAX_NAME_BYTES],
            uuid,
            9,
            2,
            7,
            Some([0x12, 0x34, 0x56, 0x78]),
        );
        let identity = decode(&payload).expect("maximum bounded identity");
        assert_eq!(identity.hardware_name.len(), MAX_NAME_BYTES);
        assert_eq!(
            identity.firmware_name.as_deref().map(str::len),
            Some(MAX_NAME_BYTES)
        );
        assert_eq!(identity.controller_uuid, uuid);
        assert_eq!(identity.test_version, 9);
        assert_eq!(identity.hardware_type, 2);
        assert_eq!(identity.custom_configuration_count, 7);
        assert_eq!(identity.hardware_configuration_crc, Some(0x1234_5678));

        let empty =
            decode(&custom_payload(b"HW", b"", uuid, 1, 0, 3, None)).expect("empty firmware name");
        assert_eq!(empty.firmware_name, None);
        assert_eq!(empty.hardware_configuration_crc, None);
    }

    #[test]
    fn replaces_invalid_utf8_without_relaxing_name_bounds() {
        let identity = decode(&custom_payload(
            &[b'H', 0xff],
            &[b'F', 0xfe],
            [0; UUID_BYTES],
            0,
            0,
            0,
            None,
        ))
        .expect("bounded replacement names");
        assert_eq!(identity.hardware_name, "H\u{fffd}");
        assert_eq!(identity.firmware_name.as_deref(), Some("F\u{fffd}"));

        let mut overlong_hardware = vec![COMM_FW_VERSION, 6, 6];
        overlong_hardware.extend([b'H'; MAX_NAME_BYTES + 1]);
        overlong_hardware.push(0);
        assert_eq!(
            decode(&overlong_hardware),
            Err(FirmwareIdentityError::TruncatedPayload)
        );

        let mut overlong_firmware = custom_payload(
            b"HW",
            &[b'F'; MAX_NAME_BYTES + 1],
            [0; UUID_BYTES],
            0,
            0,
            0,
            None,
        );
        assert_eq!(
            decode(&overlong_firmware),
            Err(FirmwareIdentityError::TruncatedPayload)
        );
        overlong_firmware.truncate(overlong_firmware.len() - (MAX_NAME_BYTES + 2));
        overlong_firmware.extend([b'F'; MAX_NAME_BYTES]);
        assert_eq!(
            decode(&overlong_firmware),
            Err(FirmwareIdentityError::TruncatedPayload)
        );
    }

    #[test]
    fn rejects_wrong_commands_and_every_common_prefix_truncation() {
        assert_eq!(
            decode(&[37]),
            Err(FirmwareIdentityError::UnsupportedCommand(37))
        );

        let stock = stock_bms_payload();
        for length in 0..stock.len() {
            assert_eq!(
                decode(&stock[..length]),
                Err(FirmwareIdentityError::TruncatedPayload),
                "accepted common prefix length {length}"
            );
        }
        assert!(decode(&stock).is_ok());

        let mut missing_hardware_terminator = vec![COMM_FW_VERSION, 6, 0];
        missing_hardware_terminator.extend([b'H'; MAX_NAME_BYTES]);
        assert_eq!(
            decode(&missing_hardware_terminator),
            Err(FirmwareIdentityError::TruncatedPayload)
        );
    }

    #[test]
    fn rejects_invalid_stock_bms_versions_and_tails() {
        let stock = stock_bms_payload();
        for (index, value) in [(1, 5), (2, 1)] {
            let mut malformed = stock.clone();
            malformed[index] = value;
            assert_eq!(
                decode(&malformed),
                Err(FirmwareIdentityError::TruncatedPayload)
            );
        }
        for tail_length in 1..=5 {
            let mut malformed = stock.clone();
            malformed.extend(vec![0; tail_length]);
            assert_eq!(
                decode(&malformed),
                Err(FirmwareIdentityError::TruncatedPayload)
            );
        }
    }

    #[test]
    fn rejects_incomplete_extended_fields_and_invalid_crc_tails() {
        let base = custom_payload(b"HW", b"FW", [0; UUID_BYTES], 0, 2, 0, None);
        let common_end = 3 + 3 + UUID_BYTES + COMMON_BYTES_AFTER_UUID;
        for length in common_end..base.len() {
            assert_eq!(
                decode(&base[..length]),
                Err(FirmwareIdentityError::TruncatedPayload),
                "accepted extended prefix length {length}"
            );
        }
        assert!(decode(&base).is_ok());

        for tail_length in [1, 2, 3, 5] {
            let mut malformed = base.clone();
            malformed.extend(vec![0x5a; tail_length]);
            assert_eq!(
                decode(&malformed),
                Err(FirmwareIdentityError::TruncatedPayload)
            );
        }

        let with_crc = custom_payload(b"HW", b"FW", [0; UUID_BYTES], 0, 0, 0, Some([1, 2, 3, 4]));
        for removed in 1..=3 {
            assert_eq!(
                decode(&with_crc[..with_crc.len() - removed]),
                Err(FirmwareIdentityError::TruncatedPayload)
            );
        }
    }

    #[test]
    fn diagnostics_redact_private_identity() {
        let identity = decode(&custom_payload(
            b"SECRET-HARDWARE",
            b"SECRET-FIRMWARE",
            [0xde, 0xad, 0xbe, 0xef, 1, 2, 3, 4, 5, 6, 7, 8],
            0,
            0,
            0,
            None,
        ))
        .expect("private identity");
        let diagnostic = format!("{identity:?}");
        assert!(!diagnostic.contains("SECRET"));
        assert!(!diagnostic.contains("222"));
        assert!(!diagnostic.contains("deadbeef"));
        assert_eq!(diagnostic.matches("[redacted]").count(), 3);

        let error = FirmwareIdentityError::UnsupportedCommand(37);
        assert_eq!(error.to_string(), "unsupported VESC command 37");
    }

    fn custom_payload(
        hardware_name: &[u8],
        firmware_name: &[u8],
        uuid: [u8; UUID_BYTES],
        test_version: u8,
        hardware_type: u8,
        custom_configuration_count: u8,
        crc: Option<[u8; 4]>,
    ) -> Vec<u8> {
        let mut payload = vec![COMM_FW_VERSION, 7, 0];
        payload.extend_from_slice(hardware_name);
        payload.push(0);
        payload.extend_from_slice(&uuid);
        payload.extend_from_slice(&[1, test_version, hardware_type, custom_configuration_count]);
        payload.extend_from_slice(&[1, 0, 1, 0]);
        payload.extend_from_slice(firmware_name);
        payload.push(0);
        if let Some(crc) = crc {
            payload.extend_from_slice(&crc);
        }
        payload
    }

    fn stock_bms_payload() -> Vec<u8> {
        let mut payload = vec![COMM_FW_VERSION, 6, 0];
        payload.extend_from_slice(b"VBMS\0");
        payload.extend(0_u8..UUID_BYTES as u8);
        payload.extend_from_slice(&[1, 2, BMS_HARDWARE_TYPE, 3]);
        payload
    }

    fn framed_payload(document: &str) -> Vec<u8> {
        let frame = hex_bytes(json_string(document, "frameHex"));
        float_view_vesc_frame::decode(&frame).expect("valid committed VESC frame")
    }

    fn bms_payload(case_name: &str) -> Vec<u8> {
        let marker = format!("\"name\": \"{case_name}\"");
        let case = BMS_IDENTITIES
            .split_once(&marker)
            .map(|(_, suffix)| suffix)
            .expect("committed BMS identity case");
        hex_bytes(json_string(case, "payloadHex"))
    }

    fn json_string<'a>(document: &'a str, field: &str) -> &'a str {
        let marker = format!("\"{field}\": \"");
        let value = document
            .split_once(&marker)
            .map(|(_, suffix)| suffix)
            .expect("committed string fixture field");
        value
            .split_once('"')
            .map(|(value, _)| value)
            .expect("terminated fixture string")
    }

    fn hex_bytes(hex: &str) -> Vec<u8> {
        assert!(hex.len().is_multiple_of(2), "even fixture hex");
        (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).expect("fixture hex byte"))
            .collect()
    }
}
