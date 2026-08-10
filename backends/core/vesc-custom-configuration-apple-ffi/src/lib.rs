//! Narrow caller-owned-buffer C ABI for VESC custom-configuration envelopes.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_apple_ffi_support::{
    Region, contain_status, input_bytes, publish_bytes, validate_disjoint, write_scalar,
};
use float_view_vesc_custom_configuration::{
    CustomConfigurationError, CustomConfigurationResponse, MAX_SCHEDULED_PAYLOAD_BYTES,
    ValuesCommand, decode_response, schema_request, values_request, write_request,
};

pub const ABI_VERSION: u32 = 1;
pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
pub const STATUS_INVALID_PAYLOAD: u32 = 2;
pub const STATUS_UNSUPPORTED_COMMAND: u32 = 3;
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 4;
pub const STATUS_INTERNAL_FAILURE: u32 = 5;

pub const OPERATION_SCHEMA: u8 = 1;
pub const OPERATION_CURRENT: u8 = 2;
pub const OPERATION_DEFAULT: u8 = 3;
pub const OPERATION_WRITE: u8 = 4;

pub const RESPONSE_SCHEMA: u8 = 1;
pub const RESPONSE_CURRENT: u8 = 2;
pub const RESPONSE_DEFAULT: u8 = 3;
pub const RESPONSE_WRITE_ACKNOWLEDGEMENT: u8 = 4;

pub const RESULT_RECORD_BYTES: usize = 28;
const RESULT_RECORD_VERSION: u8 = 1;

#[unsafe(no_mangle)]
pub extern "C" fn float_view_vesc_custom_configuration_abi_version() -> u32 {
    ABI_VERSION
}

