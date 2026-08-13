//! Stateless caller-owned-buffer C ABI for Refloat compatibility decisions.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_apple_ffi_support::{
    Region, contain_status, input_bytes, publish_bytes, validate_disjoint, write_scalar,
};
pub use float_view_refloat_compatibility::{
    COMPATIBILITY_RECORD_BYTES, INFO_REQUEST, MAX_LAYOUT_RECORD_BYTES, MAX_PAYLOAD_BYTES,
};
use float_view_refloat_compatibility::{
    CompatibilityError, CompatibilityRecordOutcome, RECORD_VERSION, admit_dynamic_layout_record,
    decode_info_record,
};

/// Version of the hand-written compatibility ABI and canonical record.
pub const ABI_VERSION: u32 = u32::from_be_bytes([0, 0, 0, RECORD_VERSION]);
pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
pub const STATUS_UNSUPPORTED_ENVELOPE: u32 = 2;
pub const STATUS_TRUNCATED_PAYLOAD: u32 = 3;
pub const STATUS_UNSUPPORTED_PROTOCOL: u32 = 4;
pub const STATUS_UNSUPPORTED_RELEASE: u32 = 5;
pub const STATUS_INVALID_LAYOUT: u32 = 6;
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 7;
pub const STATUS_INTERNAL_FAILURE: u32 = 8;

#[unsafe(no_mangle)]
pub extern "C" fn float_view_refloat_compatibility_abi_version() -> u32 {
    ABI_VERSION
}

/// Publish the exact unframed INFO request into caller-owned memory.
///
/// # Safety
///
/// The output and length regions must be valid for their declared extents,
/// naturally aligned where applicable, pairwise disjoint, and live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_compatibility_info_request(
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        validate_output(output, output_capacity, output_length, INFO_REQUEST.len())?;
        // SAFETY: validation established a writable scalar output.
        unsafe { write_scalar(output_length, 0) };
        // SAFETY: validation established disjoint bounded output regions.
        unsafe { publish_bytes(&INFO_REQUEST, output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

/// Decode INFO and atomically publish one accepted compatibility record.
///
/// # Safety
///
/// Input/output regions must be valid for their declared extents, naturally
/// aligned where applicable, pairwise disjoint, and live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_compatibility_decode_info(
    payload: *const u8,
    payload_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        if payload_length > MAX_PAYLOAD_BYTES || output_capacity > COMPATIBILITY_RECORD_BYTES {
            return Err(STATUS_INVALID_ARGUMENT);
        }
        validate_regions(
            payload,
            payload_length,
            output,
            output_capacity,
            output_length,
        )?;
        // SAFETY: validation established a writable scalar output.
        unsafe { write_scalar(output_length, 0) };
        // SAFETY: validation established a bounded readable input region.
        let payload = unsafe { input_bytes(payload, payload_length) };
        let record = match decode_info_record(payload) {
            CompatibilityRecordOutcome::Record(record) => record,
            CompatibilityRecordOutcome::InputTooLarge
            | CompatibilityRecordOutcome::TruncatedInfo => return Err(STATUS_TRUNCATED_PAYLOAD),
            CompatibilityRecordOutcome::UnsupportedEnvelope => {
                return Err(STATUS_UNSUPPORTED_ENVELOPE);
            }
            CompatibilityRecordOutcome::UnsupportedInfoProtocol => {
                return Err(STATUS_UNSUPPORTED_PROTOCOL);
            }
            CompatibilityRecordOutcome::UnsupportedRelease => {
                return Err(STATUS_UNSUPPORTED_RELEASE);
            }
            CompatibilityRecordOutcome::InternalFailure => return Err(STATUS_INTERNAL_FAILURE),
        };
        // SAFETY: validation established bounded disjoint output regions.
        unsafe { publish_bytes(record.as_bytes(), output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

/// Admit one canonical Rust dynamic-layout record.
///
/// # Safety
///
/// A nonempty input pointer must be valid and initialized for exactly the
/// declared bounded extent and remain live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_compatibility_admit_layout(
    layout_record: *const u8,
    layout_record_length: usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        if layout_record_length > MAX_LAYOUT_RECORD_BYTES {
            return Err(STATUS_INVALID_LAYOUT);
        }
        Region::optional(layout_record, layout_record_length)
            .map_err(|_| STATUS_INVALID_ARGUMENT)?;
        // SAFETY: validation established a bounded readable input region.
        let layout = unsafe { input_bytes(layout_record, layout_record_length) };
        admit_dynamic_layout_record(layout).map_err(map_layout_error)
    })
}

fn map_layout_error(error: CompatibilityError) -> u32 {
    match error {
        CompatibilityError::InvalidLayoutRecord
        | CompatibilityError::TooManyIdentifiers
        | CompatibilityError::EmptyIdentifier
        | CompatibilityError::IdentifierTooLong
        | CompatibilityError::InvalidIdentifier
        | CompatibilityError::TooManyIdentifierBytes
        | CompatibilityError::DuplicateIdentifier
        | CompatibilityError::MissingRequiredIdentifier
        | CompatibilityError::MissingRequiredRuntimeIdentifier => STATUS_INVALID_LAYOUT,
        _ => STATUS_INTERNAL_FAILURE,
    }
}

fn validate_output(
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    maximum: usize,
) -> Result<(), u32> {
    if output_capacity > maximum {
        return Err(STATUS_INVALID_ARGUMENT);
    }
    let regions = [
        Region::optional(output.cast_const(), output_capacity)
            .map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(output_length.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
    ];
    validate_disjoint(&regions).map_err(|_| STATUS_INVALID_ARGUMENT)
}

fn validate_regions(
    input: *const u8,
    input_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> Result<(), u32> {
    let regions = [
        Region::optional(input, input_length).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::optional(output.cast_const(), output_capacity)
            .map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(output_length.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
    ];
    validate_disjoint(&regions).map_err(|_| STATUS_INVALID_ARGUMENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_exact_request_and_record() {
        let mut request = [0_u8; 5];
        let mut request_length = usize::MAX;
        // SAFETY: outputs are valid, aligned, disjoint, and exactly bounded.
        assert_eq!(
            unsafe {
                float_view_refloat_compatibility_info_request(
                    request.as_mut_ptr(),
                    request.len(),
                    &mut request_length,
                )
            },
            STATUS_OK
        );
        assert_eq!(request, [36, 101, 0, 2, 0]);
        assert_eq!(request_length, 5);

        let payload = [36_u8, 101, 0, 10];
        let mut record = [0_u8; COMPATIBILITY_RECORD_BYTES];
        let mut record_length = usize::MAX;
        // SAFETY: inputs and outputs are valid, aligned, disjoint, and bounded.
        assert_eq!(
            unsafe {
                float_view_refloat_compatibility_decode_info(
                    payload.as_ptr(),
                    payload.len(),
                    record.as_mut_ptr(),
                    record.len(),
                    &mut record_length,
                )
            },
            STATUS_OK
        );
        assert_eq!(record, [1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0]);
        assert_eq!(record_length, record.len());
    }
}
