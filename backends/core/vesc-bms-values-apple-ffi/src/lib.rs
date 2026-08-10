//! Narrow caller-owned-buffer C ABI for VESC BMS values decoding.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_vesc_bms_values::{BmsValues, BmsValuesError, decode};
use std::mem::{align_of, size_of, size_of_val};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::{ptr, slice};

/// Version of the hand-written BMS values ABI and byte-record layout.
pub const ABI_VERSION: u32 = 1;
/// The operation completed successfully.
pub const STATUS_OK: u32 = 0;
/// A pointer, capacity, alignment, region, or expected-CAN contract is invalid.
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
/// The observed VESC command is not `COMM_BMS_GET_VALUES`.
pub const STATUS_UNSUPPORTED_COMMAND: u32 = 2;
/// The payload is truncated, oversized, or structurally invalid.
pub const STATUS_TRUNCATED_PAYLOAD: u32 = 3;
/// The result buffer is too small; the required length was returned.
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 4;
/// A panic was contained before it could cross C.
pub const STATUS_INTERNAL_FAILURE: u32 = 5;

/// Sentinel used when the caller has no expected reported CAN ID.
pub const ANY_EXPECTED_CAN_ID: u8 = u8::MAX;
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

/// Return the exact version of the hand-written BMS values ABI.
#[unsafe(no_mangle)]
pub extern "C" fn float_view_vesc_bms_values_abi_version() -> u32 {
    ABI_VERSION
}

/// Decode one strict BMS values payload into a caller-owned version-1 record.
///
/// # Safety
///
/// Non-empty input and output regions must be valid for their supplied byte
/// lengths. `output_length` must be valid and aligned for one `usize`, and
/// `unsupported_command` must be valid for one byte. All four regions must be
/// pairwise disjoint and remain valid for the call. A null byte pointer is
/// accepted only when its corresponding length or capacity is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_vesc_bms_values_decode(
    payload: *const u8,
    payload_length: usize,
    expected_can_id: u8,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> u32 {
    contain(|| {
        if payload_length > MAX_PAYLOAD_BYTES {
            return Err(STATUS_TRUNCATED_PAYLOAD);
        }
        let expected_can_id = match expected_can_id {
            0..=253 => Some(expected_can_id),
            ANY_EXPECTED_CAN_ID => None,
            _ => return Err(STATUS_INVALID_ARGUMENT),
        };
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
        let values = match decode(payload, expected_can_id) {
            Ok(values) => values,
            Err(BmsValuesError::UnsupportedCommand(command)) => {
                // SAFETY: validation established a writable byte output.
                unsafe { unsupported_command.write(command) };
                return Err(STATUS_UNSUPPORTED_COMMAND);
            }
            Err(BmsValuesError::TruncatedPayload) => return Err(STATUS_TRUNCATED_PAYLOAD),
        };
        let record = encode_record(&values)?;
        // SAFETY: validation established the bounded caller-owned output.
        unsafe { write_result(&record, output, output_capacity, output_length) }
    })
}

