//! Conservative C ABI over the process-owned runtime.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use host_runtime::{Runtime, RuntimeError, WaitCommand};

pub type FloatViewStatus = i32;

pub const FLOAT_VIEW_STATUS_OK: FloatViewStatus = 0;
pub const FLOAT_VIEW_STATUS_INVALID_ARGUMENT: FloatViewStatus = 1;
pub const FLOAT_VIEW_STATUS_INVALID_HANDLE: FloatViewStatus = 2;
pub const FLOAT_VIEW_STATUS_WRONG_STATE: FloatViewStatus = 3;
pub const FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL: FloatViewStatus = 4;
pub const FLOAT_VIEW_STATUS_TIMEOUT: FloatViewStatus = 5;
pub const FLOAT_VIEW_STATUS_STOPPED: FloatViewStatus = 6;
pub const FLOAT_VIEW_STATUS_QUEUE_FULL: FloatViewStatus = 7;
pub const FLOAT_VIEW_STATUS_STALE_GENERATION: FloatViewStatus = 8;
pub const FLOAT_VIEW_STATUS_DUPLICATE_OPERATION: FloatViewStatus = 9;
pub const FLOAT_VIEW_STATUS_PANIC: FloatViewStatus = 10;
pub const FLOAT_VIEW_STATUS_INTERNAL: FloatViewStatus = 11;

pub const FLOAT_VIEW_ABI_VERSION: u32 = 2;
const _: () = assert!(FLOAT_VIEW_ABI_VERSION == host_runtime::ABI_VERSION);
pub const FLOAT_VIEW_BUILD_ID_CAPACITY: usize = 16;
pub const FLOAT_VIEW_MAX_COMMAND_PAYLOAD: usize = 64 * 1024;
pub const FLOAT_VIEW_MAX_COMPLETION_PAYLOAD: usize = 8 * 1024 * 1024;
pub const FLOAT_VIEW_EVENT_KIND_PLATFORM: u32 = 1;
pub const FLOAT_VIEW_COMMAND_KIND_PLATFORM: u32 = 1;
const _: () = assert!(FLOAT_VIEW_EVENT_KIND_PLATFORM == host_runtime::EVENT_KIND_PLATFORM);
const _: () = assert!(FLOAT_VIEW_COMMAND_KIND_PLATFORM == host_runtime::HOST_COMMAND_KIND_PLATFORM);

/// Opaque host token. ABI functions never dereference this pointer.
pub enum FloatViewRuntime {}

/// Fixed-width prefix followed by `payload_len` bytes in the caller buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FloatViewCommandHeader {
    pub abi_version: u32,
    pub kind: u32,
    pub flags: u32,
    pub reserved: u32,
    pub runtime_generation: u64,
    pub operation_id: u64,
    pub payload_len: u64,
}

static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
static RUNTIMES: OnceLock<Mutex<HashMap<usize, Arc<Runtime>>>> = OnceLock::new();

fn runtimes() -> &'static Mutex<HashMap<usize, Arc<Runtime>>> {
    RUNTIMES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn handle_key(handle: *mut FloatViewRuntime) -> Option<usize> {
    let key = handle.addr();
    (key != 0).then_some(key)
}

fn runtime_for(handle: *mut FloatViewRuntime) -> Result<Arc<Runtime>, FloatViewStatus> {
    let key = handle_key(handle).ok_or(FLOAT_VIEW_STATUS_INVALID_HANDLE)?;
    let registry = runtimes().lock().map_err(|_| FLOAT_VIEW_STATUS_INTERNAL)?;
    registry
        .get(&key)
        .cloned()
        .ok_or(FLOAT_VIEW_STATUS_INVALID_HANDLE)
}

