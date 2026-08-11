//! Narrow Android JNI ABI for bounded VESC firmware identity decoding.
//!
//! Structural decoding remains in the safe `float-view-vesc-firmware-identity`
//! crate. This crate owns only bounded JNI validation and call-local copies.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_android_jni_support::{
    Environment, JNIEnv, contain, jbyteArray, jint, jintArray, jobject, with_environment,
};
use float_view_vesc_firmware_identity::{
    FirmwareIdentityRecordOutcome, RECORD_VERSION, decode_record,
};
pub use float_view_vesc_firmware_identity::{
    MAX_ENCODED_NAME_BYTES, MAX_PAYLOAD_BYTES, MAX_RECORD_BYTES, RECORD_HEADER_BYTES,
};

/// Version of the complete hand-written Android identity JNI contract.
pub const ABI_VERSION: jint = RECORD_VERSION as jint;

/// Successful operation.
pub const STATUS_OK: jint = 0;
/// A JNI environment, reference, array shape, capacity, or alias was invalid.
pub const STATUS_INVALID_ARGUMENT: jint = 1;
/// The observed VESC command is not `COMM_FW_VERSION`.
pub const STATUS_UNSUPPORTED_COMMAND: jint = 2;
/// The payload is truncated, oversized, or structurally invalid.
pub const STATUS_TRUNCATED_PAYLOAD: jint = 3;
/// The output is too small; the required record length was returned.
pub const STATUS_OUTPUT_TOO_SMALL: jint = 4;
/// A panic or unexpected JNI failure was contained at the boundary.
pub const STATUS_INTERNAL_FAILURE: jint = 5;

/// Return the exact version of the hand-written identity JNI contract.
///
/// # Safety
///
/// `env` must be null or a JNI environment supplied by the JVM for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescFirmwareIdentityBridge_abiVersion(
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

/// Decode one bounded identity payload into caller-owned primitive arrays.
///
/// `result[0]` receives the produced or required record length and `result[1]`
/// receives the unsupported command byte. Both are initialized to zero after
/// the complete caller contract has been validated.
///
/// # Safety
///
/// Every non-null JNI reference must be valid for `env` and this call. The JVM
/// must invoke this through the declared Kotlin native method so each reference
/// has the documented primitive-array type.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescFirmwareIdentityBridge_decode(
    env: *mut JNIEnv,
    _receiver: jobject,
    input: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| {
            invoke(environment, input, output, result)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

fn invoke(
    environment: &Environment<'_>,
    input: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    if environment.ensure_no_exception().is_err()
        || environment
            .require_references(&[input.cast(), output.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }

    let Ok(input_length) = environment.array_length(input.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Ok(output_capacity) = environment.array_length(output.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if environment.require_array_length(result.cast(), 2).is_err()
        || environment
            .require_distinct(&[input.cast(), output.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }

    if output_capacity > MAX_RECORD_BYTES {
        return STATUS_INVALID_ARGUMENT;
    }
    if environment.write_ints_exact(result, 2, &[0, 0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if input_length > MAX_PAYLOAD_BYTES {
        return STATUS_TRUNCATED_PAYLOAD;
    }

    let Ok(copied_input) = environment.read_bytes(input, input_length, MAX_PAYLOAD_BYTES) else {
        return STATUS_INTERNAL_FAILURE;
    };

    let (status, record, metadata) = match decode_record(&copied_input) {
        FirmwareIdentityRecordOutcome::Record(record) if record.len() > output_capacity => {
            (STATUS_OUTPUT_TOO_SMALL, None, [record.len() as jint, 0])
        }
        FirmwareIdentityRecordOutcome::Record(record) => {
            let length = record.len() as jint;
            (STATUS_OK, Some(record), [length, 0])
        }
        FirmwareIdentityRecordOutcome::UnsupportedCommand(command) => {
            (STATUS_UNSUPPORTED_COMMAND, None, [0, jint::from(command)])
        }
        FirmwareIdentityRecordOutcome::StructuralFailure => {
            (STATUS_TRUNCATED_PAYLOAD, None, [0, 0])
        }
        FirmwareIdentityRecordOutcome::InternalFailure => (STATUS_INTERNAL_FAILURE, None, [0, 0]),
    };
    if let Some(record) = record
        && environment
            .write_bytes(output, output_capacity, record.as_bytes())
            .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    if environment.write_ints_exact(result, 2, &metadata).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_panics_and_keeps_numbers_stable() {
        assert_eq!(contain(5, || panic!("contained")), 5);

        assert_eq!(ABI_VERSION, 1);
        assert_eq!(STATUS_OK, 0);
        assert_eq!(STATUS_INVALID_ARGUMENT, 1);
        assert_eq!(STATUS_UNSUPPORTED_COMMAND, 2);
        assert_eq!(STATUS_TRUNCATED_PAYLOAD, 3);
        assert_eq!(STATUS_OUTPUT_TOO_SMALL, 4);
        assert_eq!(STATUS_INTERNAL_FAILURE, 5);
        assert_eq!(MAX_RECORD_BYTES, 410);
    }
}
