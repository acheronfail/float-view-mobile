//! Narrow Android JNI ABI for bounded Refloat dynamic telemetry decoding.
//!
//! Protocol and canonical-record decisions remain in the safe
//! `float-view-refloat-dynamic-telemetry` crate. This crate owns only bounded
//! JNI validation, call-local copies, status mapping, and publication.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_android_jni_support::{
    Environment, JNIEnv, contain, jbyteArray, jint, jintArray, jobject, with_environment,
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

/// Version of the complete hand-written Android dynamic-telemetry JNI contract.
pub const ABI_VERSION: jint = RECORD_VERSION as jint;

pub const STATUS_OK: jint = 0;
pub const STATUS_INVALID_ARGUMENT: jint = 1;
pub const STATUS_UNSUPPORTED_ENVELOPE: jint = 2;
pub const STATUS_TRUNCATED_PAYLOAD: jint = 3;
pub const STATUS_INVALID_LAYOUT: jint = 4;
pub const STATUS_INVALID_TELEMETRY: jint = 5;
pub const STATUS_OUTPUT_TOO_SMALL: jint = 6;
pub const STATUS_INTERNAL_FAILURE: jint = 7;

/// Return the exact version of the hand-written dynamic-telemetry JNI contract.
///
/// # Safety
///
/// `env` must be null or the JNI environment supplied for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge_abiVersion(
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