fn map_runtime(error: RuntimeError) -> FloatViewStatus {
    match error {
        RuntimeError::InvalidArgument => FLOAT_VIEW_STATUS_INVALID_ARGUMENT,
        RuntimeError::WrongState => FLOAT_VIEW_STATUS_WRONG_STATE,
        RuntimeError::Poisoned => FLOAT_VIEW_STATUS_PANIC,
        RuntimeError::QueueFull => FLOAT_VIEW_STATUS_QUEUE_FULL,
        RuntimeError::StaleGeneration => FLOAT_VIEW_STATUS_STALE_GENERATION,
        RuntimeError::DuplicateOperation => FLOAT_VIEW_STATUS_DUPLICATE_OPERATION,
        RuntimeError::Internal => FLOAT_VIEW_STATUS_INTERNAL,
    }
}

fn ffi_boundary(
    handle: *mut FloatViewRuntime,
    operation: impl FnOnce() -> FloatViewStatus,
) -> FloatViewStatus {
    if let Ok(status) = catch_unwind(AssertUnwindSafe(operation)) {
        status
    } else {
        if let Ok(runtime) = runtime_for(handle) {
            runtime.poison();
        }
        FLOAT_VIEW_STATUS_PANIC
    }
}

unsafe fn write_value<T: Copy>(output: *mut T, value: T) -> Result<(), FloatViewStatus> {
    if output.is_null() {
        return Err(FLOAT_VIEW_STATUS_INVALID_ARGUMENT);
    }
    // SAFETY: The caller contract requires a valid, aligned writable T.
    unsafe { output.write(value) };
    Ok(())
}

fn validate_input(data: *const u8, length: usize, maximum: usize) -> Result<(), FloatViewStatus> {
    if length > maximum || (length != 0 && data.is_null()) {
        Err(FLOAT_VIEW_STATUS_INVALID_ARGUMENT)
    } else {
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn float_view_abi_version() -> u32 {
    FLOAT_VIEW_ABI_VERSION
}

/// Copies the exact build identity; `required` is mandatory and always written.
///
/// # Safety
///
/// `required` must be valid writable storage. When `capacity` is sufficient,
/// `output` must point to at least `capacity` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_build_id(
    output: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> FloatViewStatus {
    ffi_boundary(ptr::null_mut(), || {
        // SAFETY: Validated by write_value before its single write.
        if let Err(status) = unsafe { write_value(required, FLOAT_VIEW_BUILD_ID_CAPACITY) } {
            return status;
        }
        if capacity < FLOAT_VIEW_BUILD_ID_CAPACITY {
            return FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL;
        }
        if output.is_null() {
            return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
        }
        // SAFETY: The caller promises at least `capacity` writable bytes and the
        // source is a fixed in-process array of the checked smaller length.
        unsafe {
            ptr::copy_nonoverlapping(
                host_runtime::BUILD_ID_BYTES.as_ptr(),
                output,
                FLOAT_VIEW_BUILD_ID_CAPACITY,
            );
        }
        FLOAT_VIEW_STATUS_OK
    })
}

/// Creates an opaque runtime token in caller-owned pointer storage.
///
/// # Safety
///
/// `output` must point to writable storage for one runtime pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_runtime_create(
    output: *mut *mut FloatViewRuntime,
) -> FloatViewStatus {
    ffi_boundary(ptr::null_mut(), || {
        if output.is_null() {
            return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
        }
        let key = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        if key == 0 || generation == 0 {
            return FLOAT_VIEW_STATUS_INTERNAL;
        }
        let handle = ptr::without_provenance_mut::<FloatViewRuntime>(key);
        let Ok(mut registry) = runtimes().lock() else {
            return FLOAT_VIEW_STATUS_INTERNAL;
        };
        registry.insert(key, Arc::new(Runtime::new(generation)));
        // SAFETY: `output` was checked and the caller promises writable storage.
        unsafe { output.write(handle) };
        FLOAT_VIEW_STATUS_OK
    })
}

/// Starts the runtime and writes its nonzero generation.
///
/// # Safety
///
/// `generation` must point to writable storage for one `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_runtime_start(
    handle: *mut FloatViewRuntime,
    generation: *mut u64,
) -> FloatViewStatus {
    ffi_boundary(handle, || {
        let runtime = match runtime_for(handle) {
            Ok(runtime) => runtime,
            Err(status) => return status,
        };
        if generation.is_null() {
            return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
        }
        if let Err(error) = runtime.start() {
            return map_runtime(error);
        }
        // SAFETY: Checked non-null and caller promises writable storage.
        unsafe { generation.write(runtime.generation()) };
        FLOAT_VIEW_STATUS_OK
    })
}

