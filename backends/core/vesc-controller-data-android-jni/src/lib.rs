//! Narrow Android JNI ABI for bounded VESC controller-data decoding.
//!
//! Protocol decisions remain in the safe `float-view-vesc-controller-data`
//! crate. This crate owns only primitive validation and call-local JNI copies.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_vesc_controller_data::{
    ControllerDataError, CurrentLimits, FaultKind, RealtimeValues, decode_current_limits,
    decode_realtime_values,
};
use jni_sys::{JNIEnv, jbyte, jbyteArray, jint, jintArray, jobject, jsize};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub const ABI_VERSION: jint = 1;
pub const OPERATION_REALTIME: jint = 1;
pub const OPERATION_CURRENT_LIMITS: jint = 2;
pub const STATUS_OK: jint = 0;
pub const STATUS_INVALID_ARGUMENT: jint = 1;
pub const STATUS_UNSUPPORTED_COMMAND: jint = 2;
pub const STATUS_TRUNCATED_PAYLOAD: jint = 3;
pub const STATUS_OUTPUT_TOO_SMALL: jint = 4;
pub const STATUS_INTERNAL_FAILURE: jint = 5;
pub const RESULT_REALTIME: u8 = 1;
pub const RESULT_CURRENT_LIMITS: u8 = 2;
pub const FLAG_SERIES_CELLS_PRESENT: u8 = 1;
pub const FAULT_NONE: u8 = 0;
pub const FAULT_OVER_VOLTAGE: u8 = 1;
pub const FAULT_UNKNOWN: u8 = 2;
pub const MAX_PAYLOAD_BYTES: usize = 4_096;
pub const RECORD_HEADER_BYTES: usize = 8;
pub const REALTIME_RECORD_BYTES: usize = 60;
pub const CURRENT_LIMITS_RECORD_BYTES: usize = 40;
pub const MAX_RECORD_BYTES: usize = REALTIME_RECORD_BYTES;

#[derive(Debug, Eq, PartialEq)]
struct DecodeResult {
    status: jint,
    record: Vec<u8>,
    record_length: usize,
    unsupported_command: u8,
}

impl DecodeResult {
    fn failure(status: jint) -> Self {
        Self {
            status,
            record: Vec::new(),
            record_length: 0,
            unsupported_command: 0,
        }
    }
}

/// Return the exact hand-written Android controller-data JNI contract version.
///
/// # Safety
/// `env` must be null or the JNI environment supplied for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge_abiVersion(
    env: *mut JNIEnv,
    _receiver: jobject,
) -> jint {
    contain_abi(|| {
        let Some(functions) = (unsafe { environment(env) }) else {
            return 0;
        };
        if unsafe { (functions.ExceptionCheck)(env) } {
            0
        } else {
            ABI_VERSION
        }
    })
}

/// Decode one bounded command-4 or command-14 payload into JVM-owned arrays.
///
/// `result[0]` receives the produced or required record length and `result[1]`
/// receives the unsupported command byte. Both are initialized to zero after
/// the complete caller contract has been validated.
///
/// # Safety
/// References must belong to `env`, have their declared primitive-array types,
/// and remain valid for this call. Rust retains no reference.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge_decode(
    env: *mut JNIEnv,
    _receiver: jobject,
    operation: jint,
    input: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain_status(|| unsafe { invoke(env, operation, input, output, result) })
}

