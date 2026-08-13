//! Narrow caller-owned-buffer C ABI for Refloat dynamic telemetry decoding.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_apple_ffi_support::{
    Region, contain_status, input_bytes, publish_bytes, validate_disjoint, write_scalar,
};
pub use float_view_refloat_dynamic_telemetry::{
    LAYOUT_RECORD_HEADER_BYTES, MAX_LAYOUT_PAYLOAD_BYTES, MAX_LAYOUT_RECORD_BYTES,
    MAX_REALTIME_PAYLOAD_BYTES, MAX_SAMPLE_RECORD_BYTES, SAMPLE_FIXED_BYTES,
    SAMPLE_RECORD_HEADER_BYTES,
};
use float_view_refloat_dynamic_telemetry::{
    LayoutRecordOutcome, RECORD_VERSION, SampleRecordOutcome, decode_layout_to_record,
    decode_realtime_to_record,
};

pub const ABI_VERSION: u32 = u32::from_be_bytes([0, 0, 0, RECORD_VERSION]);
pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
pub const STATUS_UNSUPPORTED_ENVELOPE: u32 = 2;
pub const STATUS_TRUNCATED_PAYLOAD: u32 = 3;
pub const STATUS_INVALID_LAYOUT: u32 = 4;
pub const STATUS_INVALID_TELEMETRY: u32 = 5;
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 6;
pub const STATUS_INTERNAL_FAILURE: u32 = 7;

#[unsafe(no_mangle)]
pub extern "C" fn float_view_refloat_dynamic_telemetry_abi_version() -> u32 {
    ABI_VERSION
}

