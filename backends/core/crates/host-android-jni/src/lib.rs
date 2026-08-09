//! Explicitly registered Android JNI facade over the shared C-ABI semantics.

#[must_use]
pub const fn linked_abi_version() -> u32 {
    host_runtime::ABI_VERSION
}

#[must_use]
pub const fn linked_build_identity() -> &'static [u8; 16] {
    host_runtime::BUILD_ID_BYTES
}

#[cfg(target_os = "android")]
mod android {
    use std::ffi::c_void;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::ptr;

    use host_c_abi::{
        FLOAT_VIEW_BUILD_ID_CAPACITY, FLOAT_VIEW_MAX_COMMAND_PAYLOAD,
        FLOAT_VIEW_MAX_COMPLETION_PAYLOAD, FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL,
        FLOAT_VIEW_STATUS_INTERNAL, FLOAT_VIEW_STATUS_INVALID_ARGUMENT, FloatViewCommandHeader,
        FloatViewRuntime, float_view_abi_version, float_view_build_id,
        float_view_runtime_complete_operation, float_view_runtime_create,
        float_view_runtime_destroy, float_view_runtime_start_configured, float_view_runtime_stop,
        float_view_runtime_submit_ble_bytes, float_view_runtime_submit_bluetooth_connection,
        float_view_runtime_submit_bluetooth_device, float_view_runtime_submit_bluetooth_state,
        float_view_runtime_submit_event, float_view_runtime_submit_lifecycle,
        float_view_runtime_submit_location, float_view_runtime_submit_location_authorization,
        float_view_runtime_wait_command, runtime_start_on_port_for_jni,
    };
    use jni::objects::{JByteArray, JClass, JLongArray};
    use jni::sys::{JNI_ERR, JNI_VERSION_1_6, jboolean, jdouble, jint, jlong};
    use jni::{JNIEnv, JavaVM, NativeMethod};

    // These bounds mirror the inputs accepted immediately below the JNI layer
    // by the private C ABI. Preflighting Java arrays prevents an unbounded JNI
    // allocation before the shared Rust validation gets a chance to reject it.
    const MAX_STORAGE_ROOT_BYTES: usize = 1024;
    const MAX_FRONTEND_INDEX_BYTES: usize = FLOAT_VIEW_MAX_COMPLETION_PAYLOAD;
    const MAX_PLATFORM_ASSOCIATION_BYTES: usize = 256;
    const MAX_LOGICAL_NAME_BYTES: usize = 128;

    fn handle(raw: jlong) -> *mut FloatViewRuntime {
        ptr::with_exposed_provenance_mut(raw as usize)
    }

    fn valid_handle(raw: jlong) -> Result<*mut FloatViewRuntime, jint> {
        if raw <= 0 {
            Err(FLOAT_VIEW_STATUS_INVALID_ARGUMENT)
        } else {
            Ok(handle(raw))
        }
    }

    fn status_boundary(operation: impl FnOnce() -> jint) -> jint {
        catch_unwind(AssertUnwindSafe(operation)).unwrap_or(FLOAT_VIEW_STATUS_INTERNAL)
    }

    fn array_length(
        env: &JNIEnv<'_>,
        array: &JByteArray<'_>,
        maximum: usize,
    ) -> Result<usize, jint> {
        let length = env
            .get_array_length(array)
            .map_err(|_| FLOAT_VIEW_STATUS_INVALID_ARGUMENT)?;
        let length = usize::try_from(length).map_err(|_| FLOAT_VIEW_STATUS_INVALID_ARGUMENT)?;
        if length > maximum {
            Err(FLOAT_VIEW_STATUS_INVALID_ARGUMENT)
        } else {
            Ok(length)
        }
    }

    fn copy_array(
        env: &JNIEnv<'_>,
        array: &JByteArray<'_>,
        maximum: usize,
    ) -> Result<Vec<u8>, jint> {
        array_length(env, array, maximum)?;
        env.convert_byte_array(array)
            .map_err(|_| FLOAT_VIEW_STATUS_INVALID_ARGUMENT)
    }

    extern "system" fn abi_version(_env: JNIEnv<'_>, _class: JClass<'_>) -> jint {
        float_view_abi_version() as jint
    }

