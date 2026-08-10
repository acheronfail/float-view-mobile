//! Narrow Android JNI ABI for bounded VESC firmware identity decoding.
//!
//! Structural decoding remains in the safe `float-view-vesc-firmware-identity`
//! crate. This crate owns only bounded JNI validation and call-local copies.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_android_jni_support::{
    Environment, JNIEnv, contain, jbyteArray, jint, jintArray, jobject, with_environment,
};
use float_view_vesc_firmware_identity::{FirmwareIdentity, FirmwareIdentityError, decode};

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

    let decoded = run_decode(&copied_input, output_capacity);
    if decoded.status == STATUS_OK
        && environment
            .write_bytes(output, output_capacity, &decoded.record)
            .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    let metadata = [
        decoded.record_length as jint,
        jint::from(decoded.unsupported_command),
    ];
    if environment.write_ints_exact(result, 2, &metadata).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    decoded.status
}

fn run_decode(input: &[u8], output_capacity: usize) -> DecodeResult {
    let result = contain(Err(STATUS_INTERNAL_FAILURE), || {
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
    });
    result.unwrap_or_else(DecodeResult::failure)
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
