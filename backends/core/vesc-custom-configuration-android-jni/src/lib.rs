//! Narrow Android JNI ABI for bounded VESC custom-configuration envelopes.
//!
//! Envelope decisions remain in the safe `float-view-vesc-custom-configuration`
//! crate. This crate owns only bounded JNI validation and call-local copies.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_vesc_custom_configuration::{
    CustomConfigurationError, CustomConfigurationResponse, MAX_SCHEDULED_PAYLOAD_BYTES,
    ValuesCommand, decode_response, schema_request, values_request, write_request,
};
use jni_sys::{JNIEnv, jbyte, jbyteArray, jint, jintArray, jlong, jobject, jsize};
use std::panic::{AssertUnwindSafe, catch_unwind};

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
    contain_abi(|| {
        let Some(functions) = (unsafe { environment(env) }) else {
            return 0;
        };
        if unsafe { (functions.ExceptionCheck)(env) } {
            return 0;
        }
        ABI_VERSION
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
    contain_status(|| unsafe {
        invoke_encode(
            env, operation, index, length, offset, values, output, result,
        )
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
    contain_status(|| unsafe { invoke_decode(env, input, result) })
}

#[allow(clippy::too_many_arguments)]
unsafe fn invoke_encode(
    env: *mut JNIEnv,
    operation: jint,
    index: jint,
    length: jlong,
    offset: jlong,
    values: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    let Some(functions) = (unsafe { environment(env) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if unsafe { (functions.ExceptionCheck)(env) }
        || values.is_null()
        || output.is_null()
        || result.is_null()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    let Some(values_length) = (unsafe { array_length(env, functions, values.cast()) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Some(output_capacity) = (unsafe { array_length(env, functions, output.cast()) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Some(result_length) = (unsafe { array_length(env, functions, result.cast()) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if result_length != ENCODE_RESULT_INTS as usize
        || values_length > MAX_SCHEDULED_PAYLOAD_BYTES
        || output_capacity > MAX_SCHEDULED_PAYLOAD_BYTES
        || unsafe {
            any_alias(
                env,
                functions,
                &[values.cast(), output.cast(), result.cast()],
            )
        }
        || unsafe { (functions.ExceptionCheck)(env) }
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if write_ints(env, functions, result, &[0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    let Ok(index) = u8::try_from(index) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let mut copied_values = vec![0_u8; values_length];
    if copy_bytes_from_java(env, functions, values, &mut copied_values).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    let encoded = run_encode(
        operation,
        index,
        length,
        offset,
        &copied_values,
        output_capacity,
    );
    if encoded.status == STATUS_OK
        && copy_bytes_to_java(env, functions, output, &encoded.output).is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    let Ok(required) = jint::try_from(encoded.output_length) else {
        return STATUS_INTERNAL_FAILURE;
    };
    if write_ints(env, functions, result, &[required]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    encoded.status
}

unsafe fn invoke_decode(env: *mut JNIEnv, input: jbyteArray, result: jintArray) -> jint {
    let Some(functions) = (unsafe { environment(env) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if unsafe { (functions.ExceptionCheck)(env) } || input.is_null() || result.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let Some(input_length) = (unsafe { array_length(env, functions, input.cast()) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Some(result_length) = (unsafe { array_length(env, functions, result.cast()) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if result_length != DECODE_RESULT_INTS as usize
        || unsafe { any_alias(env, functions, &[input.cast(), result.cast()]) }
        || unsafe { (functions.ExceptionCheck)(env) }
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if write_ints(env, functions, result, &[0; DECODE_RESULT_INTS as usize]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if input_length > MAX_SCHEDULED_PAYLOAD_BYTES {
        return STATUS_INVALID_PAYLOAD;
    }
    let mut copied_input = vec![0_u8; input_length];
    if copy_bytes_from_java(env, functions, input, &mut copied_input).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    let decoded = run_decode(&copied_input);
    if write_ints(env, functions, result, &decoded.fields).is_err() {
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
    contain_encode(|| {
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
    })
}

fn run_decode(input: &[u8]) -> DecodeResult {
    contain_decode(|| {
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
    })
}

fn map_codec_error(error: CustomConfigurationError) -> jint {
    match error {
        CustomConfigurationError::InvalidPayload => STATUS_INVALID_PAYLOAD,
        CustomConfigurationError::UnsupportedCommand(_) => STATUS_INTERNAL_FAILURE,
    }
}

fn contain_encode(operation: impl FnOnce() -> Result<EncodeResult, jint>) -> EncodeResult {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(result)) => result,
        Ok(Err(status)) => EncodeResult::failure(status),
        Err(_) => EncodeResult::failure(STATUS_INTERNAL_FAILURE),
    }
}

fn contain_decode(operation: impl FnOnce() -> Result<DecodeResult, jint>) -> DecodeResult {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(result)) => result,
        Ok(Err(status)) => DecodeResult::failure(status),
        Err(_) => DecodeResult::failure(STATUS_INTERNAL_FAILURE),
    }
}

fn contain_status(operation: impl FnOnce() -> jint) -> jint {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(STATUS_INTERNAL_FAILURE)
}

fn contain_abi(operation: impl FnOnce() -> jint) -> jint {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(0)
}

unsafe fn environment<'a>(env: *mut JNIEnv) -> Option<&'a jni_sys::JNINativeInterface__1_2> {
    if env.is_null() {
        return None;
    }
    let functions = unsafe { *env };
    if functions.is_null() {
        return None;
    }
    Some(unsafe { &(*functions).v1_2 })
}

unsafe fn array_length(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    array: jni_sys::jarray,
) -> Option<usize> {
    let length = unsafe { (functions.GetArrayLength)(env, array) };
    if unsafe { (functions.ExceptionCheck)(env) } || length < 0 {
        None
    } else {
        Some(length as usize)
    }
}

unsafe fn any_alias(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    references: &[jobject],
) -> bool {
    for left in 0..references.len() {
        for right in left + 1..references.len() {
            if unsafe { (functions.IsSameObject)(env, references[left], references[right]) } {
                return true;
            }
        }
    }
    false
}

fn copy_bytes_from_java(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    input: jbyteArray,
    output: &mut [u8],
) -> Result<(), ()> {
    if !output.is_empty() {
        unsafe {
            (functions.GetByteArrayRegion)(
                env,
                input,
                0,
                output.len() as jsize,
                output.as_mut_ptr().cast::<jbyte>(),
            )
        };
    }
    if unsafe { (functions.ExceptionCheck)(env) } {
        Err(())
    } else {
        Ok(())
    }
}

fn copy_bytes_to_java(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    output: jbyteArray,
    input: &[u8],
) -> Result<(), ()> {
    if !input.is_empty() {
        unsafe {
            (functions.SetByteArrayRegion)(
                env,
                output,
                0,
                input.len() as jsize,
                input.as_ptr().cast::<jbyte>(),
            )
        };
    }
    if unsafe { (functions.ExceptionCheck)(env) } {
        Err(())
    } else {
        Ok(())
    }
}

fn write_ints(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    output: jintArray,
    values: &[jint],
) -> Result<(), ()> {
    unsafe {
        (functions.SetIntArrayRegion)(env, output, 0, values.len() as jsize, values.as_ptr())
    };
    if unsafe { (functions.ExceptionCheck)(env) } {
        Err(())
    } else {
        Ok(())
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
        assert_eq!(
            contain_encode(|| -> Result<EncodeResult, jint> { panic!("contained") }),
            EncodeResult::failure(STATUS_INTERNAL_FAILURE)
        );
        assert_eq!(
            contain_decode(|| -> Result<DecodeResult, jint> { panic!("contained") }),
            DecodeResult::failure(STATUS_INTERNAL_FAILURE)
        );
        assert_eq!(
            contain_status(|| panic!("contained")),
            STATUS_INTERNAL_FAILURE
        );
        assert_eq!(contain_abi(|| panic!("contained")), 0);
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
