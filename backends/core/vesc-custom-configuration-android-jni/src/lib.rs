//! Narrow Android JNI ABI for bounded VESC custom-configuration envelopes.
//!
//! Envelope decisions remain in the safe `float-view-vesc-custom-configuration`
//! crate. This crate owns only bounded JNI validation and call-local copies.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_android_jni_support::{
    Environment, JNIEnv, contain, jbyteArray, jint, jintArray, jlong, jobject, jsize,
    with_environment,
};
use float_view_vesc_custom_configuration::{
    CustomConfigurationError, CustomConfigurationResponse, MAX_SCHEDULED_PAYLOAD_BYTES,
    ValuesCommand, decode_response, schema_request, values_request, write_request,
};

pub const ABI_VERSION: jint = 1;

pub const STATUS_OK: jint = 0;
pub const STATUS_INVALID_ARGUMENT: jint = 1;
pub const STATUS_INVALID_PAYLOAD: jint = 2;
pub const STATUS_UNSUPPORTED_COMMAND: jint = 3;
pub const STATUS_OUTPUT_TOO_SMALL: jint = 4;
pub const STATUS_INTERNAL_FAILURE: jint = 5;

pub const OPERATION_SCHEMA: jint = 1;
pub const OPERATION_CURRENT: jint = 2;
pub const OPERATION_DEFAULT: jint = 3;
pub const OPERATION_WRITE: jint = 4;

pub const RESPONSE_SCHEMA: jint = 1;
pub const RESPONSE_CURRENT: jint = 2;
pub const RESPONSE_DEFAULT: jint = 3;
pub const RESPONSE_WRITE_ACKNOWLEDGEMENT: jint = 4;

pub const ENCODE_RESULT_INTS: jsize = 1;
pub const DECODE_RESULT_INTS: jsize = 8;

#[derive(Debug, Eq, PartialEq)]
struct EncodeResult {
    status: jint,
    output: Vec<u8>,
    output_length: usize,
}

