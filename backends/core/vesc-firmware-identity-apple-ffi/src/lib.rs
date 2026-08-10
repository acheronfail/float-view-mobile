//! Narrow caller-owned-buffer C ABI for VESC firmware identity decoding.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_vesc_firmware_identity::{FirmwareIdentity, FirmwareIdentityError, decode};
use std::mem::{align_of, size_of};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::{ptr, slice};

/// Version of the hand-written identity ABI and byte-record layout.
pub const ABI_VERSION: u32 = 1;
/// The operation completed successfully.
pub const STATUS_OK: u32 = 0;
/// A pointer, capacity, alignment, or region contract is invalid.
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
/// The observed VESC command is not `COMM_FW_VERSION`.
pub const STATUS_UNSUPPORTED_COMMAND: u32 = 2;
/// The payload is truncated, oversized, or structurally invalid.
pub const STATUS_TRUNCATED_PAYLOAD: u32 = 3;
/// The result buffer is too small; the required length was returned.
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 4;
/// A panic was contained before it could cross C.
pub const STATUS_INTERNAL_FAILURE: u32 = 5;

/// Largest structurally valid `COMM_FW_VERSION` payload.
pub const MAX_PAYLOAD_BYTES: usize = 157;
/// Fixed bytes before the two replacement-decoded UTF-8 names.
pub const RECORD_HEADER_BYTES: usize = 26;
/// Worst-case UTF-8 bytes after replacement decoding one 64-byte name.
pub const MAX_ENCODED_NAME_BYTES: usize = 64 * 3;
/// Largest version-1 identity record.
pub const MAX_RECORD_BYTES: usize = RECORD_HEADER_BYTES + MAX_ENCODED_NAME_BYTES * 2;

const FLAG_FIRMWARE_NAME_PRESENT: u8 = 1;
const FLAG_HARDWARE_CRC_PRESENT: u8 = 2;

/// Return the exact version of the hand-written identity ABI.
#[unsafe(no_mangle)]
pub extern "C" fn float_view_vesc_firmware_identity_abi_version() -> u32 {
    ABI_VERSION
}

/// Decode one strict firmware identity into a caller-owned version-1 record.
///
/// # Safety
///
/// Non-empty input and output regions must be valid for their supplied byte
/// lengths. `output_length` must be valid and aligned for one `usize`;
/// `unsupported_command` must be valid for one byte. All four regions must be
/// pairwise disjoint and remain valid for the call. A null byte pointer is
/// accepted only when its corresponding length or capacity is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_vesc_firmware_identity_decode(
    payload: *const u8,
    payload_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> u32 {
    contain(|| {
        if payload_length > MAX_PAYLOAD_BYTES {
            return Err(STATUS_TRUNCATED_PAYLOAD);
        }
        validate_regions(
            payload,
            payload_length,
            output,
            output_capacity,
            output_length,
            unsupported_command,
        )?;
        // SAFETY: validation established distinct writable scalar outputs.
        unsafe {
            output_length.write(0);
            unsupported_command.write(0);
        }
        // SAFETY: validation established a bounded readable region.
        let payload = unsafe { input_slice(payload, payload_length) };
        let identity = match decode(payload) {
            Ok(identity) => identity,
            Err(FirmwareIdentityError::UnsupportedCommand(command)) => {
                // SAFETY: validation established a writable byte output.
                unsafe { unsupported_command.write(command) };
                return Err(STATUS_UNSUPPORTED_COMMAND);
            }
            Err(FirmwareIdentityError::TruncatedPayload) => {
                return Err(STATUS_TRUNCATED_PAYLOAD);
            }
        };
        let record = encode_record(&identity)?;
        // SAFETY: validation established the bounded caller-owned output.
        unsafe { write_result(&record, output, output_capacity, output_length) }
    })
}

