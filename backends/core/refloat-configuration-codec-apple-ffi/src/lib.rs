//! Temporary four-symbol caller-owned-buffer C ABI for the safe Refloat configuration codec.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_apple_ffi_support::{
    Region, contain_status, input_bytes, publish_bytes, validate_disjoint, write_scalar,
};
use float_view_refloat_configuration_codec::{
    ConfigurationError, RecordError, decode_qt_compressed_to_record, decode_values_to_record,
    encode_values_from_records,
};
pub use float_view_refloat_configuration_codec::{
    MAX_COMPRESSED_BYTES, MAX_SCHEMA_RECORD_BYTES, MAX_VALUE_PAYLOAD_BYTES, MAX_VALUE_RECORD_BYTES,
};

pub const ABI_VERSION: u32 = 1;
pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
pub const STATUS_INVALID_SCHEMA_RECORD: u32 = 2;
pub const STATUS_INVALID_VALUE_RECORD: u32 = 3;
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 4;
pub const STATUS_INTERNAL_FAILURE: u32 = 5;
pub const STATUS_TRUNCATED_ENVELOPE: u32 = 10;
pub const STATUS_COMPRESSED_TOO_LARGE: u32 = 11;
pub const STATUS_EXPANDED_TOO_LARGE: u32 = 12;
pub const STATUS_EXPANDED_LENGTH_MISMATCH: u32 = 13;
pub const STATUS_DECOMPRESSION_FAILED: u32 = 14;
pub const STATUS_PROHIBITED_XML: u32 = 15;
pub const STATUS_MALFORMED_XML: u32 = 16;
pub const STATUS_STRUCTURAL_LIMIT: u32 = 17;
pub const STATUS_DUPLICATE_FIELD: u32 = 18;
pub const STATUS_DUPLICATE_ORDER: u32 = 19;
pub const STATUS_MISSING_REFERENCE: u32 = 20;
pub const STATUS_UNSUPPORTED_TYPE: u32 = 21;
pub const STATUS_UNSUPPORTED_ENCODING: u32 = 22;
pub const STATUS_INVALID_METADATA: u32 = 23;
pub const STATUS_INVALID_NUMBER: u32 = 24;
pub const STATUS_SIGNATURE_MISMATCH: u32 = 25;
pub const STATUS_TRUNCATED_VALUES: u32 = 26;
pub const STATUS_REMAINDER: u32 = 27;
pub const STATUS_MISSING_VALUE: u32 = 28;
pub const STATUS_EXTRA_VALUE: u32 = 29;
pub const STATUS_INVALID_VALUE: u32 = 30;

#[unsafe(no_mangle)]
pub extern "C" fn float_view_refloat_configuration_codec_abi_version() -> u32 {
    ABI_VERSION
}

