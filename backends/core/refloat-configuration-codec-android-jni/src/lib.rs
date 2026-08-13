//! Temporary four-operation Android JNI boundary for the safe Refloat configuration codec.
//!
//! The safe crate owns every protocol and canonical-record decision. This crate owns only
//! call-local JNI validation, bounded primitive-array copies, fixed status mapping, atomic
//! publication, and panic containment. It retains no JVM reference or codec state.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_android_jni_support::{
    Environment, JNIEnv, contain, jbyteArray, jint, jintArray, jobject, with_environment,
};
use float_view_refloat_configuration_codec::{
    ConfigurationError, RecordError, decode_qt_compressed_to_record, decode_values_to_record,
    encode_values_from_records,
};
pub use float_view_refloat_configuration_codec::{
    MAX_COMPRESSED_BYTES, MAX_SCHEMA_RECORD_BYTES, MAX_VALUE_PAYLOAD_BYTES, MAX_VALUE_RECORD_BYTES,
};

pub const ABI_VERSION: jint = 1;
pub const STATUS_OK: jint = 0;
pub const STATUS_INVALID_ARGUMENT: jint = 1;
pub const STATUS_INVALID_SCHEMA_RECORD: jint = 2;
pub const STATUS_INVALID_VALUE_RECORD: jint = 3;
pub const STATUS_OUTPUT_TOO_SMALL: jint = 4;
pub const STATUS_INTERNAL_FAILURE: jint = 5;
pub const STATUS_TRUNCATED_ENVELOPE: jint = 10;
pub const STATUS_COMPRESSED_TOO_LARGE: jint = 11;
pub const STATUS_EXPANDED_TOO_LARGE: jint = 12;
pub const STATUS_EXPANDED_LENGTH_MISMATCH: jint = 13;
pub const STATUS_DECOMPRESSION_FAILED: jint = 14;
pub const STATUS_PROHIBITED_XML: jint = 15;
pub const STATUS_MALFORMED_XML: jint = 16;
pub const STATUS_STRUCTURAL_LIMIT: jint = 17;
pub const STATUS_DUPLICATE_FIELD: jint = 18;
pub const STATUS_DUPLICATE_ORDER: jint = 19;
pub const STATUS_MISSING_REFERENCE: jint = 20;
pub const STATUS_UNSUPPORTED_TYPE: jint = 21;
pub const STATUS_UNSUPPORTED_ENCODING: jint = 22;
pub const STATUS_INVALID_METADATA: jint = 23;
pub const STATUS_INVALID_NUMBER: jint = 24;
pub const STATUS_SIGNATURE_MISMATCH: jint = 25;
pub const STATUS_TRUNCATED_VALUES: jint = 26;
pub const STATUS_REMAINDER: jint = 27;
pub const STATUS_MISSING_VALUE: jint = 28;
pub const STATUS_EXTRA_VALUE: jint = 29;
pub const STATUS_INVALID_VALUE: jint = 30;

/// Return the exact version of the hand-written JNI contract.
///
/// # Safety
/// `env` must be null or the JNI environment supplied for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_abiVersion(
    env: *mut JNIEnv,
    _receiver: jobject,
) -> jint {
    contain(0, || unsafe {
        with_environment(env, |environment| {
            if environment.ensure_no_exception().is_ok() {
                ABI_VERSION
            } else {
                0
            }
        })
        .unwrap_or(0)
    })
}

