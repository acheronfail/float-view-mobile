//! Thin callback-free JNI ABI for the shared application runtime host envelope.

#![deny(unsafe_op_in_unsafe_fn)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use float_view_application_runtime_host as host;
use jni_sys::{JNIEnv, jarray, jbyte, jbyteArray, jint, jintArray, jlong, jobject, jsize};

const STATUS_INVALID_ARGUMENT: jint = host::Status::InvalidArgument as jint;
const STATUS_INTERNAL: jint = host::Status::Internal as jint;

/// Returns the exact hand-written runtime JNI contract version.
///
/// # Safety
/// `env` must be null or the call-local environment supplied by the JVM.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_abiVersion(
    env: *mut JNIEnv,
    _receiver: jobject,
) -> jint {
    contain(0, || unsafe {
        with_environment(env, |environment| {
            environment.ensure_no_exception()?;
            Ok(host::abi_version() as jint)
        })
        .unwrap_or(0)
    })
}

/// Creates one registry-backed runtime and returns its nonzero handle.
///
/// # Safety
/// `env` must be null or the call-local environment supplied by the JVM.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_create(
    env: *mut JNIEnv,
    _receiver: jobject,
) -> jlong {
    contain(0, || unsafe {
        with_environment(env, |environment| {
            environment.ensure_no_exception()?;
            host::create().map(|handle| handle as jlong).map_err(|_| ())
        })
        .unwrap_or(0)
    })
}