fn encode_record(identity: &FirmwareIdentity) -> Result<Vec<u8>, u32> {
    let hardware_name = identity.hardware_name.as_bytes();
    let firmware_name = identity
        .firmware_name
        .as_deref()
        .unwrap_or_default()
        .as_bytes();
    if hardware_name.len() > MAX_ENCODED_NAME_BYTES || firmware_name.len() > MAX_ENCODED_NAME_BYTES
    {
        return Err(STATUS_INTERNAL_FAILURE);
    }
    let hardware_length =
        u16::try_from(hardware_name.len()).map_err(|_| STATUS_INTERNAL_FAILURE)?;
    let firmware_length =
        u16::try_from(firmware_name.len()).map_err(|_| STATUS_INTERNAL_FAILURE)?;
    let mut flags = 0;
    if identity.firmware_name.is_some() {
        flags |= FLAG_FIRMWARE_NAME_PRESENT;
    }
    if identity.hardware_configuration_crc.is_some() {
        flags |= FLAG_HARDWARE_CRC_PRESENT;
    }

    let mut record =
        Vec::with_capacity(RECORD_HEADER_BYTES + hardware_name.len() + firmware_name.len());
    record.extend_from_slice(&[
        identity.major,
        identity.minor,
        identity.test_version,
        identity.hardware_type,
        identity.custom_configuration_count,
        flags,
    ]);
    record.extend_from_slice(&hardware_length.to_be_bytes());
    record.extend_from_slice(&firmware_length.to_be_bytes());
    record.extend_from_slice(&identity.controller_uuid);
    record.extend_from_slice(
        &identity
            .hardware_configuration_crc
            .unwrap_or_default()
            .to_be_bytes(),
    );
    record.extend_from_slice(hardware_name);
    record.extend_from_slice(firmware_name);
    debug_assert!(record.len() <= MAX_RECORD_BYTES);
    Ok(record)
}

fn contain(operation: impl FnOnce() -> Result<(), u32>) -> u32 {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => STATUS_OK,
        Ok(Err(status)) => status,
        Err(_) => STATUS_INTERNAL_FAILURE,
    }
}