fn encode_record(values: &BmsValues<'_>) -> Result<Vec<u8>, u32> {
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
    use std::ptr::NonNull;

    #[test]
    fn decodes_exact_legacy_and_extended_records() {
        let legacy = payload(2, 1, None);
        let record = decode_record(&legacy, 42).expect("legacy record");
        assert_eq!(&record[..8], &[1, 0, 2, 1, 42, 0, 0, 0]);
        assert_eq!(
            record.len(),
            RECORD_HEADER_BYTES + FIXED_VALUE_BYTES + 16 + 2 + 8
        );
        assert_eq!(read_f64(&record, 8), 50.4);
        assert_eq!(read_f64(&record, 8 + 2 * 8), -12.345);
        assert_eq!(read_f64(&record, 8 + 16 * 8), 101_320.0);
        let cells_start = RECORD_HEADER_BYTES + FIXED_VALUE_BYTES;
        assert_eq!(read_f64(&record, cells_start), 4.0);
        assert_eq!(read_f64(&record, cells_start + 8), 4.001);
        assert_eq!(&record[cells_start + 16..cells_start + 18], &[0, 1]);
        assert_eq!(read_f64(&record, cells_start + 18), 20.0);

        let extended = payload(0, 0, Some((7, b"OK")));
        let record = decode_record(&extended, ANY_EXPECTED_CAN_ID).expect("extended record");
        assert_eq!(&record[..8], &[1, 1, 0, 0, 42, 7, 2, 0]);
        assert_eq!(&record[RECORD_HEADER_BYTES + FIXED_VALUE_BYTES..], b"OK");
    }

    #[test]
    fn accepts_exact_zero_and_maximum_collection_bounds() {
        let minimum = decode_record(&payload(0, 0, None), 42).expect("minimum");
        assert_eq!(minimum.len(), RECORD_HEADER_BYTES + FIXED_VALUE_BYTES);
        let maximum =
            decode_record(&payload(50, 50, Some((255, &[b'A'; 40]))), 42).expect("maximum");
        assert_eq!(maximum.len(), MAX_RECORD_BYTES);
        assert_eq!(&maximum[..8], &[1, 1, 50, 50, 42, 255, 40, 0]);
    }

    #[test]
    fn maps_semantic_failures_and_validates_expected_can() {
        let mut output = [0_u8; MAX_RECORD_BYTES];
        let mut output_length = 9;
        let mut unsupported = 9;
        assert_eq!(
            call(&[37], 42, &mut output, &mut output_length, &mut unsupported),
            STATUS_UNSUPPORTED_COMMAND
        );
        assert_eq!((output_length, unsupported), (0, 37));
        assert_eq!(
            call(&[96], 42, &mut output, &mut output_length, &mut unsupported),
            STATUS_TRUNCATED_PAYLOAD
        );
        assert_eq!((output_length, unsupported), (0, 0));
        assert_eq!(
            call(
                &payload(0, 0, None),
                254,
                &mut output,
                &mut output_length,
                &mut unsupported
            ),
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            call(
                &payload(0, 0, None),
                41,
                &mut output,
                &mut output_length,
                &mut unsupported
            ),
            STATUS_TRUNCATED_PAYLOAD
        );
    }

    #[test]
    fn validates_caller_regions_and_reports_required_output() {
        let source = payload(0, 0, None);
        let mut small = [0_u8; 2];
        let mut output_length = 0;
        let mut unsupported = 0;
        assert_eq!(
            call(
                &source,
                42,
                &mut small,
                &mut output_length,
                &mut unsupported
            ),
            STATUS_OUTPUT_TOO_SMALL
        );
        assert_eq!(output_length, RECORD_HEADER_BYTES + FIXED_VALUE_BYTES);

        // SAFETY: the deliberately null input is rejected before access.
        assert_eq!(
            unsafe {
                float_view_vesc_bms_values_decode(
                    ptr::null(),
                    1,
                    42,
                    small.as_mut_ptr(),
                    small.len(),
                    &mut output_length,
                    &mut unsupported,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        let mut overlap = [0_u8; 160];
        let overlap_length = overlap.as_mut_ptr().cast::<usize>();
        // SAFETY: the deliberately overlapping output regions are rejected.
        assert_eq!(
            unsafe {
                float_view_vesc_bms_values_decode(
                    source.as_ptr(),
                    source.len(),
                    42,
                    overlap.as_mut_ptr(),
                    overlap.len(),
                    overlap_length,
                    &mut unsupported,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: oversized input is rejected before the dangling pointer is read.
        assert_eq!(
            unsafe {
                float_view_vesc_bms_values_decode(
                    NonNull::<u8>::dangling().as_ptr(),
                    MAX_PAYLOAD_BYTES + 1,
                    42,
                    ptr::null_mut(),
                    0,
                    &mut output_length,
                    &mut unsupported,
                )
            },
            STATUS_TRUNCATED_PAYLOAD
        );
    }

    #[test]
    fn contains_panics() {
        assert_eq!(
            contain(|| -> Result<(), u32> { panic!("test-only contained panic") }),
            STATUS_INTERNAL_FAILURE
        );
    }

    fn call(
        payload: &[u8],
        expected_can_id: u8,
        output: &mut [u8],
        output_length: &mut usize,
        unsupported: &mut u8,
    ) -> u32 {
        // SAFETY: all regions are valid and pairwise disjoint.
        unsafe {
            float_view_vesc_bms_values_decode(
                payload.as_ptr(),
                payload.len(),
                expected_can_id,
                output.as_mut_ptr(),
                output.len(),
                output_length,
                unsupported,
            )
        }
    }

    fn decode_record(payload: &[u8], expected_can_id: u8) -> Result<Vec<u8>, u32> {
        let mut output = [0_u8; MAX_RECORD_BYTES];
        let mut output_length = 0;
        let mut unsupported = 0;
        let status = call(
            payload,
            expected_can_id,
            &mut output,
            &mut output_length,
            &mut unsupported,
        );
        if status == STATUS_OK {
            Ok(output[..output_length].to_vec())
        } else {
            Err(status)
        }
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
