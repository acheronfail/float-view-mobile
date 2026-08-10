//! Narrow caller-owned-buffer C ABI for VESC controller data decoding.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_vesc_controller_data::{
    ControllerDataError, CurrentLimits, FaultKind, RealtimeValues, decode_current_limits,
    decode_realtime_values,
};
use std::mem::{align_of, size_of};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::{ptr, slice};

pub const ABI_VERSION: u32 = 1;
pub const OPERATION_REALTIME: u32 = 1;
pub const OPERATION_CURRENT_LIMITS: u32 = 2;
pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
pub const STATUS_UNSUPPORTED_COMMAND: u32 = 2;
pub const STATUS_TRUNCATED_PAYLOAD: u32 = 3;
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 4;
pub const STATUS_INTERNAL_FAILURE: u32 = 5;
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

#[unsafe(no_mangle)]
pub extern "C" fn float_view_vesc_controller_data_abi_version() -> u32 {
    ABI_VERSION
}

/// Decode one controller-data payload into a caller-owned version-1 record.
///
/// # Safety
///
/// Non-empty input and output regions must be valid for their supplied byte
/// lengths. `output_length` must be valid and aligned for one `usize`, and
/// `unsupported_command` must be valid for one byte. All regions must be
/// pairwise disjoint and remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_vesc_controller_data_decode(
    operation: u32,
    payload: *const u8,
    payload_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> u32 {
    contain(|| {
        if !matches!(operation, OPERATION_REALTIME | OPERATION_CURRENT_LIMITS) {
            return Err(STATUS_INVALID_ARGUMENT);
        }
        if payload_length > MAX_PAYLOAD_BYTES {
            return Err(STATUS_TRUNCATED_PAYLOAD);
        }
        validate_regions(
            payload,
            payload_length,
            output,
            output_capacity,
            output_length,
            unsupported_command,
        )?;
        // SAFETY: validation established distinct writable scalar outputs.
        unsafe {
            output_length.write(0);
            unsupported_command.write(0);
        }
        // SAFETY: validation established a bounded readable region.
        let payload = unsafe { input_slice(payload, payload_length) };
        let (record, length) = match operation {
            OPERATION_REALTIME => match decode_realtime_values(payload) {
                Ok(values) => encode_realtime(values)?,
                Err(error) => return map_decode_error(error, unsupported_command),
            },
            OPERATION_CURRENT_LIMITS => match decode_current_limits(payload) {
                Ok(values) => encode_current_limits(values)?,
                Err(error) => return map_decode_error(error, unsupported_command),
            },
            _ => unreachable!(),
        };
        // SAFETY: validation established the bounded caller-owned output.
        unsafe { write_result(&record[..length], output, output_capacity, output_length) }
    })
}

fn map_decode_error(error: ControllerDataError, unsupported_command: *mut u8) -> Result<(), u32> {
    match error {
        ControllerDataError::UnsupportedCommand(command) => {
            // SAFETY: the exported function validates this writable byte first.
            unsafe { unsupported_command.write(command) };
            Err(STATUS_UNSUPPORTED_COMMAND)
        }
        ControllerDataError::TruncatedPayload => Err(STATUS_TRUNCATED_PAYLOAD),
    }
}

fn encode_realtime(values: RealtimeValues) -> Result<([u8; MAX_RECORD_BYTES], usize), u32> {
    let mut record = [0_u8; MAX_RECORD_BYTES];
    record[..RECORD_HEADER_BYTES].copy_from_slice(&[
        ABI_VERSION as u8,
        RESULT_REALTIME,
        0,
        fault_code(values.fault),
        values.fault_code,
        0,
        0,
        0,
    ]);
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
    let mut cursor = RECORD_HEADER_BYTES;
    for value in scalars {
        write_f64(&mut record, &mut cursor, value);
    }
    record[cursor..cursor + size_of::<i32>()].copy_from_slice(&values.erpm.to_be_bytes());
    cursor += size_of::<i32>();
    debug_assert_eq!(cursor, REALTIME_RECORD_BYTES);
    Ok((record, cursor))
}

fn encode_current_limits(values: CurrentLimits) -> Result<([u8; MAX_RECORD_BYTES], usize), u32> {
    let mut record = [0_u8; MAX_RECORD_BYTES];
    let (flags, cells) = values
        .battery_series_cells
        .map_or((0, 0), |value| (FLAG_SERIES_CELLS_PRESENT, value));
    if flags != 0 && cells == 0 {
        return Err(STATUS_INTERNAL_FAILURE);
    }
    record[..RECORD_HEADER_BYTES].copy_from_slice(&[
        ABI_VERSION as u8,
        RESULT_CURRENT_LIMITS,
        flags,
        0,
        0,
        cells,
        0,
        0,
    ]);
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
    let mut cursor = RECORD_HEADER_BYTES;
    for value in scalars {
        write_f64(&mut record, &mut cursor, value);
    }
    debug_assert_eq!(cursor, CURRENT_LIMITS_RECORD_BYTES);
    Ok((record, cursor))
}

