//! Narrow Apple C ABI for the bounded VESC frame codec.
//!
//! Framing behavior remains in the safe `float-view-vesc-frame` crate. This
//! crate owns only bounded C representation validation and copying.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_apple_ffi_support::{
    Region, contain_status, input_bytes, publish_bytes, validate_disjoint, write_scalar,
};
use float_view_vesc_frame::{
    FrameError, MAX_PAYLOAD_BYTES, MAX_STREAM_BYTES, StreamScan, decode, encode, scan_next,
};

/// Version of the complete hand-written C contract.
pub const ABI_VERSION: u32 = 2;

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
/// The stream contains no complete candidate; consumption outputs are valid.
pub const STATUS_NEED_MORE_INPUT: u32 = 7;

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
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
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
        let payload = unsafe { input_bytes(payload, payload_length) };
        let frame = encode(payload).map_err(frame_status)?;
        // SAFETY: all writable regions were checked above and the frame is bounded.
        unsafe { publish_bytes(&frame, output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
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
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
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
        let frame = unsafe { input_bytes(frame, frame_length) };
        let payload = decode(frame).map_err(frame_status)?;
        // SAFETY: all writable regions were checked above and the payload is bounded.
        unsafe { publish_bytes(&payload, output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

/// Scan one bounded borrowed stream region for the next candidate frame.
///
/// # Safety
///
/// Non-empty input must be valid for its supplied byte length. Each output
/// pointer must be valid and aligned for one `usize`. The input and all three
/// output regions must be pairwise disjoint and remain valid for the duration
/// of the call. A null input is accepted only when `input_length` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_vesc_frame_stream_scan_next(
    input: *const u8,
    input_length: usize,
    frame_offset: *mut usize,
    frame_length: *mut usize,
    consumed_length: *mut usize,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        validate_scan_regions(
            input,
            input_length,
            frame_offset,
            frame_length,
            consumed_length,
        )?;

        if input_length > MAX_STREAM_BYTES {
            // SAFETY: the scanner-specific validator established disjoint,
            // aligned, writable output fields.
            unsafe {
                write_scan_result(
                    frame_offset,
                    frame_length,
                    consumed_length,
                    0,
                    0,
                    input_length,
                )
            };
            return Err(STATUS_INPUT_TOO_LARGE);
        }

        // SAFETY: `validate_scan_regions` checked the pointer contract and the
        // input was bounded before constructing this borrowed slice.
        let input = unsafe { input_bytes(input, input_length) };
        let (status, offset, length, consumed) = match scan_next(input) {
            StreamScan::NeedMore { consumed } => (STATUS_NEED_MORE_INPUT, 0, 0, consumed),
            StreamScan::Frame {
                frame_offset,
                frame_length,
                consumed,
            } => (STATUS_OK, frame_offset, frame_length, consumed),
            StreamScan::PayloadTooLarge { consumed } | StreamScan::StreamTooLarge { consumed } => {
                (STATUS_INPUT_TOO_LARGE, 0, 0, consumed)
            }
        };
        // SAFETY: the scanner-specific validator established disjoint,
        // aligned, writable output fields.
        unsafe {
            write_scan_result(
                frame_offset,
                frame_length,
                consumed_length,
                offset,
                length,
                consumed,
            )
        };
        if status == STATUS_OK {
            Ok(())
        } else {
            Err(status)
        }
    })
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
    if output_capacity > maximum_output_capacity {
        return Err(STATUS_INVALID_ARGUMENT);
    }

    let regions = [
        Region::optional(input, input_length).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::optional(output.cast_const(), output_capacity)
            .map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(output_length.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
    ];
    validate_disjoint(&regions).map_err(|_| STATUS_INVALID_ARGUMENT)?;

    // SAFETY: the caller guarantees a valid aligned `output_length`; null,
    // alignment, integer range, and declared-region overlap were checked above.
    unsafe { write_scalar(output_length, 0) };
    Ok(())
}

fn validate_scan_regions(
    input: *const u8,
    input_length: usize,
    frame_offset: *mut usize,
    frame_length: *mut usize,
    consumed_length: *mut usize,
) -> Result<(), u32> {
    let regions = [
        Region::optional(input, input_length).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(frame_offset.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(frame_length.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(consumed_length.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
    ];
    validate_disjoint(&regions).map_err(|_| STATUS_INVALID_ARGUMENT)
}

unsafe fn write_scan_result(
    frame_offset: *mut usize,
    frame_length: *mut usize,
    consumed_length: *mut usize,
    offset: usize,
    length: usize,
    consumed: usize,
) {
    // SAFETY: `validate_scan_regions` established three distinct aligned,
    // writable fields for the duration of the exported call.
    unsafe {
        write_scalar(frame_offset, offset);
        write_scalar(frame_length, length);
        write_scalar(consumed_length, consumed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{self, NonNull};

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
    fn scanner_reports_borrowed_boundaries_without_mutating_input() {
        let input = [
            [0xff, 0].as_slice(),
            SHORT_FRAME.as_slice(),
            SHORT_FRAME.as_slice(),
        ]
        .concat();
        let unchanged = input.clone();
        let mut frame_offset = usize::MAX;
        let mut frame_length = usize::MAX;
        let mut consumed_length = usize::MAX;
        // SAFETY: all supplied regions are valid and disjoint.
        let status = unsafe {
            float_view_vesc_frame_stream_scan_next(
                input.as_ptr(),
                input.len(),
                &mut frame_offset,
                &mut frame_length,
                &mut consumed_length,
            )
        };
        assert_eq!(status, STATUS_OK);
        assert_eq!(frame_offset, 2);
        assert_eq!(frame_length, SHORT_FRAME.len());
        assert_eq!(consumed_length, 2 + SHORT_FRAME.len());
        assert_eq!(input, unchanged);

        let incomplete = [0xff, 3, 0];
        // SAFETY: all supplied regions are valid and disjoint.
        let status = unsafe {
            float_view_vesc_frame_stream_scan_next(
                incomplete.as_ptr(),
                incomplete.len(),
                &mut frame_offset,
                &mut frame_length,
                &mut consumed_length,
            )
        };
        assert_eq!(status, STATUS_NEED_MORE_INPUT);
        assert_eq!((frame_offset, frame_length, consumed_length), (0, 0, 1));

        let oversized_declaration = [0xff, 3, 0x10, 0x01];
        // SAFETY: all supplied regions are valid and disjoint.
        let status = unsafe {
            float_view_vesc_frame_stream_scan_next(
                oversized_declaration.as_ptr(),
                oversized_declaration.len(),
                &mut frame_offset,
                &mut frame_length,
                &mut consumed_length,
            )
        };
        assert_eq!(status, STATUS_INPUT_TOO_LARGE);
        assert_eq!((frame_offset, frame_length, consumed_length), (0, 0, 2));

        // SAFETY: the oversized input is rejected before its non-null pointer
        // is accessed; all output fields are valid and disjoint.
        let status = unsafe {
            float_view_vesc_frame_stream_scan_next(
                NonNull::<u8>::dangling().as_ptr(),
                MAX_STREAM_BYTES + 1,
                &mut frame_offset,
                &mut frame_length,
                &mut consumed_length,
            )
        };
        assert_eq!(status, STATUS_INPUT_TOO_LARGE);
        assert_eq!(
            (frame_offset, frame_length, consumed_length),
            (0, 0, MAX_STREAM_BYTES + 1)
        );
    }

    #[test]
    fn scanner_rejects_invalid_output_regions_without_writing() {
        let mut frame_offset = 11_usize;
        let mut frame_length = 22_usize;
        let mut consumed_length = 33_usize;

        // SAFETY: the deliberately null output is rejected before any write.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_stream_scan_next(
                    SHORT_FRAME.as_ptr(),
                    SHORT_FRAME.len(),
                    ptr::null_mut(),
                    &mut frame_length,
                    &mut consumed_length,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!((frame_offset, frame_length, consumed_length), (11, 22, 33));

        let frame_offset_pointer = &mut frame_offset as *mut usize;
        // SAFETY: the deliberately aliased output fields are rejected before access.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_stream_scan_next(
                    SHORT_FRAME.as_ptr(),
                    SHORT_FRAME.len(),
                    frame_offset_pointer,
                    frame_offset_pointer,
                    &mut consumed_length,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!((frame_offset, frame_length, consumed_length), (11, 22, 33));

        // SAFETY: the output deliberately overlaps the input and is rejected
        // before either region is accessed.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_stream_scan_next(
                    frame_offset_pointer.cast_const().cast(),
                    size_of::<usize>(),
                    frame_offset_pointer,
                    &mut frame_length,
                    &mut consumed_length,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!((frame_offset, frame_length, consumed_length), (11, 22, 33));

        let mut aligned = [0_usize; 2];
        // SAFETY: the deliberately misaligned output is rejected before access.
        assert_eq!(
            unsafe {
                float_view_vesc_frame_stream_scan_next(
                    SHORT_FRAME.as_ptr(),
                    SHORT_FRAME.len(),
                    aligned.as_mut_ptr().cast::<u8>().add(1).cast(),
                    &mut frame_length,
                    &mut consumed_length,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!((frame_offset, frame_length, consumed_length), (11, 22, 33));
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
}
