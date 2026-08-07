use float_view_domain::FirmwareRecord;

use super::{ProtocolError, VescCommand};

pub const MOTOR_HARDWARE_TYPE: u8 = 0;
pub const BMS_HARDWARE_TYPE: u8 = 1;
const MAX_NAME_BYTES: usize = 64;
const SUPPORTED: &[(u8, u8)] = &[(6, 2), (6, 5), (6, 6), (7, 0)];

/// Decodes the strict bounded `COMM_FW_VERSION` common prefix and tail.
///
/// # Errors
///
/// Rejects the wrong command, missing/oversized names, truncated common
/// fields, unsupported BMS tail shapes, and unknown extended tail lengths.
pub fn decode_firmware(payload: &[u8]) -> Result<FirmwareRecord, ProtocolError> {
    let command = *payload.first().ok_or(ProtocolError::TruncatedPayload)?;
    if command != VescCommand::GetFirmwareVersion as u8 {
        return Err(ProtocolError::UnsupportedCommand(command));
    }
    if payload.len() < 3 {
        return Err(ProtocolError::TruncatedPayload);
    }
    let hardware_end = terminated(payload, 3)?;
    let hardware_name = std::str::from_utf8(&payload[3..hardware_end])
        .map_err(|_| ProtocolError::InvalidUtf8)?
        .to_owned();
    let uuid_start = hardware_end + 1;
    let common_end = uuid_start
        .checked_add(16)
        .ok_or(ProtocolError::TruncatedPayload)?;
    let common = payload
        .get(uuid_start..common_end)
        .ok_or(ProtocolError::TruncatedPayload)?;
    let controller_uuid: [u8; 12] = common[..12]
        .try_into()
        .map_err(|_| ProtocolError::TruncatedPayload)?;
    let test_version = common[13];
    let hardware_type = common[14];
    let custom_configuration_count = common[15];

    let (firmware_name, hardware_configuration_crc) = if hardware_type == BMS_HARDWARE_TYPE {
        if payload[1] != 6 || payload[2] != 0 || payload.len() != common_end {
            return Err(ProtocolError::TruncatedPayload);
        }
        (None, None)
    } else {
        extended_tail(payload, common_end)?
    };

    Ok(FirmwareRecord {
        major: payload[1],
        minor: payload[2],
        test_version,
        hardware_name: Some(hardware_name),
        firmware_name,
        hardware_type,
        can_id: None,
        controller_uuid: Some(controller_uuid),
        custom_configuration_count,
        hardware_configuration_crc,
    })
}

fn extended_tail(
    payload: &[u8],
    common_end: usize,
) -> Result<(Option<String>, Option<u32>), ProtocolError> {
    let name_start = common_end
        .checked_add(4)
        .ok_or(ProtocolError::TruncatedPayload)?;
    let name_end = terminated(payload, name_start)?;
    let name = std::str::from_utf8(&payload[name_start..name_end])
        .map_err(|_| ProtocolError::InvalidUtf8)?;
    let tail = &payload[name_end + 1..];
    let crc = match tail {
        [] => None,
        [a, b, c, d] => Some(u32::from_be_bytes([*a, *b, *c, *d])),
        _ => return Err(ProtocolError::TruncatedPayload),
    };
    Ok(((!name.is_empty()).then(|| name.to_owned()), crc))
}

fn terminated(payload: &[u8], start: usize) -> Result<usize, ProtocolError> {
    let bytes = payload
        .get(start..)
        .ok_or(ProtocolError::TruncatedPayload)?;
    let relative = bytes
        .iter()
        .take(MAX_NAME_BYTES + 1)
        .position(|byte| *byte == 0)
        .ok_or(ProtocolError::TruncatedPayload)?;
    Ok(start + relative)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Read,
    Write,
}

#[must_use]
pub fn firmware_access(firmware: &FirmwareRecord) -> Option<AccessMode> {
    if firmware.hardware_type != MOTOR_HARDWARE_TYPE
        || !SUPPORTED.contains(&(firmware.major, firmware.minor))
    {
        return None;
    }
    Some(if firmware.test_version == 0 {
        AccessMode::ReadWrite
    } else {
        AccessMode::ReadOnly
    })
}

#[must_use]
pub fn permits(operation: Operation, firmware: &FirmwareRecord) -> bool {
    matches!(
        (operation, firmware_access(firmware)),
        (Operation::Read, Some(_)) | (Operation::Write, Some(AccessMode::ReadWrite))
    )
}

#[must_use]
pub fn supports_shutdown(firmware: &FirmwareRecord) -> bool {
    firmware_access(firmware) == Some(AccessMode::ReadWrite)
        && [(6, 5), (6, 6), (7, 0)].contains(&(firmware.major, firmware.minor))
}
