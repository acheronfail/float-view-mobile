//! Narrow Android JNI ABI for the bounded VESC frame codec.
//!
//! Framing behavior remains in the safe `float-view-vesc-frame` crate. This
//! crate owns only bounded JNI representation validation and call-local copies.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_vesc_frame::{FrameError, MAX_PAYLOAD_BYTES, decode, encode};
use jni_sys::{JNIEnv, jbyte, jbyteArray, jint, jintArray, jobject, jsize};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Version of the complete hand-written Android JNI contract.
pub const ABI_VERSION: jint = 1;

/// Successful operation.
pub const STATUS_OK: jint = 0;
/// A JNI environment, reference, array shape, capacity, or alias was invalid.
pub const STATUS_INVALID_ARGUMENT: jint = 1;
/// The input exceeds the product payload or exact-frame bound.
pub const STATUS_INPUT_TOO_LARGE: jint = 2;
/// The exact frame has an invalid header, length, footer, or trailing bytes.
pub const STATUS_INVALID_FRAME: jint = 3;
/// The exact frame payload does not match its CRC-16.
pub const STATUS_INVALID_CRC: jint = 4;
/// The output buffer is too small; the required length was returned.
pub const STATUS_OUTPUT_TOO_SMALL: jint = 5;
/// A panic or unexpected JNI failure was contained at the boundary.
pub const STATUS_INTERNAL_FAILURE: jint = 6;

/// Largest encoded frame: long header, maximum payload, CRC, and footer.
pub const MAX_FRAME_BYTES: usize = MAX_PAYLOAD_BYTES + 6;

#[derive(Clone, Copy)]
enum Operation {
    Encode,
    DecodeExact,
}

struct OperationResult {
    status: jint,
    output: Vec<u8>,
    output_length: usize,
}

impl OperationResult {
    fn failure(status: jint) -> Self {
        Self {
            status,
            output: Vec::new(),
            output_length: 0,
        }
    }
}

/// Return the exact version of the hand-written JNI contract.
///
/// # Safety
///
/// `env` must be either null or a JNI environment pointer supplied by the JVM
/// for this call. The function reads no other caller-owned memory.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescFrameBridge_abiVersion(
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

/// Encode one bounded payload into a caller-owned Java byte array.
///
/// # Safety
///
/// Every non-null JNI reference must be valid for the supplied `env` and the
/// duration of this call. The JVM must invoke this through the declared Kotlin
/// native method so each reference has the documented primitive-array type.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescFrameBridge_encode(
    env: *mut JNIEnv,
    _receiver: jobject,
    input: jbyteArray,
    output: jbyteArray,
    output_length: jintArray,
) -> jint {
    contain_status(|| unsafe { invoke(env, input, output, output_length, Operation::Encode) })
}

/// Decode exactly one bounded frame into a caller-owned Java byte array.
///
/// # Safety
///
/// Every non-null JNI reference must be valid for the supplied `env` and the
/// duration of this call. The JVM must invoke this through the declared Kotlin
/// native method so each reference has the documented primitive-array type.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescFrameBridge_decodeExact(
    env: *mut JNIEnv,
    _receiver: jobject,
    input: jbyteArray,
    output: jbyteArray,
    output_length: jintArray,
) -> jint {
    contain_status(|| unsafe { invoke(env, input, output, output_length, Operation::DecodeExact) })
}

unsafe fn invoke(
    env: *mut JNIEnv,
    input: jbyteArray,
    output: jbyteArray,
    output_length: jintArray,
    operation: Operation,
) -> jint {
    let Some(functions) = (unsafe { environment(env) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if unsafe { (functions.ExceptionCheck)(env) }
        || input.is_null()
        || output.is_null()
        || output_length.is_null()
    {
        return STATUS_INVALID_ARGUMENT;
    }

    let input_length = unsafe { (functions.GetArrayLength)(env, input.cast()) };
    if unsafe { (functions.ExceptionCheck)(env) } || input_length < 0 {
        return STATUS_INVALID_ARGUMENT;
    }
    let output_capacity = unsafe { (functions.GetArrayLength)(env, output.cast()) };
    if unsafe { (functions.ExceptionCheck)(env) } || output_capacity < 0 {
        return STATUS_INVALID_ARGUMENT;
    }
    let output_length_count = unsafe { (functions.GetArrayLength)(env, output_length.cast()) };
    if unsafe { (functions.ExceptionCheck)(env) } || output_length_count != 1 {
        return STATUS_INVALID_ARGUMENT;
    }

    if unsafe { references_alias(env, functions, input.cast(), output.cast()) }
        || unsafe { references_alias(env, functions, input.cast(), output_length.cast()) }
        || unsafe { references_alias(env, functions, output.cast(), output_length.cast()) }
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if unsafe { (functions.ExceptionCheck)(env) } {
        return STATUS_INVALID_ARGUMENT;
    }

    let input_length = input_length as usize;
    let output_capacity = output_capacity as usize;
    let (maximum_input, maximum_output) = match operation {
        Operation::Encode => (MAX_PAYLOAD_BYTES, MAX_FRAME_BYTES),
        Operation::DecodeExact => (MAX_FRAME_BYTES, MAX_PAYLOAD_BYTES),
    };
    if output_capacity > maximum_output {
        return STATUS_INVALID_ARGUMENT;
    }
    if write_length(env, functions, output_length, 0).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if input_length > maximum_input {
        return STATUS_INPUT_TOO_LARGE;
    }

    let mut copied_input = vec![0_u8; input_length];
    unsafe {
        (functions.GetByteArrayRegion)(
            env,
            input,
            0,
            input_length as jsize,
            copied_input.as_mut_ptr().cast::<jbyte>(),
        )
    };
    if unsafe { (functions.ExceptionCheck)(env) } {
        return STATUS_INTERNAL_FAILURE;
    }

    let result = run_operation(operation, &copied_input, output_capacity);
    if result.status == STATUS_OK {
        unsafe {
            (functions.SetByteArrayRegion)(
                env,
                output,
                0,
                result.output_length as jsize,
                result.output.as_ptr().cast::<jbyte>(),
            )
        };
        if unsafe { (functions.ExceptionCheck)(env) } {
            return STATUS_INTERNAL_FAILURE;
        }
    }
    if write_length(env, functions, output_length, result.output_length as jint).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    result.status
}

fn run_operation(operation: Operation, input: &[u8], output_capacity: usize) -> OperationResult {
    contain(|| {
        let result = match operation {
            Operation::Encode => encode(input),
            Operation::DecodeExact => decode(input),
        }
        .map_err(frame_status)?;
        let output_length = result.len();
        if output_length > output_capacity {
            return Ok(OperationResult {
                status: STATUS_OUTPUT_TOO_SMALL,
                output: Vec::new(),
                output_length,
            });
        }
        Ok(OperationResult {
            status: STATUS_OK,
            output: result,
            output_length,
        })
    })
}

fn contain(operation: impl FnOnce() -> Result<OperationResult, jint>) -> OperationResult {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(result)) => result,
        Ok(Err(status)) => OperationResult::failure(status),
        Err(_) => OperationResult::failure(STATUS_INTERNAL_FAILURE),
    }
}