/// Destroys a runtime handle idempotently.
///
/// # Safety
/// `env` must be null or the call-local environment supplied by the JVM.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_destroy(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
) -> jint {
    contain(STATUS_INTERNAL, || unsafe {
        with_environment(env, |environment| {
            environment.ensure_no_exception()?;
            Ok(host::destroy(handle as u64) as jint)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Interrupts the current bounded reader or pruner operation, if any.
///
/// # Safety
/// `env` must be null or the call-local environment supplied by the JVM.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_interrupt(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
) -> jint {
    contain(STATUS_INTERNAL, || unsafe {
        with_environment(env, |environment| {
            environment.ensure_no_exception()?;
            Ok(host::interrupt(handle as u64) as jint)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Applies one bounded UTF-8 JSON command and publishes its result length.
///
/// # Safety
/// `env` and both array references must be call-local values supplied by the
/// JVM with the primitive-array types declared by the matching Kotlin method.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_apply(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
    command: jbyteArray,
    required: jintArray,
) -> jint {
    contain(STATUS_INTERNAL, || unsafe {
        with_environment(env, |environment| {
            environment.require_references(&[command.cast(), required.cast()])?;
            environment.require_distinct(command.cast(), required.cast())?;
            environment.require_length(required.cast(), 1)?;
            let bytes = environment.read_bytes(command, host::MAX_COMMAND_BYTES)?;
            environment.write_int(required, 0)?;
            match host::apply(handle as u64, &bytes) {
                Ok(length) => {
                    let length = jint::try_from(length).map_err(|_| ())?;
                    environment.write_int(required, length)?;
                    Ok(host::Status::Ok as jint)
                }
                Err(error) => Ok(error as jint),
            }
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Copies the pending bounded JSON result into caller-owned storage.
///
/// # Safety
/// `env` and both array references must be call-local values supplied by the
/// JVM with the primitive-array types declared by the matching Kotlin method.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_copyResult(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
    output: jbyteArray,
    written: jintArray,
) -> jint {
    contain(STATUS_INTERNAL, || unsafe {
        with_environment(env, |environment| {
            environment.require_references(&[output.cast(), written.cast()])?;
            environment.require_distinct(output.cast(), written.cast())?;
            environment.require_length(written.cast(), 1)?;
            let capacity = environment.length(output.cast())?;
            if capacity > host::MAX_RESULT_BYTES {
                return Err(());
            }
            environment.write_int(written, 0)?;
            let mut bytes = vec![0_u8; capacity];
            match host::copy_result(handle as u64, &mut bytes) {
                Ok(length) => {
                    environment.write_bytes(output, &bytes[..length])?;
                    environment.write_int(written, jint::try_from(length).map_err(|_| ())?)?;
                    Ok(host::Status::Ok as jint)
                }
                Err((error, length)) => {
                    environment.write_int(written, jint::try_from(length).map_err(|_| ())?)?;
                    Ok(error as jint)
                }
            }
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Exchanges a bounded process-service value.
/// # Safety
/// The JVM supplies call-local primitive arrays and environment references.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serviceApply(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
    command: jbyteArray,
    required: jintArray,
) -> jint {
    contain(STATUS_INTERNAL, || unsafe {
        with_environment(env, |environment| {
            environment.require_references(&[command.cast(), required.cast()])?;
            environment.require_distinct(command.cast(), required.cast())?;
            environment.require_length(required.cast(), 1)?;
            let bytes = environment.read_bytes(command, host::MAX_COMMAND_BYTES)?;
            environment.write_int(required, 0)?;
            match host::service::apply(handle as u64, &bytes) {
                Ok(length) => {
                    let length = jint::try_from(length).map_err(|_| ())?;
                    environment.write_int(required, length)?;
                    Ok(host::Status::Ok as jint)
                }
                Err(error) => Ok(error as jint),
            }
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Exchanges a bounded process-service value.
/// # Safety
/// The JVM supplies call-local primitive arrays and environment references.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serviceCopyResult(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
    output: jbyteArray,
    written: jintArray,
) -> jint {
    contain(STATUS_INTERNAL, || unsafe {
        with_environment(env, |environment| {
            environment.require_references(&[output.cast(), written.cast()])?;
            environment.require_distinct(output.cast(), written.cast())?;
            environment.require_length(written.cast(), 1)?;
            let capacity = environment.length(output.cast())?;
            if capacity > host::MAX_RESULT_BYTES {
                return Err(());
            }
            environment.write_int(written, 0)?;
            let mut bytes = vec![0_u8; capacity];
            match host::service::copy_result(handle as u64, &mut bytes) {
                Ok(length) => {
                    environment.write_bytes(output, &bytes[..length])?;
                    environment.write_int(written, jint::try_from(length).map_err(|_| ())?)?;
                    Ok(host::Status::Ok as jint)
                }
                Err((error, length)) => {
                    environment.write_int(written, jint::try_from(length).map_err(|_| ())?)?;
                    Ok(error as jint)
                }
            }
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Exchanges a bounded process-service value.
/// # Safety
/// The JVM supplies call-local primitive arrays and environment references.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serviceDestroy(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
) -> jint {
    contain(STATUS_INTERNAL, || unsafe {
        with_environment(env, |environment| {
            environment.ensure_no_exception()?;
            Ok(host::service::destroy(handle as u64) as jint)
        })
        .unwrap_or(STATUS_INVALID_ARGUMENT)
    })
}

/// Creates a process service from a prepared root.
/// # Safety
/// Root is a JVM-supplied byte array and env is call-local.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serviceCreate(
    env: *mut JNIEnv,
    _receiver: jobject,
    root: jbyteArray,
) -> jlong {
    contain(0, || unsafe {
        with_environment(env, |environment| {
            environment.ensure_no_exception()?;
            {
                let bytes = environment.read_bytes(root, 4096)?;
                host::service::create(&bytes)
                    .map(|handle| handle as jlong)
                    .map_err(|_| ())
            }
        })
        .unwrap_or(0)
    })
}

/// Creates a listener that borrows the process service.
/// # Safety
/// Arrays and env are JVM-supplied call-local values.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serviceServerCreate(
    env: *mut JNIEnv,
    _receiver: jobject,
    service: jlong,
    platform: jint,
    html: jbyteArray,
) -> jlong {
    contain(0, || unsafe {
        with_environment(env, |e| {
            e.require_references(&[html.cast()])?;
            let bytes = e.read_bytes(html, 8 * 1024 * 1024)?;
            server_host::create_shared(
                platform as u32,
                &bytes,
                host::service::owner(service as u64).map_err(|_| ())?,
            )
            .map(|id| id as jlong)
            .map_err(|_| ())
        })
        .unwrap_or(0)
    })
}
fn contain<T>(fallback: T, operation: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(fallback)
}

struct Environment<'call> {
    raw: *mut JNIEnv,
    functions: &'call jni_sys::JNINativeInterface__1_2,
}

unsafe fn with_environment<T>(
    raw: *mut JNIEnv,
    operation: impl for<'call> FnOnce(&Environment<'call>) -> Result<T, ()>,
) -> Result<T, ()> {
    if raw.is_null() {
        return Err(());
    }
    let table = unsafe { *raw };
    if table.is_null() {
        return Err(());
    }
    operation(&Environment {
        raw,
        functions: unsafe { &(*table).v1_2 },
    })
}

impl Environment<'_> {
    fn ensure_no_exception(&self) -> Result<(), ()> {
        if unsafe { (self.functions.ExceptionCheck)(self.raw) } {
            Err(())
        } else {
            Ok(())
        }
    }

    fn require_references(&self, references: &[jobject]) -> Result<(), ()> {
        self.ensure_no_exception()?;
        if references.iter().any(|reference| reference.is_null()) {
            Err(())
        } else {
            Ok(())
        }
    }

    fn require_distinct(&self, left: jobject, right: jobject) -> Result<(), ()> {
        if unsafe { (self.functions.IsSameObject)(self.raw, left, right) } {
            Err(())
        } else {
            self.ensure_no_exception()
        }
    }

    fn length(&self, array: jarray) -> Result<usize, ()> {
        let length = unsafe { (self.functions.GetArrayLength)(self.raw, array) };
        self.ensure_no_exception()?;
        usize::try_from(length).map_err(|_| ())
    }

    fn require_length(&self, array: jarray, expected: usize) -> Result<(), ()> {
        if self.length(array)? == expected {
            Ok(())
        } else {
            Err(())
        }
    }

    fn read_bytes(&self, array: jbyteArray, maximum: usize) -> Result<Vec<u8>, ()> {
        let length = self.length(array.cast())?;
        if length == 0 || length > maximum || length > jsize::MAX as usize {
            return Err(());
        }
        let mut bytes = vec![0_u8; length];
        unsafe {
            (self.functions.GetByteArrayRegion)(
                self.raw,
                array,
                0,
                length as jsize,
                bytes.as_mut_ptr().cast::<jbyte>(),
            )
        };
        self.ensure_no_exception()?;
        Ok(bytes)
    }

    fn write_bytes(&self, array: jbyteArray, bytes: &[u8]) -> Result<(), ()> {
        if bytes.len() > self.length(array.cast())? || bytes.len() > jsize::MAX as usize {
            return Err(());
        }
        if !bytes.is_empty() {
            unsafe {
                (self.functions.SetByteArrayRegion)(
                    self.raw,
                    array,
                    0,
                    bytes.len() as jsize,
                    bytes.as_ptr().cast::<jbyte>(),
                )
            };
        }
        self.ensure_no_exception()
    }

    fn write_int(&self, array: jintArray, value: jint) -> Result<(), ()> {
        unsafe { (self.functions.SetIntArrayRegion)(self.raw, array, 0, 1, &value) };
        self.ensure_no_exception()
    }
}

use float_view_loopback_server::{host as server_host, limits as server_limits};
/// Creates a server with no additional application runtime.
/// # Safety
/// The JVM supplies call-local primitive array and environment references.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serverCreate(
    env: *mut JNIEnv,
    _receiver: jobject,
    platform: jint,
    html: jbyteArray,
) -> jlong {
    contain(0, || unsafe {
        with_environment(env, |e| {
            e.require_references(&[html.cast()])?;
            let bytes = e.read_bytes(html, 8 * 1024 * 1024)?;
            server_host::create(platform as u32, &bytes)
                .map(|id| id as jlong)
                .map_err(|_| ())
        })
        .unwrap_or(0)
    })
}
/// Destroys a server handle idempotently.
/// # Safety
/// The JVM supplies a call-local environment reference.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serverDestroy(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
) -> jint {
    contain(8, || unsafe {
        with_environment(env, |e| {
            e.ensure_no_exception()?;
            Ok(server_host::destroy(handle as u64) as jint)
        })
        .unwrap_or(1)
    })
}
/// Exchanges a bounded closed command with caller-owned output.
/// # Safety
/// All references have the primitive-array types declared by Kotlin and remain
/// valid for this call. The environment is never retained.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serverCommand(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
    input: jbyteArray,
    output: jbyteArray,
    written: jintArray,
) -> jint {
    contain(8, || unsafe {
        with_environment(env, |e| {
            e.require_references(&[input.cast(), output.cast(), written.cast()])?;
            e.require_distinct(input.cast(), output.cast())?;
            e.require_distinct(input.cast(), written.cast())?;
            e.require_distinct(output.cast(), written.cast())?;
            e.require_length(written.cast(), 1)?;
            e.require_length(output.cast(), server_limits::MAX_HOST_ENVELOPE_BYTES)?;
            let input = e.read_bytes(input, server_limits::MAX_HOST_ENVELOPE_BYTES)?;
            e.write_int(written, 0)?;
            match server_host::command(handle as u64, &input) {
                Ok(bytes) => {
                    e.write_bytes(output, &bytes)?;
                    e.write_int(written, bytes.len() as jint)?;
                    Ok(0)
                }
                Err(status) => Ok(status as jint),
            }
        })
        .unwrap_or(1)
    })
}
/// Reads a binary import chunk.
/// # Safety
/// References are call-local primitive arrays of their declared Kotlin types.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serverReadTransfer(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
    generation: jlong,
    transfer: jlong,
    output: jbyteArray,
    written: jintArray,
) -> jint {
    contain(8, || unsafe {
        with_environment(env, |e| {
            e.require_references(&[output.cast(), written.cast()])?;
            e.require_distinct(output.cast(), written.cast())?;
            e.require_length(written.cast(), 1)?;
            e.require_length(output.cast(), server_limits::MAX_TRANSFER_CHUNK_BYTES)?;
            e.write_int(written, 0)?;
            match server_host::read_chunk(handle as u64, generation as u64, transfer as u64) {
                Ok(bytes) => {
                    e.write_bytes(output, &bytes)?;
                    e.write_int(written, bytes.len() as jint)?;
                    Ok(0)
                }
                Err(status) => Ok(status as jint),
            }
        })
        .unwrap_or(1)
    })
}
/// Writes a binary export chunk.
/// # Safety
/// The input is a call-local byte array and the JVM environment is valid.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_application_RustApplicationRuntime_serverWriteTransfer(
    env: *mut JNIEnv,
    _receiver: jobject,
    handle: jlong,
    generation: jlong,
    transfer: jlong,
    offset: jlong,
    input: jbyteArray,
) -> jint {
    contain(8, || unsafe {
        with_environment(env, |e| {
            e.require_references(&[input.cast()])?;
            let bytes = e.read_bytes(input, server_limits::MAX_TRANSFER_CHUNK_BYTES)?;
            Ok(server_host::write_chunk(
                handle as u64,
                generation as u64,
                transfer as u64,
                offset as u64,
                &bytes,
            ) as jint)
        })
        .unwrap_or(1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_environment_and_panics_fail_closed() {
        assert_eq!(contain(7, || panic!("private payload")), 7);
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_application_RustApplicationRuntime_abiVersion(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_application_RustApplicationRuntime_create(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
    }
}
