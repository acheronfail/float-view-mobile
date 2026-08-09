//! Shared immutable domain vocabulary.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RuntimeGeneration(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationId(pub u64);

pub const MAX_PLATFORM_ASSOCIATION_BYTES: usize = 256;

/// Private platform transport association used only to rediscover and connect
/// before a VESC firmware identity is available.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PlatformAssociation(Vec<u8>);

impl PlatformAssociation {
    /// Copies a bounded non-empty private platform association.
    ///
    /// # Errors
    ///
    /// Rejects empty values and values larger than the private bridge bound.
    pub fn new(value: Vec<u8>) -> Result<Self, DomainError> {
        if value.is_empty() || value.len() > MAX_PLATFORM_ASSOCIATION_BYTES {
            Err(DomainError::InvalidPlatformAssociation)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

/// Meaningful UUID returned in the fixed 12-byte VESC firmware field.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FirmwareDeviceUuid([u8; 12]);

impl FirmwareDeviceUuid {
    /// Accepts only meaningful non-zero firmware UUIDs.
    ///
    /// # Errors
    ///
    /// The all-zero value means the firmware has no usable hardware identity.
    pub const fn new(value: [u8; 12]) -> Result<Self, DomainError> {
        let mut index = 0;
        while index < value.len() {
            if value[index] != 0 {
                return Ok(Self(value));
            }
            index += 1;
        }
        Err(DomainError::UnavailableFirmwareUuid)
    }

    #[must_use]
    pub const fn bytes(self) -> [u8; 12] {
        self.0
    }

    /// Returns the canonical application representation: 24 lowercase hex characters.
    #[must_use]
    pub fn canonical(self) -> String {
        let mut value = String::with_capacity(24);
        for byte in self.0 {
            use std::fmt::Write as _;
            let _ = write!(&mut value, "{byte:02x}");
        }
        value
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CanId(u8);

impl CanId {
    /// Constructs a controller-area-network node identifier.
    ///
    /// # Errors
    ///
    /// Rejects the VESC broadcast/reserved range above 253.
    pub const fn new(value: u8) -> Result<Self, DomainError> {
        if value <= 253 {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidCanId)
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainError {
    InvalidCanId,
    InvalidPlatformAssociation,
    UnavailableFirmwareUuid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RideState {
    Idle,
    Riding,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareRecord {
    pub major: u8,
    pub minor: u8,
    pub test_version: u8,
    pub hardware_name: Option<String>,
    pub firmware_name: Option<String>,
    pub hardware_type: u8,
    pub can_id: Option<CanId>,
    /// Canonical VESC controller identity, available to application surfaces.
    pub controller_uuid: Option<[u8; 12]>,
    pub custom_configuration_count: u8,
    /// Private cache hint; never use it alone for compatibility.
    pub hardware_configuration_crc: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerCandidate {
    pub can_id: CanId,
    pub hardware_name: Option<String>,
    pub firmware_name: Option<String>,
    pub vesc_version: String,
    pub vesc_supported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_id_rejects_reserved_values() {
        assert_eq!(CanId::new(253).unwrap().get(), 253);
        assert_eq!(CanId::new(254), Err(DomainError::InvalidCanId));
    }

    #[test]
    fn identity_types_enforce_distinct_bounds_and_canonical_uuid() {
        assert_eq!(
            PlatformAssociation::new(Vec::new()),
            Err(DomainError::InvalidPlatformAssociation)
        );
        assert!(PlatformAssociation::new(vec![7; MAX_PLATFORM_ASSOCIATION_BYTES]).is_ok());
        assert_eq!(
            PlatformAssociation::new(vec![7; MAX_PLATFORM_ASSOCIATION_BYTES + 1]),
            Err(DomainError::InvalidPlatformAssociation)
        );
        assert_eq!(
            FirmwareDeviceUuid::new([0; 12]),
            Err(DomainError::UnavailableFirmwareUuid)
        );
        let uuid = FirmwareDeviceUuid::new([9; 12]).unwrap();
        assert_eq!(uuid.bytes(), [9; 12]);
        assert_eq!(uuid.canonical(), "09".repeat(12));
    }
}
