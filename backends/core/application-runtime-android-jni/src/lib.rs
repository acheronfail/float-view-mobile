//! Thin callback-free JNI ABI for the shared application runtime host envelope.

#![deny(unsafe_op_in_unsafe_fn)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use float_view_application_runtime_host as host;
use jni_sys::{JNIEnv, jarray, jbyte, jbyteArray, jint, jintArray, jlong, jobject, jsize};

const STATUS_INVALID_ARGUMENT: jint = host::Status::InvalidArgument as jint;
const STATUS_INTERNAL: jint = host::Status::Internal as jint;

/// Applies one bounded OS fact or drain command.
/// # Safety
/// The JVM supplies call-local primitive arrays and a valid environment.
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn null_environment_and_panics_fail_closed() {
        assert_eq!(contain(7, || panic!("private payload")), 7);
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_application_RustApplicationRuntime_serviceCreate(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                Java_fail_acheron_floatview_application_RustApplicationRuntime_serviceDestroy(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    1,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
    }
}
