//! Narrow Android JNI ABI for bounded Refloat live controls.
//!
//! Protocol decisions remain in the safe `float-view-refloat-control` crate.
//! This crate owns only primitive validation and call-local JNI copies.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_refloat_control::{
    ControlError, ControlProfile, LightingProfile, LightingState, LightingTarget,
    MAX_CONTROL_PAYLOAD_BYTES, decode_lighting_response, fatal_clear_request, flywheel_request,
    lighting_request, package_request,
};
use jni_sys::{JNIEnv, jbyte, jbyteArray, jint, jintArray, jobject, jsize};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub const ABI_VERSION: jint = 1;
pub const STATUS_OK: jint = 0;
pub const STATUS_INVALID_ARGUMENT: jint = 1;
pub const STATUS_INVALID_PAYLOAD: jint = 2;
pub const STATUS_UNSUPPORTED_COMMAND: jint = 3;
pub const STATUS_OUTPUT_TOO_SMALL: jint = 4;
pub const STATUS_INTERNAL_FAILURE: jint = 5;

pub const OPERATION_PACKAGE: jint = 1;
pub const OPERATION_FLYWHEEL: jint = 2;
pub const OPERATION_LIGHTING: jint = 3;
pub const OPERATION_FATAL_CLEAR: jint = 4;

pub const LIGHTING_PROFILE_NONE: jint = 0;
pub const LIGHTING_PROFILE_LEGACY: jint = 1;
pub const LIGHTING_PROFILE_STABLE: jint = 2;

pub const LIGHTING_TARGET_NONE: jint = 0;
pub const LIGHTING_TARGET_LEDS: jint = 1;
pub const LIGHTING_TARGET_HEADLIGHTS: jint = 2;

#[derive(Debug, Eq, PartialEq)]
struct EncodeResult {
    status: jint,
    payload: Vec<u8>,
    required_length: usize,
    unsupported_command: u8,
}

impl EncodeResult {
    fn failure(status: jint) -> Self {
        Self {
            status,
            payload: Vec::new(),
            required_length: 0,
            unsupported_command: 0,
        }
    }
}

/// Return the exact hand-written JNI contract version.
///
/// # Safety
/// `env` must be null or the JNI environment supplied for this call.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatControlBridge_abiVersion(
    env: *mut JNIEnv,
    _receiver: jobject,
) -> jint {
    contain_abi(|| {
        let Some(functions) = (unsafe { environment(env) }) else {
            return 0;
        };
        if unsafe { (functions.ExceptionCheck)(env) } {
            0
        } else {
            ABI_VERSION
        }
    })
}

/// Encode one control into JVM-owned output and result arrays.
///
/// # Safety
/// References must belong to `env`, have their declared primitive-array types,
/// and remain valid for this call. Rust retains no reference.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatControlBridge_encode(
    env: *mut JNIEnv,
    _receiver: jobject,
    operation: jint,
    lighting_profile: jint,
    fatal_clear_available: jint,
    target: jint,
    enabled: jint,
    current_flags: jint,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    contain_status(|| unsafe {
        invoke_encode(
            env,
            operation,
            lighting_profile,
            fatal_clear_available,
            target,
            enabled,
            current_flags,
            output,
            result,
        )
    })
}

/// Decode one lighting response into a JVM-owned integer result.
///
/// # Safety
/// References must belong to `env`, have their declared primitive-array types,
/// and remain valid for this call. Rust retains no reference.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_fail_acheron_floatview_protocol_RustRefloatControlBridge_decodeLighting(
    env: *mut JNIEnv,
    _receiver: jobject,
    input: jbyteArray,
    lighting_profile: jint,
    result: jintArray,
) -> jint {
    contain_status(|| unsafe { invoke_decode(env, input, lighting_profile, result) })
}

