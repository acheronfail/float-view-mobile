//! Narrow Android JNI ABI for bounded Refloat compatibility decisions.
//!
//! Protocol, catalogue, and canonical-record decisions remain in the safe
//! `float-view-refloat-compatibility` crate. This crate owns only bounded JNI
//! validation, call-local copies, fixed status mapping, and publication.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_android_jni_support::{
    Environment, JNIEnv, contain, jbyteArray, jint, jintArray, jobject, with_environment,
};
pub use float_view_refloat_compatibility::{
    COMPATIBILITY_RECORD_BYTES, INFO_REQUEST, MAX_LAYOUT_RECORD_BYTES, MAX_PAYLOAD_BYTES,
};
use float_view_refloat_compatibility::{
    CompatibilityError, CompatibilityRecordOutcome, RECORD_VERSION, admit_dynamic_layout_record,
    decode_info_record,
};

/// Version of the complete hand-written Android compatibility JNI contract.
pub const ABI_VERSION: jint = RECORD_VERSION as jint;

pub const STATUS_OK: jint = 0;
pub const STATUS_INVALID_ARGUMENT: jint = 1;
pub const STATUS_UNSUPPORTED_ENVELOPE: jint = 2;
pub const STATUS_TRUNCATED_PAYLOAD: jint = 3;
pub const STATUS_UNSUPPORTED_PROTOCOL: jint = 4;
pub const STATUS_UNSUPPORTED_RELEASE: jint = 5;
pub const STATUS_INVALID_LAYOUT: jint = 6;
pub const STATUS_OUTPUT_TOO_SMALL: jint = 7;
pub const STATUS_INTERNAL_FAILURE: jint = 8;

/// Return the exact version of the hand-written compatibility JNI contract.
///
/// # Safety
///
/// `env` must be null or the JNI environment supplied for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_abiVersion(
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

