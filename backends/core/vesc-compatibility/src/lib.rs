//! Exact VESC motor-controller firmware compatibility policy.
//!
//! This unselected safe candidate consumes structurally validated firmware
//! identities directly. It deliberately owns no decoding, routing, discovery,
//! host bridge, persistence, transport, or OS behavior.

#![forbid(unsafe_code)]

use float_view_vesc_firmware_identity::{FirmwareIdentity, MOTOR_HARDWARE_TYPE};

/// One exact audited VESC firmware version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FirmwareVersion {
    /// Firmware major byte.
    pub major: u8,
    /// Firmware minor byte.
    pub minor: u8,
}

impl FirmwareVersion {
    /// Construct one exact version.
    #[must_use]
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    /// Read the public version from a structurally validated identity.
    #[must_use]
    pub const fn from_identity(identity: &FirmwareIdentity) -> Self {
        Self::new(identity.major, identity.minor)
    }
}

/// Exact controller versions covered by immutable product fixtures.
pub const SUPPORTED_VERSIONS: [FirmwareVersion; 4] = [
    FirmwareVersion::new(6, 2),
    FirmwareVersion::new(6, 5),
    FirmwareVersion::new(6, 6),
    FirmwareVersion::new(7, 0),
];

/// Access allowed for one admitted firmware identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    /// Audited nonzero test builds permit reads but no mutations.
    ReadOnly,
    /// Audited stable builds permit reads and guarded mutations.
    ReadWrite,
}

/// Compatibility-gated operation class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    /// A read-only protocol operation.
    Read,
    /// A controller mutation.
    Write,
}

/// Return the access mode for one exact audited motor-controller identity.
///
/// Names, UUID, CRC, custom-configuration count, and all other provenance are
/// intentionally irrelevant to admission.
#[must_use]
pub const fn access_mode(identity: &FirmwareIdentity) -> Option<AccessMode> {
    if identity.hardware_type != MOTOR_HARDWARE_TYPE
        || !is_supported_version(FirmwareVersion::from_identity(identity))
    {
        return None;
    }
    if identity.test_version == 0 {
        Some(AccessMode::ReadWrite)
    } else {
        Some(AccessMode::ReadOnly)
    }
}

/// Return whether an identity belongs to the exact audited controller family.
#[must_use]
pub const fn supports(identity: &FirmwareIdentity) -> bool {
    access_mode(identity).is_some()
}

/// Return whether the requested operation is allowed by exact compatibility.
#[must_use]
pub const fn permits(operation: Operation, identity: &FirmwareIdentity) -> bool {
    matches!(
        (operation, access_mode(identity)),
        (Operation::Read, Some(_)) | (Operation::Write, Some(AccessMode::ReadWrite))
    )
}

/// Return whether the exact stable release supports controller shutdown.
#[must_use]
pub const fn supports_shutdown(identity: &FirmwareIdentity) -> bool {
    if !matches!(access_mode(identity), Some(AccessMode::ReadWrite)) {
        return false;
    }
    matches!(
        FirmwareVersion::from_identity(identity),
        FirmwareVersion {
            major: 6,
            minor: 5 | 6
        } | FirmwareVersion { major: 7, minor: 0 }
    )
}

