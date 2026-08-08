//! Shared immutable domain vocabulary.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RuntimeGeneration(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationId(pub u64);

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
    /// Private controller identity; never expose it through API or logs.
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
}
