//! Narrow Android JNI ABI for bounded Refloat legacy telemetry decoding.
//!
//! Protocol and canonical-record decisions remain in the safe
//! `float-view-refloat-legacy-telemetry` crate. This crate owns only bounded
//! JNI validation, call-local copies, status mapping, and publication.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_android_jni_support::{
    Environment, JNIEnv, contain, jbyteArray, jint, jintArray, jobject, with_environment,
};
pub use float_view_refloat_legacy_telemetry::{
    ACTIVE_FAULT_RECORD_BYTES, MAX_PAYLOAD_BYTES, MAX_RECORD_BYTES, RECORD_HEADER_BYTES,
    SAMPLE_RECORD_BYTES,
};
use float_view_refloat_legacy_telemetry::{
    LegacyTelemetryRecord, LegacyTelemetryRecordOutcome, RECORD_VERSION, decode_record,
};

/// Version of the complete hand-written Android telemetry JNI contract.
pub const ABI_VERSION: jint = RECORD_VERSION as jint;

pub const STATUS_OK: jint = 0;
pub const STATUS_INVALID_ARGUMENT: jint = 1;
pub const STATUS_UNSUPPORTED_ENVELOPE: jint = 2;
pub const STATUS_TRUNCATED_PAYLOAD: jint = 3;
pub const STATUS_INVALID_PAYLOAD: jint = 4;
pub const STATUS_OUTPUT_TOO_SMALL: jint = 5;
pub const STATUS_INTERNAL_FAILURE: jint = 6;

/// Return the exact version of the hand-written telemetry JNI contract.
///
/// # Safety
///
/// `env` must be null or the JNI environment supplied for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatLegacyTelemetryBridge_abiVersion(
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