impl EncodeResult {
    fn failure(status: jint) -> Self {
        Self {
            status,
            output: Vec::new(),
            output_length: 0,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct DecodeResult {
    status: jint,
    fields: [jint; DECODE_RESULT_INTS as usize],
}

impl DecodeResult {
    const fn failure(status: jint) -> Self {
        Self {
            status,
            fields: [0; DECODE_RESULT_INTS as usize],
        }
    }
}

/// Return the exact version of the hand-written custom-configuration JNI ABI.
///
/// # Safety
///
/// `env` must be null or a JNI environment supplied by the JVM for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge_abiVersion(
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

/// Encode one commands 92–95 request into caller-owned primitive arrays.
///
/// `result[0]` receives the produced or required output length.
///
/// # Safety
///
/// Every non-null JNI reference must be valid for `env` and this call. The JVM
/// must invoke this through the declared Kotlin native method.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge_encode(
    env: *mut JNIEnv,
    _receiver: jobject,
    operation: jint,
    index: jint,
    length: jlong,
    offset: jlong,
    values: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| {
            invoke_encode(
                environment,
                operation,
                index,
                length,
                offset,
                values,
                output,
                result,
            )
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Decode one commands 92–95 response into caller-owned primitive metadata.
///
/// The exact eight integers are response kind, command, index, total length,
/// offset, opaque-data offset, opaque-data length, and unsupported command.
///
/// # Safety
///
/// Every non-null JNI reference must be valid for `env` and this call. The JVM
/// must invoke this through the declared Kotlin native method.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge_decode(
    env: *mut JNIEnv,
    _receiver: jobject,
    input: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| invoke_decode(environment, input, result))
            .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

#[allow(clippy::too_many_arguments)]
fn invoke_encode(
    environment: &Environment<'_>,
    operation: jint,
    index: jint,
    length: jlong,
    offset: jlong,
    values: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    if environment.ensure_no_exception().is_err()
        || environment
            .require_references(&[values.cast(), output.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    let Ok(values_length) = environment.array_length(values.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Ok(output_capacity) = environment.array_length(output.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if environment
        .require_array_length(result.cast(), ENCODE_RESULT_INTS as usize)
        .is_err()
        || values_length > MAX_SCHEDULED_PAYLOAD_BYTES
        || output_capacity > MAX_SCHEDULED_PAYLOAD_BYTES
        || environment
            .require_distinct(&[values.cast(), output.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if environment.write_ints_exact(result, 1, &[0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    let Ok(index) = u8::try_from(index) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Ok(copied_values) =
        environment.read_bytes(values, values_length, MAX_SCHEDULED_PAYLOAD_BYTES)
    else {
        return STATUS_INTERNAL_FAILURE;
    };
    let encoded = run_encode(
        operation,
        index,
        length,
        offset,
        &copied_values,
        output_capacity,
    );
    if encoded.status == STATUS_OK
        && environment
            .write_bytes(output, output_capacity, &encoded.output)
            .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    let Ok(required) = jint::try_from(encoded.output_length) else {
        return STATUS_INTERNAL_FAILURE;
    };
    if environment
        .write_ints_exact(result, 1, &[required])
        .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    encoded.status
}

fn invoke_decode(environment: &Environment<'_>, input: jbyteArray, result: jintArray) -> jint {
    if environment.ensure_no_exception().is_err()
        || environment
            .require_references(&[input.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    let Ok(input_length) = environment.array_length(input.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if environment
        .require_array_length(result.cast(), DECODE_RESULT_INTS as usize)
        .is_err()
        || environment
            .require_distinct(&[input.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if environment
        .write_ints_exact(
            result,
            DECODE_RESULT_INTS as usize,
            &[0; DECODE_RESULT_INTS as usize],
        )
        .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    if input_length > MAX_SCHEDULED_PAYLOAD_BYTES {
        return STATUS_INVALID_PAYLOAD;
    }
    let Ok(copied_input) = environment.read_bytes(input, input_length, MAX_SCHEDULED_PAYLOAD_BYTES)
    else {
        return STATUS_INTERNAL_FAILURE;
    };
    let decoded = run_decode(&copied_input);
    if environment
        .write_ints_exact(result, DECODE_RESULT_INTS as usize, &decoded.fields)
        .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    decoded.status
}

fn run_encode(
    operation: jint,
    index: u8,
    length: jlong,
    offset: jlong,
    values: &[u8],
    output_capacity: usize,
) -> EncodeResult {
    let result = contain(Err(STATUS_INTERNAL_FAILURE), || {
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
        let output_length = request.len();
        if output_length > output_capacity {
            return Ok(EncodeResult {
                status: STATUS_OUTPUT_TOO_SMALL,
                output: Vec::new(),
                output_length,
            });
        }
        Ok(EncodeResult {
            status: STATUS_OK,
            output: request,
            output_length,
        })
    });
    result.unwrap_or_else(EncodeResult::failure)
}

fn run_decode(input: &[u8]) -> DecodeResult {
    let result = contain(Err(STATUS_INTERNAL_FAILURE), || {
        let response = match decode_response(input) {
            Ok(response) => response,
            Err(CustomConfigurationError::UnsupportedCommand(command)) => {
                let mut result = DecodeResult::failure(STATUS_UNSUPPORTED_COMMAND);
                result.fields[7] = jint::from(command);
                return Ok(result);
            }
            Err(CustomConfigurationError::InvalidPayload) => {
                return Ok(DecodeResult::failure(STATUS_INVALID_PAYLOAD));
            }
        };
        let (kind, command, index, total, offset, data_offset, data_length) = match response {
            CustomConfigurationResponse::SchemaChunk {
                index,
                total_length,
                offset,
                bytes,
            } => (
                RESPONSE_SCHEMA,
                92,
                jint::from(index),
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
                jint::from(command.wire_value()),
                jint::from(index),
                0,
                0,
                2,
                bytes.len(),
            ),
            CustomConfigurationResponse::WriteAcknowledgement => {
                (RESPONSE_WRITE_ACKNOWLEDGEMENT, 95, 0, 0, 0, 0, 0)
            }
        };
        let fields = [
            kind,
            command,
            index,
            jint::try_from(total).map_err(|_| STATUS_INTERNAL_FAILURE)?,
            jint::try_from(offset).map_err(|_| STATUS_INTERNAL_FAILURE)?,
            jint::try_from(data_offset).map_err(|_| STATUS_INTERNAL_FAILURE)?,
            jint::try_from(data_length).map_err(|_| STATUS_INTERNAL_FAILURE)?,
            0,
        ];
        Ok(DecodeResult {
            status: STATUS_OK,
            fields,
        })
    });
    result.unwrap_or_else(DecodeResult::failure)
}

fn map_codec_error(error: CustomConfigurationError) -> jint {
    match error {
        CustomConfigurationError::InvalidPayload => STATUS_INVALID_PAYLOAD,
        CustomConfigurationError::UnsupportedCommand(_) => STATUS_INTERNAL_FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ReleaseFixtures {
        schema: &'static [u8],
        current: &'static [u8],
        defaults: &'static [u8],
        write: &'static [u8],
        readback: &'static [u8],
    }

    macro_rules! release {
        ($version:literal) => {
            ReleaseFixtures {
                schema: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/schemas/",
                    $version,
                    ".qcompress"
                )),
                current: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/values/",
                    $version,
                    "-current.bin"
                )),
                defaults: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/values/",
                    $version,
                    "-default.bin"
                )),
                write: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/values/",
                    $version,
                    "-write.bin"
                )),
                readback: include_bytes!(concat!(
                    "../../../fixtures/refloat-configuration/values/",
                    $version,
                    "-readback.bin"
                )),
            }
        };
    }

    const RELEASES: &[ReleaseFixtures] = &[
        release!("1.0.0"),
        release!("1.0.1"),
        release!("1.1.0"),
        release!("1.1.1"),
        release!("1.1.2"),
        release!("1.2.0"),
        release!("1.2.1"),
        release!("1.2.2"),
        release!("1.2.3"),
    ];

    #[test]
    fn encodes_every_operation_and_release_write() {
        assert_eq!(
            run_encode(OPERATION_SCHEMA, 7, 400, 3, &[], 10),
            EncodeResult {
                status: STATUS_OK,
                output: vec![92, 7, 0, 0, 1, 144, 0, 0, 0, 3],
                output_length: 10,
            }
        );
        assert_eq!(
            run_encode(OPERATION_CURRENT, 8, 0, 0, &[], 2).output,
            [93, 8]
        );
        assert_eq!(
            run_encode(OPERATION_DEFAULT, 9, 0, 0, &[], 2).output,
            [94, 9]
        );
        for fixtures in RELEASES {
            let encoded = run_encode(
                OPERATION_WRITE,
                0,
                0,
                0,
                fixtures.write,
                fixtures.write.len() + 2,
            );
            assert_eq!(encoded.status, STATUS_OK);
            assert_eq!(&encoded.output[..2], &[95, 0]);
            assert_eq!(&encoded.output[2..], fixtures.write);
        }
    }

    #[test]
    fn decodes_every_release_response_kind_and_opaque_range() {
        for fixtures in RELEASES {
            let chunk_length = fixtures.schema.len().min(400);
            let mut schema = vec![92, 0];
            schema.extend_from_slice(&(fixtures.schema.len() as i32).to_be_bytes());
            schema.extend_from_slice(&0_i32.to_be_bytes());
            schema.extend_from_slice(&fixtures.schema[..chunk_length]);
            assert_eq!(
                run_decode(&schema),
                DecodeResult {
                    status: STATUS_OK,
                    fields: [
                        RESPONSE_SCHEMA,
                        92,
                        0,
                        fixtures.schema.len() as jint,
                        0,
                        10,
                        chunk_length as jint,
                        0,
                    ],
                }
            );
            for (command, kind, bytes) in [
                (93, RESPONSE_CURRENT, fixtures.current),
                (94, RESPONSE_DEFAULT, fixtures.defaults),
                (93, RESPONSE_CURRENT, fixtures.readback),
            ] {
                let mut payload = vec![command, 0];
                payload.extend_from_slice(bytes);
                assert_eq!(
                    run_decode(&payload).fields,
                    [kind, command.into(), 0, 0, 0, 2, bytes.len() as jint, 0]
                );
            }
        }
        assert_eq!(
            run_decode(&[95]),
            DecodeResult {
                status: STATUS_OK,
                fields: [RESPONSE_WRITE_ACKNOWLEDGEMENT, 95, 0, 0, 0, 0, 0, 0],
            }
        );
    }

    #[test]
    fn maps_failures_and_required_capacity_without_partial_output() {
        assert_eq!(
            run_encode(OPERATION_SCHEMA, 0, 0, 0, &[], 10),
            EncodeResult::failure(STATUS_INVALID_PAYLOAD)
        );
        assert_eq!(
            run_encode(OPERATION_CURRENT, 0, 0, 0, &[], 1),
            EncodeResult {
                status: STATUS_OUTPUT_TOO_SMALL,
                output: Vec::new(),
                output_length: 2,
            }
        );
        assert_eq!(
            run_encode(99, 0, 0, 0, &[], 10),
            EncodeResult::failure(STATUS_INVALID_ARGUMENT)
        );
        assert_eq!(
            run_decode(&[0]),
            DecodeResult {
                status: STATUS_UNSUPPORTED_COMMAND,
                fields: [0, 0, 0, 0, 0, 0, 0, 0],
            }
        );
        assert_eq!(
            run_decode(&[91]),
            DecodeResult::failure(STATUS_INVALID_PAYLOAD)
        );
        assert_eq!(
            run_decode(&[95, 0]),
            DecodeResult::failure(STATUS_INVALID_PAYLOAD)
        );
    }

    #[test]
    fn contains_panics_and_keeps_numbers_stable() {
        assert_eq!(contain(5, || panic!("contained")), 5);
        assert_eq!(ABI_VERSION, 1);
        assert_eq!(
            [
                STATUS_OK,
                STATUS_INVALID_ARGUMENT,
                STATUS_INVALID_PAYLOAD,
                STATUS_UNSUPPORTED_COMMAND,
                STATUS_OUTPUT_TOO_SMALL,
                STATUS_INTERNAL_FAILURE,
            ],
            [0, 1, 2, 3, 4, 5]
        );
    }
}
