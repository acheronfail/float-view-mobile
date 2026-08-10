//! Narrow Android JNI ABI for bounded VESC BMS values decoding.
//!
//! Structural decoding remains in the safe `float-view-vesc-bms-values`
//! crate. This crate owns only bounded JNI validation and call-local copies.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_vesc_bms_values::{BmsValues, BmsValuesError, decode};
use jni_sys::{JNIEnv, jbyte, jbyteArray, jint, jintArray, jobject, jsize};
use std::mem::{size_of, size_of_val};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Version of the complete hand-written Android BMS values JNI contract.
pub const ABI_VERSION: jint = 1;

/// Successful operation.
pub const STATUS_OK: jint = 0;
/// A JNI environment, reference, array shape, capacity, alias, or expected CAN was invalid.
pub const STATUS_INVALID_ARGUMENT: jint = 1;
/// The observed VESC command is not `COMM_BMS_GET_VALUES`.
pub const STATUS_UNSUPPORTED_COMMAND: jint = 2;
/// The payload is truncated, oversized, or structurally invalid.
pub const STATUS_TRUNCATED_PAYLOAD: jint = 3;
/// The output is too small; the required record length was returned.
pub const STATUS_OUTPUT_TOO_SMALL: jint = 4;
/// A panic or unexpected JNI failure was contained at the boundary.
pub const STATUS_INTERNAL_FAILURE: jint = 5;

/// Sentinel used when the caller has no expected reported CAN ID.
pub const ANY_EXPECTED_CAN_ID: jint = 255;
/// Largest structurally valid command-96 payload.
pub const MAX_PAYLOAD_BYTES: usize = 350;
/// Fixed bytes before scalar values and counted collections.
pub const RECORD_HEADER_BYTES: usize = 8;
/// Number of fixed scalar values encoded in every result.
pub const FIXED_VALUE_COUNT: usize = 17;
/// Bytes occupied by the fixed scalar values.
pub const FIXED_VALUE_BYTES: usize = FIXED_VALUE_COUNT * size_of::<f64>();
/// Largest version-1 result record.
pub const MAX_RECORD_BYTES: usize = RECORD_HEADER_BYTES
    + FIXED_VALUE_BYTES
    + 50 * size_of::<f64>()
    + 50
    + 50 * size_of::<f64>()
    + 40;

const FLAG_EXTENDED_TAIL_PRESENT: u8 = 1;

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

/// Return the exact version of the hand-written BMS values JNI contract.
///
/// # Safety
///
/// `env` must be null or a JNI environment supplied by the JVM for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescBmsValuesBridge_abiVersion(
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

/// Decode one bounded BMS values payload into caller-owned primitive arrays.
///
/// `expected_can_id` accepts `0...253` or `255` for no expected route.
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
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustVescBmsValuesBridge_decode(
    env: *mut JNIEnv,
    _receiver: jobject,
    input: jbyteArray,
    expected_can_id: jint,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain_status(|| unsafe { invoke(env, input, expected_can_id, output, result) })
}

