//! Refloat 1.2+ command 35, including faults that ended between telemetry polls.
use super::{DynamicTelemetryError, FirmwareFault, firmware_fault};

pub const ALERTS_COMMAND: u8 = 35;
pub const MAX_FAULT_RECORDS: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlertKind {
    FirmwareFault,
    ImuTimeout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FaultRecord {
    pub kind: AlertKind,
    /// Refloat system ticks; retained verbatim rather than treated as wall time.
    pub source_ticks: u32,
    pub active: bool,
    pub raw_code: u8,
}

impl FaultRecord {
    pub fn valid(self) -> bool {
        match self.kind {
            AlertKind::FirmwareFault => self.active == (self.raw_code != 0),
            AlertKind::ImuTimeout => self.raw_code == 0,
        }
    }

    pub fn fault(self) -> Option<FirmwareFault> {
        (self.kind == AlertKind::FirmwareFault && self.raw_code != 0)
            .then(|| firmware_fault(self.raw_code))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Alerts {
    pub imu_timeout_active: Option<bool>,
    pub active_fault_code: u8,
    pub records: Vec<FaultRecord>,
}

pub fn alerts_request(since_ticks: u32) -> Vec<u8> {
    let mut payload = vec![36, 101, ALERTS_COMMAND];
    payload.extend(since_ticks.to_be_bytes());
    payload
}

pub fn decode_alerts(payload: &[u8]) -> Result<Alerts, DynamicTelemetryError> {
    decode_alerts_for_profile(payload, false)
}

pub fn decode_alerts_for_profile(
    payload: &[u8],
    imu_timeout_available: bool,
) -> Result<Alerts, DynamicTelemetryError> {
    let invalid = DynamicTelemetryError::InvalidTelemetry;
    // Header, active fault name, and twenty bounded firmware records.
    if payload.len() < 13
        || payload.len() > 13 + 51 + MAX_FAULT_RECORDS * 58
        || payload[..3] != [36, 101, ALERTS_COMMAND]
    {
        return Err(invalid);
    }
    let active = u32::from_be_bytes(payload[3..7].try_into().map_err(|_| invalid)?);
    let extra = u32::from_be_bytes(payload[7..11].try_into().map_err(|_| invalid)?);
    let active_fault_code = payload[11];
    if active & !(if imu_timeout_available { 3 } else { 1 }) != 0
        || extra != 0
        || (active & 1 != 0) != (active_fault_code != 0)
    {
        return Err(invalid);
    }
    let mut index = 12;
    skip_name(payload, &mut index, active_fault_code)?;
    let count = *payload.get(index).ok_or(invalid)? as usize;
    index += 1;
    if count > MAX_FAULT_RECORDS {
        return Err(invalid);
    }
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let row = payload.get(index..index + 7).ok_or(invalid)?;
        let kind = match row[4] {
            1 => AlertKind::FirmwareFault,
            2 if imu_timeout_available => AlertKind::ImuTimeout,
            _ => return Err(invalid),
        };
        let record = FaultRecord {
            kind,
            source_ticks: u32::from_be_bytes(row[..4].try_into().map_err(|_| invalid)?),
            active: row[5] == 1,
            raw_code: row[6],
        };
        if row[5] > 1 || !record.valid() {
            return Err(invalid);
        }
        index += 7;
        skip_name(payload, &mut index, record.raw_code)?;
        records.push(record);
    }
    if index != payload.len() {
        return Err(invalid);
    }
    Ok(Alerts {
        imu_timeout_active: imu_timeout_available.then_some(active & 2 != 0),
        active_fault_code,
        records,
    })
}

fn skip_name(payload: &[u8], index: &mut usize, code: u8) -> Result<(), DynamicTelemetryError> {
    if code == 0 {
        return Ok(());
    }
    let invalid = DynamicTelemetryError::InvalidTelemetry;
    let length = *payload.get(*index).ok_or(invalid)? as usize;
    *index += 1;
    if length > 50 || payload.get(*index..*index + length).is_none() {
        return Err(invalid);
    }
    // Names are controller-provided display strings. Use our closed enum instead.
    *index += length;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn beta_timeout_records_are_not_firmware_faults() {
        let mut bytes = vec![36, 101, 35, 0, 0, 0, 3, 0, 0, 0, 0, 4, 0, 3];
        bytes.extend([0, 0, 0, 100, 1, 1, 4, 0]);
        bytes.extend([0, 0, 0, 100, 2, 1, 0]);
        bytes.extend([0, 0, 0, 101, 2, 0, 0]);
        let alerts = decode_alerts_for_profile(&bytes, true).unwrap();
        assert_eq!(alerts.imu_timeout_active, Some(true));
        assert_eq!(alerts.active_fault_code, 4);
        assert_eq!(alerts.records[1].kind, AlertKind::ImuTimeout);
        assert!(alerts.records[1].active);
        assert_eq!(alerts.records[1].fault(), None);
        assert!(!alerts.records[2].active);
        assert!(decode_alerts(&bytes).is_err());
        for end in 0..bytes.len() {
            assert!(decode_alerts_for_profile(&bytes[..end], true).is_err());
        }
        for (index, value) in [(6, 4), (10, 1), (26, 3), (27, 2), (28, 4)] {
            let mut invalid = bytes.clone();
            invalid[index] = value;
            assert!(
                decode_alerts_for_profile(&invalid, true).is_err(),
                "byte {index}"
            );
        }
        bytes.push(0);
        assert!(decode_alerts_for_profile(&bytes, true).is_err());
    }

    #[test]
    fn recovers_a_fault_that_has_already_cleared_and_keeps_unknown_codes() {
        let mut bytes = vec![36, 101, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2];
        bytes.extend([0, 0, 0, 100, 1, 1, 255, 3, b'N', b'E', b'W']);
        bytes.extend([0, 0, 0, 101, 1, 0, 0]);
        let alerts = decode_alerts(&bytes).unwrap();
        assert_eq!(alerts.active_fault_code, 0);
        assert_eq!(alerts.records[0].fault(), Some(FirmwareFault::Unknown));
        assert_eq!(alerts.records[0].raw_code, 255);
        assert_eq!(alerts.records[1].fault(), None);
        for end in 0..bytes.len() {
            assert!(decode_alerts(&bytes[..end]).is_err());
        }
        bytes.push(0);
        assert!(decode_alerts(&bytes).is_err());
    }
}