#[allow(clippy::too_many_arguments)]
unsafe fn invoke_encode(
    env: *mut JNIEnv,
    operation: jint,
    lighting_profile: jint,
    fatal_clear_available: jint,
    target: jint,
    enabled: jint,
    current_flags: jint,
    output: jbyteArray,
    result: jintArray,
) -> jint {
    let Some(functions) = (unsafe { environment(env) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if unsafe { (functions.ExceptionCheck)(env) } || output.is_null() || result.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let output_capacity = unsafe { (functions.GetArrayLength)(env, output.cast()) };
    let result_length = unsafe { (functions.GetArrayLength)(env, result.cast()) };
    if unsafe { (functions.ExceptionCheck)(env) }
        || output_capacity < 0
        || output_capacity as usize > MAX_CONTROL_PAYLOAD_BYTES
        || result_length != 2
        || unsafe { references_alias(env, functions, output.cast(), result.cast()) }
        || unsafe { (functions.ExceptionCheck)(env) }
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if write_ints(env, functions, result, &[0, 0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }

    let encoded = run_encode(
        operation,
        lighting_profile,
        fatal_clear_available,
        target,
        enabled,
        current_flags,
        output_capacity as usize,
    );
    if encoded.status == STATUS_OK {
        unsafe {
            (functions.SetByteArrayRegion)(
                env,
                output,
                0,
                encoded.payload.len() as jsize,
                encoded.payload.as_ptr().cast::<jbyte>(),
            )
        };
        if unsafe { (functions.ExceptionCheck)(env) } {
            return STATUS_INTERNAL_FAILURE;
        }
    }
    if write_ints(
        env,
        functions,
        result,
        &[
            encoded.required_length as jint,
            jint::from(encoded.unsupported_command),
        ],
    )
    .is_err()
    {
        return STATUS_INTERNAL_FAILURE;
    }
    encoded.status
}

unsafe fn invoke_decode(
    env: *mut JNIEnv,
    input: jbyteArray,
    lighting_profile: jint,
    result: jintArray,
) -> jint {
    let Some(functions) = (unsafe { environment(env) }) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if unsafe { (functions.ExceptionCheck)(env) } || input.is_null() || result.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let input_length = unsafe { (functions.GetArrayLength)(env, input.cast()) };
    let result_length = unsafe { (functions.GetArrayLength)(env, result.cast()) };
    if unsafe { (functions.ExceptionCheck)(env) }
        || input_length < 0
        || result_length != 1
        || unsafe { references_alias(env, functions, input.cast(), result.cast()) }
        || unsafe { (functions.ExceptionCheck)(env) }
    {
        return STATUS_INVALID_ARGUMENT;
    }
    if write_ints(env, functions, result, &[0]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    if input_length > 4 {
        return STATUS_INVALID_PAYLOAD;
    }
    let mut copied_input = vec![0_u8; input_length as usize];
    unsafe {
        (functions.GetByteArrayRegion)(
            env,
            input,
            0,
            input_length,
            copied_input.as_mut_ptr().cast::<jbyte>(),
        )
    };
    if unsafe { (functions.ExceptionCheck)(env) } {
        return STATUS_INTERNAL_FAILURE;
    }
    let (status, flags) = run_decode(&copied_input, lighting_profile);
    if status == STATUS_OK && write_ints(env, functions, result, &[flags]).is_err() {
        return STATUS_INTERNAL_FAILURE;
    }
    status
}

#[allow(clippy::too_many_arguments)]
fn run_encode(
    operation: jint,
    lighting_profile: jint,
    fatal_clear_available: jint,
    target: jint,
    enabled: jint,
    current_flags: jint,
    output_capacity: usize,
) -> EncodeResult {
    contain_encode(|| {
        let payload = match operation {
            OPERATION_PACKAGE
                if canonical_common(
                    lighting_profile,
                    fatal_clear_available,
                    target,
                    current_flags,
                ) && is_bool(enabled) =>
            {
                package_request(enabled != 0)
            }
            OPERATION_FLYWHEEL
                if canonical_common(
                    lighting_profile,
                    fatal_clear_available,
                    target,
                    current_flags,
                ) && is_bool(enabled) =>
            {
                flywheel_request(enabled != 0)
            }
            OPERATION_LIGHTING
                if fatal_clear_available == 0
                    && is_bool(enabled)
                    && (0..=3).contains(&current_flags)
                    && (target != LIGHTING_TARGET_NONE || enabled == 0) =>
            {
                lighting_request(
                    parse_profile(lighting_profile)?,
                    parse_target(target)?,
                    enabled != 0,
                    state(current_flags),
                )
            }
            OPERATION_FATAL_CLEAR
                if lighting_profile == LIGHTING_PROFILE_NONE
                    && is_bool(fatal_clear_available)
                    && target == LIGHTING_TARGET_NONE
                    && enabled == 0
                    && current_flags == 0 =>
            {
                match fatal_clear_request(ControlProfile {
                    lighting: LightingProfile::StableCommand20,
                    fatal_clear_available: fatal_clear_available != 0,
                }) {
                    Ok(payload) => payload,
                    Err(ControlError::UnsupportedCommand(command)) => {
                        return Ok(EncodeResult {
                            status: STATUS_UNSUPPORTED_COMMAND,
                            payload: Vec::new(),
                            required_length: 0,
                            unsupported_command: command,
                        });
                    }
                    Err(ControlError::InvalidPayload) => return Err(STATUS_INTERNAL_FAILURE),
                }
            }
            _ => return Err(STATUS_INVALID_ARGUMENT),
        };
        let required_length = payload.len();
        if required_length > output_capacity {
            return Ok(EncodeResult {
                status: STATUS_OUTPUT_TOO_SMALL,
                payload: Vec::new(),
                required_length,
                unsupported_command: 0,
            });
        }
        Ok(EncodeResult {
            status: STATUS_OK,
            payload: payload.as_slice().to_vec(),
            required_length,
            unsupported_command: 0,
        })
    })
}

fn run_decode(input: &[u8], lighting_profile: jint) -> (jint, jint) {
    contain_decode(|| {
        if input.len() > 4 {
            return Err(STATUS_INVALID_PAYLOAD);
        }
        let state = decode_lighting_response(input, parse_profile(lighting_profile)?)
            .map_err(|_| STATUS_INVALID_PAYLOAD)?;
        Ok(jint::from(state.leds_enabled) | (jint::from(state.headlights_enabled) << 1))
    })
}

fn canonical_common(profile: jint, fatal: jint, target: jint, current: jint) -> bool {
    profile == LIGHTING_PROFILE_NONE && fatal == 0 && target == LIGHTING_TARGET_NONE && current == 0
}

fn is_bool(value: jint) -> bool {
    (0..=1).contains(&value)
}

fn parse_profile(value: jint) -> Result<LightingProfile, jint> {
    match value {
        LIGHTING_PROFILE_LEGACY => Ok(LightingProfile::LegacyCommand202),
        LIGHTING_PROFILE_STABLE => Ok(LightingProfile::StableCommand20),
        _ => Err(STATUS_INVALID_ARGUMENT),
    }
}

fn parse_target(value: jint) -> Result<Option<LightingTarget>, jint> {
    match value {
        LIGHTING_TARGET_NONE => Ok(None),
        LIGHTING_TARGET_LEDS => Ok(Some(LightingTarget::Leds)),
        LIGHTING_TARGET_HEADLIGHTS => Ok(Some(LightingTarget::Headlights)),
        _ => Err(STATUS_INVALID_ARGUMENT),
    }
}

fn state(flags: jint) -> LightingState {
    LightingState {
        leds_enabled: flags & 1 != 0,
        headlights_enabled: flags & 2 != 0,
    }
}

fn contain_encode(operation: impl FnOnce() -> Result<EncodeResult, jint>) -> EncodeResult {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(result)) => result,
        Ok(Err(status)) => EncodeResult::failure(status),
        Err(_) => EncodeResult::failure(STATUS_INTERNAL_FAILURE),
    }
}

fn contain_decode(operation: impl FnOnce() -> Result<jint, jint>) -> (jint, jint) {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(flags)) => (STATUS_OK, flags),
        Ok(Err(status)) => (status, 0),
        Err(_) => (STATUS_INTERNAL_FAILURE, 0),
    }
}