    extern "system" fn build_identity(
        env: JNIEnv<'_>,
        _class: JClass<'_>,
        output: JByteArray<'_>,
    ) -> jint {
        status_boundary(|| {
            let Ok(capacity) = env.get_array_length(&output) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            if capacity < i32::try_from(FLOAT_VIEW_BUILD_ID_CAPACITY).unwrap_or(i32::MAX) {
                return FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL;
            }
            let mut identity = [0_u8; FLOAT_VIEW_BUILD_ID_CAPACITY];
            let mut required = 0_usize;
            // SAFETY: Both pointers refer to writable call-scoped fixed storage.
            let status = unsafe {
                float_view_build_id(identity.as_mut_ptr(), identity.len(), &raw mut required)
            };
            if status != 0 || required != identity.len() {
                return if status == 0 {
                    FLOAT_VIEW_STATUS_INTERNAL
                } else {
                    status
                };
            }
            let copied: Vec<i8> = identity.iter().map(|byte| *byte as i8).collect();
            if env.set_byte_array_region(&output, 0, &copied).is_err() {
                FLOAT_VIEW_STATUS_INTERNAL
            } else {
                0
            }
        })
    }

    extern "system" fn create(_env: JNIEnv<'_>, _class: JClass<'_>) -> jlong {
        catch_unwind(AssertUnwindSafe(|| {
            let mut runtime = ptr::null_mut();
            // SAFETY: `runtime` is writable pointer storage scoped to this call.
            let status = unsafe { float_view_runtime_create(&raw mut runtime) };
            if status == 0 {
                runtime.addr() as jlong
            } else {
                -jlong::from(status)
            }
        }))
        .unwrap_or(-jlong::from(FLOAT_VIEW_STATUS_INTERNAL))
    }