/// Decode one command-32 payload into a caller-owned canonical layout record.
///
/// # Safety
///
/// Non-empty input/output regions must be valid for their supplied byte lengths.
/// `output_length` must be valid and aligned for one `usize`. All regions must
/// be pairwise disjoint and remain valid for the call. Null byte pointers are
/// accepted only when their corresponding length or capacity is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_dynamic_telemetry_decode_layout(
    payload: *const u8,
    payload_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        if payload_length > MAX_LAYOUT_PAYLOAD_BYTES {
            return Err(STATUS_TRUNCATED_PAYLOAD);
        }
        validate_regions(
            &[(payload, payload_length)],
            output,
            output_capacity,
            MAX_LAYOUT_RECORD_BYTES,
            output_length,
        )?;
        // SAFETY: validation established a distinct writable scalar.
        unsafe { write_scalar(output_length, 0) };
        // SAFETY: validation established a bounded readable region.
        let payload = unsafe { input_bytes(payload, payload_length) };
        let record = match decode_layout_to_record(payload) {
            LayoutRecordOutcome::Record(record) => record,
            LayoutRecordOutcome::UnsupportedEnvelope => return Err(STATUS_UNSUPPORTED_ENVELOPE),
            LayoutRecordOutcome::TruncatedPayload => return Err(STATUS_TRUNCATED_PAYLOAD),
            LayoutRecordOutcome::InvalidLayout => return Err(STATUS_INVALID_LAYOUT),
        };
        // SAFETY: validation established the caller-owned output and length scalar.
        unsafe { publish_bytes(record.as_bytes(), output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

/// Decode one command-31 payload using a canonical layout record.
///
/// # Safety
///
/// Both non-empty inputs and the output must be valid for their declared byte
/// extents. `output_length` must be valid and aligned. All four regions must be
/// pairwise disjoint and remain valid for the call. Null byte pointers are
/// accepted only for zero-length regions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_dynamic_telemetry_decode_realtime(
    layout_record: *const u8,
    layout_record_length: usize,
    payload: *const u8,
    payload_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        if layout_record_length > MAX_LAYOUT_RECORD_BYTES {
            return Err(STATUS_INVALID_LAYOUT);
        }
        if payload_length > MAX_REALTIME_PAYLOAD_BYTES {
            return Err(STATUS_TRUNCATED_PAYLOAD);
        }
        validate_regions(
            &[
                (layout_record, layout_record_length),
                (payload, payload_length),
            ],
            output,
            output_capacity,
            MAX_SAMPLE_RECORD_BYTES,
            output_length,
        )?;
        // SAFETY: validation established a distinct writable scalar.
        unsafe { write_scalar(output_length, 0) };
        // SAFETY: validation established bounded readable input regions.
        let layout_record = unsafe { input_bytes(layout_record, layout_record_length) };
        // SAFETY: validation established bounded readable input regions.
        let payload = unsafe { input_bytes(payload, payload_length) };
        let record = match decode_realtime_to_record(layout_record, payload) {
            SampleRecordOutcome::Record(record) => record,
            SampleRecordOutcome::InvalidLayoutRecord => return Err(STATUS_INVALID_LAYOUT),
            SampleRecordOutcome::UnsupportedEnvelope => return Err(STATUS_UNSUPPORTED_ENVELOPE),
            SampleRecordOutcome::TruncatedPayload => return Err(STATUS_TRUNCATED_PAYLOAD),
            SampleRecordOutcome::InvalidTelemetry => return Err(STATUS_INVALID_TELEMETRY),
        };
        // SAFETY: validation established the caller-owned output and length scalar.
        unsafe { publish_bytes(record.as_bytes(), output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

fn validate_regions(
    inputs: &[(*const u8, usize)],
    output: *mut u8,
    output_capacity: usize,
    maximum_output: usize,
    output_length: *mut usize,
) -> Result<(), u32> {
    if output_capacity > maximum_output {
        return Err(STATUS_INVALID_ARGUMENT);
    }
    let mut regions = Vec::with_capacity(inputs.len() + 2);
    for (pointer, length) in inputs {
        regions.push(Region::optional(*pointer, *length).map_err(|_| STATUS_INVALID_ARGUMENT)?);
    }
    regions.push(
        Region::optional(output.cast_const(), output_capacity)
            .map_err(|_| STATUS_INVALID_ARGUMENT)?,
    );
    regions
        .push(Region::required(output_length.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?);
    validate_disjoint(&regions).map_err(|_| STATUS_INVALID_ARGUMENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{self, NonNull};

    fn layout_payload() -> Vec<u8> {
        let names = ["motor.speed", "motor.batt_voltage", "future.value"];
        let mut payload = vec![36, 101, 32, names.len() as u8];
        for name in names {
            payload.push(name.len() as u8);
            payload.extend_from_slice(name.as_bytes());
        }
        payload.push(0);
        payload
    }

    fn realtime_payload() -> Vec<u8> {
        let mut payload = vec![36, 101, 31, 0, 0, 0, 0, 0, 9, 2, 0, 0, 0];
        payload.extend_from_slice(&0x3c00_u16.to_be_bytes());
        payload.extend_from_slice(&0x4000_u16.to_be_bytes());
        payload.extend_from_slice(&0x4200_u16.to_be_bytes());
        payload
    }

    #[test]
    fn publishes_exact_layout_and_sample_records() {
        assert_eq!(float_view_refloat_dynamic_telemetry_abi_version(), 1);
        let layout_payload = layout_payload();
        let mut layout = [0_u8; MAX_LAYOUT_RECORD_BYTES];
        let mut layout_length = usize::MAX;
        // SAFETY: all regions are valid and pairwise disjoint.
        assert_eq!(
            unsafe {
                float_view_refloat_dynamic_telemetry_decode_layout(
                    layout_payload.as_ptr(),
                    layout_payload.len(),
                    layout.as_mut_ptr(),
                    layout.len(),
                    &mut layout_length,
                )
            },
            STATUS_OK
        );
        assert_eq!(&layout[..8], &[1, 1, 3, 0, 0, 41, 0, 0]);

        let payload = realtime_payload();
        let mut sample = [0_u8; MAX_SAMPLE_RECORD_BYTES];
        let mut sample_length = usize::MAX;
        // SAFETY: all regions are valid and pairwise disjoint.
        assert_eq!(
            unsafe {
                float_view_refloat_dynamic_telemetry_decode_realtime(
                    layout.as_ptr(),
                    layout_length,
                    payload.as_ptr(),
                    payload.len(),
                    sample.as_mut_ptr(),
                    sample.len(),
                    &mut sample_length,
                )
            },
            STATUS_OK
        );
        assert_eq!(
            &sample[..16],
            &[1, 2, 3, 1, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0]
        );
        assert_eq!(&sample[16..20], &[0, 0, 0, 9]);
        assert_eq!(sample_length, SAMPLE_FIXED_BYTES + 1 + 12 + 8);
    }

    #[test]
    fn maps_safe_failures_and_initializes_lengths() {
        let mut output = [0_u8; MAX_LAYOUT_RECORD_BYTES];
        let mut output_length = usize::MAX;
        for (payload, expected) in [
            (vec![37, 101, 32], STATUS_UNSUPPORTED_ENVELOPE),
            (vec![36, 101, 32], STATUS_TRUNCATED_PAYLOAD),
            (vec![36, 101, 32, 0, 0], STATUS_INVALID_LAYOUT),
        ] {
            // SAFETY: all regions are valid and pairwise disjoint.
            assert_eq!(
                unsafe {
                    float_view_refloat_dynamic_telemetry_decode_layout(
                        payload.as_ptr(),
                        payload.len(),
                        output.as_mut_ptr(),
                        output.len(),
                        &mut output_length,
                    )
                },
                expected
            );
            assert_eq!(output_length, 0);
            output_length = usize::MAX;
        }
    }

    #[test]
    fn validates_every_region_and_reports_capacity() {
        let payload = layout_payload();
        let mut small = [9_u8; 2];
        let mut required = 0;
        // SAFETY: all regions are valid and pairwise disjoint.
        assert_eq!(
            unsafe {
                float_view_refloat_dynamic_telemetry_decode_layout(
                    payload.as_ptr(),
                    payload.len(),
                    small.as_mut_ptr(),
                    small.len(),
                    &mut required,
                )
            },
            STATUS_OUTPUT_TOO_SMALL
        );
        assert!(required > small.len());
        assert_eq!(small, [9, 9]);

        // SAFETY: null input is rejected before access.
        assert_eq!(
            unsafe {
                float_view_refloat_dynamic_telemetry_decode_layout(
                    ptr::null(),
                    1,
                    small.as_mut_ptr(),
                    small.len(),
                    &mut required,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        let mut overlap = [0_u8; 32];
        let overlap_length = overlap.as_mut_ptr().cast::<usize>();
        // SAFETY: overlapping output regions are rejected.
        assert_eq!(
            unsafe {
                float_view_refloat_dynamic_telemetry_decode_layout(
                    payload.as_ptr(),
                    payload.len(),
                    overlap.as_mut_ptr(),
                    overlap.len(),
                    overlap_length,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: oversized input is rejected before the dangling pointer is read.
        assert_eq!(
            unsafe {
                float_view_refloat_dynamic_telemetry_decode_layout(
                    NonNull::<u8>::dangling().as_ptr(),
                    MAX_LAYOUT_PAYLOAD_BYTES + 1,
                    ptr::null_mut(),
                    0,
                    &mut required,
                )
            },
            STATUS_TRUNCATED_PAYLOAD
        );
    }
}
