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

use float_view_loopback_server::{host as server_host, limits as server_limits};

/// Creates a server without creating an application runtime.
/// # Safety
/// HTML must be readable for length bytes and out_handle must be aligned,
/// writable and disjoint from that region.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_application_runtime_server_create(
    platform: u32,
    html: *const u8,
    length: usize,
    out_handle: *mut u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if length == 0
            || length > 8 * 1024 * 1024
            || !valid_region(html, length)
            || !valid_scalar(out_handle)
            || regions_overlap(html, length, out_handle.cast(), 8)
        {
            return 1;
        }
        unsafe { out_handle.write(0) };
        match server_host::create(platform, unsafe { slice::from_raw_parts(html, length) }) {
            Ok(handle) => {
                unsafe { out_handle.write(handle) };
                0
            }
            Err(status) => status as i32,
        }
    }))
    .unwrap_or(8)
}
#[unsafe(no_mangle)]
pub extern "C" fn float_view_application_runtime_server_destroy(handle: u64) -> i32 {
    catch_unwind(AssertUnwindSafe(|| server_host::destroy(handle) as i32)).unwrap_or(8)
}

/// Executes a closed versioned command into caller-owned output.
/// # Safety
/// Input and output regions and the aligned writable out_written scalar must
/// be valid and mutually disjoint. No pointer is retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_application_runtime_server_command(
    handle: u64,
    input: *const u8,
    length: usize,
    output: *mut u8,
    capacity: usize,
    out_written: *mut usize,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if length == 0
            || length > server_limits::MAX_HOST_ENVELOPE_BYTES
            || !valid_region(input, length)
            || !valid_scalar(out_written)
            || capacity != server_limits::MAX_HOST_ENVELOPE_BYTES
            || !valid_region(output, capacity)
            || regions_overlap(input, length, output, capacity)
            || regions_overlap(
                input,
                length,
                out_written.cast(),
                std::mem::size_of::<usize>(),
            )
            || regions_overlap(
                output,
                capacity,
                out_written.cast(),
                std::mem::size_of::<usize>(),
            )
        {
            return 1;
        }
        unsafe { out_written.write(0) };
        match server_host::command(handle, unsafe { slice::from_raw_parts(input, length) }) {
            Ok(bytes) => {
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
                    out_written.write(bytes.len());
                }
                0
            }
            Err(status) => status as i32,
        }
    }))
    .unwrap_or(8)
}
/// Copies one incoming archive chunk into caller-owned storage.
/// # Safety
/// Output and out_written must be valid, aligned where applicable and disjoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_application_runtime_server_read_transfer(
    handle: u64,
    generation: u64,
    transfer: u64,
    output: *mut u8,
    capacity: usize,
    out_written: *mut usize,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if capacity != server_limits::MAX_TRANSFER_CHUNK_BYTES
            || !valid_region(output, capacity)
            || !valid_scalar(out_written)
            || regions_overlap(
                output,
                capacity,
                out_written.cast(),
                std::mem::size_of::<usize>(),
            )
        {
            return 1;
        }
        unsafe { out_written.write(0) };
        match server_host::read_chunk(handle, generation, transfer) {
            Ok(bytes) => {
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
                    out_written.write(bytes.len());
                }
                0
            }
            Err(status) => status as i32,
        }
    }))
    .unwrap_or(8)
}
/// Copies one outgoing archive chunk from caller-owned storage.
/// # Safety
/// Input must remain readable for length bytes for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_application_runtime_server_write_transfer(
    handle: u64,
    generation: u64,
    transfer: u64,
    offset: u64,
    input: *const u8,
    length: usize,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if length == 0
            || length > server_limits::MAX_TRANSFER_CHUNK_BYTES
            || !valid_region(input, length)
        {
            return 1;
        }
        server_host::write_chunk(handle, generation, transfer, offset, unsafe {
            slice::from_raw_parts(input, length)
        }) as i32
    }))
    .unwrap_or(8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_buffers_validate_before_mutation_and_destroy_is_idempotent() {
        let mut handle = 0;
        let html = b"html";
        assert_eq!(
            unsafe {
                float_view_application_runtime_server_create(
                    0,
                    html.as_ptr(),
                    html.len(),
                    &mut handle,
                )
            },
            0
        );
        let command = br#"{"version":1,"action":{"command":"stop"}}"#;
        let mut output = vec![0; server_limits::MAX_HOST_ENVELOPE_BYTES];
        let mut written = 0;
        assert_eq!(
            unsafe {
                float_view_application_runtime_server_command(
                    handle,
                    command.as_ptr(),
                    command.len(),
                    output.as_mut_ptr(),
                    1,
                    &mut written,
                )
            },
            1
        );
        assert_eq!(
            unsafe {
                float_view_application_runtime_server_command(
                    handle,
                    output.as_ptr(),
                    output.len(),
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                )
            },
            1
        );
        assert_eq!(
            unsafe {
                float_view_application_runtime_server_command(
                    handle,
                    command.as_ptr(),
                    command.len(),
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                )
            },
            0
        );
        assert!(written > 0);
        assert_eq!(float_view_application_runtime_server_destroy(handle), 0);
        assert_eq!(float_view_application_runtime_server_destroy(handle), 0);
        assert_eq!(
            unsafe {
                float_view_application_runtime_server_command(
                    handle,
                    command.as_ptr(),
                    command.len(),
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                )
            },
            2
        );
        assert_eq!(
            unsafe {
                float_view_application_runtime_server_create(
                    0,
                    std::ptr::null(),
                    html.len(),
                    &mut handle,
                )
            },
            1
        );
    }

    #[test]
    fn overlap_detection_is_half_open_and_overflow_safe() {
        let bytes = [0_u8; 16];
        let start = bytes.as_ptr();
        assert!(regions_overlap(start, 8, unsafe { start.add(7) }, 2));
        assert!(!regions_overlap(start, 8, unsafe { start.add(8) }, 8));
        assert!(regions_overlap(usize::MAX as *const u8, 2, start, 1));
    }
}