fn contain_status(operation: impl FnOnce() -> jint) -> jint {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(STATUS_INTERNAL_FAILURE)
}

fn contain_abi(operation: impl FnOnce() -> jint) -> jint {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(0)
}

fn frame_status(error: FrameError) -> jint {
    match error {
        FrameError::InvalidFrame => STATUS_INVALID_FRAME,
        FrameError::InvalidCrc => STATUS_INVALID_CRC,
        FrameError::PayloadTooLarge => STATUS_INPUT_TOO_LARGE,
    }
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

unsafe fn references_alias(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    left: jobject,
    right: jobject,
) -> bool {
    unsafe { (functions.IsSameObject)(env, left, right) }
}

fn write_length(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    output_length: jintArray,
    value: jint,
) -> Result<(), ()> {
    unsafe { (functions.SetIntArrayRegion)(env, output_length, 0, 1, &value) };
    if unsafe { (functions.ExceptionCheck)(env) } {
        Err(())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_and_exact_decode_share_the_safe_codec() {
        let payload = [0x11, 0x22, 0x33];
        let encoded = run_operation(Operation::Encode, &payload, MAX_FRAME_BYTES);
        assert_eq!(encoded.status, STATUS_OK);
        assert_eq!(encoded.output_length, 8);

        let decoded = run_operation(Operation::DecodeExact, &encoded.output, MAX_PAYLOAD_BYTES);
        assert_eq!(decoded.status, STATUS_OK);
        assert_eq!(decoded.output, payload);
    }

    #[test]
    fn maps_semantic_failures_and_reports_required_capacity() {
        let too_small = run_operation(Operation::Encode, &[0x01], 0);
        assert_eq!(too_small.status, STATUS_OUTPUT_TOO_SMALL);
        assert_eq!(too_small.output_length, 6);
        assert!(too_small.output.is_empty());

        let invalid = run_operation(Operation::DecodeExact, &[0x02], MAX_PAYLOAD_BYTES);
        assert_eq!(invalid.status, STATUS_INVALID_FRAME);
        assert_eq!(invalid.output_length, 0);

        let mut invalid_crc = encode(&[0x01]).expect("fixture frame");
        invalid_crc[3] ^= 0xff;
        let invalid_crc = run_operation(Operation::DecodeExact, &invalid_crc, MAX_PAYLOAD_BYTES);
        assert_eq!(invalid_crc.status, STATUS_INVALID_CRC);
    }

    #[test]
    fn contains_panics() {
        let result = contain(|| -> Result<OperationResult, jint> { panic!("contained") });
        assert_eq!(result.status, STATUS_INTERNAL_FAILURE);
        assert_eq!(result.output_length, 0);
        assert!(result.output.is_empty());
        assert_eq!(
            contain_status(|| panic!("contained status")),
            STATUS_INTERNAL_FAILURE
        );
        assert_eq!(contain_abi(|| panic!("contained ABI")), 0);
    }

    #[test]
    fn abi_and_status_numbers_are_stable() {
        assert_eq!(ABI_VERSION, 1);
        assert_eq!(STATUS_OK, 0);
        assert_eq!(STATUS_INVALID_ARGUMENT, 1);
        assert_eq!(STATUS_INPUT_TOO_LARGE, 2);
        assert_eq!(STATUS_INVALID_FRAME, 3);
        assert_eq!(STATUS_INVALID_CRC, 4);
        assert_eq!(STATUS_OUTPUT_TOO_SMALL, 5);
        assert_eq!(STATUS_INTERNAL_FAILURE, 6);
    }
}