    extern "system" fn start(
        env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        requested_port: jint,
        output: JLongArray<'_>,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            if !(0..=u16::MAX.into()).contains(&requested_port)
                || env.get_array_length(&output).ok() != Some(1)
            {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            }
            let mut generation = 0_u64;
            let status =
                runtime_start_on_port_for_jni(runtime, requested_port as u16, &mut generation);
            if status == 0
                && env
                    .set_long_array_region(&output, 0, &[generation as jlong])
                    .is_err()
            {
                return FLOAT_VIEW_STATUS_INTERNAL;
            }
            status
        })
    }

    extern "system" fn start_configured(
        env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        requested_port: jint,
        transport: jint,
        storage_root: JByteArray<'_>,
        frontend_index: JByteArray<'_>,
        output: JLongArray<'_>,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            if !(0..=u16::MAX.into()).contains(&requested_port)
                || env.get_array_length(&output).ok() != Some(1)
            {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            }
            let Ok(storage_root) = copy_array(&env, &storage_root, MAX_STORAGE_ROOT_BYTES) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            let Ok(frontend_index) = copy_array(&env, &frontend_index, MAX_FRONTEND_INDEX_BYTES)
            else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            let mut generation = 0_u64;
            // SAFETY: Array storage remains readable for this copying call.
            let status = unsafe {
                float_view_runtime_start_configured(
                    runtime,
                    requested_port as u16,
                    transport as u32,
                    storage_root.as_ptr(),
                    storage_root.len(),
                    frontend_index.as_ptr(),
                    frontend_index.len(),
                    &raw mut generation,
                )
            };
            if status == 0
                && env
                    .set_long_array_region(&output, 0, &[generation as jlong])
                    .is_err()
            {
                return FLOAT_VIEW_STATUS_INTERNAL;
            }
            status
        })
    }

    extern "system" fn submit(
        env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        kind: jint,
        payload: JByteArray<'_>,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            if generation <= 0 || kind < 0 {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            }
            let Ok(bytes) = copy_array(&env, &payload, FLOAT_VIEW_MAX_COMMAND_PAYLOAD) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            // SAFETY: `bytes` remains readable for the call and the ABI copies it.
            unsafe {
                float_view_runtime_submit_event(
                    runtime,
                    generation as u64,
                    kind as u32,
                    bytes.as_ptr(),
                    bytes.len(),
                )
            }
        })
    }

    extern "system" fn submit_ble(
        env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        connection_generation: jlong,
        adapter_order: jlong,
        payload: JByteArray<'_>,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            if generation <= 0 || connection_generation <= 0 || adapter_order < 0 {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            }
            let Ok(bytes) = copy_array(&env, &payload, FLOAT_VIEW_MAX_COMMAND_PAYLOAD) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            // SAFETY: `bytes` is call-scoped and the shared ABI copies it.
            unsafe {
                float_view_runtime_submit_ble_bytes(
                    runtime,
                    generation as u64,
                    connection_generation as u64,
                    adapter_order as u64,
                    bytes.as_ptr(),
                    bytes.len(),
                )
            }
        })
    }

    extern "system" fn submit_bluetooth_state(
        _env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        connection_generation: jlong,
        adapter_order: jlong,
        radio: jint,
        authorization: jint,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            if generation <= 0 || connection_generation < 0 || adapter_order < 0 {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            }
            float_view_runtime_submit_bluetooth_state(
                runtime,
                generation as u64,
                connection_generation as u64,
                adapter_order as u64,
                radio as u32,
                authorization as u32,
            )
        })
    }

    extern "system" fn submit_bluetooth_device(
        env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        connection_generation: jlong,
        adapter_order: jlong,
        association: JByteArray<'_>,
        name: JByteArray<'_>,
        rssi: jint,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            let Ok(association) = copy_array(&env, &association, MAX_PLATFORM_ASSOCIATION_BYTES)
            else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            let Ok(name) = copy_array(&env, &name, MAX_LOGICAL_NAME_BYTES) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            if generation <= 0 || connection_generation < 0 || adapter_order < 0 {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            }
            // SAFETY: Both arrays remain readable for the duration of this copying call.
            unsafe {
                float_view_runtime_submit_bluetooth_device(
                    runtime,
                    generation as u64,
                    connection_generation as u64,
                    adapter_order as u64,
                    association.as_ptr(),
                    association.len(),
                    name.as_ptr(),
                    name.len(),
                    rssi as i16,
                )
            }
        })
    }

    extern "system" fn submit_bluetooth_connection(
        _env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        connection_generation: jlong,
        adapter_order: jlong,
        state: jint,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            float_view_runtime_submit_bluetooth_connection(
                runtime,
                generation as u64,
                connection_generation as u64,
                adapter_order as u64,
                state as u32,
            )
        })
    }

    extern "system" fn submit_location_authorization(
        _env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        connection_generation: jlong,
        adapter_order: jlong,
        authorization: jint,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            float_view_runtime_submit_location_authorization(
                runtime,
                generation as u64,
                connection_generation as u64,
                adapter_order as u64,
                authorization as u32,
            )
        })
    }

    extern "system" fn submit_location(
        _env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        connection_generation: jlong,
        adapter_order: jlong,
        latitude: jdouble,
        longitude: jdouble,
        accuracy_m: jdouble,
        has_accuracy: jboolean,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            float_view_runtime_submit_location(
                runtime,
                generation as u64,
                connection_generation as u64,
                adapter_order as u64,
                latitude,
                longitude,
                accuracy_m,
                has_accuracy,
            )
        })
    }

    extern "system" fn submit_lifecycle(
        _env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        connection_generation: jlong,
        adapter_order: jlong,
        foreground: jboolean,
        background_eligible: jboolean,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            float_view_runtime_submit_lifecycle(
                runtime,
                generation as u64,
                connection_generation as u64,
                adapter_order as u64,
                foreground,
                background_eligible,
            )
        })
    }

    extern "system" fn wait_command(
        env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        timeout_ms: jint,
        header_output: JLongArray<'_>,
        payload_output: JByteArray<'_>,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            if generation <= 0
                || env.get_array_length(&header_output).ok() != Some(7)
                || timeout_ms < 0
            {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            }
            let Ok(capacity) = array_length(&env, &payload_output, FLOAT_VIEW_MAX_COMMAND_PAYLOAD)
            else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            let mut header = FloatViewCommandHeader::default();
            let mut payload = vec![0_u8; capacity];
            let mut required = 0_usize;
            // SAFETY: All output pointers reference call-scoped writable storage.
            let status = unsafe {
                float_view_runtime_wait_command(
                    runtime,
                    generation as u64,
                    timeout_ms as u32,
                    &raw mut header,
                    payload.as_mut_ptr(),
                    payload.len(),
                    &raw mut required,
                )
            };
            let values = [
                jlong::from(header.abi_version),
                jlong::from(header.kind),
                jlong::from(header.flags),
                header.runtime_generation as jlong,
                header.operation_id as jlong,
                header.payload_len as jlong,
                required as jlong,
            ];
            if env
                .set_long_array_region(&header_output, 0, &values)
                .is_err()
            {
                return FLOAT_VIEW_STATUS_INTERNAL;
            }
            if status == 0 {
                let copied: Vec<i8> = payload[..required].iter().map(|byte| *byte as i8).collect();
                if env
                    .set_byte_array_region(&payload_output, 0, &copied)
                    .is_err()
                {
                    return FLOAT_VIEW_STATUS_INTERNAL;
                }
            }
            status
        })
    }

    extern "system" fn complete(
        env: JNIEnv<'_>,
        _class: JClass<'_>,
        raw: jlong,
        generation: jlong,
        operation_id: jlong,
        result_kind: jint,
        payload: JByteArray<'_>,
    ) -> jint {
        status_boundary(|| {
            let Ok(runtime) = valid_handle(raw) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            // Operation identifiers are opaque u64 bit patterns. Values with
            // the high bit set are negative Java longs and remain valid.
            if generation <= 0 || operation_id == 0 || result_kind < 0 {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            }
            let Ok(bytes) = copy_array(&env, &payload, FLOAT_VIEW_MAX_COMPLETION_PAYLOAD) else {
                return FLOAT_VIEW_STATUS_INVALID_ARGUMENT;
            };
            // SAFETY: `bytes` remains readable for the call and the ABI copies it.
            unsafe {
                float_view_runtime_complete_operation(
                    runtime,
                    generation as u64,
                    operation_id as u64,
                    result_kind as u32,
                    bytes.as_ptr(),
                    bytes.len(),
                )
            }
        })
    }

    extern "system" fn stop(_env: JNIEnv<'_>, _class: JClass<'_>, raw: jlong) -> jint {
        status_boundary(|| {
            valid_handle(raw)
                .map_or_else(|status| status, |runtime| float_view_runtime_stop(runtime))
        })
    }

    extern "system" fn destroy(_env: JNIEnv<'_>, _class: JClass<'_>, raw: jlong) -> jint {
        status_boundary(|| {
            valid_handle(raw).map_or_else(
                |status| status,
                |runtime| float_view_runtime_destroy(runtime),
            )
        })
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn JNI_OnLoad(vm: *mut jni::sys::JavaVM, _: *mut c_void) -> jint {
        let registered = std::panic::catch_unwind(|| {
            // SAFETY: Android passes a valid process JavaVM for the duration of load.
            let vm = unsafe { JavaVM::from_raw(vm) }.map_err(|_| ())?;
            let mut env = vm.get_env().map_err(|_| ())?;
            let class = env
                .find_class("fail/acheron/floatview/core/NativeCore")
                .map_err(|_| ())?;
            let methods = [
                NativeMethod {
                    name: "abiVersion".into(),
                    sig: "()I".into(),
                    fn_ptr: abi_version as *mut c_void,
                },
                NativeMethod {
                    name: "buildIdentity".into(),
                    sig: "([B)I".into(),
                    fn_ptr: build_identity as *mut c_void,
                },
                NativeMethod {
                    name: "create".into(),
                    sig: "()J".into(),
                    fn_ptr: create as *mut c_void,
                },
                NativeMethod {
                    name: "start".into(),
                    sig: "(JI[J)I".into(),
                    fn_ptr: start as *mut c_void,
                },
                NativeMethod {
                    name: "startConfigured".into(),
                    sig: "(JII[B[B[J)I".into(),
                    fn_ptr: start_configured as *mut c_void,
                },
                NativeMethod {
                    name: "submit".into(),
                    sig: "(JJI[B)I".into(),
                    fn_ptr: submit as *mut c_void,
                },
                NativeMethod {
                    name: "submitBle".into(),
                    sig: "(JJJJ[B)I".into(),
                    fn_ptr: submit_ble as *mut c_void,
                },
                NativeMethod {
                    name: "submitBluetoothState".into(),
                    sig: "(JJJJII)I".into(),
                    fn_ptr: submit_bluetooth_state as *mut c_void,
                },
                NativeMethod {
                    name: "submitBluetoothDevice".into(),
                    sig: "(JJJJ[B[BI)I".into(),
                    fn_ptr: submit_bluetooth_device as *mut c_void,
                },
                NativeMethod {
                    name: "submitBluetoothConnection".into(),
                    sig: "(JJJJI)I".into(),
                    fn_ptr: submit_bluetooth_connection as *mut c_void,
                },
                NativeMethod {
                    name: "submitLocationAuthorization".into(),
                    sig: "(JJJJI)I".into(),
                    fn_ptr: submit_location_authorization as *mut c_void,
                },
                NativeMethod {
                    name: "submitLocation".into(),
                    sig: "(JJJJDDDZ)I".into(),
                    fn_ptr: submit_location as *mut c_void,
                },
                NativeMethod {
                    name: "submitLifecycle".into(),
                    sig: "(JJJJZZ)I".into(),
                    fn_ptr: submit_lifecycle as *mut c_void,
                },
                NativeMethod {
                    name: "waitCommand".into(),
                    sig: "(JJI[J[B)I".into(),
                    fn_ptr: wait_command as *mut c_void,
                },
                NativeMethod {
                    name: "complete".into(),
                    sig: "(JJJI[B)I".into(),
                    fn_ptr: complete as *mut c_void,
                },
                NativeMethod {
                    name: "stop".into(),
                    sig: "(J)I".into(),
                    fn_ptr: stop as *mut c_void,
                },
                NativeMethod {
                    name: "destroy".into(),
                    sig: "(J)I".into(),
                    fn_ptr: destroy as *mut c_void,
                },
            ];
            env.register_native_methods(class, &methods)
                .map_err(|_| ())?;
            Ok::<(), ()>(())
        });
        if matches!(registered, Ok(Ok(()))) {
            JNI_VERSION_1_6
        } else {
            JNI_ERR
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn facade_links_the_shared_runtime() {
        assert_eq!(super::linked_abi_version(), 4);
        assert_eq!(super::linked_build_identity(), b"float-view-abi04");
    }
}