/// Decode one complete Qt-compressed schema to a canonical schema record.
///
/// # Safety
/// Input/output byte regions must be valid for their declared extents,
/// `output_length` must be valid and aligned, and all regions must be disjoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_configuration_codec_decode_schema(
    compressed: *const u8,
    compressed_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        if compressed_length > MAX_COMPRESSED_BYTES {
            return Err(STATUS_COMPRESSED_TOO_LARGE);
        }
        validate_regions(
            &[(compressed, compressed_length)],
            output,
            output_capacity,
            MAX_SCHEMA_RECORD_BYTES,
            output_length,
        )?;
        // SAFETY: validation established one distinct writable scalar.
        unsafe { write_scalar(output_length, 0) };
        // SAFETY: validation established one bounded readable region.
        let compressed = unsafe { input_bytes(compressed, compressed_length) };
        let record = decode_qt_compressed_to_record(compressed).map_err(map_record_error)?;
        // SAFETY: validation established the caller-owned output and length scalar.
        unsafe { publish_bytes(&record, output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

/// Decode one complete signature-prefixed value payload to a canonical value record.
///
/// # Safety
/// Both inputs, the output, and the length scalar must satisfy the same bounded,
/// aligned, pairwise-disjoint caller contract as `decode_schema`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_configuration_codec_decode_values(
    schema_record: *const u8,
    schema_record_length: usize,
    payload: *const u8,
    payload_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        if schema_record_length > MAX_SCHEMA_RECORD_BYTES {
            return Err(STATUS_INVALID_SCHEMA_RECORD);
        }
        if payload_length > MAX_VALUE_PAYLOAD_BYTES {
            return Err(STATUS_INVALID_VALUE);
        }
        validate_regions(
            &[
                (schema_record, schema_record_length),
                (payload, payload_length),
            ],
            output,
            output_capacity,
            MAX_VALUE_RECORD_BYTES,
            output_length,
        )?;
        // SAFETY: validation established one distinct writable scalar.
        unsafe { write_scalar(output_length, 0) };
        // SAFETY: validation established bounded readable regions.
        let schema_record = unsafe { input_bytes(schema_record, schema_record_length) };
        // SAFETY: validation established bounded readable regions.
        let payload = unsafe { input_bytes(payload, payload_length) };
        let record = decode_values_to_record(schema_record, payload).map_err(map_record_error)?;
        // SAFETY: validation established the caller-owned output and length scalar.
        unsafe { publish_bytes(&record, output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

/// Encode one canonical value record using one exact canonical schema record.
///
/// # Safety
/// Both inputs, the output, and the length scalar must satisfy the same bounded,
/// aligned, pairwise-disjoint caller contract as `decode_schema`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_configuration_codec_encode_values(
    schema_record: *const u8,
    schema_record_length: usize,
    value_record: *const u8,
    value_record_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        if schema_record_length > MAX_SCHEMA_RECORD_BYTES {
            return Err(STATUS_INVALID_SCHEMA_RECORD);
        }
        if value_record_length > MAX_VALUE_RECORD_BYTES {
            return Err(STATUS_INVALID_VALUE_RECORD);
        }
        validate_regions(
            &[
                (schema_record, schema_record_length),
                (value_record, value_record_length),
            ],
            output,
            output_capacity,
            MAX_VALUE_PAYLOAD_BYTES,
            output_length,
        )?;
        // SAFETY: validation established one distinct writable scalar.
        unsafe { write_scalar(output_length, 0) };
        // SAFETY: validation established bounded readable regions.
        let schema_record = unsafe { input_bytes(schema_record, schema_record_length) };
        // SAFETY: validation established bounded readable regions.
        let value_record = unsafe { input_bytes(value_record, value_record_length) };
        let payload =
            encode_values_from_records(schema_record, value_record).map_err(map_record_error)?;
        // SAFETY: validation established the caller-owned output and length scalar.
        unsafe { publish_bytes(&payload, output, output_capacity, output_length) }
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

fn map_record_error(error: RecordError) -> u32 {
    match error {
        RecordError::InvalidSchemaRecord => STATUS_INVALID_SCHEMA_RECORD,
        RecordError::InvalidValueRecord => STATUS_INVALID_VALUE_RECORD,
        RecordError::Codec(error) => map_codec_error(error),
    }
}

fn map_codec_error(error: ConfigurationError) -> u32 {
    match error {
        ConfigurationError::TruncatedEnvelope => STATUS_TRUNCATED_ENVELOPE,
        ConfigurationError::CompressedTooLarge => STATUS_COMPRESSED_TOO_LARGE,
        ConfigurationError::ExpandedTooLarge => STATUS_EXPANDED_TOO_LARGE,
        ConfigurationError::ExpandedLengthMismatch => STATUS_EXPANDED_LENGTH_MISMATCH,
        ConfigurationError::DecompressionFailed => STATUS_DECOMPRESSION_FAILED,
        ConfigurationError::ProhibitedXml => STATUS_PROHIBITED_XML,
        ConfigurationError::MalformedXml => STATUS_MALFORMED_XML,
        ConfigurationError::StructuralLimit => STATUS_STRUCTURAL_LIMIT,
        ConfigurationError::DuplicateField => STATUS_DUPLICATE_FIELD,
        ConfigurationError::DuplicateOrder => STATUS_DUPLICATE_ORDER,
        ConfigurationError::MissingReference => STATUS_MISSING_REFERENCE,
        ConfigurationError::UnsupportedType => STATUS_UNSUPPORTED_TYPE,
        ConfigurationError::UnsupportedEncoding => STATUS_UNSUPPORTED_ENCODING,
        ConfigurationError::InvalidMetadata => STATUS_INVALID_METADATA,
        ConfigurationError::InvalidNumber => STATUS_INVALID_NUMBER,
        ConfigurationError::SignatureMismatch => STATUS_SIGNATURE_MISMATCH,
        ConfigurationError::TruncatedValues => STATUS_TRUNCATED_VALUES,
        ConfigurationError::Remainder => STATUS_REMAINDER,
        ConfigurationError::MissingValue => STATUS_MISSING_VALUE,
        ConfigurationError::ExtraValue => STATUS_EXTRA_VALUE,
        ConfigurationError::InvalidValue => STATUS_INVALID_VALUE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    const SCHEMA: &[u8] =
        include_bytes!("../../../fixtures/refloat-configuration/schemas/1.2.3.qcompress");
    const VALUES: &[u8] =
        include_bytes!("../../../fixtures/refloat-configuration/values/1.2.3-current.bin");

    fn schema_record() -> Vec<u8> {
        let mut record = vec![0_u8; MAX_SCHEMA_RECORD_BYTES];
        let mut length = 0;
        // SAFETY: test-owned regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_refloat_configuration_codec_decode_schema(
                    SCHEMA.as_ptr(),
                    SCHEMA.len(),
                    record.as_mut_ptr(),
                    record.len(),
                    &mut length,
                )
            },
            STATUS_OK
        );
        record.truncate(length);
        record
    }

    #[test]
    fn all_operations_publish_exact_deterministic_records() {
        assert_eq!(float_view_refloat_configuration_codec_abi_version(), 1);
        let schema = schema_record();
        assert_eq!(&schema[..4], &[1, 1, 0, 0]);
        let mut values = vec![0_u8; MAX_VALUE_RECORD_BYTES];
        let mut values_length = usize::MAX;
        // SAFETY: test-owned regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_refloat_configuration_codec_decode_values(
                    schema.as_ptr(),
                    schema.len(),
                    VALUES.as_ptr(),
                    VALUES.len(),
                    values.as_mut_ptr(),
                    values.len(),
                    &mut values_length,
                )
            },
            STATUS_OK
        );
        assert_eq!(&values[..4], &[1, 2, 0, 0]);
        let mut encoded = vec![0_u8; MAX_VALUE_PAYLOAD_BYTES];
        let mut encoded_length = usize::MAX;
        // SAFETY: test-owned regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_refloat_configuration_codec_encode_values(
                    schema.as_ptr(),
                    schema.len(),
                    values.as_ptr(),
                    values_length,
                    encoded.as_mut_ptr(),
                    encoded.len(),
                    &mut encoded_length,
                )
            },
            STATUS_OK
        );
        assert_eq!(&encoded[..encoded_length], VALUES);
        assert_eq!(schema_record(), schema);
    }

    #[test]
    fn failures_zero_or_publish_only_required_length() {
        let schema = schema_record();
        let mut small = [9_u8; 2];
        let mut required = usize::MAX;
        // SAFETY: test-owned regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_refloat_configuration_codec_decode_values(
                    schema.as_ptr(),
                    schema.len(),
                    VALUES.as_ptr(),
                    VALUES.len(),
                    small.as_mut_ptr(),
                    small.len(),
                    &mut required,
                )
            },
            STATUS_OUTPUT_TOO_SMALL
        );
        assert!(required > small.len());
        assert_eq!(small, [9, 9]);

        let mut corrupt = schema.clone();
        corrupt[2] = 1;
        required = usize::MAX;
        // SAFETY: test-owned regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_refloat_configuration_codec_decode_values(
                    corrupt.as_ptr(),
                    corrupt.len(),
                    VALUES.as_ptr(),
                    VALUES.len(),
                    ptr::null_mut(),
                    0,
                    &mut required,
                )
            },
            STATUS_INVALID_SCHEMA_RECORD
        );
        assert_eq!(required, 0);
    }

    #[test]
    fn invalid_and_overlapping_regions_fail_before_dereference() {
        let mut length = usize::MAX;
        // SAFETY: deliberately invalid regions are rejected before dereference.
        assert_eq!(
            unsafe {
                float_view_refloat_configuration_codec_decode_schema(
                    ptr::null(),
                    1,
                    ptr::null_mut(),
                    0,
                    &mut length,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        let mut overlap = [0_u8; 64];
        // SAFETY: overlapping regions are rejected before dereference.
        assert_eq!(
            unsafe {
                float_view_refloat_configuration_codec_decode_schema(
                    SCHEMA.as_ptr(),
                    SCHEMA.len(),
                    overlap.as_mut_ptr(),
                    overlap.len(),
                    overlap.as_mut_ptr().cast::<usize>(),
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: oversize is rejected before the sentinel pointer is read.
        assert_eq!(
            unsafe {
                float_view_refloat_configuration_codec_decode_schema(
                    ptr::dangling::<u8>(),
                    MAX_COMPRESSED_BYTES + 1,
                    ptr::null_mut(),
                    0,
                    &mut length,
                )
            },
            STATUS_COMPRESSED_TOO_LARGE
        );
        // SAFETY: the deliberately misaligned scalar is rejected before use.
        assert_eq!(
            unsafe {
                float_view_refloat_configuration_codec_decode_schema(
                    SCHEMA.as_ptr(),
                    SCHEMA.len(),
                    ptr::null_mut(),
                    0,
                    overlap.as_mut_ptr().wrapping_add(1).cast::<usize>(),
                )
            },
            STATUS_INVALID_ARGUMENT
        );
    }

    #[test]
    fn every_codec_error_has_one_fixed_status_and_panics_are_contained() {
        let mappings = [
            (
                ConfigurationError::TruncatedEnvelope,
                STATUS_TRUNCATED_ENVELOPE,
            ),
            (
                ConfigurationError::CompressedTooLarge,
                STATUS_COMPRESSED_TOO_LARGE,
            ),
            (
                ConfigurationError::ExpandedTooLarge,
                STATUS_EXPANDED_TOO_LARGE,
            ),
            (
                ConfigurationError::ExpandedLengthMismatch,
                STATUS_EXPANDED_LENGTH_MISMATCH,
            ),
            (
                ConfigurationError::DecompressionFailed,
                STATUS_DECOMPRESSION_FAILED,
            ),
            (ConfigurationError::ProhibitedXml, STATUS_PROHIBITED_XML),
            (ConfigurationError::MalformedXml, STATUS_MALFORMED_XML),
            (ConfigurationError::StructuralLimit, STATUS_STRUCTURAL_LIMIT),
            (ConfigurationError::DuplicateField, STATUS_DUPLICATE_FIELD),
            (ConfigurationError::DuplicateOrder, STATUS_DUPLICATE_ORDER),
            (
                ConfigurationError::MissingReference,
                STATUS_MISSING_REFERENCE,
            ),
            (ConfigurationError::UnsupportedType, STATUS_UNSUPPORTED_TYPE),
            (
                ConfigurationError::UnsupportedEncoding,
                STATUS_UNSUPPORTED_ENCODING,
            ),
            (ConfigurationError::InvalidMetadata, STATUS_INVALID_METADATA),
            (ConfigurationError::InvalidNumber, STATUS_INVALID_NUMBER),
            (
                ConfigurationError::SignatureMismatch,
                STATUS_SIGNATURE_MISMATCH,
            ),
            (ConfigurationError::TruncatedValues, STATUS_TRUNCATED_VALUES),
            (ConfigurationError::Remainder, STATUS_REMAINDER),
            (ConfigurationError::MissingValue, STATUS_MISSING_VALUE),
            (ConfigurationError::ExtraValue, STATUS_EXTRA_VALUE),
            (ConfigurationError::InvalidValue, STATUS_INVALID_VALUE),
        ];
        for (error, status) in mappings {
            assert_eq!(map_codec_error(error), status);
        }
        assert_eq!(
            contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || -> Result<(), u32> {
                panic!("contained bridge panic")
            }),
            STATUS_INTERNAL_FAILURE
        );
    }
}