/// Decode one bounded command-32 payload into JVM-owned primitive arrays.
///
/// `result[0]` receives the produced or required record length and is
/// initialized to zero after the complete caller contract has been validated.
///
/// # Safety
///
/// Every non-null reference must belong to `env`, have the primitive-array type
/// declared by the Kotlin native method, and remain valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge_decodeLayout(
    env: *mut JNIEnv,
    _receiver: jobject,
    input: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| {
            invoke_layout(environment, input, output, result)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Decode one bounded command-31 payload using one canonical layout record.
///
/// `result[0]` receives the produced or required record length and is
/// initialized to zero after the complete caller contract has been validated.
///
/// # Safety
///
/// Every non-null reference must belong to `env`, have the primitive-array type
/// declared by the Kotlin native method, and remain valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge_decodeRealtime(
    env: *mut JNIEnv,
    _receiver: jobject,
    layout: jbyteArray,
    input: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain(STATUS_INTERNAL_FAILURE, || unsafe {
        with_environment(env, |environment| {
            invoke_realtime(environment, layout, input, output, result)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

fn invoke_layout(
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
    if environment.require_array_length(result.cast(), 1).is_err()
        || environment
            .require_distinct(&[input.cast(), output.cast(), result.cast()])
            .is_err()
        || output_capacity > MAX_LAYOUT_RECORD_BYTES
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if environment.write_ints_exact(result, 1, &[0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if input_length > MAX_LAYOUT_PAYLOAD_BYTES {
        return STATUS_TRUNCATED_PAYLOAD;
    }
    let Ok(input) = environment.read_bytes(input, input_length, MAX_LAYOUT_PAYLOAD_BYTES) else {
        return STATUS_INTERNAL_FAILURE;
    };
    publish(
        environment,
        output,
        output_capacity,
        result,
        run_layout(&input, output_capacity),
    )
}

fn invoke_realtime(
    environment: &Environment<'_>,
    layout: jbyteArray,
    input: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    if environment.ensure_no_exception().is_err()
        || environment
            .require_references(&[layout.cast(), input.cast(), output.cast(), result.cast()])
            .is_err()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    let Ok(layout_length) = environment.array_length(layout.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Ok(input_length) = environment.array_length(input.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Ok(output_capacity) = environment.array_length(output.cast()) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if environment.require_array_length(result.cast(), 1).is_err()
        || environment
            .require_distinct(&[layout.cast(), input.cast(), output.cast(), result.cast()])
            .is_err()
        || output_capacity > MAX_SAMPLE_RECORD_BYTES
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if environment.write_ints_exact(result, 1, &[0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if layout_length > MAX_LAYOUT_RECORD_BYTES {
        return STATUS_INVALID_LAYOUT;
    }
    if input_length > MAX_REALTIME_PAYLOAD_BYTES {
        return STATUS_TRUNCATED_PAYLOAD;
    }
    let Ok(layout) = environment.read_bytes(layout, layout_length, MAX_LAYOUT_RECORD_BYTES) else {
        return STATUS_INTERNAL_FAILURE;
    };
    let Ok(input) = environment.read_bytes(input, input_length, MAX_REALTIME_PAYLOAD_BYTES) else {
        return STATUS_INTERNAL_FAILURE;
    };
    publish(
        environment,
        output,
        output_capacity,
        result,
        run_realtime(&layout, &input, output_capacity),
    )
}

fn publish(
    environment: &Environment<'_>,
    output: jbyteArray,
    output_capacity: usize,
    result: jintArray,
    decoded: DecodeResult,
) -> jint {
    if let Some(record) = decoded.record
        && environment
            .write_bytes(output, output_capacity, &record)
            .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    if environment
        .write_ints_exact(result, 1, &[decoded.record_length as jint])
        .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    decoded.status
}

#[derive(Debug, Eq, PartialEq)]
struct DecodeResult {
    status: jint,
    record: Option<Vec<u8>>,
    record_length: usize,
}

impl DecodeResult {
    fn failure(status: jint) -> Self {
        Self {
            status,
            record: None,
            record_length: 0,
        }
    }

    fn record(bytes: &[u8], output_capacity: usize) -> Self {
        if bytes.len() > output_capacity {
            Self {
                status: STATUS_OUTPUT_TOO_SMALL,
                record: None,
                record_length: bytes.len(),
            }
        } else {
            Self {
                status: STATUS_OK,
                record: Some(bytes.to_vec()),
                record_length: bytes.len(),
            }
        }
    }
}

fn run_layout(input: &[u8], output_capacity: usize) -> DecodeResult {
    match contain(None, || Some(decode_layout_to_record(input))) {
        Some(LayoutRecordOutcome::Record(record)) => {
            DecodeResult::record(record.as_bytes(), output_capacity)
        }
        Some(LayoutRecordOutcome::UnsupportedEnvelope) => {
            DecodeResult::failure(STATUS_UNSUPPORTED_ENVELOPE)
        }
        Some(LayoutRecordOutcome::TruncatedPayload) => {
            DecodeResult::failure(STATUS_TRUNCATED_PAYLOAD)
        }
        Some(LayoutRecordOutcome::InvalidLayout) => DecodeResult::failure(STATUS_INVALID_LAYOUT),
        None => DecodeResult::failure(STATUS_INTERNAL_FAILURE),
    }
}

fn run_realtime(layout: &[u8], input: &[u8], output_capacity: usize) -> DecodeResult {
    match contain(None, || Some(decode_realtime_to_record(layout, input))) {
        Some(SampleRecordOutcome::Record(record)) => {
            DecodeResult::record(record.as_bytes(), output_capacity)
        }
        Some(SampleRecordOutcome::InvalidLayoutRecord) => {
            DecodeResult::failure(STATUS_INVALID_LAYOUT)
        }
        Some(SampleRecordOutcome::UnsupportedEnvelope) => {
            DecodeResult::failure(STATUS_UNSUPPORTED_ENVELOPE)
        }
        Some(SampleRecordOutcome::TruncatedPayload) => {
            DecodeResult::failure(STATUS_TRUNCATED_PAYLOAD)
        }
        Some(SampleRecordOutcome::InvalidTelemetry) => {
            DecodeResult::failure(STATUS_INVALID_TELEMETRY)
        }
        None => DecodeResult::failure(STATUS_INTERNAL_FAILURE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

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
    fn publishes_the_safe_crates_exact_records() {
        let layout = run_layout(&layout_payload(), MAX_LAYOUT_RECORD_BYTES);
        assert_eq!(layout.status, STATUS_OK);
        assert_eq!(layout.record_length, 52);
        let layout_record = layout.record.unwrap();
        assert_eq!(&layout_record[..8], &[1, 1, 3, 0, 0, 41, 0, 0]);

        let sample = run_realtime(&layout_record, &realtime_payload(), MAX_SAMPLE_RECORD_BYTES);
        assert_eq!(sample.status, STATUS_OK);
        assert_eq!(sample.record_length, SAMPLE_FIXED_BYTES + 1 + 12 + 8);
        assert_eq!(
            &sample.record.unwrap()[..16],
            &[1, 2, 3, 1, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0]
        );
    }

    #[test]
    fn maps_every_safe_outcome_and_required_capacity() {
        for (payload, status) in [
            (vec![37, 101, 32], STATUS_UNSUPPORTED_ENVELOPE),
            (vec![36, 101, 32], STATUS_TRUNCATED_PAYLOAD),
            (vec![36, 101, 32, 0, 0], STATUS_INVALID_LAYOUT),
        ] {
            assert_eq!(run_layout(&payload, MAX_LAYOUT_RECORD_BYTES).status, status);
        }

        let layout = run_layout(&layout_payload(), MAX_LAYOUT_RECORD_BYTES)
            .record
            .unwrap();
        let mut invalid_telemetry = realtime_payload();
        invalid_telemetry[3] = 0x08;
        for (layout_record, payload, status) in [
            (vec![], realtime_payload(), STATUS_INVALID_LAYOUT),
            (
                layout.clone(),
                vec![37, 101, 31],
                STATUS_UNSUPPORTED_ENVELOPE,
            ),
            (layout.clone(), vec![36, 101, 31], STATUS_TRUNCATED_PAYLOAD),
            (layout.clone(), invalid_telemetry, STATUS_INVALID_TELEMETRY),
        ] {
            assert_eq!(
                run_realtime(&layout_record, &payload, MAX_SAMPLE_RECORD_BYTES).status,
                status
            );
        }

        let small_layout = run_layout(&layout_payload(), 2);
        assert_eq!(small_layout.status, STATUS_OUTPUT_TOO_SMALL);
        assert_eq!(small_layout.record_length, 52);
        assert!(small_layout.record.is_none());

        let small_sample = run_realtime(&layout, &realtime_payload(), 2);
        assert_eq!(small_sample.status, STATUS_OUTPUT_TOO_SMALL);
        assert!(small_sample.record_length > 2);
        assert!(small_sample.record.is_none());
    }

    #[test]
    fn keeps_the_contract_closed_and_contains_boundary_failures() {
        assert_eq!(contain(STATUS_INTERNAL_FAILURE, || panic!("contained")), 7);
        assert_eq!(ABI_VERSION, 1);
        assert_eq!(STATUS_OK, 0);
        assert_eq!(STATUS_INVALID_ARGUMENT, 1);
        assert_eq!(STATUS_UNSUPPORTED_ENVELOPE, 2);
        assert_eq!(STATUS_TRUNCATED_PAYLOAD, 3);
        assert_eq!(STATUS_INVALID_LAYOUT, 4);
        assert_eq!(STATUS_INVALID_TELEMETRY, 5);
        assert_eq!(STATUS_OUTPUT_TOO_SMALL, 6);
        assert_eq!(STATUS_INTERNAL_FAILURE, 7);

        // SAFETY: null environment pointers are explicitly rejected without access.
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge_abiVersion(
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            0
        );
        // SAFETY: null environment pointers are explicitly rejected without access.
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge_decodeLayout(
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
                Java_fail_acheron_floatview_protocol_RustRefloatDynamicTelemetryBridge_decodeRealtime(
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            STATUS_INVALID_ARGUMENT
        );
    }
}