/// Publish the exact unframed INFO request into JVM-owned primitive arrays.
///
/// `result[0]` receives the produced or required request length and is
/// initialized to zero after the complete caller contract is validated.
///
/// # Safety
///
/// Every non-null reference must belong to `env`, have the primitive-array type
/// declared by the Kotlin native method, and remain valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_infoRequest(
    env: *mut JNIEnv,
    _receiver: jobject,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| {
            invoke_request(environment, output, result)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Decode INFO and atomically publish one accepted canonical record.
///
/// `result[0]` receives the produced or required record length and is
/// initialized to zero after the complete caller contract is validated.
///
/// # Safety
///
/// Every non-null reference must belong to `env`, have the primitive-array type
/// declared by the Kotlin native method, and remain valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_decodeInfo(
    env: *mut JNIEnv,
    _receiver: jobject,
    input: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| {
            invoke_decode(environment, input, output, result)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Admit one canonical Rust dynamic-layout record.
///
/// # Safety
///
/// The non-null reference must belong to `env`, have the primitive-array type
/// declared by the Kotlin native method, and remain valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_admitLayout(
    env: *mut JNIEnv,
    _receiver: jobject,
    layout: jbyteArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| invoke_admit(environment, layout))
            .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

fn invoke_request(environment: &Environment<'_>, output: jbyteArray, result: jintArray) -> jint {
    if environment.ensure_no_exception().is_err()
        || environment
            .require_references(&[output.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    let Ok(output_capacity) = environment.array_length(output.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if output_capacity > INFO_REQUEST.len()
        || environment.require_array_length(result.cast(), 1).is_err()
        || environment
            .require_distinct(&[output.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if environment.write_ints_exact(result, 1, &[0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    publish(
        environment,
        output,
        output_capacity,
        result,
        OutputResult::record(&INFO_REQUEST, output_capacity),
    )
}

fn invoke_decode(
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
    if input_length > MAX_PAYLOAD_BYTES
        || output_capacity > COMPATIBILITY_RECORD_BYTES
        || environment.require_array_length(result.cast(), 1).is_err()
        || environment
            .require_distinct(&[input.cast(), output.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if environment.write_ints_exact(result, 1, &[0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    let Ok(input) = environment.read_bytes(input, input_length, MAX_PAYLOAD_BYTES) else {
        return STATUS_INTERNAL_FAILURE;
    };
    publish(
        environment,
        output,
        output_capacity,
        result,
        run_decode(&input, output_capacity),
    )
}

fn invoke_admit(environment: &Environment<'_>, layout: jbyteArray) -> jint {
    if environment.ensure_no_exception().is_err()
        || environment.require_references(&[layout.cast()]).is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    let Ok(layout_length) = environment.array_length(layout.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if layout_length > MAX_LAYOUT_RECORD_BYTES {
        return STATUS_INVALID_LAYOUT;
    }
    let Ok(layout) = environment.read_bytes(layout, layout_length, MAX_LAYOUT_RECORD_BYTES) else {
        return STATUS_INTERNAL_FAILURE;
    };
    run_admit(&layout)
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
    if environment
        .write_ints_exact(result, 1, &[produced.required_length as jint])
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
    fn failure(status: jint) -> Self {
        Self {
            status,
            bytes: None,
            required_length: 0,
        }
    }

    fn record(bytes: &[u8], output_capacity: usize) -> Self {
        if bytes.len() > output_capacity {
            Self {
                status: STATUS_OUTPUT_TOO_SMALL,
                bytes: None,
                required_length: bytes.len(),
            }
        } else {
            Self {
                status: STATUS_OK,
                bytes: Some(bytes.to_vec()),
                required_length: bytes.len(),
            }
        }
    }
}

fn run_decode(input: &[u8], output_capacity: usize) -> OutputResult {
    match contain(CompatibilityRecordOutcome::InternalFailure, || {
        decode_info_record(input)
    }) {
        CompatibilityRecordOutcome::Record(record) => {
            OutputResult::record(record.as_bytes(), output_capacity)
        }
        CompatibilityRecordOutcome::InputTooLarge | CompatibilityRecordOutcome::TruncatedInfo => {
            OutputResult::failure(STATUS_TRUNCATED_PAYLOAD)
        }
        CompatibilityRecordOutcome::UnsupportedEnvelope => {
            OutputResult::failure(STATUS_UNSUPPORTED_ENVELOPE)
        }
        CompatibilityRecordOutcome::UnsupportedInfoProtocol => {
            OutputResult::failure(STATUS_UNSUPPORTED_PROTOCOL)
        }
        CompatibilityRecordOutcome::UnsupportedRelease => {
            OutputResult::failure(STATUS_UNSUPPORTED_RELEASE)
        }
        CompatibilityRecordOutcome::InternalFailure => {
            OutputResult::failure(STATUS_INTERNAL_FAILURE)
        }
    }
}

fn run_admit(layout: &[u8]) -> jint {
    match contain(Err(CompatibilityError::InvalidLayoutRecord), || {
        admit_dynamic_layout_record(layout)
    }) {
        Ok(()) => STATUS_OK,
        Err(
            CompatibilityError::InvalidLayoutRecord
            | CompatibilityError::TooManyIdentifiers
            | CompatibilityError::EmptyIdentifier
            | CompatibilityError::IdentifierTooLong
            | CompatibilityError::InvalidIdentifier
            | CompatibilityError::TooManyIdentifierBytes
            | CompatibilityError::DuplicateIdentifier
            | CompatibilityError::MissingRequiredIdentifier
            | CompatibilityError::MissingRequiredRuntimeIdentifier,
        ) => STATUS_INVALID_LAYOUT,
        Err(_) => STATUS_INTERNAL_FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_view_refloat_dynamic_telemetry::{LayoutRecordOutcome, decode_layout_to_record};
    use std::ptr;

    fn info_v2(major: u8, minor: u8, patch: u8) -> [u8; 61] {
        let mut payload = [0_u8; 61];
        payload[..5].copy_from_slice(&INFO_REQUEST);
        payload[25] = major;
        payload[26] = minor;
        payload[27] = patch;
        payload
    }

    fn required_layout() -> Vec<u8> {
        let fields = [
            "motor.speed",
            "motor.erpm",
            "motor.current",
            "motor.dir_current",
            "motor.filt_current",
            "motor.duty_cycle",
            "motor.batt_voltage",
            "motor.batt_current",
            "motor.mosfet_temp",
            "motor.motor_temp",
            "imu.pitch",
            "imu.balance_pitch",
            "imu.roll",
            "footpad.adc1",
            "footpad.adc2",
            "remote.input",
        ];
        let runtime = [
            "setpoint",
            "atr.setpoint",
            "brake_tilt.setpoint",
            "torque_tilt.setpoint",
            "turn_tilt.setpoint",
            "remote.setpoint",
            "balance_current",
            "atr.accel_diff",
            "atr.speed_boost",
            "booster.current",
        ];
        let mut payload = vec![36, 101, 32, fields.len() as u8];
        for name in fields {
            payload.push(name.len() as u8);
            payload.extend_from_slice(name.as_bytes());
        }
        payload.push(runtime.len() as u8);
        for name in runtime {
            payload.push(name.len() as u8);
            payload.extend_from_slice(name.as_bytes());
        }
        let LayoutRecordOutcome::Record(record) = decode_layout_to_record(&payload) else {
            panic!("required layout must decode");
        };
        record.as_bytes().to_vec()
    }

    #[test]
    fn publishes_the_exact_safe_request_and_records() {
        assert_eq!(
            OutputResult::record(&INFO_REQUEST, INFO_REQUEST.len()),
            OutputResult {
                status: STATUS_OK,
                bytes: Some(vec![36, 101, 0, 2, 0]),
                required_length: 5,
            }
        );
        let legacy = run_decode(&[36, 101, 0, 10], COMPATIBILITY_RECORD_BYTES);
        assert_eq!(legacy.status, STATUS_OK);
        assert_eq!(legacy.required_length, COMPATIBILITY_RECORD_BYTES);
        assert_eq!(legacy.bytes.unwrap(), [1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0]);

        let stable = run_decode(&info_v2(1, 2, 3), COMPATIBILITY_RECORD_BYTES);
        assert_eq!(stable.status, STATUS_OK);
        assert_eq!(stable.bytes.unwrap(), [1, 1, 1, 2, 1, 3, 3, 2, 2, 1, 0, 0]);
    }

    #[test]
    fn maps_every_safe_outcome_and_required_capacity() {
        for (payload, status) in [
            (vec![], STATUS_TRUNCATED_PAYLOAD),
            (vec![37, 101, 0, 10], STATUS_UNSUPPORTED_ENVELOPE),
            (vec![36, 101, 0, 1], STATUS_UNSUPPORTED_PROTOCOL),
            (info_v2(1, 2, 4).to_vec(), STATUS_UNSUPPORTED_RELEASE),
        ] {
            assert_eq!(
                run_decode(&payload, COMPATIBILITY_RECORD_BYTES).status,
                status
            );
        }
        let too_small = run_decode(&info_v2(1, 2, 3), COMPATIBILITY_RECORD_BYTES - 1);
        assert_eq!(too_small.status, STATUS_OUTPUT_TOO_SMALL);
        assert_eq!(too_small.required_length, COMPATIBILITY_RECORD_BYTES);
        assert!(too_small.bytes.is_none());

        assert_eq!(run_admit(&required_layout()), STATUS_OK);
        assert_eq!(run_admit(&[]), STATUS_INVALID_LAYOUT);
    }

    #[test]
    fn contains_panics_and_keeps_the_contract_closed() {
        assert_eq!(contain(STATUS_INTERNAL_FAILURE, || panic!("contained")), 8);
        assert_eq!(ABI_VERSION, 1);
        assert_eq!(STATUS_OK, 0);
        assert_eq!(STATUS_INVALID_ARGUMENT, 1);
        assert_eq!(STATUS_UNSUPPORTED_ENVELOPE, 2);
        assert_eq!(STATUS_TRUNCATED_PAYLOAD, 3);
        assert_eq!(STATUS_UNSUPPORTED_PROTOCOL, 4);
        assert_eq!(STATUS_UNSUPPORTED_RELEASE, 5);
        assert_eq!(STATUS_INVALID_LAYOUT, 6);
        assert_eq!(STATUS_OUTPUT_TOO_SMALL, 7);
        assert_eq!(STATUS_INTERNAL_FAILURE, 8);

        // SAFETY: null environment pointers are explicitly rejected without access.
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_abiVersion(
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            0
        );
        // SAFETY: null environment pointers are explicitly rejected without access.
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_infoRequest(
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: null environment pointers are explicitly rejected without access.
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_decodeInfo(
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: null environment pointers are explicitly rejected without access.
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_protocol_RustRefloatCompatibilityBridge_admitLayout(
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            STATUS_INVALID_ARGUMENT
        );
    }
}