/// Decode one complete Qt-compressed schema into the canonical schema record.
///
/// # Safety
/// Every non-null reference must belong to `env`, have the primitive-array type declared by
/// the Kotlin method, and remain valid only for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_decodeSchema(
    env: *mut JNIEnv,
    _receiver: jobject,
    compressed: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| {
            invoke_schema(environment, compressed, output, result)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Decode one signature-prefixed value payload into the canonical value record.
///
/// # Safety
/// Every non-null reference must satisfy the same call-local contract as `decodeSchema`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_decodeValues(
    env: *mut JNIEnv,
    _receiver: jobject,
    schema_record: jbyteArray,
    payload: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| {
            invoke_values_decode(environment, schema_record, payload, output, result)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Encode one canonical value record using one exact canonical schema record.
///
/// # Safety
/// Every non-null reference must satisfy the same call-local contract as `decodeSchema`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_encodeValues(
    env: *mut JNIEnv,
    _receiver: jobject,
    schema_record: jbyteArray,
    value_record: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| {
            invoke_values_encode(environment, schema_record, value_record, output, result)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

fn invoke_schema(
    environment: &Environment<'_>,
    compressed: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    let Some((lengths, output_capacity)) = validate_call(
        environment,
        &[compressed, output],
        result,
        MAX_SCHEMA_RECORD_BYTES,
    ) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let compressed_length = lengths[0];
    if initialize_result(environment, result).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if compressed_length > MAX_COMPRESSED_BYTES {
        return STATUS_COMPRESSED_TOO_LARGE;
    }
    let Ok(compressed) =
        environment.read_bytes(compressed, compressed_length, MAX_COMPRESSED_BYTES)
    else {
        return STATUS_INTERNAL_FAILURE;
    };
    publish(
        environment,
        output,
        output_capacity,
        result,
        run_schema(&compressed, output_capacity),
    )
}

fn invoke_values_decode(
    environment: &Environment<'_>,
    schema_record: jbyteArray,
    payload: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    let Some((lengths, output_capacity)) = validate_call(
        environment,
        &[schema_record, payload, output],
        result,
        MAX_VALUE_RECORD_BYTES,
    ) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if initialize_result(environment, result).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if lengths[0] > MAX_SCHEMA_RECORD_BYTES {
        return STATUS_INVALID_SCHEMA_RECORD;
    }
    if lengths[1] > MAX_VALUE_PAYLOAD_BYTES {
        return STATUS_INVALID_VALUE;
    }
    let Ok(schema_record) =
        environment.read_bytes(schema_record, lengths[0], MAX_SCHEMA_RECORD_BYTES)
    else {
        return STATUS_INTERNAL_FAILURE;
    };
    let Ok(payload) = environment.read_bytes(payload, lengths[1], MAX_VALUE_PAYLOAD_BYTES) else {
        return STATUS_INTERNAL_FAILURE;
    };
    publish(
        environment,
        output,
        output_capacity,
        result,
        run_values_decode(&schema_record, &payload, output_capacity),
    )
}

fn invoke_values_encode(
    environment: &Environment<'_>,
    schema_record: jbyteArray,
    value_record: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    let Some((lengths, output_capacity)) = validate_call(
        environment,
        &[schema_record, value_record, output],
        result,
        MAX_VALUE_PAYLOAD_BYTES,
    ) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if initialize_result(environment, result).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if lengths[0] > MAX_SCHEMA_RECORD_BYTES {
        return STATUS_INVALID_SCHEMA_RECORD;
    }
    if lengths[1] > MAX_VALUE_RECORD_BYTES {
        return STATUS_INVALID_VALUE_RECORD;
    }
    let Ok(schema_record) =
        environment.read_bytes(schema_record, lengths[0], MAX_SCHEMA_RECORD_BYTES)
    else {
        return STATUS_INTERNAL_FAILURE;
    };
    let Ok(value_record) = environment.read_bytes(value_record, lengths[1], MAX_VALUE_RECORD_BYTES)
    else {
        return STATUS_INTERNAL_FAILURE;
    };
    publish(
        environment,
        output,
        output_capacity,
        result,
        run_values_encode(&schema_record, &value_record, output_capacity),
    )
}

fn validate_call(
    environment: &Environment<'_>,
    byte_arrays: &[jbyteArray],
    result: jintArray,
    maximum_output: usize,
) -> Option<(Vec<usize>, usize)> {
    let mut references: Vec<jobject> = byte_arrays.iter().map(|array| array.cast()).collect();
    references.push(result.cast());
    if environment.ensure_no_exception().is_err()
        || environment.require_references(&references).is_err()
        || environment.require_array_length(result.cast(), 1).is_err()
        || environment.require_distinct(&references).is_err()
    {
        return None;
    }
    let mut lengths = Vec::with_capacity(byte_arrays.len());
    for array in byte_arrays {
        lengths.push(environment.array_length(array.cast()).ok()?);
    }
    let output_capacity = *lengths.last()?;
    if output_capacity > maximum_output {
        return None;
    }
    Some((lengths, output_capacity))
}

fn initialize_result(environment: &Environment<'_>, result: jintArray) -> Result<(), ()> {
    environment
        .write_ints_exact(result, 1, &[0])
        .map_err(|_| ())
}

fn publish(
    environment: &Environment<'_>,
    output: jbyteArray,
    output_capacity: usize,
    result: jintArray,
    produced: OutputResult,
) -> jint {
    if let Some(bytes) = produced.bytes
        && environment
            .write_bytes(output, output_capacity, &bytes)
            .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    let Ok(required_length) = jint::try_from(produced.required_length) else {
        return STATUS_INTERNAL_FAILURE;
    };
    if environment
        .write_ints_exact(result, 1, &[required_length])
        .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    produced.status
}

#[derive(Debug, Eq, PartialEq)]
struct OutputResult {
    status: jint,
    bytes: Option<Vec<u8>>,
    required_length: usize,
}

impl OutputResult {
    fn from_record(result: Result<Vec<u8>, RecordError>, capacity: usize) -> Self {
        match result {
            Ok(bytes) if bytes.len() > capacity => Self {
                status: STATUS_OUTPUT_TOO_SMALL,
                required_length: bytes.len(),
                bytes: None,
            },
            Ok(bytes) => Self {
                status: STATUS_OK,
                required_length: bytes.len(),
                bytes: Some(bytes),
            },
            Err(error) => Self {
                status: map_record_error(error),
                required_length: 0,
                bytes: None,
            },
        }
    }
}

fn run_schema(compressed: &[u8], output_capacity: usize) -> OutputResult {
    OutputResult::from_record(
        contain(Err(RecordError::InvalidSchemaRecord), || {
            decode_qt_compressed_to_record(compressed)
        }),
        output_capacity,
    )
}

fn run_values_decode(schema_record: &[u8], payload: &[u8], output_capacity: usize) -> OutputResult {
    OutputResult::from_record(
        contain(Err(RecordError::InvalidValueRecord), || {
            decode_values_to_record(schema_record, payload)
        }),
        output_capacity,
    )
}

fn run_values_encode(
    schema_record: &[u8],
    value_record: &[u8],
    output_capacity: usize,
) -> OutputResult {
    OutputResult::from_record(
        contain(Err(RecordError::InvalidValueRecord), || {
            encode_values_from_records(schema_record, value_record)
        }),
        output_capacity,
    )
}

fn map_record_error(error: RecordError) -> jint {
    match error {
        RecordError::InvalidSchemaRecord => STATUS_INVALID_SCHEMA_RECORD,
        RecordError::InvalidValueRecord => STATUS_INVALID_VALUE_RECORD,
        RecordError::Codec(error) => map_codec_error(error),
    }
}

fn map_codec_error(error: ConfigurationError) -> jint {
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
        run_schema(SCHEMA, MAX_SCHEMA_RECORD_BYTES).bytes.unwrap()
    }

    #[test]
    fn every_operation_reuses_exact_safe_records_and_payloads() {
        let schema = schema_record();
        assert_eq!(&schema[..4], &[1, 1, 0, 0]);
        let decoded = run_values_decode(&schema, VALUES, MAX_VALUE_RECORD_BYTES);
        assert_eq!(decoded.status, STATUS_OK);
        let record = decoded.bytes.unwrap();
        assert_eq!(&record[..4], &[1, 2, 0, 0]);
        assert_eq!(
            run_values_encode(&schema, &record, MAX_VALUE_PAYLOAD_BYTES),
            OutputResult {
                status: STATUS_OK,
                bytes: Some(VALUES.to_vec()),
                required_length: VALUES.len(),
            }
        );
        assert_eq!(schema_record(), schema);
    }

    #[test]
    fn two_pass_results_are_exact_atomic_and_deterministic() {
        let first = run_schema(SCHEMA, 0);
        assert_eq!(first.status, STATUS_OUTPUT_TOO_SMALL);
        assert!(first.bytes.is_none());
        assert!(first.required_length > 0);
        let second = run_schema(SCHEMA, first.required_length);
        assert_eq!(second.status, STATUS_OK);
        assert_eq!(second.required_length, first.required_length);
        assert_eq!(second.bytes.as_deref(), Some(schema_record().as_slice()));

        let schema = schema_record();
        let values = run_values_decode(&schema, VALUES, 1);
        assert_eq!(values.status, STATUS_OUTPUT_TOO_SMALL);
        assert!(values.bytes.is_none());
        assert!(values.required_length > 1);
    }

    #[test]
    fn malformed_records_and_payloads_map_to_fixed_failures() {
        let schema = schema_record();
        let mut invalid_schema = schema.clone();
        invalid_schema[2] = 1;
        assert_eq!(
            run_values_decode(&invalid_schema, VALUES, MAX_VALUE_RECORD_BYTES).status,
            STATUS_INVALID_SCHEMA_RECORD
        );
        assert_eq!(
            run_values_encode(&schema, &[], MAX_VALUE_PAYLOAD_BYTES).status,
            STATUS_INVALID_VALUE_RECORD
        );
        assert_eq!(
            run_values_decode(&schema, &[], MAX_VALUE_RECORD_BYTES).status,
            STATUS_TRUNCATED_VALUES
        );
        assert_eq!(
            run_schema(&[], MAX_SCHEMA_RECORD_BYTES).status,
            STATUS_TRUNCATED_ENVELOPE
        );
    }

    #[test]
    fn every_codec_error_has_one_payload_free_status() {
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
        for (error, expected) in mappings {
            assert_eq!(map_codec_error(error), expected);
        }
    }

    #[test]
    fn null_environments_and_panics_are_contained() {
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_abiVersion(
                ptr::null_mut(), ptr::null_mut(),
            )
            },
            0
        );
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_protocol_refloat_RustRefloatConfigurationCodecBridge_decodeSchema(
                ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut(),
            )
            },
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            contain(STATUS_INTERNAL_FAILURE, || panic!("contained JNI panic")),
            STATUS_INTERNAL_FAILURE
        );
    }
}