/// Decode one bounded command-10 payload into JVM-owned primitive arrays.
///
/// `result[0]` receives the produced or required record length and `result[1]`
/// receives the unsupported command byte. Both values are initialized to zero
/// after the complete caller contract has been validated.
///
/// # Safety
///
/// Every non-null reference must belong to `env`, have the primitive-array type
/// declared by the Kotlin native method, and remain valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatLegacyTelemetryBridge_decode(
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
        || output_capacity > MAX_RECORD_BYTES
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if environment.write_ints_exact(result, 2, &[0, 0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if input_length > MAX_PAYLOAD_BYTES {
        return STATUS_TRUNCATED_PAYLOAD;
    }
    let Ok(input) = environment.read_bytes(input, input_length, MAX_PAYLOAD_BYTES) else {
        return STATUS_INTERNAL_FAILURE;
    };
    let decoded = run_decode(&input, output_capacity);
    if let Some(record) = decoded.record
        && environment
            .write_bytes(output, output_capacity, record.as_bytes())
            .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    if environment
        .write_ints_exact(result, 2, &decoded.metadata)
        .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    decoded.status
}

#[derive(Debug)]
struct DecodeResult {
    status: jint,
    record: Option<LegacyTelemetryRecord>,
    metadata: [jint; 2],
}

fn run_decode(input: &[u8], output_capacity: usize) -> DecodeResult {
    let outcome = contain(LegacyTelemetryRecordOutcome::InternalFailure, || {
        decode_record(input)
    });
    match outcome {
        LegacyTelemetryRecordOutcome::Record(record) if record.len() > output_capacity => {
            DecodeResult {
                status: STATUS_OUTPUT_TOO_SMALL,
                record: None,
                metadata: [record.len() as jint, 0],
            }
        }
        LegacyTelemetryRecordOutcome::Record(record) => DecodeResult {
            status: STATUS_OK,
            metadata: [record.len() as jint, 0],
            record: Some(record),
        },
        LegacyTelemetryRecordOutcome::UnsupportedEnvelope(command) => DecodeResult {
            status: STATUS_UNSUPPORTED_ENVELOPE,
            record: None,
            metadata: [0, jint::from(command)],
        },
        LegacyTelemetryRecordOutcome::TruncatedPayload => DecodeResult {
            status: STATUS_TRUNCATED_PAYLOAD,
            record: None,
            metadata: [0, 0],
        },
        LegacyTelemetryRecordOutcome::InvalidPayload => DecodeResult {
            status: STATUS_INVALID_PAYLOAD,
            record: None,
            metadata: [0, 0],
        },
        LegacyTelemetryRecordOutcome::InternalFailure => DecodeResult {
            status: STATUS_INTERNAL_FAILURE,
            record: None,
            metadata: [0, 0],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detail_one() -> [u8; 35] {
        let mut payload = [0_u8; 35];
        payload[..4].copy_from_slice(&[36, 101, 10, 1]);
        payload[10] = 1;
        payload[11] = 2;
        payload[12] = 50;
        payload[13] = 100;
        payload[14..20].fill(128);
        payload[22] = 128;
        payload[23..25].copy_from_slice(&500_i16.to_be_bytes());
        payload[33] = 128;
        payload[34] = 222;
        payload
    }

    #[test]
    fn publishes_the_safe_crates_exact_records() {
        let sample = run_decode(&detail_one(), MAX_RECORD_BYTES);
        assert_eq!(sample.status, STATUS_OK);
        assert_eq!(sample.metadata, [SAMPLE_RECORD_BYTES as jint, 0]);
        assert_eq!(
            &sample.record.unwrap().as_bytes()[..RECORD_HEADER_BYTES],
            &[1, 1, 1, 2, 1, 4, 1, 1, 0, 1, 0, 0, 0, 0, 0, 0]
        );

        let fault = run_decode(&[36, 101, 10, 69, 3], MAX_RECORD_BYTES);
        assert_eq!(fault.status, STATUS_OK);
        assert_eq!(fault.metadata, [ACTIVE_FAULT_RECORD_BYTES as jint, 0]);
        assert_eq!(
            fault.record.unwrap().as_bytes(),
            &[1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 0, 0, 0, 0]
        );
    }

    #[test]
    fn maps_every_safe_outcome_and_required_capacity() {
        for (payload, status, metadata) in [
            (vec![37, 101, 10, 1], STATUS_UNSUPPORTED_ENVELOPE, [0, 37]),
            (vec![36, 101], STATUS_TRUNCATED_PAYLOAD, [0, 0]),
            (vec![36, 101, 10, 0], STATUS_INVALID_PAYLOAD, [0, 0]),
        ] {
            let result = run_decode(&payload, MAX_RECORD_BYTES);
            assert_eq!(result.status, status);
            assert_eq!(result.metadata, metadata);
            assert!(result.record.is_none());
        }
        let too_small = run_decode(&detail_one(), SAMPLE_RECORD_BYTES - 1);
        assert_eq!(too_small.status, STATUS_OUTPUT_TOO_SMALL);
        assert_eq!(too_small.metadata, [SAMPLE_RECORD_BYTES as jint, 0]);
        assert!(too_small.record.is_none());
    }

    #[test]
    fn contains_panics_and_keeps_the_contract_closed() {
        assert_eq!(contain(STATUS_INTERNAL_FAILURE, || panic!("contained")), 6);
        assert_eq!(ABI_VERSION, 1);
        assert_eq!(STATUS_OK, 0);
        assert_eq!(STATUS_INVALID_ARGUMENT, 1);
        assert_eq!(STATUS_UNSUPPORTED_ENVELOPE, 2);
        assert_eq!(STATUS_TRUNCATED_PAYLOAD, 3);
        assert_eq!(STATUS_INVALID_PAYLOAD, 4);
        assert_eq!(STATUS_OUTPUT_TOO_SMALL, 5);
        assert_eq!(STATUS_INTERNAL_FAILURE, 6);
        assert_eq!(MAX_PAYLOAD_BYTES, 59);
        assert_eq!(MAX_RECORD_BYTES, 264);
    }
}