unsafe fn invoke(
    env: *mut JNIEnv,
    input: jbyteArray,
    expected_can_id: jint,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    let Some(functions) = (unsafe { environment(env) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if unsafe { (functions.ExceptionCheck)(env) }
        || input.is_null()
        || output.is_null()
        || result.is_null()
    {
        return STATUS_INVALID_ARGUMENT;
    }
    let expected_can_id = match expected_can_id {
        0..=253 => Some(expected_can_id as u8),
        ANY_EXPECTED_CAN_ID => None,
        _ => return STATUS_INVALID_ARGUMENT,
    };

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

    let decoded = run_decode(&copied_input, expected_can_id, output_capacity);
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
    let metadata = [
        decoded.record_length as jint,
        jint::from(decoded.unsupported_command),
    ];
    if write_metadata(env, functions, result, metadata).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    decoded.status
}

fn run_decode(input: &[u8], expected_can_id: Option<u8>, output_capacity: usize) -> DecodeResult {
    contain(|| {
        let values = match decode(input, expected_can_id) {
            Ok(values) => values,
            Err(BmsValuesError::UnsupportedCommand(command)) => {
                return Ok(DecodeResult {
                    status: STATUS_UNSUPPORTED_COMMAND,
                    record: Vec::new(),
                    record_length: 0,
                    unsupported_command: command,
                });
            }
            Err(BmsValuesError::TruncatedPayload) => {
                return Ok(DecodeResult::failure(STATUS_TRUNCATED_PAYLOAD));
            }
        };
        let record = encode_record(&values)?;
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

fn encode_record(values: &BmsValues<'_>) -> Result<Vec<u8>, jint> {
    let cells = values.cell_voltages_v();
    let balancing = values.balancing();
    let temperatures = values.temperatures_c();
    let status = values.status.unwrap_or_default().as_bytes();
    if cells.len() > 50
        || balancing.len() != cells.len()
        || temperatures.len() > 50
        || status.len() > 40
        || values.data_version.is_some() != values.status.is_some()
    {
        return Err(STATUS_INTERNAL_FAILURE);
    }

    let mut record = Vec::with_capacity(
        RECORD_HEADER_BYTES
            + FIXED_VALUE_BYTES
            + size_of_val(cells)
            + balancing.len()
            + size_of_val(temperatures)
            + status.len(),
    );
    let flags = if values.data_version.is_some() {
        FLAG_EXTENDED_TAIL_PRESENT
    } else {
        0
    };
    record.extend_from_slice(&[
        ABI_VERSION as u8,
        flags,
        u8::try_from(cells.len()).map_err(|_| STATUS_INTERNAL_FAILURE)?,
        u8::try_from(temperatures.len()).map_err(|_| STATUS_INTERNAL_FAILURE)?,
        values.reported_can_id,
        values.data_version.unwrap_or_default(),
        u8::try_from(status.len()).map_err(|_| STATUS_INTERNAL_FAILURE)?,
        0,
    ]);
    for value in [
        values.total_voltage_v,
        values.charge_voltage_v,
        values.input_current_a,
        values.input_ic_current_a,
        values.session_amp_hours_ah,
        values.session_watt_hours_wh,
        values.ic_temperature_c,
        values.humidity_sensor_temperature_c,
        values.humidity_percent,
        values.highest_cell_temperature_c,
        values.state_of_charge,
        values.state_of_health,
        values.lifetime_charge_amp_hours_ah,
        values.lifetime_charge_watt_hours_wh,
        values.lifetime_discharge_amp_hours_ah,
        values.lifetime_discharge_watt_hours_wh,
        values.pressure_pa,
    ] {
        record.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    for value in cells {
        record.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    record.extend(balancing.iter().map(|value| u8::from(*value)));
    for value in temperatures {
        record.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    record.extend_from_slice(status);
    debug_assert!(record.len() <= MAX_RECORD_BYTES);
    Ok(record)
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

    #[test]
    fn decodes_exact_legacy_and_extended_records() {
        let legacy = run_decode(&payload(2, 1, None), Some(42), MAX_RECORD_BYTES);
        assert_eq!(legacy.status, STATUS_OK);
        assert_eq!(&legacy.record[..8], &[1, 0, 2, 1, 42, 0, 0, 0]);
        assert_eq!(
            legacy.record_length,
            RECORD_HEADER_BYTES + FIXED_VALUE_BYTES + 16 + 2 + 8
        );
        assert_eq!(read_f64(&legacy.record, 8), 50.4);
        assert_eq!(read_f64(&legacy.record, 8 + 2 * 8), -12.345);
        assert_eq!(read_f64(&legacy.record, 8 + 16 * 8), 101_320.0);
        let cells_start = RECORD_HEADER_BYTES + FIXED_VALUE_BYTES;
        assert_eq!(read_f64(&legacy.record, cells_start), 4.0);
        assert_eq!(read_f64(&legacy.record, cells_start + 8), 4.001);
        assert_eq!(&legacy.record[cells_start + 16..cells_start + 18], &[0, 1]);
        assert_eq!(read_f64(&legacy.record, cells_start + 18), 20.0);

        let extended = run_decode(&payload(0, 0, Some((7, b"OK"))), None, MAX_RECORD_BYTES);
        assert_eq!(extended.status, STATUS_OK);
        assert_eq!(&extended.record[..8], &[1, 1, 0, 0, 42, 7, 2, 0]);
        assert_eq!(
            &extended.record[RECORD_HEADER_BYTES + FIXED_VALUE_BYTES..],
            b"OK"
        );
    }

    #[test]
    fn accepts_exact_zero_and_maximum_collection_bounds() {
        let minimum = run_decode(&payload(0, 0, None), Some(42), MAX_RECORD_BYTES);
        assert_eq!(minimum.status, STATUS_OK);
        assert_eq!(
            minimum.record_length,
            RECORD_HEADER_BYTES + FIXED_VALUE_BYTES
        );

        let maximum = run_decode(
            &payload(50, 50, Some((255, &[b'A'; 40]))),
            Some(42),
            MAX_RECORD_BYTES,
        );
        assert_eq!(maximum.status, STATUS_OK);
        assert_eq!(maximum.record_length, MAX_RECORD_BYTES);
        assert_eq!(&maximum.record[..8], &[1, 1, 50, 50, 42, 255, 40, 0]);
    }

    #[test]
    fn maps_semantic_failures_and_required_capacity() {
        let unsupported = run_decode(&[37], Some(42), MAX_RECORD_BYTES);
        assert_eq!(unsupported.status, STATUS_UNSUPPORTED_COMMAND);
        assert_eq!(unsupported.unsupported_command, 37);
        assert_eq!(unsupported.record_length, 0);
        assert!(unsupported.record.is_empty());

        assert_eq!(
            run_decode(&[96], Some(42), MAX_RECORD_BYTES),
            DecodeResult::failure(STATUS_TRUNCATED_PAYLOAD)
        );
        assert_eq!(
            run_decode(&payload(0, 0, None), Some(41), MAX_RECORD_BYTES),
            DecodeResult::failure(STATUS_TRUNCATED_PAYLOAD)
        );

        let too_small = run_decode(&payload(0, 0, None), Some(42), 2);
        assert_eq!(too_small.status, STATUS_OUTPUT_TOO_SMALL);
        assert_eq!(
            too_small.record_length,
            RECORD_HEADER_BYTES + FIXED_VALUE_BYTES
        );
        assert!(too_small.record.is_empty());
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
        assert_eq!(ANY_EXPECTED_CAN_ID, 255);
        assert_eq!(MAX_PAYLOAD_BYTES, 350);
        assert_eq!(MAX_RECORD_BYTES, 1_034);
    }

    fn read_f64(record: &[u8], offset: usize) -> f64 {
        f64::from_bits(u64::from_be_bytes(
            record[offset..offset + 8].try_into().expect("eight bytes"),
        ))
    }

    fn payload(cells: usize, temperatures: usize, extended: Option<(u8, &[u8])>) -> Vec<u8> {
        let mut payload = vec![96];
        for value in [
            50_400_000_i32,
            54_600_000,
            -12_345_000,
            -12_300_000,
            1_250,
            62_500,
        ] {
            payload.extend_from_slice(&value.to_be_bytes());
        }
        payload.push(cells as u8);
        for index in 0..cells {
            payload.extend_from_slice(&(4_000_i16 + index as i16).to_be_bytes());
        }
        payload.extend((0..cells).map(|index| (index % 2) as u8));
        payload.push(temperatures as u8);
        for index in 0..temperatures {
            payload.extend_from_slice(&(2_000_i16 + index as i16).to_be_bytes());
        }
        for value in [2_750_i16, 2_475, 5_250, 2_925, 875, 950] {
            payload.extend_from_slice(&value.to_be_bytes());
        }
        payload.push(42);
        for raw in [0x4280_0000_u32, 0x4480_0000, 0x4200_0000, 0x4400_0000] {
            payload.extend_from_slice(&raw.to_be_bytes());
        }
        payload.extend_from_slice(&10_132_i16.to_be_bytes());
        if let Some((version, status)) = extended {
            payload.push(version);
            payload.extend_from_slice(status);
            payload.push(0);
        }
        payload
    }
}
