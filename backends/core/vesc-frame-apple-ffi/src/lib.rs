//! Narrow Apple C ABI for the bounded VESC frame codec.
//!
//! Framing behavior remains in the safe `float-view-vesc-frame` crate. This
//! crate owns only bounded C representation validation and copying.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_vesc_frame::{FrameError, MAX_PAYLOAD_BYTES, decode, encode};
use std::mem::{align_of, size_of};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;

/// Version of the complete hand-written C contract.
pub const ABI_VERSION: u32 = 1;

/// Successful operation.
pub const STATUS_OK: u32 = 0;
/// A pointer, alignment, capacity, or overlap contract was invalid.
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
/// The input exceeds the product payload or exact-frame bound.
pub const STATUS_INPUT_TOO_LARGE: u32 = 2;
/// The exact frame has an invalid header, length, footer, or trailing bytes.
pub const STATUS_INVALID_FRAME: u32 = 3;
/// The exact frame payload does not match its CRC-16.
pub const STATUS_INVALID_CRC: u32 = 4;
/// The output buffer is too small; the required length was returned.
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 5;
/// A panic was contained before it could cross the C boundary.
pub const STATUS_INTERNAL_FAILURE: u32 = 6;

/// Largest encoded frame: long header, maximum payload, CRC, and footer.
pub const MAX_FRAME_BYTES: usize = MAX_PAYLOAD_BYTES + 6;

/// Return the exact version of the hand-written ABI.
#[unsafe(no_mangle)]
pub extern "C" fn float_view_vesc_frame_abi_version() -> u32 {
    ABI_VERSION
}

/// Encode one bounded payload into a caller-owned VESC frame buffer.
///
/// # Safety
///
/// Non-empty input and output regions must be valid for their supplied byte
/// lengths. `output_length` must be valid and aligned for one `usize`. All
/// three regions must be disjoint and remain valid for the duration of the
/// call. A null byte pointer is accepted only when its corresponding length is
/// zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_vesc_frame_encode(
    payload: *const u8,
    payload_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain(|| {
        if payload_length > MAX_PAYLOAD_BYTES {
            return Err(STATUS_INPUT_TOO_LARGE);
        }
        validate_regions(
            payload,
            payload_length,
            output,
            output_capacity,
            MAX_FRAME_BYTES,
            output_length,
        )?;
        // SAFETY: `validate_regions` checked the pointer contract and bound.
        let payload = unsafe { input_slice(payload, payload_length) };
        let frame = encode(payload).map_err(frame_status)?;
        // SAFETY: all writable regions were checked above and the frame is bounded.
        unsafe { write_result(&frame, output, output_capacity, output_length) }
    })
}

/// Decode exactly one bounded VESC frame into a caller-owned payload buffer.
///
/// # Safety
///
/// Non-empty input and output regions must be valid for their supplied byte
/// lengths. `output_length` must be valid and aligned for one `usize`. All
/// three regions must be disjoint and remain valid for the duration of the
/// call. A null byte pointer is accepted only when its corresponding length is
/// zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_vesc_frame_decode_exact(
    frame: *const u8,
    frame_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> u32 {
    contain(|| {
        if frame_length > MAX_FRAME_BYTES {
            return Err(STATUS_INPUT_TOO_LARGE);
        }
        validate_regions(
            frame,
            frame_length,
            output,
            output_capacity,
            MAX_PAYLOAD_BYTES,
            output_length,
        )?;
        // SAFETY: `validate_regions` checked the pointer contract and bound.
        let frame = unsafe { input_slice(frame, frame_length) };
        let payload = decode(frame).map_err(frame_status)?;
        // SAFETY: all writable regions were checked above and the payload is bounded.
        unsafe { write_result(&payload, output, output_capacity, output_length) }
    })
}

fn contain(operation: impl FnOnce() -> Result<(), u32>) -> u32 {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => STATUS_OK,
        Ok(Err(status)) => status,
        Err(_) => STATUS_INTERNAL_FAILURE,
    }
}

fn frame_status(error: FrameError) -> u32 {
    match error {
        FrameError::InvalidFrame => STATUS_INVALID_FRAME,
        FrameError::InvalidCrc => STATUS_INVALID_CRC,
        FrameError::PayloadTooLarge => STATUS_INPUT_TOO_LARGE,
    }
}

fn validate_regions(
    input: *const u8,
    input_length: usize,
    output: *mut u8,
    output_capacity: usize,
    maximum_output_capacity: usize,
    output_length: *mut usize,
) -> Result<(), u32> {
    if output_length.is_null()
        || !(output_length as usize).is_multiple_of(align_of::<usize>())
        || (input.is_null() && input_length != 0)
        || (output.is_null() && output_capacity != 0)
        || output_capacity > maximum_output_capacity
    {
        return Err(STATUS_INVALID_ARGUMENT);
    }

    let input_region = address_region(input as usize, input_length)?;
    let output_region = address_region(output as usize, output_capacity)?;
    let length_region = address_region(output_length as usize, size_of::<usize>())?;
    if regions_overlap(input_region, output_region)
        || regions_overlap(input_region, length_region)
        || regions_overlap(output_region, length_region)
    {
        return Err(STATUS_INVALID_ARGUMENT);
    }

    // SAFETY: the caller guarantees a valid aligned `output_length`; null,
    // alignment, integer range, and declared-region overlap were checked above.
    unsafe { output_length.write(0) };
    Ok(())
}

fn address_region(start: usize, length: usize) -> Result<(usize, usize), u32> {
    if length == 0 {
        return Ok((start, start));
    }
    let end = start.checked_add(length).ok_or(STATUS_INVALID_ARGUMENT)?;
    Ok((start, end))
}