/// Starts the runtime on a requested loopback port and writes its nonzero generation.
/// Port zero selects an ephemeral port.
///
/// # Safety
///
/// `generation` must point to writable storage for one `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_runtime_start_on_port(
    handle: *mut FloatViewRuntime,
    requested_port: u16,
    generation: *mut u64,
) -> FloatViewStatus {
    ffi_boundary(handle, || {
        let runtime = match runtime_for(handle) {
            Ok(runtime) => runtime,
            Err(status) => return status,
        };
        if generation.is_null() {
            return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
        }
        if let Err(error) = runtime.start_on_port(requested_port) {
            return map_runtime(error);
        }
        // SAFETY: Checked non-null and caller promises writable storage.
        unsafe { generation.write(runtime.generation()) };
        FLOAT_VIEW_STATUS_OK
    })
}

/// Starts a runtime on a requested loopback port for the explicit Android JNI facade.
/// This Rust-symbol helper is intentionally not part of the exported C ABI.
pub fn runtime_start_on_port_for_jni(
    handle: *mut FloatViewRuntime,
    requested_port: u16,
    generation: &mut u64,
) -> FloatViewStatus {
    ffi_boundary(handle, || {
        let runtime = match runtime_for(handle) {
            Ok(runtime) => runtime,
            Err(status) => return status,
        };
        if let Err(error) = runtime.start_on_port(requested_port) {
            return map_runtime(error);
        }
        *generation = runtime.generation();
        FLOAT_VIEW_STATUS_OK
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn float_view_runtime_stop(handle: *mut FloatViewRuntime) -> FloatViewStatus {
    ffi_boundary(handle, || match runtime_for(handle) {
        Ok(runtime) => runtime
            .stop()
            .map_or_else(map_runtime, |()| FLOAT_VIEW_STATUS_OK),
        Err(status) => status,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn float_view_runtime_destroy(handle: *mut FloatViewRuntime) -> FloatViewStatus {
    ffi_boundary(handle, || {
        let Some(key) = handle_key(handle) else {
            return FLOAT_VIEW_STATUS_INVALID_HANDLE;
        };
        let Ok(mut registry) = runtimes().lock() else {
            return FLOAT_VIEW_STATUS_INTERNAL;
        };
        let Some(runtime) = registry.get(&key) else {
            return FLOAT_VIEW_STATUS_INVALID_HANDLE;
        };
        match runtime.can_destroy() {
            Ok(true) => {
                registry.remove(&key);
                FLOAT_VIEW_STATUS_OK
            }
            Ok(false) => FLOAT_VIEW_STATUS_WRONG_STATE,
            Err(error) => map_runtime(error),
        }
    })
}

fn validate_runtime(
    handle: *mut FloatViewRuntime,
    generation: u64,
) -> Result<Arc<Runtime>, FloatViewStatus> {
    let runtime = runtime_for(handle)?;
    if runtime.generation() != generation {
        return Err(FLOAT_VIEW_STATUS_STALE_GENERATION);
    }
    Ok(runtime)
}

unsafe fn copy_input(
    data: *const u8,
    length: usize,
    maximum: usize,
) -> Result<Vec<u8>, FloatViewStatus> {
    validate_input(data, length, maximum)?;
    if length == 0 {
        return Ok(Vec::new());
    }
    // SAFETY: Validation requires a non-null pointer for nonzero lengths and
    // the caller contract guarantees `length` readable bytes for this call.
    Ok(unsafe { std::slice::from_raw_parts(data, length) }.to_vec())
}

/// Copies and serializes a bounded platform event through the application actor.
///
/// # Safety
///
/// For nonzero `payload_len`, `payload` must cover that many readable bytes.
///
/// # Panics
///
/// The test-only maximum event kind deliberately panics inside the ABI
/// containment boundary; the panic is caught and returned as a stable status.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_runtime_submit_event(
    handle: *mut FloatViewRuntime,
    generation: u64,
    kind: u32,
    payload: *const u8,
    payload_len: usize,
) -> FloatViewStatus {
    ffi_boundary(handle, || {
        let runtime = match validate_runtime(handle, generation) {
            Ok(runtime) => runtime,
            Err(status) => return status,
        };
        #[cfg(feature = "abi-test-hooks")]
        assert_ne!(kind, u32::MAX, "ABI panic containment hook");
        // SAFETY: The caller contract is checked by copy_input.
        let payload =
            match unsafe { copy_input(payload, payload_len, FLOAT_VIEW_MAX_COMMAND_PAYLOAD) } {
                Ok(payload) => payload,
                Err(status) => return status,
            };
        let result = if kind == FLOAT_VIEW_EVENT_KIND_PLATFORM {
            runtime.submit_platform_event(&payload).map(|()| 0)
        } else {
            runtime.dispatch(kind, &payload)
        };
        result.map_or_else(map_runtime, |_| FLOAT_VIEW_STATUS_OK)
    })
}

/// Copies bounded Bluetooth bytes into the serialized actor.
///
/// # Safety
///
/// For nonzero `bytes_len`, `bytes` must cover that many readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_runtime_submit_ble_bytes(
    handle: *mut FloatViewRuntime,
    generation: u64,
    connection_generation: u64,
    adapter_order: u64,
    bytes: *const u8,
    bytes_len: usize,
) -> FloatViewStatus {
    ffi_boundary(handle, || {
        let runtime = match validate_runtime(handle, generation) {
            Ok(runtime) => runtime,
            Err(status) => return status,
        };
        // SAFETY: The caller contract is checked by copy_input.
        let bytes = match unsafe { copy_input(bytes, bytes_len, host_ports::MAX_BLUETOOTH_BYTES) } {
            Ok(bytes) => bytes,
            Err(status) => return status,
        };
        runtime
            .submit_bluetooth(connection_generation, adapter_order, &bytes)
            .map_or_else(map_runtime, |()| FLOAT_VIEW_STATUS_OK)
    })
}

/// Copies a bounded result and accepts its operation identifier at most once.
///
/// # Safety
///
/// For nonzero `payload_len`, `payload` must cover that many readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_runtime_complete_operation(
    handle: *mut FloatViewRuntime,
    generation: u64,
    operation_id: u64,
    result_kind: u32,
    payload: *const u8,
    payload_len: usize,
) -> FloatViewStatus {
    ffi_boundary(handle, || {
        let runtime = match validate_runtime(handle, generation) {
            Ok(runtime) => runtime,
            Err(status) => return status,
        };
        // SAFETY: The caller contract is checked by copy_input.
        let payload =
            match unsafe { copy_input(payload, payload_len, FLOAT_VIEW_MAX_COMPLETION_PAYLOAD) } {
                Ok(payload) => payload,
                Err(status) => return status,
            };
        let result = if operation_id & (1 << 63) == 0 {
            runtime.complete_platform(operation_id, result_kind, &payload)
        } else {
            runtime.complete(operation_id)
        };
        result.map_or_else(map_runtime, |()| FLOAT_VIEW_STATUS_OK)
    })
}

/// Blocks for one command and copies it into caller-owned storage.
///
/// # Safety
///
/// `required_payload` must be writable. When a command is available, `header`
/// must be writable and `payload` must cover `payload_capacity` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_runtime_wait_command(
    handle: *mut FloatViewRuntime,
    generation: u64,
    timeout_ms: u32,
    header: *mut FloatViewCommandHeader,
    payload: *mut u8,
    payload_capacity: usize,
    required_payload: *mut usize,
) -> FloatViewStatus {
    ffi_boundary(handle, || {
        let runtime = match validate_runtime(handle, generation) {
            Ok(runtime) => runtime,
            Err(status) => return status,
        };
        if required_payload.is_null()
            || header.is_null()
            || (payload_capacity != 0 && payload.is_null())
        {
            return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
        }
        match runtime.wait_command(
            Duration::from_millis(u64::from(timeout_ms)),
            payload_capacity,
        ) {
            Ok((WaitCommand::Required(required), None)) => {
                // SAFETY: Checked non-null above.
                unsafe { required_payload.write(required) };
                FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL
            }
            Ok((WaitCommand::Timeout, None)) => {
                // SAFETY: Checked non-null above.
                unsafe { required_payload.write(0) };
                FLOAT_VIEW_STATUS_TIMEOUT
            }
            Ok((WaitCommand::Stopped, None)) => {
                // SAFETY: Checked non-null above.
                unsafe { required_payload.write(0) };
                FLOAT_VIEW_STATUS_STOPPED
            }
            Ok((WaitCommand::Ready, Some(command))) => {
                let Ok(payload_len) = u64::try_from(command.payload.len()) else {
                    return FLOAT_VIEW_STATUS_INTERNAL;
                };
                let output_header = FloatViewCommandHeader {
                    abi_version: FLOAT_VIEW_ABI_VERSION,
                    kind: command.kind,
                    flags: command.flags,
                    reserved: 0,
                    runtime_generation: command.generation,
                    operation_id: command.operation_id,
                    payload_len,
                };
                // SAFETY: Header and payload storage satisfy the caller contract
                // and capacity was checked without dequeueing by the queue.
                unsafe {
                    header.write(output_header);
                    required_payload.write(command.payload.len());
                    if !command.payload.is_empty() {
                        ptr::copy_nonoverlapping(
                            command.payload.as_ptr(),
                            payload,
                            command.payload.len(),
                        );
                    }
                }
                FLOAT_VIEW_STATUS_OK
            }
            Ok(_) => FLOAT_VIEW_STATUS_INTERNAL,
            Err(error) => map_runtime(error),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_and_lifecycle_are_stable() {
        assert_eq!(float_view_abi_version(), 2);
        let mut build = [0_u8; FLOAT_VIEW_BUILD_ID_CAPACITY];
        let mut required = 0;
        // SAFETY: All pointers reference correctly sized writable values.
        assert_eq!(
            unsafe { float_view_build_id(build.as_mut_ptr(), build.len(), &raw mut required) },
            FLOAT_VIEW_STATUS_OK
        );
        assert_eq!(required, build.len());
        assert_eq!(&build, host_runtime::BUILD_ID_BYTES);

        let mut handle = ptr::null_mut();
        // SAFETY: `handle` is writable pointer storage.
        assert_eq!(
            unsafe { float_view_runtime_create(&raw mut handle) },
            FLOAT_VIEW_STATUS_OK
        );
        let mut generation = 0;
        // SAFETY: Handle came from create and generation is writable.
        assert_eq!(
            unsafe { float_view_runtime_start(handle, &raw mut generation) },
            FLOAT_VIEW_STATUS_OK
        );
        assert_ne!(generation, 0);
        assert_eq!(float_view_runtime_stop(handle), FLOAT_VIEW_STATUS_OK);
        assert_eq!(float_view_runtime_destroy(handle), FLOAT_VIEW_STATUS_OK);
        assert_eq!(
            float_view_runtime_destroy(handle),
            FLOAT_VIEW_STATUS_INVALID_HANDLE
        );
    }

    #[test]
    fn output_capacity_is_reported_without_write() {
        let mut output = [0xA5_u8; 4];
        let mut required = 0;
        // SAFETY: Output and required pointers are valid for their sizes.
        assert_eq!(
            unsafe { float_view_build_id(output.as_mut_ptr(), output.len(), &raw mut required) },
            FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL
        );
        assert_eq!(required, FLOAT_VIEW_BUILD_ID_CAPACITY);
        assert_eq!(output, [0xA5; 4]);
    }
}