unsafe fn invoke(
    env: *mut JNIEnv,
    operation: jint,
    input: jbyteArray,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    let Some(functions) = (unsafe { environment(env) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if !matches!(operation, OPERATION_REALTIME | OPERATION_CURRENT_LIMITS)
        || unsafe { (functions.ExceptionCheck)(env) }
        || input.is_null()
        || output.is_null()
        || result.is_null()
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
    let result_length = unsafe { (functions.GetArrayLength)(env, result.cast()) };
    if unsafe { (functions.ExceptionCheck)(env) } || result_length != 2 {
        return STATUS_INVALID_ARGUMENT;
    }
    if unsafe { references_alias(env, functions, input.cast(), output.cast()) }
        || unsafe { references_alias(env, functions, input.cast(), result.cast()) }
        || unsafe { references_alias(env, functions, output.cast(), result.cast()) }
        || unsafe { (functions.ExceptionCheck)(env) }
    {
        return STATUS_INVALID_ARGUMENT;
    }

    let input_length = input_length as usize;
    let output_capacity = output_capacity as usize;
    if output_capacity > MAX_RECORD_BYTES {
        return STATUS_INVALID_ARGUMENT;
    }
    if write_metadata(env, functions, result, [0, 0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if input_length > MAX_PAYLOAD_BYTES {
        return STATUS_TRUNCATED_PAYLOAD;
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

    let decoded = run_decode(operation, &copied_input, output_capacity);
    if decoded.status == STATUS_OK {
        unsafe {
            (functions.SetByteArrayRegion)(
                env,
                output,
                0,
                decoded.record_length as jsize,
                decoded.record.as_ptr().cast::<jbyte>(),
            )
        };
        if unsafe { (functions.ExceptionCheck)(env) } {
            return STATUS_INTERNAL_FAILURE;
        }
    }
    if write_metadata(
        env,
        functions,
        result,
        [
            decoded.record_length as jint,
            jint::from(decoded.unsupported_command),
        ],
    )
    .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    decoded.status
}

fn run_decode(operation: jint, input: &[u8], output_capacity: usize) -> DecodeResult {
    contain(|| {
        let record = match operation {
            OPERATION_REALTIME => match decode_realtime_values(input) {
                Ok(values) => encode_realtime(values)?,
                Err(error) => return Ok(map_decode_error(error)),
            },
            OPERATION_CURRENT_LIMITS => match decode_current_limits(input) {
                Ok(values) => encode_current_limits(values)?,
                Err(error) => return Ok(map_decode_error(error)),
            },
            _ => return Ok(DecodeResult::failure(STATUS_INVALID_ARGUMENT)),
        };
        let record_length = record.len();
        if record_length > output_capacity {
            return Ok(DecodeResult {
                status: STATUS_OUTPUT_TOO_SMALL,
                record: Vec::new(),
                record_length,
                unsupported_command: 0,
            });
        }
        Ok(DecodeResult {
            status: STATUS_OK,
            record,
            record_length,
            unsupported_command: 0,
        })
    })
}

fn map_decode_error(error: ControllerDataError) -> DecodeResult {
    match error {
        ControllerDataError::UnsupportedCommand(command) => DecodeResult {
            status: STATUS_UNSUPPORTED_COMMAND,
            record: Vec::new(),
            record_length: 0,
            unsupported_command: command,
        },
        ControllerDataError::TruncatedPayload => DecodeResult::failure(STATUS_TRUNCATED_PAYLOAD),
    }
}

fn encode_realtime(values: RealtimeValues) -> Result<Vec<u8>, jint> {
    let scalars = [
        values.controller_temp_c,
        values.motor_temp_c,
        values.motor_current_a,
        values.battery_current_a,
        values.duty_cycle,
        values.pack_voltage_v,
    ];
    if scalars.iter().any(|value| !value.is_finite()) {
        return Err(STATUS_INTERNAL_FAILURE);
    }
    let mut record = Vec::with_capacity(REALTIME_RECORD_BYTES);
    record.extend_from_slice(&[
        ABI_VERSION as u8,
        RESULT_REALTIME,
        0,
        fault_code(values.fault),
        values.fault_code,
        0,
        0,
        0,
    ]);
    for value in scalars {
        record.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    record.extend_from_slice(&values.erpm.to_be_bytes());
    debug_assert_eq!(record.len(), REALTIME_RECORD_BYTES);
    Ok(record)
}

fn encode_current_limits(values: CurrentLimits) -> Result<Vec<u8>, jint> {
    let scalars = [
        values.motor_current_max_a,
        values.motor_current_min_a,
        values.battery_current_max_a,
        values.battery_current_min_a,
    ];
    if scalars
        .iter()
        .any(|value| !value.is_finite() || value.abs() <= 0.0)
    {
        return Err(STATUS_INTERNAL_FAILURE);
    }
    let (flags, cells) = values
        .battery_series_cells
        .map_or((0, 0), |value| (FLAG_SERIES_CELLS_PRESENT, value));
    if flags != 0 && cells == 0 {
        return Err(STATUS_INTERNAL_FAILURE);
    }
    let mut record = Vec::with_capacity(CURRENT_LIMITS_RECORD_BYTES);
    record.extend_from_slice(&[
        ABI_VERSION as u8,
        RESULT_CURRENT_LIMITS,
        flags,
        0,
        0,
        cells,
        0,
        0,
    ]);
    for value in scalars {
        record.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    debug_assert_eq!(record.len(), CURRENT_LIMITS_RECORD_BYTES);
    Ok(record)
}

const fn fault_code(fault: FaultKind) -> u8 {
    match fault {
        FaultKind::None => FAULT_NONE,
        FaultKind::OverVoltage => FAULT_OVER_VOLTAGE,
        FaultKind::Unknown => FAULT_UNKNOWN,
    }
}

fn contain(operation: impl FnOnce() -> Result<DecodeResult, jint>) -> DecodeResult {
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

unsafe fn references_alias(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    left: jobject,
    right: jobject,
) -> bool {
    unsafe { (functions.IsSameObject)(env, left, right) }
}

fn write_metadata(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    result: jintArray,
    values: [jint; 2],
) -> Result<(), ()> {
    unsafe { (functions.SetIntArrayRegion)(env, result, 0, 2, values.as_ptr()) };
    if unsafe { (functions.ExceptionCheck)(env) } {
        Err(())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: [[u8; 4]; 4] = [
        0x42C8_0000_u32.to_be_bytes(),
        0xC2C8_0000_u32.to_be_bytes(),
        0x4248_0000_u32.to_be_bytes(),
        0xC1A0_0000_u32.to_be_bytes(),
    ];

    #[test]
    fn encodes_exact_realtime_records() {
        let normal = run_decode(OPERATION_REALTIME, &realtime_payload(0), MAX_RECORD_BYTES);
        assert_eq!(normal.status, STATUS_OK);
        assert_eq!(normal.record_length, REALTIME_RECORD_BYTES);
        assert_eq!(&normal.record[..8], &[1, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(read_f64(&normal.record, 8), 25.0);
        assert_eq!(read_f64(&normal.record, 16), -5.5);
        assert_eq!(read_f64(&normal.record, 24), 12.34);
        assert_eq!(read_f64(&normal.record, 32), -2.5);
        assert_eq!(read_f64(&normal.record, 40), 0.5);
        assert_eq!(read_f64(&normal.record, 48), 50.4);
        assert_eq!(&normal.record[56..], &(-1_234_i32).to_be_bytes());

        let fault = run_decode(OPERATION_REALTIME, &realtime_payload(1), MAX_RECORD_BYTES);
        assert_eq!(&fault.record[..8], &[1, 1, 0, 1, 1, 0, 0, 0]);
        let unknown = run_decode(OPERATION_REALTIME, &realtime_payload(255), MAX_RECORD_BYTES);
        assert_eq!(&unknown.record[..8], &[1, 1, 0, 2, 255, 0, 0, 0]);
    }

    #[test]
    fn encodes_exact_current_limit_records() {
        let absent = run_decode(
            OPERATION_CURRENT_LIMITS,
            &limits_payload(None),
            MAX_RECORD_BYTES,
        );
        assert_eq!(absent.status, STATUS_OK);
        assert_eq!(absent.record_length, CURRENT_LIMITS_RECORD_BYTES);
        assert_eq!(&absent.record[..8], &[1, 2, 0, 0, 0, 0, 0, 0]);
        assert_eq!(read_f64(&absent.record, 8), 100.0);
        assert_eq!(read_f64(&absent.record, 16), -100.0);
        assert_eq!(read_f64(&absent.record, 24), 50.0);
        assert_eq!(read_f64(&absent.record, 32), -20.0);

        let present = run_decode(
            OPERATION_CURRENT_LIMITS,
            &limits_payload(Some(18)),
            MAX_RECORD_BYTES,
        );
        assert_eq!(&present.record[..8], &[1, 2, 1, 0, 0, 18, 0, 0]);
    }

    #[test]
    fn maps_decoder_operation_and_output_failures() {
        let unsupported = run_decode(OPERATION_REALTIME, &[14], MAX_RECORD_BYTES);
        assert_eq!(unsupported.status, STATUS_UNSUPPORTED_COMMAND);
        assert_eq!(unsupported.unsupported_command, 14);
        assert_eq!(unsupported.record_length, 0);
        assert!(unsupported.record.is_empty());

        assert_eq!(
            run_decode(OPERATION_REALTIME, &[4], MAX_RECORD_BYTES),
            DecodeResult::failure(STATUS_TRUNCATED_PAYLOAD)
        );
        assert_eq!(
            run_decode(99, &realtime_payload(0), MAX_RECORD_BYTES),
            DecodeResult::failure(STATUS_INVALID_ARGUMENT)
        );

        let small = run_decode(OPERATION_REALTIME, &realtime_payload(0), 8);
        assert_eq!(small.status, STATUS_OUTPUT_TOO_SMALL);
        assert_eq!(small.record_length, REALTIME_RECORD_BYTES);
        assert!(small.record.is_empty());
    }

    #[test]
    fn contains_panics_and_keeps_contract_numbers_stable() {
        let contained = contain(|| -> Result<DecodeResult, jint> { panic!("contained") });
        assert_eq!(contained, DecodeResult::failure(STATUS_INTERNAL_FAILURE));
        assert_eq!(
            contain_status(|| panic!("contained status")),
            STATUS_INTERNAL_FAILURE
        );
        assert_eq!(contain_abi(|| panic!("contained ABI")), 0);
        assert_eq!(ABI_VERSION, 1);
        assert_eq!(STATUS_OK, 0);
        assert_eq!(STATUS_INVALID_ARGUMENT, 1);
        assert_eq!(STATUS_UNSUPPORTED_COMMAND, 2);
        assert_eq!(STATUS_TRUNCATED_PAYLOAD, 3);
        assert_eq!(STATUS_OUTPUT_TOO_SMALL, 4);
        assert_eq!(STATUS_INTERNAL_FAILURE, 5);
        assert_eq!(MAX_PAYLOAD_BYTES, 4_096);
        assert_eq!(MAX_RECORD_BYTES, 60);
        assert_eq!(std::mem::size_of::<i32>(), 4);
    }

    #[test]
    fn exported_boundary_rejects_a_missing_environment() {
        // SAFETY: null is an explicitly supported rejection input.
        unsafe {
            assert_eq!(
                Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge_abiVersion(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ),
                0,
            );
            assert_eq!(
                Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge_decode(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    OPERATION_REALTIME,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ),
                STATUS_INVALID_ARGUMENT,
            );
        }
    }

    fn realtime_payload(fault: u8) -> Vec<u8> {
        let mut payload = vec![0_u8; 30];
        payload[0] = 4;
        payload[1..3].copy_from_slice(&250_i16.to_be_bytes());
        payload[3..5].copy_from_slice(&(-55_i16).to_be_bytes());
        payload[5..9].copy_from_slice(&1_234_i32.to_be_bytes());
        payload[9..13].copy_from_slice(&(-250_i32).to_be_bytes());
        payload[21..23].copy_from_slice(&500_i16.to_be_bytes());
        payload[23..27].copy_from_slice(&(-1_234_i32).to_be_bytes());
        payload[27..29].copy_from_slice(&504_i16.to_be_bytes());
        payload[29] = fault;
        payload
    }

    fn limits_payload(cells: Option<u8>) -> Vec<u8> {
        let mut payload = vec![0_u8; if cells.is_some() { 464 } else { 25 }];
        payload[0] = 14;
        payload[1..5].copy_from_slice(&0x57AD_8F53_u32.to_be_bytes());
        for (index, value) in LIMITS.into_iter().enumerate() {
            let start = 9 + index * 4;
            payload[start..start + 4].copy_from_slice(&value);
        }
        if let Some(value) = cells {
            payload[463] = value;
        }
        payload
    }

    fn read_f64(record: &[u8], offset: usize) -> f64 {
        f64::from_bits(u64::from_be_bytes(
            record[offset..offset + std::mem::size_of::<u64>()]
                .try_into()
                .expect("eight bytes"),
        ))
    }
}
