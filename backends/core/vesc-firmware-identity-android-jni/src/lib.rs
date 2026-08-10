//! Narrow Android JNI ABI for bounded VESC firmware identity decoding.
//!
//! Structural decoding remains in the safe `float-view-vesc-firmware-identity`
//! crate. This crate owns only bounded JNI validation and call-local copies.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_vesc_firmware_identity::{FirmwareIdentity, FirmwareIdentityError, decode};
use jni_sys::{JNIEnv, jbyte, jbyteArray, jint, jintArray, jobject, jsize};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Version of the complete hand-written Android identity JNI contract.
pub const ABI_VERSION: jint = 1;

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

/// Largest structurally valid `COMM_FW_VERSION` payload.
pub const MAX_PAYLOAD_BYTES: usize = 157;
/// Fixed bytes before the two replacement-decoded UTF-8 names.
pub const RECORD_HEADER_BYTES: usize = 26;
/// Worst-case UTF-8 bytes after replacement decoding one 64-byte name.
pub const MAX_ENCODED_NAME_BYTES: usize = 64 * 3;
/// Largest version-1 identity record.
pub const MAX_RECORD_BYTES: usize = RECORD_HEADER_BYTES + MAX_ENCODED_NAME_BYTES * 2;

const FLAG_FIRMWARE_NAME_PRESENT: u8 = 1;
const FLAG_HARDWARE_CRC_PRESENT: u8 = 2;

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
    contain_status(|| unsafe { invoke(env, input, output, result) })
}

unsafe fn invoke(
    env: *mut JNIEnv,
    input: jbyteArray,
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

    let decoded = run_decode(&copied_input, output_capacity);
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

fn run_decode(input: &[u8], output_capacity: usize) -> DecodeResult {
    contain(|| {
        let identity = match decode(input) {
            Ok(identity) => identity,
            Err(FirmwareIdentityError::UnsupportedCommand(command)) => {
                return Ok(DecodeResult {
                    status: STATUS_UNSUPPORTED_COMMAND,
                    record: Vec::new(),
                    record_length: 0,
                    unsupported_command: command,
                });
            }
            Err(FirmwareIdentityError::TruncatedPayload) => {
                return Ok(DecodeResult::failure(STATUS_TRUNCATED_PAYLOAD));
            }
        };
        let record = encode_record(&identity)?;
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

fn encode_record(identity: &FirmwareIdentity) -> Result<Vec<u8>, jint> {
    let hardware_name = identity.hardware_name.as_bytes();
    let firmware_name = identity
        .firmware_name
        .as_deref()
        .unwrap_or_default()
        .as_bytes();
    if hardware_name.len() > MAX_ENCODED_NAME_BYTES || firmware_name.len() > MAX_ENCODED_NAME_BYTES
    {
        return Err(STATUS_INTERNAL_FAILURE);
    }
    let hardware_length =
        u16::try_from(hardware_name.len()).map_err(|_| STATUS_INTERNAL_FAILURE)?;
    let firmware_length =
        u16::try_from(firmware_name.len()).map_err(|_| STATUS_INTERNAL_FAILURE)?;
    let mut flags = 0;
    if identity.firmware_name.is_some() {
        flags |= FLAG_FIRMWARE_NAME_PRESENT;
    }
    if identity.hardware_configuration_crc.is_some() {
        flags |= FLAG_HARDWARE_CRC_PRESENT;
    }

    let mut record =
        Vec::with_capacity(RECORD_HEADER_BYTES + hardware_name.len() + firmware_name.len());
    record.extend_from_slice(&[
        identity.major,
        identity.minor,
        identity.test_version,
        identity.hardware_type,
        identity.custom_configuration_count,
        flags,
    ]);
    record.extend_from_slice(&hardware_length.to_be_bytes());
    record.extend_from_slice(&firmware_length.to_be_bytes());
    record.extend_from_slice(&identity.controller_uuid);
    record.extend_from_slice(
        &identity
            .hardware_configuration_crc
            .unwrap_or_default()
            .to_be_bytes(),
    );
    record.extend_from_slice(hardware_name);
    record.extend_from_slice(firmware_name);
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

    const STOCK_BMS: [u8; 24] = [
        0, 6, 0, b'V', b'B', b'M', b'S', 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0, 0, 1, 1,
    ];

    #[test]
    fn decodes_identity_into_the_fixed_record() {
        let result = run_decode(&STOCK_BMS, MAX_RECORD_BYTES);
        assert_eq!(result.status, STATUS_OK);
        assert_eq!(result.record_length, RECORD_HEADER_BYTES + 4);
        assert_eq!(&result.record[..10], &[6, 0, 0, 1, 1, 0, 0, 4, 0, 0]);
        assert_eq!(
            &result.record[10..22],
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
        );
        assert_eq!(&result.record[22..26], &[0, 0, 0, 0]);
        assert_eq!(&result.record[26..], b"VBMS");
        assert_eq!(result.unsupported_command, 0);
    }

    #[test]
    fn maps_failures_and_reports_required_capacity_without_partial_record() {
        let unsupported = run_decode(&[37], MAX_RECORD_BYTES);
        assert_eq!(unsupported.status, STATUS_UNSUPPORTED_COMMAND);
        assert_eq!(unsupported.unsupported_command, 37);
        assert_eq!(unsupported.record_length, 0);
        assert!(unsupported.record.is_empty());

        assert_eq!(
            run_decode(&[0], MAX_RECORD_BYTES),
            DecodeResult::failure(STATUS_TRUNCATED_PAYLOAD)
        );

        let too_small = run_decode(&STOCK_BMS, 2);
        assert_eq!(too_small.status, STATUS_OUTPUT_TOO_SMALL);
        assert_eq!(too_small.record_length, RECORD_HEADER_BYTES + 4);
        assert!(too_small.record.is_empty());
    }

    #[test]
    fn replacement_utf8_and_optional_fields_have_bounded_representation() {
        let mut payload = vec![0, 7, 0, b'H', 0xff, 0];
        payload.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        payload.extend_from_slice(&[1, 9, 2, 7]);
        payload.extend_from_slice(&[1, 0, 1, 0]);
        payload.extend_from_slice(&[b'F', 0xfe, 0, 0x12, 0x34, 0x56, 0x78]);

        let result = run_decode(&payload, MAX_RECORD_BYTES);
        assert_eq!(result.status, STATUS_OK);
        assert_eq!(&result.record[..6], &[7, 0, 9, 2, 7, 3]);
        assert_eq!(&result.record[6..10], &[0, 4, 0, 4]);
        assert_eq!(&result.record[22..26], &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(&result.record[26..30], "H�".as_bytes());
        assert_eq!(&result.record[30..34], "F�".as_bytes());
    }

    #[test]
    fn contains_panics_and_keeps_numbers_stable() {
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
        assert_eq!(MAX_RECORD_BYTES, 410);
    }
}