fn regions_overlap(left: (usize, usize), right: (usize, usize)) -> bool {
    left.0 != left.1 && right.0 != right.1 && left.0 < right.1 && right.0 < left.1
}

unsafe fn input_slice<'a>(input: *const u8, input_length: usize) -> &'a [u8] {
    if input_length == 0 {
        &[]
    } else {
        // SAFETY: the exported function's documented caller contract and
        // `validate_regions` establish a non-null bounded readable region.
        unsafe { slice::from_raw_parts(input, input_length) }
    }
}

unsafe fn write_result(
    bytes: &[u8],
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> Result<(), u32> {
    // SAFETY: `validate_regions` established a writable aligned length field.
    unsafe { output_length.write(bytes.len()) };
    if output_capacity < bytes.len() {
        return Err(STATUS_OUTPUT_TOO_SMALL);
    }
    if !bytes.is_empty() {
        // SAFETY: `validate_regions` established a disjoint caller-owned output
        // region and the capacity check above proves it can hold every byte.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len()) };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::NonNull;

    const SHORT_PAYLOAD: [u8; 11] = [0, 6, 6, 84, 72, 79, 82, 51, 48, 48, 0];
    const SHORT_FRAME: [u8; 16] = [2, 11, 0, 6, 6, 84, 72, 79, 82, 51, 48, 48, 0, 42, 216, 3];

    #[test]
    fn exact_symbols_encode_and_decode_source_vector() {
        assert_eq!(float_view_vesc_frame_abi_version(), ABI_VERSION);
        let mut frame = [0_u8; MAX_FRAME_BYTES];
        let mut frame_length = 0;
        // SAFETY: all supplied regions are valid and disjoint.
        let status = unsafe {
            float_view_vesc_frame_encode(
                SHORT_PAYLOAD.as_ptr(),
                SHORT_PAYLOAD.len(),
                frame.as_mut_ptr(),
                frame.len(),
                &mut frame_length,
            )
        };
        assert_eq!(status, STATUS_OK);
        assert_eq!(&frame[..frame_length], &SHORT_FRAME);

        let mut payload = [0_u8; MAX_PAYLOAD_BYTES];
        let mut payload_length = 0;
        // SAFETY: all supplied regions are valid and disjoint.
        let status = unsafe {
            float_view_vesc_frame_decode_exact(
                SHORT_FRAME.as_ptr(),
                SHORT_FRAME.len(),
                payload.as_mut_ptr(),
                payload.len(),
                &mut payload_length,
            )
        };
        assert_eq!(status, STATUS_OK);
        assert_eq!(&payload[..payload_length], &SHORT_PAYLOAD);
    }

    #[test]
    fn caller_buffer_contract_fails_closed() {
        let mut required = 99;
        let mut small = [0_u8; 3];
        // SAFETY: all supplied regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_encode(
                    SHORT_PAYLOAD.as_ptr(),
                    SHORT_PAYLOAD.len(),
                    small.as_mut_ptr(),
                    small.len(),
                    &mut required,
                )
            },
            STATUS_OUTPUT_TOO_SMALL
        );
        assert_eq!(required, SHORT_FRAME.len());

        // SAFETY: deliberately invalid null input is rejected before access.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_encode(
                    ptr::null(),
                    1,
                    small.as_mut_ptr(),
                    small.len(),
                    &mut required,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        // SAFETY: deliberately invalid null length pointer is rejected before access.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_encode(
                    SHORT_PAYLOAD.as_ptr(),
                    SHORT_PAYLOAD.len(),
                    small.as_mut_ptr(),
                    small.len(),
                    ptr::null_mut(),
                )
            },
            STATUS_INVALID_ARGUMENT
        );

        let mut overlapping = [7_u8; 32];
        let mut overlap_length = 0;
        // SAFETY: deliberately overlapping declared regions are rejected before access.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_encode(
                    overlapping.as_ptr(),
                    3,
                    overlapping.as_mut_ptr(),
                    10,
                    &mut overlap_length,
                )
            },
            STATUS_INVALID_ARGUMENT
        );

        let dangling = NonNull::<u8>::dangling().as_ptr();
        // SAFETY: oversized input is rejected before the dangling pointer is accessed.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_encode(
                    dangling,
                    MAX_PAYLOAD_BYTES + 1,
                    ptr::null_mut(),
                    0,
                    &mut required,
                )
            },
            STATUS_INPUT_TOO_LARGE
        );
    }

    #[test]
    fn semantic_failures_keep_fixed_statuses() {
        let mut payload = [0_u8; MAX_PAYLOAD_BYTES];
        let mut payload_length = 0;
        let mut corrupt = SHORT_FRAME;
        corrupt[13] ^= 1;
        // SAFETY: all supplied regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_decode_exact(
                    corrupt.as_ptr(),
                    corrupt.len(),
                    payload.as_mut_ptr(),
                    payload.len(),
                    &mut payload_length,
                )
            },
            STATUS_INVALID_CRC
        );
        let invalid = [2_u8, 0, 0, 0];
        // SAFETY: all supplied regions are valid and disjoint.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_decode_exact(
                    invalid.as_ptr(),
                    invalid.len(),
                    payload.as_mut_ptr(),
                    payload.len(),
                    &mut payload_length,
                )
            },
            STATUS_INVALID_FRAME
        );
    }

    #[test]
    fn panic_is_contained_as_internal_failure() {
        assert_eq!(
            contain(|| -> Result<(), u32> { panic!("test-only contained panic") }),
            STATUS_INTERNAL_FAILURE
        );
    }
}
