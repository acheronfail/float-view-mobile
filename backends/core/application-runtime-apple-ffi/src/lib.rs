//! Thin callback-free C ABI for the shared application runtime host envelope.

#![deny(unsafe_op_in_unsafe_fn)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::slice;

use float_view_application_runtime_host as host;

pub use host::{ABI_VERSION, MAX_COMMAND_BYTES, MAX_RESULT_BYTES, Status};

#[unsafe(no_mangle)]
pub extern "C" fn float_view_application_runtime_abi_version() -> u32 {
    host::abi_version()
}

/// Creates a runtime and writes its nonzero registry handle.
///
/// # Safety
///
/// `out_handle` must be non-null, aligned, and writable for one `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_application_runtime_create(out_handle: *mut u64) -> i32 {
    contain(|| {
        if !valid_scalar(out_handle) {
            return Status::InvalidArgument;
        }
        unsafe { out_handle.write(0) };
        match host::create() {
            Ok(handle) => {
                unsafe { out_handle.write(handle) };
                Status::Ok
            }
            Err(error) => error,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn float_view_application_runtime_destroy(handle: u64) -> i32 {
    contain(|| host::destroy(handle))
}

#[unsafe(no_mangle)]
pub extern "C" fn float_view_application_runtime_interrupt(handle: u64) -> i32 {
    contain(|| host::interrupt(handle))
}

/// Applies one JSON command and records the required result-buffer length.
///
/// # Safety
///
/// `command` must identify `command_length` readable bytes. `out_required` must
/// be non-null, aligned, and writable for one `usize`; the regions must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_application_runtime_apply(
    handle: u64,
    command: *const u8,
    command_length: usize,
    out_required: *mut usize,
) -> i32 {
    contain(|| {
        if !valid_scalar(out_required)
            || command_length == 0
            || command_length > MAX_COMMAND_BYTES
            || !valid_region(command, command_length)
            || regions_overlap(
                command.cast(),
                command_length,
                out_required.cast(),
                std::mem::size_of::<usize>(),
            )
        {
            return Status::InvalidArgument;
        }
        unsafe { out_required.write(0) };
        let command = unsafe { slice::from_raw_parts(command, command_length) };
        match host::apply(handle, command) {
            Ok(required) => {
                unsafe { out_required.write(required) };
                Status::Ok
            }
            Err(error) => error,
        }
    })
}

/// Copies the pending JSON result into caller-owned storage.
///
/// # Safety
///
/// `output` must identify `output_capacity` writable bytes when nonzero.
/// `out_written` must be non-null, aligned, writable, and non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_application_runtime_copy_result(
    handle: u64,
    output: *mut u8,
    output_capacity: usize,
    out_written: *mut usize,
) -> i32 {
    contain(|| {
        if !valid_scalar(out_written)
            || output_capacity > MAX_RESULT_BYTES
            || (output_capacity > 0 && !valid_region(output, output_capacity))
            || regions_overlap(
                output.cast(),
                output_capacity,
                out_written.cast(),
                std::mem::size_of::<usize>(),
            )
        {
            return Status::InvalidArgument;
        }
        unsafe { out_written.write(0) };
        let output = if output_capacity == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(output, output_capacity) }
        };
        match host::copy_result(handle, output) {
            Ok(written) => {
                unsafe { out_written.write(written) };
                Status::Ok
            }
            Err((error, written)) => {
                unsafe { out_written.write(written) };
                error
            }
        }
    })
}

fn valid_scalar<T>(pointer: *const T) -> bool {
    !pointer.is_null()
        && (pointer as usize).is_multiple_of(std::mem::align_of::<T>())
        && (pointer as usize)
            .checked_add(std::mem::size_of::<T>().saturating_sub(1))
            .is_some()
}

fn valid_region<T>(pointer: *const T, length: usize) -> bool {
    length == 0
        || (!pointer.is_null()
            && length <= isize::MAX as usize
            && (pointer as usize).checked_add(length - 1).is_some())
}

fn regions_overlap(
    left: *const u8,
    left_length: usize,
    right: *const u8,
    right_length: usize,
) -> bool {
    if left_length == 0 || right_length == 0 {
        return false;
    }
    let left_start = left as usize;
    let right_start = right as usize;
    let Some(left_end) = left_start.checked_add(left_length) else {
        return true;
    };
    let Some(right_end) = right_start.checked_add(right_length) else {
        return true;
    };
    left_start < right_end && right_start < left_end
}

fn contain(operation: impl FnOnce() -> Status) -> i32 {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(Status::Internal) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_detection_is_half_open_and_overflow_safe() {
        let bytes = [0_u8; 16];
        let start = bytes.as_ptr();
        assert!(regions_overlap(start, 8, unsafe { start.add(7) }, 2));
        assert!(!regions_overlap(start, 8, unsafe { start.add(8) }, 8));
        assert!(regions_overlap(usize::MAX as *const u8, 2, start, 1));
    }
}