fn validate_regions(
    input: *const u8,
    input_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> Result<(), u32> {
    if output_length.is_null()
        || unsupported_command.is_null()
        || !(output_length as usize).is_multiple_of(align_of::<usize>())
        || (input.is_null() && input_length != 0)
        || (output.is_null() && output_capacity != 0)
        || output_capacity > MAX_RECORD_BYTES
    {
        return Err(STATUS_INVALID_ARGUMENT);
    }
    let regions = [
        address_region(input as usize, input_length)?,
        address_region(output as usize, output_capacity)?,
        address_region(output_length as usize, size_of::<usize>())?,
        address_region(unsupported_command as usize, size_of::<u8>())?,
    ];
    for left in 0..regions.len() {
        for right in left + 1..regions.len() {
            if regions_overlap(regions[left], regions[right]) {
                return Err(STATUS_INVALID_ARGUMENT);
            }
        }
    }
    Ok(())
}

fn address_region(start: usize, length: usize) -> Result<(usize, usize), u32> {
    if length == 0 {
        return Ok((start, start));
    }
    Ok((
        start,
        start.checked_add(length).ok_or(STATUS_INVALID_ARGUMENT)?,
    ))
}

fn regions_overlap(left: (usize, usize), right: (usize, usize)) -> bool {
    left.0 != left.1 && right.0 != right.1 && left.0 < right.1 && right.0 < left.1
}

unsafe fn input_slice<'a>(input: *const u8, input_length: usize) -> &'a [u8] {
    if input_length == 0 {
        &[]
    } else {
        // SAFETY: the exported caller contract and validation establish this region.
        unsafe { slice::from_raw_parts(input, input_length) }
    }
}

unsafe fn write_result(
    bytes: &[u8],
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> Result<(), u32> {
    // SAFETY: validation established a writable aligned length field.
    unsafe { output_length.write(bytes.len()) };
    if output_capacity < bytes.len() {
        return Err(STATUS_OUTPUT_TOO_SMALL);
    }
    if !bytes.is_empty() {
        // SAFETY: validation and the capacity check establish the output region.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len()) };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::NonNull;

    const STOCK_BMS: [u8; 24] = [
        0, 6, 0, b'V', b'B', b'M', b'S', 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0, 0, 1, 1,
    ];

    #[test]
    fn decodes_source_identity_into_versioned_record() {
        assert_eq!(float_view_vesc_firmware_identity_abi_version(), ABI_VERSION);
        let mut output = [0_u8; MAX_RECORD_BYTES];
        let mut output_length = 0;
        let mut unsupported = 99;
        // SAFETY: all regions are valid and pairwise disjoint.
        let status = unsafe {
            float_view_vesc_firmware_identity_decode(
                STOCK_BMS.as_ptr(),
                STOCK_BMS.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut output_length,
                &mut unsupported,
            )
        };
        assert_eq!(status, STATUS_OK);
        assert_eq!(output_length, RECORD_HEADER_BYTES + 4);
        assert_eq!(&output[..10], &[6, 0, 0, 1, 1, 0, 0, 4, 0, 0]);
        assert_eq!(&output[10..22], &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(&output[22..26], &[0, 0, 0, 0]);
        assert_eq!(&output[26..output_length], b"VBMS");
        assert_eq!(unsupported, 0);
    }

    #[test]
    fn maps_semantic_failures_to_fixed_statuses() {
        let mut output = [0_u8; MAX_RECORD_BYTES];
        let mut output_length = 9;
        let mut unsupported = 9;
        // SAFETY: all regions are valid and pairwise disjoint.
        assert_eq!(
            unsafe {
                float_view_vesc_firmware_identity_decode(
                    [37_u8].as_ptr(),
                    1,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut output_length,
                    &mut unsupported,
                )
            },
            STATUS_UNSUPPORTED_COMMAND
        );
        assert_eq!((output_length, unsupported), (0, 37));
        // SAFETY: all regions are valid and pairwise disjoint.
        assert_eq!(
            unsafe {
                float_view_vesc_firmware_identity_decode(
                    [0_u8].as_ptr(),
                    1,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut output_length,
                    &mut unsupported,
                )
            },
            STATUS_TRUNCATED_PAYLOAD
        );
        assert_eq!((output_length, unsupported), (0, 0));
    }

    #[test]
    fn validates_caller_regions_and_reports_required_output() {
        let mut small = [0_u8; 2];
        let mut output_length = 0;
        let mut unsupported = 0;
        // SAFETY: all regions are valid and pairwise disjoint.
        assert_eq!(
            unsafe {
                float_view_vesc_firmware_identity_decode(
                    STOCK_BMS.as_ptr(),
                    STOCK_BMS.len(),
                    small.as_mut_ptr(),
                    small.len(),
                    &mut output_length,
                    &mut unsupported,
                )
            },
            STATUS_OUTPUT_TOO_SMALL
        );
        assert_eq!(output_length, RECORD_HEADER_BYTES + 4);

        // SAFETY: the deliberately null input is rejected before access.
        assert_eq!(
            unsafe {
                float_view_vesc_firmware_identity_decode(
                    ptr::null(),
                    1,
                    small.as_mut_ptr(),
                    small.len(),
                    &mut output_length,
                    &mut unsupported,
                )
            },
            STATUS_INVALID_ARGUMENT
        );

        let mut overlap = [0_u8; 32];
        let overlap_length = overlap.as_mut_ptr().cast::<usize>();
        // SAFETY: the deliberately overlapping output regions are rejected.
        assert_eq!(
            unsafe {
                float_view_vesc_firmware_identity_decode(
                    STOCK_BMS.as_ptr(),
                    STOCK_BMS.len(),
                    overlap.as_mut_ptr(),
                    overlap.len(),
                    overlap_length,
                    &mut unsupported,
                )
            },
            STATUS_INVALID_ARGUMENT
        );

        // SAFETY: oversized input is rejected before the dangling pointer is read.
        assert_eq!(
            unsafe {
                float_view_vesc_firmware_identity_decode(
                    NonNull::<u8>::dangling().as_ptr(),
                    MAX_PAYLOAD_BYTES + 1,
                    ptr::null_mut(),
                    0,
                    &mut output_length,
                    &mut unsupported,
                )
            },
            STATUS_TRUNCATED_PAYLOAD
        );
    }

    #[test]
    fn contains_panics() {
        assert_eq!(
            contain(|| -> Result<(), u32> { panic!("test-only contained panic") }),
            STATUS_INTERNAL_FAILURE
        );
    }
}
