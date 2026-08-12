//! Narrow caller-owned-buffer C ABI for Refloat legacy telemetry decoding.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_apple_ffi_support::{
    Region, contain_status, input_bytes, publish_bytes, validate_disjoint, write_scalar,
};
pub use float_view_refloat_legacy_telemetry::{
    ACTIVE_FAULT_RECORD_BYTES, MAX_PAYLOAD_BYTES, MAX_RECORD_BYTES, RECORD_HEADER_BYTES,
    SAMPLE_RECORD_BYTES,
};
use float_view_refloat_legacy_telemetry::{
    LegacyTelemetryRecordOutcome, RECORD_VERSION, decode_record,
};

/// Version of the hand-written telemetry ABI and byte-record layout.
pub const ABI_VERSION: u32 = RECORD_VERSION;
pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
pub const STATUS_UNSUPPORTED_ENVELOPE: u32 = 2;
pub const STATUS_TRUNCATED_PAYLOAD: u32 = 3;
pub const STATUS_INVALID_PAYLOAD: u32 = 4;
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 5;
pub const STATUS_INTERNAL_FAILURE: u32 = 6;

#[unsafe(no_mangle)]
pub extern "C" fn float_view_refloat_legacy_telemetry_abi_version() -> u32 {
    ABI_VERSION
}

/// Decode one strict command-10 payload into a caller-owned version-1 record.
///
/// # Safety
///
/// Non-empty input and output regions must be valid for their supplied byte
/// lengths. `output_length` must be valid and aligned for one `usize`;
/// `unsupported_command` must be valid for one byte. All four regions must be
/// pairwise disjoint and remain valid for the call. A null byte pointer is
/// accepted only when its corresponding length or capacity is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_legacy_telemetry_decode(
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
            LegacyTelemetryRecordOutcome::Record(record) => record,
            LegacyTelemetryRecordOutcome::UnsupportedEnvelope(command) => {
                // SAFETY: validation established a writable byte output.
                unsafe { write_scalar(unsupported_command, command) };
                return Err(STATUS_UNSUPPORTED_ENVELOPE);
            }
            LegacyTelemetryRecordOutcome::TruncatedPayload => {
                return Err(STATUS_TRUNCATED_PAYLOAD);
            }
            LegacyTelemetryRecordOutcome::InvalidPayload => return Err(STATUS_INVALID_PAYLOAD),
            LegacyTelemetryRecordOutcome::InternalFailure => return Err(STATUS_INTERNAL_FAILURE),
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

    fn detail_one() -> [u8; 35] {
        let mut payload = [0_u8; 35];
        payload[..4].copy_from_slice(&[36, 101, 10, 1]);
        payload[10] = 1;
        payload[11] = 2;
        payload[12] = 50;
        payload[13] = 100;
        payload[14..20].fill(128);
        payload[22] = 128;
        payload[23..25].copy_from_slice(&500_i16.to_be_bytes());
        payload[33] = 128;
        payload[34] = 222;
        payload
    }

    #[test]
    fn publishes_the_safe_crates_exact_record() {
        assert_eq!(
            float_view_refloat_legacy_telemetry_abi_version(),
            ABI_VERSION
        );
        let payload = detail_one();
        let mut output = [0_u8; MAX_RECORD_BYTES];
        let mut output_length = usize::MAX;
        let mut unsupported = u8::MAX;
        // SAFETY: all regions are valid and pairwise disjoint.
        let status = unsafe {
            float_view_refloat_legacy_telemetry_decode(
                payload.as_ptr(),
                payload.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut output_length,
                &mut unsupported,
            )
        };
        assert_eq!(status, STATUS_OK);
        assert_eq!(output_length, SAMPLE_RECORD_BYTES);
        assert_eq!(
            &output[..16],
            &[1, 1, 1, 2, 1, 4, 1, 1, 0, 1, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(unsupported, 0);
    }

    #[test]
    fn maps_every_safe_failure_and_initializes_outputs() {
        let mut output = [0_u8; MAX_RECORD_BYTES];
        let mut output_length = usize::MAX;
        let mut unsupported = u8::MAX;
        for (payload, expected, command) in [
            (vec![37], STATUS_TRUNCATED_PAYLOAD, 0),
            (vec![37, 101, 10, 1], STATUS_UNSUPPORTED_ENVELOPE, 37),
            (vec![36, 101, 10, 0], STATUS_INVALID_PAYLOAD, 0),
        ] {
            // SAFETY: all regions are valid and pairwise disjoint.
            assert_eq!(
                unsafe {
                    float_view_refloat_legacy_telemetry_decode(
                        payload.as_ptr(),
                        payload.len(),
                        output.as_mut_ptr(),
                        output.len(),
                        &mut output_length,
                        &mut unsupported,
                    )
                },
                expected
            );
            assert_eq!((output_length, unsupported), (0, command));
            output_length = usize::MAX;
            unsupported = u8::MAX;
        }
    }

    #[test]
    fn validates_regions_and_reports_required_capacity() {
        let payload = detail_one();
        let mut small = [0_u8; 2];
        let mut output_length = 0;
        let mut unsupported = 0;
        // SAFETY: all regions are valid and pairwise disjoint.
        assert_eq!(
            unsafe {
                float_view_refloat_legacy_telemetry_decode(
                    payload.as_ptr(),
                    payload.len(),
                    small.as_mut_ptr(),
                    small.len(),
                    &mut output_length,
                    &mut unsupported,
                )
            },
            STATUS_OUTPUT_TOO_SMALL
        );
        assert_eq!(output_length, SAMPLE_RECORD_BYTES);

        // SAFETY: the null input is rejected before access.
        assert_eq!(
            unsafe {
                float_view_refloat_legacy_telemetry_decode(
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
        // SAFETY: overlapping output regions are rejected.
        assert_eq!(
            unsafe {
                float_view_refloat_legacy_telemetry_decode(
                    payload.as_ptr(),
                    payload.len(),
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
                float_view_refloat_legacy_telemetry_decode(
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
