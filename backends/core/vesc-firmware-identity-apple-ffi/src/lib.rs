//! Narrow caller-owned-buffer C ABI for VESC firmware identity decoding.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_apple_ffi_support::{
    Region, contain_status, input_bytes, publish_bytes, validate_disjoint, write_scalar,
};
use float_view_vesc_firmware_identity::{
    FirmwareIdentityRecordOutcome, RECORD_VERSION, decode_record,
};
pub use float_view_vesc_firmware_identity::{
    MAX_ENCODED_NAME_BYTES, MAX_PAYLOAD_BYTES, MAX_RECORD_BYTES, RECORD_HEADER_BYTES,
};

/// Version of the hand-written identity ABI and byte-record layout.
pub const ABI_VERSION: u32 = RECORD_VERSION;
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
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
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
            write_scalar(output_length, 0);
            write_scalar(unsupported_command, 0);
        }
        // SAFETY: validation established a bounded readable region.
        let payload = unsafe { input_bytes(payload, payload_length) };
        let record = match decode_record(payload) {
            FirmwareIdentityRecordOutcome::Record(record) => record,
            FirmwareIdentityRecordOutcome::UnsupportedCommand(command) => {
                // SAFETY: validation established a writable byte output.
                unsafe { write_scalar(unsupported_command, command) };
                return Err(STATUS_UNSUPPORTED_COMMAND);
            }
            FirmwareIdentityRecordOutcome::StructuralFailure => {
                return Err(STATUS_TRUNCATED_PAYLOAD);
            }
            FirmwareIdentityRecordOutcome::InternalFailure => return Err(STATUS_INTERNAL_FAILURE),
        };
        // SAFETY: validation established the bounded caller-owned output.
        unsafe { publish_bytes(record.as_bytes(), output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

fn validate_regions(
    input: *const u8,
    input_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> Result<(), u32> {
    if output_capacity > MAX_RECORD_BYTES {
        return Err(STATUS_INVALID_ARGUMENT);
    }
    let regions = [
        Region::optional(input, input_length).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::optional(output.cast_const(), output_capacity)
            .map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(output_length.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(unsupported_command.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
    ];
    validate_disjoint(&regions).map_err(|_| STATUS_INVALID_ARGUMENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{self, NonNull};

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
}