const fn is_supported_version(version: FirmwareVersion) -> bool {
    matches!(
        version,
        FirmwareVersion {
            major: 6,
            minor: 2 | 5 | 6
        } | FirmwareVersion { major: 7, minor: 0 }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_view_vesc_firmware_identity::decode;
    use float_view_vesc_frame::decode as decode_frame;

    const FIXTURES: [(&str, FirmwareVersion, bool); 4] = [
        (
            include_str!("../../../fixtures/vesc/firmware-thor300-6.02.json"),
            FirmwareVersion::new(6, 2),
            false,
        ),
        (
            include_str!("../../../fixtures/vesc/firmware-thor300-6.05.json"),
            FirmwareVersion::new(6, 5),
            true,
        ),
        (
            include_str!("../../../fixtures/vesc/firmware-thor300-6.06.json"),
            FirmwareVersion::new(6, 6),
            true,
        ),
        (
            include_str!("../../../fixtures/vesc/firmware-thor300-7.00.json"),
            FirmwareVersion::new(7, 0),
            true,
        ),
    ];

    #[test]
    fn immutable_firmware_frames_compose_through_safe_components() {
        for (source, version, shutdown) in FIXTURES {
            let frame = decode_hex(json_string(source, "frameHex"));
            let payload = decode_frame(&frame).expect("fixture frame is valid");
            let identity = decode(&payload).expect("fixture identity is valid");
            assert_eq!(FirmwareVersion::from_identity(&identity), version);
            assert_eq!(access_mode(&identity), Some(AccessMode::ReadWrite));
            assert!(supports(&identity));
            assert!(permits(Operation::Read, &identity));
            assert!(permits(Operation::Write, &identity));
            assert_eq!(supports_shutdown(&identity), shutdown);
        }
    }

    #[test]
    fn complete_major_minor_domain_matches_exact_source_catalogue() {
        for major in u8::MIN..=u8::MAX {
            for minor in u8::MIN..=u8::MAX {
                let expected = matches!((major, minor), (6, 2 | 5 | 6) | (7, 0));
                for test_version in [0, 1, u8::MAX] {
                    let identity = identity(major, minor, test_version, MOTOR_HARDWARE_TYPE);
                    let expected_mode = expected.then_some(if test_version == 0 {
                        AccessMode::ReadWrite
                    } else {
                        AccessMode::ReadOnly
                    });
                    assert_eq!(access_mode(&identity), expected_mode);
                    assert_eq!(supports(&identity), expected);
                    assert_eq!(permits(Operation::Read, &identity), expected);
                    assert_eq!(
                        permits(Operation::Write, &identity),
                        expected && test_version == 0
                    );
                    let expected_shutdown =
                        test_version == 0 && matches!((major, minor), (6, 5 | 6) | (7, 0));
                    assert_eq!(supports_shutdown(&identity), expected_shutdown);
                }
            }
        }
    }

    #[test]
    fn every_non_motor_hardware_boundary_is_rejected() {
        for hardware_type in [1, 2, 3, 127, 253, 254, u8::MAX] {
            for version in SUPPORTED_VERSIONS {
                let identity = identity(version.major, version.minor, 0, hardware_type);
                assert_eq!(access_mode(&identity), None);
                assert!(!supports(&identity));
                assert!(!permits(Operation::Read, &identity));
                assert!(!permits(Operation::Write, &identity));
                assert!(!supports_shutdown(&identity));
            }
        }
    }

    #[test]
    fn unrelated_identity_fields_cannot_change_policy() {
        let baseline = identity(6, 6, 0, MOTOR_HARDWARE_TYPE);
        let changed = FirmwareIdentity {
            major: baseline.major,
            minor: baseline.minor,
            hardware_name: "private-hardware-value".repeat(4),
            firmware_name: Some("private-firmware-value".repeat(3)),
            controller_uuid: [0xde; 12],
            test_version: baseline.test_version,
            hardware_type: baseline.hardware_type,
            custom_configuration_count: u8::MAX,
            hardware_configuration_crc: Some(u32::MAX),
        };
        for operation in [Operation::Read, Operation::Write] {
            assert_eq!(permits(operation, &baseline), permits(operation, &changed));
        }
        assert_eq!(access_mode(&baseline), access_mode(&changed));
        assert_eq!(supports_shutdown(&baseline), supports_shutdown(&changed));

        let diagnostic = format!("{changed:?}");
        assert!(!diagnostic.contains("private-hardware-value"));
        assert!(!diagnostic.contains("private-firmware-value"));
        assert!(!diagnostic.contains("222"));
        assert!(diagnostic.contains("[redacted]"));
    }

    #[test]
    fn catalogue_is_sorted_unique_and_exact() {
        assert_eq!(
            SUPPORTED_VERSIONS,
            [
                FirmwareVersion::new(6, 2),
                FirmwareVersion::new(6, 5),
                FirmwareVersion::new(6, 6),
                FirmwareVersion::new(7, 0),
            ]
        );
        assert!(SUPPORTED_VERSIONS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    fn identity(major: u8, minor: u8, test_version: u8, hardware_type: u8) -> FirmwareIdentity {
        FirmwareIdentity {
            major,
            minor,
            hardware_name: "THOR300".to_owned(),
            firmware_name: None,
            controller_uuid: [0; 12],
            test_version,
            hardware_type,
            custom_configuration_count: 0,
            hardware_configuration_crc: None,
        }
    }

    fn json_string<'a>(source: &'a str, key: &str) -> &'a str {
        let marker = format!("\"{key}\": \"");
        source
            .split_once(&marker)
            .and_then(|(_, tail)| tail.split_once('"'))
            .map(|(value, _)| value)
            .expect("fixture string field exists")
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0);
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).expect("fixture hex is ASCII");
                u8::from_str_radix(text, 16).expect("fixture hex byte is valid")
            })
            .collect()
    }
}