fn contain_status(operation: impl FnOnce() -> jint) -> jint {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(STATUS_INTERNAL_FAILURE)
}

fn contain_abi(operation: impl FnOnce() -> jint) -> jint {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(0)
}

unsafe fn environment<'a>(env: *mut JNIEnv) -> Option<&'a jni_sys::JNINativeInterface__1_2> {
    if env.is_null() {
        return None;
    }
    let functions = unsafe { *env };
    if functions.is_null() {
        return None;
    }
    Some(unsafe { &(*functions).v1_2 })
}

unsafe fn references_alias(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    left: jobject,
    right: jobject,
) -> bool {
    unsafe { (functions.IsSameObject)(env, left, right) }
}

fn write_ints(
    env: *mut JNIEnv,
    functions: &jni_sys::JNINativeInterface__1_2,
    result: jintArray,
    values: &[jint],
) -> Result<(), ()> {
    unsafe {
        (functions.SetIntArrayRegion)(env, result, 0, values.len() as jsize, values.as_ptr())
    };
    if unsafe { (functions.ExceptionCheck)(env) } {
        Err(())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(
        operation: jint,
        profile: jint,
        fatal: jint,
        target: jint,
        enabled: jint,
        flags: jint,
    ) -> EncodeResult {
        run_encode(
            operation,
            profile,
            fatal,
            target,
            enabled,
            flags,
            MAX_CONTROL_PAYLOAD_BYTES,
        )
    }

    #[test]
    fn encodes_all_control_operations_exactly() {
        assert_eq!(
            encode(OPERATION_PACKAGE, 0, 0, 0, 1, 0).payload,
            [36, 101, 12, 0]
        );
        assert_eq!(
            encode(OPERATION_PACKAGE, 0, 0, 0, 0, 0).payload,
            [36, 101, 12, 1]
        );
        assert_eq!(
            encode(OPERATION_FLYWHEEL, 0, 0, 0, 1, 0).payload,
            [36, 101, 22, 129, 0, 0, 0, 0, 1]
        );
        assert_eq!(
            encode(OPERATION_FLYWHEEL, 0, 0, 0, 0, 0).payload,
            [36, 101, 22, 128, 0, 0, 0, 0, 1]
        );
        assert_eq!(
            encode(OPERATION_FATAL_CLEAR, 0, 1, 0, 0, 0).payload,
            [36, 101, 36, 1]
        );
    }

    #[test]
    fn encodes_exhaustive_lighting_matrix() {
        for profile in [LIGHTING_PROFILE_LEGACY, LIGHTING_PROFILE_STABLE] {
            for flags in 0..=3 {
                for target in [
                    LIGHTING_TARGET_NONE,
                    LIGHTING_TARGET_LEDS,
                    LIGHTING_TARGET_HEADLIGHTS,
                ] {
                    for enabled in 0..=1 {
                        let result = encode(OPERATION_LIGHTING, profile, 0, target, enabled, flags);
                        if target == LIGHTING_TARGET_NONE && enabled == 1 {
                            assert_eq!(result.status, STATUS_INVALID_ARGUMENT);
                        } else {
                            assert_eq!(result.status, STATUS_OK);
                            let mask = target as u8;
                            let expected_flags = match target {
                                LIGHTING_TARGET_LEDS if enabled == 1 => flags | 1,
                                LIGHTING_TARGET_LEDS => flags & !1,
                                LIGHTING_TARGET_HEADLIGHTS if enabled == 1 => flags | 2,
                                LIGHTING_TARGET_HEADLIGHTS => flags & !2,
                                _ => flags,
                            };
                            let tail = &result.payload[result.payload.len() - 2..];
                            assert_eq!(tail, &[mask, expected_flags as u8]);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn decodes_every_defined_state_and_rejects_malformed_responses() {
        for (profile, command) in [
            (LIGHTING_PROFILE_LEGACY, 202),
            (LIGHTING_PROFILE_STABLE, 20),
        ] {
            for flags in 0..=3 {
                assert_eq!(
                    run_decode(&[36, 101, command, flags as u8], profile),
                    (STATUS_OK, flags)
                );
            }
            for payload in [
                vec![],
                vec![36, 101, command],
                vec![36, 101, command, 4],
                vec![36, 101, command, 0, 0],
                vec![35, 101, command, 0],
            ] {
                assert_eq!(run_decode(&payload, profile), (STATUS_INVALID_PAYLOAD, 0));
            }
        }
        assert_eq!(
            run_decode(&[36, 101, 20, 0], 0),
            (STATUS_INVALID_ARGUMENT, 0)
        );
    }

    #[test]
    fn rejects_noncanonical_inputs_and_reports_capacity_and_unsupported() {
        for invalid in [-1, 2, 256] {
            assert_eq!(
                encode(OPERATION_PACKAGE, 0, 0, 0, invalid, 0).status,
                STATUS_INVALID_ARGUMENT
            );
        }
        assert_eq!(encode(0, 0, 0, 0, 0, 0).status, STATUS_INVALID_ARGUMENT);
        assert_eq!(
            encode(OPERATION_LIGHTING, 0, 0, 1, 0, 0).status,
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            encode(OPERATION_LIGHTING, 1, 1, 1, 0, 0).status,
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            encode(OPERATION_LIGHTING, 1, 0, 3, 0, 0).status,
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            encode(OPERATION_LIGHTING, 1, 0, 1, 0, 4).status,
            STATUS_INVALID_ARGUMENT
        );
        let unavailable = encode(OPERATION_FATAL_CLEAR, 0, 0, 0, 0, 0);
        assert_eq!(
            (unavailable.status, unavailable.unsupported_command),
            (STATUS_UNSUPPORTED_COMMAND, 36)
        );
        let small = run_encode(OPERATION_FLYWHEEL, 0, 0, 0, 1, 0, 8);
        assert_eq!(
            (small.status, small.required_length),
            (STATUS_OUTPUT_TOO_SMALL, 9)
        );
        assert!(small.payload.is_empty());
    }

    #[test]
    fn contains_panics_and_keeps_contract_numbers_stable() {
        assert_eq!(
            contain_encode(|| panic!("contained")),
            EncodeResult::failure(STATUS_INTERNAL_FAILURE)
        );
        assert_eq!(
            contain_decode(|| panic!("contained")),
            (STATUS_INTERNAL_FAILURE, 0)
        );
        assert_eq!(
            contain_status(|| panic!("contained")),
            STATUS_INTERNAL_FAILURE
        );
        assert_eq!(contain_abi(|| panic!("contained")), 0);
        assert_eq!(
            [
                ABI_VERSION,
                STATUS_OK,
                STATUS_INVALID_ARGUMENT,
                STATUS_INVALID_PAYLOAD,
                STATUS_UNSUPPORTED_COMMAND,
                STATUS_OUTPUT_TOO_SMALL,
                STATUS_INTERNAL_FAILURE
            ],
            [1, 0, 1, 2, 3, 4, 5]
        );
    }
}