const fn fault_code(fault: FaultKind) -> u8 {
    match fault {
        FaultKind::None => FAULT_NONE,
        FaultKind::OverVoltage => FAULT_OVER_VOLTAGE,
        FaultKind::Unknown => FAULT_UNKNOWN,
    }
}

fn write_f64(record: &mut [u8], cursor: &mut usize, value: f64) {
    let end = *cursor + size_of::<u64>();
    record[*cursor..end].copy_from_slice(&value.to_bits().to_be_bytes());
    *cursor = end;
}

fn contain(operation: impl FnOnce() -> Result<(), u32>) -> u32 {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => STATUS_OK,
        Ok(Err(status)) => status,
        Err(_) => STATUS_INTERNAL_FAILURE,
    }
}

fn validate_regions(
    input: *const u8,
    input_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> Result<(), u32> {
    if output_length.is_null()
        || unsupported_command.is_null()
        || !(output_length as usize).is_multiple_of(align_of::<usize>())
        || (input.is_null() && input_length != 0)
        || (output.is_null() && output_capacity != 0)
        || output_capacity > MAX_RECORD_BYTES
    {
        return Err(STATUS_INVALID_ARGUMENT);
    }
    let regions = [
        address_region(input as usize, input_length)?,
        address_region(output as usize, output_capacity)?,
        address_region(output_length as usize, size_of::<usize>())?,
        address_region(unsupported_command as usize, size_of::<u8>())?,
    ];
    for left in 0..regions.len() {
        for right in left + 1..regions.len() {
            if regions_overlap(regions[left], regions[right]) {
                return Err(STATUS_INVALID_ARGUMENT);
            }
        }
    }
    Ok(())
}

fn address_region(start: usize, length: usize) -> Result<(usize, usize), u32> {
    if length == 0 {
        return Ok((start, start));
    }
    Ok((
        start,
        start.checked_add(length).ok_or(STATUS_INVALID_ARGUMENT)?,
    ))
}

fn regions_overlap(left: (usize, usize), right: (usize, usize)) -> bool {
    left.0 != left.1 && right.0 != right.1 && left.0 < right.1 && right.0 < left.1
}

unsafe fn input_slice<'a>(input: *const u8, input_length: usize) -> &'a [u8] {
    if input_length == 0 {
        &[]
    } else {
        // SAFETY: the exported caller contract and validation establish this region.
        unsafe { slice::from_raw_parts(input, input_length) }
    }
}

unsafe fn write_result(
    bytes: &[u8],
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> Result<(), u32> {
    // SAFETY: validation established a writable aligned length field.
    unsafe { output_length.write(bytes.len()) };
    if output_capacity < bytes.len() {
        return Err(STATUS_OUTPUT_TOO_SMALL);
    }
    if !bytes.is_empty() {
        // SAFETY: validation and the capacity check establish the output region.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len()) };
    }
    Ok(())
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
    fn encodes_exact_realtime_record() {
        let payload = realtime_payload(1);
        let record = decode_record(OPERATION_REALTIME, &payload).unwrap();
        assert_eq!(record.len(), REALTIME_RECORD_BYTES);
        assert_eq!(&record[..8], &[1, 1, 0, 1, 1, 0, 0, 0]);
        assert_eq!(read_f64(&record, 8), 25.0);
        assert_eq!(read_f64(&record, 16), -5.5);
        assert_eq!(read_f64(&record, 24), 12.34);
        assert_eq!(read_f64(&record, 32), -2.5);
        assert_eq!(read_f64(&record, 40), 0.5);
        assert_eq!(read_f64(&record, 48), 50.4);
        assert_eq!(&record[56..60], &(-1_234_i32).to_be_bytes());
    }

    #[test]
    fn encodes_exact_current_limit_records() {
        let absent = decode_record(OPERATION_CURRENT_LIMITS, &limits_payload(None)).unwrap();
        assert_eq!(absent.len(), CURRENT_LIMITS_RECORD_BYTES);
        assert_eq!(&absent[..8], &[1, 2, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            (read_f64(&absent, 8), read_f64(&absent, 16)),
            (100.0, -100.0)
        );
        assert_eq!(
            (read_f64(&absent, 24), read_f64(&absent, 32)),
            (50.0, -20.0)
        );

        let present = decode_record(OPERATION_CURRENT_LIMITS, &limits_payload(Some(18))).unwrap();
        assert_eq!(&present[..8], &[1, 2, 1, 0, 0, 18, 0, 0]);
    }

    #[test]
    fn maps_decoder_and_output_failures() {
        let mut output = [0_u8; MAX_RECORD_BYTES];
        let mut length = 9;
        let mut unsupported = 9;
        assert_eq!(
            call(
                OPERATION_REALTIME,
                &[14],
                &mut output,
                &mut length,
                &mut unsupported
            ),
            STATUS_UNSUPPORTED_COMMAND
        );
        assert_eq!((length, unsupported), (0, 14));
        assert_eq!(
            call(
                OPERATION_REALTIME,
                &[4],
                &mut output,
                &mut length,
                &mut unsupported
            ),
            STATUS_TRUNCATED_PAYLOAD
        );
        assert_eq!((length, unsupported), (0, 0));
        let mut small = [0_u8; 8];
        assert_eq!(
            call(
                OPERATION_REALTIME,
                &realtime_payload(0),
                &mut small,
                &mut length,
                &mut unsupported
            ),
            STATUS_OUTPUT_TOO_SMALL
        );
        assert_eq!(length, REALTIME_RECORD_BYTES);
    }

    #[test]
    fn rejects_invalid_operations_regions_and_bounds() {
        let payload = realtime_payload(0);
        let mut output = [0_u8; MAX_RECORD_BYTES];
        let mut length = 0_usize;
        let mut unsupported = 0_u8;
        assert_eq!(
            call(99, &payload, &mut output, &mut length, &mut unsupported),
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: invalid contracts are intentionally supplied and rejected before dereference.
        unsafe {
            assert_eq!(
                float_view_vesc_controller_data_decode(
                    OPERATION_REALTIME,
                    ptr::null(),
                    1,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut length,
                    &mut unsupported
                ),
                STATUS_INVALID_ARGUMENT
            );
            assert_eq!(
                float_view_vesc_controller_data_decode(
                    OPERATION_REALTIME,
                    payload.as_ptr(),
                    MAX_PAYLOAD_BYTES + 1,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut length,
                    &mut unsupported
                ),
                STATUS_TRUNCATED_PAYLOAD
            );
            assert_eq!(
                float_view_vesc_controller_data_decode(
                    OPERATION_REALTIME,
                    payload.as_ptr(),
                    payload.len(),
                    output.as_mut_ptr(),
                    MAX_RECORD_BYTES + 1,
                    &mut length,
                    &mut unsupported
                ),
                STATUS_INVALID_ARGUMENT
            );
            assert_eq!(
                float_view_vesc_controller_data_decode(
                    OPERATION_REALTIME,
                    payload.as_ptr(),
                    payload.len(),
                    output.as_mut_ptr(),
                    output.len(),
                    output.as_mut_ptr().cast(),
                    &mut unsupported
                ),
                STATUS_INVALID_ARGUMENT
            );
            assert_eq!(
                float_view_vesc_controller_data_decode(
                    OPERATION_REALTIME,
                    usize::MAX as *const u8,
                    2,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut length,
                    &mut unsupported
                ),
                STATUS_INVALID_ARGUMENT
            );
        }
    }

    #[test]
    fn contains_panics() {
        assert_eq!(
            contain(|| -> Result<(), u32> { panic!("contained") }),
            STATUS_INTERNAL_FAILURE
        );
    }

    fn decode_record(operation: u32, payload: &[u8]) -> Result<Vec<u8>, u32> {
        let mut output = [0_u8; MAX_RECORD_BYTES];
        let mut length = 0;
        let mut unsupported = 0;
        let status = call(
            operation,
            payload,
            &mut output,
            &mut length,
            &mut unsupported,
        );
        if status == STATUS_OK {
            Ok(output[..length].to_vec())
        } else {
            Err(status)
        }
    }

    fn call(
        operation: u32,
        payload: &[u8],
        output: &mut [u8],
        output_length: &mut usize,
        unsupported: &mut u8,
    ) -> u32 {
        // SAFETY: all supplied regions are valid and distinct.
        unsafe {
            float_view_vesc_controller_data_decode(
                operation,
                payload.as_ptr(),
                payload.len(),
                output.as_mut_ptr(),
                output.len(),
                output_length,
                unsupported,
            )
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
            record[offset..offset + 8].try_into().unwrap(),
        ))
    }
}