/// Encode one commands 92–95 request into a caller-owned output buffer.
///
/// # Safety
/// Non-empty byte regions must be valid for their lengths. `output_length`
/// must be valid and aligned for one `usize`. All regions must be pairwise
/// disjoint and remain valid for the call. Rust retains nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_vesc_custom_configuration_encode(
    operation: u8,
    index: u8,
    length: i64,
    offset: i64,
    values: *const u8,
    values_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        validate_byte_regions(
            values,
            values_length,
            output,
            output_capacity,
            output_length,
            MAX_SCHEDULED_PAYLOAD_BYTES,
        )?;
        // SAFETY: validation established a writable aligned scalar.
        unsafe { write_scalar(output_length, 0) };
        // SAFETY: validation established a bounded readable region.
        let values = unsafe { input_bytes(values, values_length) };
        let request = match operation {
            OPERATION_SCHEMA if values.is_empty() => schema_request(index, length, offset)
                .map(|request| request.to_vec())
                .map_err(map_codec_error)?,
            OPERATION_CURRENT if length == 0 && offset == 0 && values.is_empty() => {
                values_request(index, false).to_vec()
            }
            OPERATION_DEFAULT if length == 0 && offset == 0 && values.is_empty() => {
                values_request(index, true).to_vec()
            }
            OPERATION_WRITE if length == 0 && offset == 0 => {
                write_request(index, values).map_err(map_codec_error)?
            }
            _ => return Err(STATUS_INVALID_ARGUMENT),
        };
        // SAFETY: validation established a bounded caller-owned output.
        unsafe { publish_bytes(&request, output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

/// Decode one commands 92–95 response into a fixed caller-owned metadata record.
///
/// Successful version-1 record layout:
/// - 0 record version, 1 response kind, 2 command, 3 index
/// - 4..12 total length (u64 big endian)
/// - 12..20 offset (u64 big endian)
/// - 20..24 opaque data offset in the original payload (u32 big endian)
/// - 24..28 opaque data length (u32 big endian)
///
/// # Safety
/// The caller contract matches `encode`; `unsupported_command` must also be
/// valid for one byte and every region must be pairwise disjoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_vesc_custom_configuration_decode(
    payload: *const u8,
    payload_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        if payload_length > MAX_SCHEDULED_PAYLOAD_BYTES {
            return Err(STATUS_INVALID_PAYLOAD);
        }
        validate_decode_regions(
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
        let response = match decode_response(payload) {
            Ok(response) => response,
            Err(CustomConfigurationError::UnsupportedCommand(command)) => {
                // SAFETY: validation established a writable byte output.
                unsafe { write_scalar(unsupported_command, command) };
                return Err(STATUS_UNSUPPORTED_COMMAND);
            }
            Err(CustomConfigurationError::InvalidPayload) => return Err(STATUS_INVALID_PAYLOAD),
        };
        let record = result_record(response)?;
        // SAFETY: validation established the bounded caller-owned output.
        unsafe { publish_bytes(&record, output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

fn result_record(
    response: CustomConfigurationResponse<'_>,
) -> Result<[u8; RESULT_RECORD_BYTES], u32> {
    let (kind, command, index, total, offset, data_offset, data_length) = match response {
        CustomConfigurationResponse::SchemaChunk {
            index,
            total_length,
            offset,
            bytes,
        } => (
            RESPONSE_SCHEMA,
            92,
            index,
            total_length,
            offset,
            10,
            bytes.len(),
        ),
        CustomConfigurationResponse::Values {
            command,
            index,
            bytes,
        } => (
            match command {
                ValuesCommand::Current => RESPONSE_CURRENT,
                ValuesCommand::Default => RESPONSE_DEFAULT,
            },
            command.wire_value(),
            index,
            0,
            0,
            2,
            bytes.len(),
        ),
        CustomConfigurationResponse::WriteAcknowledgement => {
            (RESPONSE_WRITE_ACKNOWLEDGEMENT, 95, 0, 0, 0, 0, 0)
        }
    };
    let total = u64::try_from(total).map_err(|_| STATUS_INTERNAL_FAILURE)?;
    let offset = u64::try_from(offset).map_err(|_| STATUS_INTERNAL_FAILURE)?;
    let data_offset = u32::try_from(data_offset).map_err(|_| STATUS_INTERNAL_FAILURE)?;
    let data_length = u32::try_from(data_length).map_err(|_| STATUS_INTERNAL_FAILURE)?;
    let mut record = [0_u8; RESULT_RECORD_BYTES];
    record[..4].copy_from_slice(&[RESULT_RECORD_VERSION, kind, command, index]);
    record[4..12].copy_from_slice(&total.to_be_bytes());
    record[12..20].copy_from_slice(&offset.to_be_bytes());
    record[20..24].copy_from_slice(&data_offset.to_be_bytes());
    record[24..28].copy_from_slice(&data_length.to_be_bytes());
    Ok(record)
}

fn map_codec_error(error: CustomConfigurationError) -> u32 {
    match error {
        CustomConfigurationError::InvalidPayload => STATUS_INVALID_PAYLOAD,
        CustomConfigurationError::UnsupportedCommand(_) => STATUS_INTERNAL_FAILURE,
    }
}

fn validate_byte_regions(
    input: *const u8,
    input_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    maximum_output: usize,
) -> Result<(), u32> {
    if input_length > MAX_SCHEDULED_PAYLOAD_BYTES || output_capacity > maximum_output {
        return Err(STATUS_INVALID_ARGUMENT);
    }
    let regions = [
        Region::optional(input, input_length).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::optional(output.cast_const(), output_capacity)
            .map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(output_length.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
    ];
    validate_disjoint(&regions).map_err(|_| STATUS_INVALID_ARGUMENT)
}

fn validate_decode_regions(
    input: *const u8,
    input_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> Result<(), u32> {
    if input_length > MAX_SCHEDULED_PAYLOAD_BYTES || output_capacity > RESULT_RECORD_BYTES {
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

    unsafe fn encode(
        operation: u8,
        index: u8,
        length: i64,
        offset: i64,
        values: &[u8],
        output: &mut [u8],
        output_length: &mut usize,
    ) -> u32 {
        // SAFETY: test-owned regions are valid and disjoint.
        unsafe {
            float_view_vesc_custom_configuration_encode(
                operation,
                index,
                length,
                offset,
                values.as_ptr(),
                values.len(),
                output.as_mut_ptr(),
                output.len(),
                output_length,
            )
        }
    }

    #[test]
    fn encodes_every_operation_exactly() {
        assert_eq!(float_view_vesc_custom_configuration_abi_version(), 1);
        let mut output = [0_u8; 32];
        let mut length = 0;
        // SAFETY: helper receives disjoint test-owned regions.
        assert_eq!(
            unsafe { encode(OPERATION_SCHEMA, 7, 400, 3, &[], &mut output, &mut length) },
            STATUS_OK
        );
        assert_eq!(&output[..length], &[92, 7, 0, 0, 1, 144, 0, 0, 0, 3]);
        // SAFETY: helper receives disjoint test-owned regions.
        assert_eq!(
            unsafe { encode(OPERATION_CURRENT, 8, 0, 0, &[], &mut output, &mut length) },
            STATUS_OK
        );
        assert_eq!(&output[..length], &[93, 8]);
        // SAFETY: helper receives disjoint test-owned regions.
        assert_eq!(
            unsafe { encode(OPERATION_DEFAULT, 9, 0, 0, &[], &mut output, &mut length) },
            STATUS_OK
        );
        assert_eq!(&output[..length], &[94, 9]);
        // SAFETY: helper receives disjoint test-owned regions.
        assert_eq!(
            unsafe {
                encode(
                    OPERATION_WRITE,
                    10,
                    0,
                    0,
                    &[1, 2, 3, 4],
                    &mut output,
                    &mut length,
                )
            },
            STATUS_OK
        );
        assert_eq!(&output[..length], &[95, 10, 1, 2, 3, 4]);
    }

    #[test]
    fn decodes_every_response_into_fixed_records() {
        let cases: &[(&[u8], [u8; RESULT_RECORD_BYTES])] = &[
            (
                &[92, 7, 0, 0, 0, 4, 0, 0, 0, 0, 1, 2, 3, 4],
                [
                    1, 1, 92, 7, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0,
                    0, 4,
                ],
            ),
            (
                &[93, 8, 0, 0, 0, 1],
                [
                    1, 2, 93, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0,
                    0, 4,
                ],
            ),
            (
                &[94, 9, 0, 0, 0, 2],
                [
                    1, 3, 94, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0,
                    0, 4,
                ],
            ),
            (
                &[95],
                [
                    1, 4, 95, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0,
                ],
            ),
        ];
        for (payload, expected) in cases {
            let mut record = [0_u8; RESULT_RECORD_BYTES];
            let mut record_length = 0;
            let mut unsupported = 0;
            // SAFETY: all test-owned regions are valid and disjoint.
            let status = unsafe {
                float_view_vesc_custom_configuration_decode(
                    payload.as_ptr(),
                    payload.len(),
                    record.as_mut_ptr(),
                    record.len(),
                    &mut record_length,
                    &mut unsupported,
                )
            };
            assert_eq!(status, STATUS_OK);
            assert_eq!(record_length, RESULT_RECORD_BYTES);
            assert_eq!(&record, expected);
            assert_eq!(unsupported, 0);
        }
    }

    #[test]
    fn maps_failures_and_bounds_regions() {
        let mut output = [0_u8; RESULT_RECORD_BYTES];
        let mut length = 99;
        let mut unsupported = 99;
        // SAFETY: all test-owned regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_vesc_custom_configuration_decode(
                    [36].as_ptr(),
                    1,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut length,
                    &mut unsupported,
                )
            },
            STATUS_UNSUPPORTED_COMMAND
        );
        assert_eq!((length, unsupported), (0, 36));
        // SAFETY: all test-owned regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_vesc_custom_configuration_decode(
                    [1].as_ptr(),
                    1,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut length,
                    &mut unsupported,
                )
            },
            STATUS_INVALID_PAYLOAD
        );
        assert_eq!((length, unsupported), (0, 0));

        let mut small = [0_u8; 2];
        // SAFETY: all test-owned regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_vesc_custom_configuration_decode(
                    [95].as_ptr(),
                    1,
                    small.as_mut_ptr(),
                    small.len(),
                    &mut length,
                    &mut unsupported,
                )
            },
            STATUS_OUTPUT_TOO_SMALL
        );
        assert_eq!(length, RESULT_RECORD_BYTES);
        // SAFETY: oversized capacity is rejected before access.
        assert_eq!(
            unsafe {
                float_view_vesc_custom_configuration_decode(
                    [95].as_ptr(),
                    1,
                    output.as_mut_ptr(),
                    RESULT_RECORD_BYTES + 1,
                    &mut length,
                    &mut unsupported,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: oversized payload is rejected before the dangling pointer is read.
        assert_eq!(
            unsafe {
                float_view_vesc_custom_configuration_decode(
                    NonNull::<u8>::dangling().as_ptr(),
                    MAX_SCHEDULED_PAYLOAD_BYTES + 1,
                    ptr::null_mut(),
                    0,
                    &mut length,
                    &mut unsupported,
                )
            },
            STATUS_INVALID_PAYLOAD
        );

        let mut overlap = [0_u8; 64];
        let overlap_length = overlap.as_mut_ptr().cast::<usize>();
        // SAFETY: deliberately overlapping outputs are rejected.
        assert_eq!(
            unsafe {
                float_view_vesc_custom_configuration_decode(
                    [95].as_ptr(),
                    1,
                    overlap.as_mut_ptr(),
                    RESULT_RECORD_BYTES,
                    overlap_length,
                    &mut unsupported,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: deliberately null input is rejected before access.
        assert_eq!(
            unsafe {
                float_view_vesc_custom_configuration_encode(
                    OPERATION_WRITE,
                    0,
                    0,
                    0,
                    ptr::null(),
                    1,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut length,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: operation shape is invalid but regions are valid.
        assert_eq!(
            unsafe { encode(OPERATION_CURRENT, 0, 1, 0, &[], &mut output, &mut length) },
            STATUS_INVALID_ARGUMENT
        );
    }

    #[test]
    fn reports_required_encode_output_and_contains_panics() {
        let mut small = [0_u8; 2];
        let mut length = 0;
        // SAFETY: helper receives disjoint test-owned regions.
        assert_eq!(
            unsafe {
                encode(
                    OPERATION_WRITE,
                    0,
                    0,
                    0,
                    &[1, 2, 3, 4],
                    &mut small,
                    &mut length,
                )
            },
            STATUS_OUTPUT_TOO_SMALL
        );
        assert_eq!(length, 6);
    }
}
