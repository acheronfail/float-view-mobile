//! Narrow caller-owned-buffer C ABI for bounded Refloat live controls.

#![deny(unsafe_op_in_unsafe_fn)]

use float_view_apple_ffi_support::{
    Region, contain_status, input_bytes, publish_bytes, validate_disjoint, write_scalar,
};
use float_view_refloat_control::{
    ControlError, ControlProfile, LightingProfile, LightingState, LightingTarget,
    MAX_CONTROL_PAYLOAD_BYTES, decode_lighting_response, fatal_clear_request, flywheel_request,
    lighting_request, package_request,
};

pub const ABI_VERSION: u32 = 1;
pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
pub const STATUS_INVALID_PAYLOAD: u32 = 2;
pub const STATUS_UNSUPPORTED_COMMAND: u32 = 3;
pub const STATUS_OUTPUT_TOO_SMALL: u32 = 4;
pub const STATUS_INTERNAL_FAILURE: u32 = 5;

pub const OPERATION_PACKAGE: u8 = 1;
pub const OPERATION_FLYWHEEL: u8 = 2;
pub const OPERATION_LIGHTING: u8 = 3;
pub const OPERATION_FATAL_CLEAR: u8 = 4;

pub const LIGHTING_PROFILE_NONE: u8 = 0;
pub const LIGHTING_PROFILE_LEGACY: u8 = 1;
pub const LIGHTING_PROFILE_STABLE: u8 = 2;

pub const LIGHTING_TARGET_NONE: u8 = 0;
pub const LIGHTING_TARGET_LEDS: u8 = 1;
pub const LIGHTING_TARGET_HEADLIGHTS: u8 = 2;

#[unsafe(no_mangle)]
pub extern "C" fn float_view_refloat_control_abi_version() -> u32 {
    ABI_VERSION
}

/// Encode one Refloat control into a caller-owned output buffer.
///
/// # Safety
/// Non-empty output must be valid for its capacity. Scalar outputs must be
/// valid, aligned, pairwise disjoint from the output and each other, and live
/// for the call. Rust retains nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_control_encode(
    operation: u8,
    lighting_profile: u8,
    fatal_clear_available: u8,
    target: u8,
    enabled: u8,
    current_flags: u8,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        validate_encode_regions(output, output_capacity, output_length, unsupported_command)?;
        // SAFETY: validation established two distinct writable scalar regions.
        unsafe {
            write_scalar(output_length, 0);
            write_scalar(unsupported_command, 0);
        }

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
                    && current_flags <= 3
                    && (target != LIGHTING_TARGET_NONE || enabled == 0) =>
            {
                let profile = parse_lighting_profile(lighting_profile)?;
                let target = parse_lighting_target(target)?;
                lighting_request(profile, target, enabled != 0, state(current_flags))
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
                        // SAFETY: validation established a writable byte output.
                        unsafe { write_scalar(unsupported_command, command) };
                        return Err(STATUS_UNSUPPORTED_COMMAND);
                    }
                    Err(ControlError::InvalidPayload) => return Err(STATUS_INTERNAL_FAILURE),
                }
            }
            _ => return Err(STATUS_INVALID_ARGUMENT),
        };

        // SAFETY: validation established the bounded caller-owned regions.
        unsafe { publish_bytes(payload.as_slice(), output, output_capacity, output_length) }
            .map_err(|_| STATUS_OUTPUT_TOO_SMALL)
    })
}

/// Decode one exact Refloat lighting response into its two defined flag bits.
///
/// # Safety
/// Non-empty input must be valid for its length. `lighting_flags` must be a
/// valid writable byte disjoint from input and live for the call. Rust retains
/// nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn float_view_refloat_control_decode_lighting(
    payload: *const u8,
    payload_length: usize,
    lighting_profile: u8,
    lighting_flags: *mut u8,
) -> u32 {
    contain_status(STATUS_OK, STATUS_INTERNAL_FAILURE, || {
        validate_decode_regions(payload, payload_length, lighting_flags)?;
        // SAFETY: validation established a writable result byte.
        unsafe { write_scalar(lighting_flags, 0) };
        if payload_length > 4 {
            return Err(STATUS_INVALID_PAYLOAD);
        }
        let profile = parse_lighting_profile(lighting_profile)?;
        // SAFETY: validation established a bounded readable input region.
        let payload = unsafe { input_bytes(payload, payload_length) };
        let decoded = decode_lighting_response(payload, profile).map_err(|error| match error {
            ControlError::InvalidPayload => STATUS_INVALID_PAYLOAD,
            ControlError::UnsupportedCommand(_) => STATUS_INTERNAL_FAILURE,
        })?;
        let flags = (decoded.leds_enabled as u8) | ((decoded.headlights_enabled as u8) << 1);
        // SAFETY: validation established a writable result byte.
        unsafe { write_scalar(lighting_flags, flags) };
        Ok(())
    })
}

fn canonical_common(profile: u8, fatal: u8, target: u8, current: u8) -> bool {
    profile == LIGHTING_PROFILE_NONE && fatal == 0 && target == LIGHTING_TARGET_NONE && current == 0
}

fn is_bool(value: u8) -> bool {
    value <= 1
}

fn parse_lighting_profile(value: u8) -> Result<LightingProfile, u32> {
    match value {
        LIGHTING_PROFILE_LEGACY => Ok(LightingProfile::LegacyCommand202),
        LIGHTING_PROFILE_STABLE => Ok(LightingProfile::StableCommand20),
        _ => Err(STATUS_INVALID_ARGUMENT),
    }
}

fn parse_lighting_target(value: u8) -> Result<Option<LightingTarget>, u32> {
    match value {
        LIGHTING_TARGET_NONE => Ok(None),
        LIGHTING_TARGET_LEDS => Ok(Some(LightingTarget::Leds)),
        LIGHTING_TARGET_HEADLIGHTS => Ok(Some(LightingTarget::Headlights)),
        _ => Err(STATUS_INVALID_ARGUMENT),
    }
}

fn state(flags: u8) -> LightingState {
    LightingState {
        leds_enabled: flags & 1 != 0,
        headlights_enabled: flags & 2 != 0,
    }
}

fn validate_encode_regions(
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
    unsupported_command: *mut u8,
) -> Result<(), u32> {
    if output_capacity > MAX_CONTROL_PAYLOAD_BYTES {
        return Err(STATUS_INVALID_ARGUMENT);
    }
    let regions = [
        Region::optional(output.cast_const(), output_capacity)
            .map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(output_length.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(unsupported_command.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
    ];
    validate_disjoint(&regions).map_err(|_| STATUS_INVALID_ARGUMENT)
}

fn validate_decode_regions(
    input: *const u8,
    input_length: usize,
    lighting_flags: *mut u8,
) -> Result<(), u32> {
    let regions = [
        Region::optional(input, input_length).map_err(|_| STATUS_INVALID_ARGUMENT)?,
        Region::required(lighting_flags.cast_const()).map_err(|_| STATUS_INVALID_ARGUMENT)?,
    ];
    validate_disjoint(&regions).map_err(|_| STATUS_INVALID_ARGUMENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn exact_operations_and_all_lighting_states_cross_the_abi() {
        assert_eq!(
            encode(OPERATION_PACKAGE, 0, 0, 0, 1, 0).unwrap(),
            [36, 101, 12, 0]
        );
        assert_eq!(
            encode(OPERATION_PACKAGE, 0, 0, 0, 0, 0).unwrap(),
            [36, 101, 12, 1]
        );
        assert_eq!(
            encode(OPERATION_FLYWHEEL, 0, 0, 0, 1, 0).unwrap(),
            [36, 101, 22, 0x81, 0, 0, 0, 0, 1]
        );
        assert_eq!(
            encode(OPERATION_FATAL_CLEAR, 0, 1, 0, 0, 0).unwrap(),
            [36, 101, 36, 1]
        );

        for profile in [LIGHTING_PROFILE_LEGACY, LIGHTING_PROFILE_STABLE] {
            for current in 0..=3 {
                assert_lighting(profile, LIGHTING_TARGET_NONE, 0, current, 0, current);
                for target in [LIGHTING_TARGET_LEDS, LIGHTING_TARGET_HEADLIGHTS] {
                    for enabled in 0..=1 {
                        let bit = target;
                        let expected = if enabled == 1 {
                            current | bit
                        } else {
                            current & !bit
                        };
                        assert_lighting(profile, target, enabled, current, bit, expected);
                    }
                }
            }
            let command = if profile == LIGHTING_PROFILE_LEGACY {
                202
            } else {
                20
            };
            for flags in 0..=3 {
                let mut result = u8::MAX;
                let response = [36, 101, command, flags];
                let status = unsafe {
                    float_view_refloat_control_decode_lighting(
                        response.as_ptr(),
                        response.len(),
                        profile,
                        &mut result,
                    )
                };
                assert_eq!(status, STATUS_OK);
                assert_eq!(result, flags);
            }
        }
    }

    #[test]
    fn invalid_primitives_and_unsupported_control_fail_closed() {
        for arguments in [
            (0, 0, 0, 0, 0, 0),
            (5, 0, 0, 0, 0, 0),
            (OPERATION_PACKAGE, 1, 0, 0, 0, 0),
            (OPERATION_PACKAGE, 0, 1, 0, 0, 0),
            (OPERATION_PACKAGE, 0, 0, 1, 0, 0),
            (OPERATION_PACKAGE, 0, 0, 0, 2, 0),
            (OPERATION_FLYWHEEL, 0, 0, 0, 1, 1),
            (OPERATION_LIGHTING, 0, 0, 0, 0, 0),
            (OPERATION_LIGHTING, 3, 0, 0, 0, 0),
            (OPERATION_LIGHTING, 1, 1, 0, 0, 0),
            (OPERATION_LIGHTING, 1, 0, 3, 0, 0),
            (OPERATION_LIGHTING, 1, 0, 0, 1, 0),
            (OPERATION_LIGHTING, 1, 0, 1, 2, 0),
            (OPERATION_LIGHTING, 1, 0, 1, 0, 4),
            (OPERATION_FATAL_CLEAR, 1, 1, 0, 0, 0),
            (OPERATION_FATAL_CLEAR, 0, 2, 0, 0, 0),
            (OPERATION_FATAL_CLEAR, 0, 1, 1, 0, 0),
        ] {
            assert_eq!(
                encode_status(arguments),
                STATUS_INVALID_ARGUMENT,
                "{arguments:?}"
            );
        }

        let mut output = [0xAA; MAX_CONTROL_PAYLOAD_BYTES];
        let mut length = usize::MAX;
        let mut unsupported = u8::MAX;
        let status = unsafe {
            float_view_refloat_control_encode(
                OPERATION_FATAL_CLEAR,
                0,
                0,
                0,
                0,
                0,
                output.as_mut_ptr(),
                output.len(),
                &mut length,
                &mut unsupported,
            )
        };
        assert_eq!(status, STATUS_UNSUPPORTED_COMMAND);
        assert_eq!(length, 0);
        assert_eq!(unsupported, 36);
        assert_eq!(output, [0xAA; MAX_CONTROL_PAYLOAD_BYTES]);
    }

    #[test]
    fn malformed_responses_and_result_regions_are_bounded() {
        for payload in [
            vec![],
            vec![36],
            vec![36, 101],
            vec![36, 101, 20],
            vec![36, 101, 20, 0, 0],
            vec![35, 101, 20, 0],
            vec![36, 100, 20, 0],
            vec![36, 101, 202, 0],
            vec![36, 101, 20, 4],
            vec![36, 101, 20, 255],
        ] {
            let mut flags = u8::MAX;
            let status = unsafe {
                float_view_refloat_control_decode_lighting(
                    payload.as_ptr(),
                    payload.len(),
                    LIGHTING_PROFILE_STABLE,
                    &mut flags,
                )
            };
            assert_eq!(status, STATUS_INVALID_PAYLOAD, "length {}", payload.len());
            assert_eq!(flags, 0);
        }

        let response = [36, 101, 20, 3];
        let mut flags = u8::MAX;
        assert_eq!(
            unsafe {
                float_view_refloat_control_decode_lighting(response.as_ptr(), 4, 0, &mut flags)
            },
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(flags, 0);
        assert_eq!(
            unsafe {
                float_view_refloat_control_decode_lighting(response.as_ptr(), 4, 2, ptr::null_mut())
            },
            STATUS_INVALID_ARGUMENT
        );
        let mut overlap = response;
        assert_eq!(
            unsafe {
                float_view_refloat_control_decode_lighting(
                    overlap.as_ptr(),
                    overlap.len(),
                    2,
                    overlap.as_mut_ptr().add(1),
                )
            },
            STATUS_INVALID_ARGUMENT
        );
    }

    #[test]
    fn output_capacity_null_overlap_alignment_and_panics_are_contained() {
        let mut length = usize::MAX;
        let mut unsupported = u8::MAX;
        assert_eq!(
            unsafe {
                float_view_refloat_control_encode(
                    OPERATION_FLYWHEEL,
                    0,
                    0,
                    0,
                    1,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut length,
                    &mut unsupported,
                )
            },
            STATUS_OUTPUT_TOO_SMALL
        );
        assert_eq!(length, 9);
        assert_eq!(unsupported, 0);
        let mut output = [0_u8; MAX_CONTROL_PAYLOAD_BYTES + 1];
        assert_eq!(
            unsafe {
                float_view_refloat_control_encode(
                    OPERATION_PACKAGE,
                    0,
                    0,
                    0,
                    1,
                    0,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut length,
                    &mut unsupported,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            unsafe {
                float_view_refloat_control_encode(
                    OPERATION_PACKAGE,
                    0,
                    0,
                    0,
                    1,
                    0,
                    output.as_mut_ptr(),
                    9,
                    output.as_mut_ptr().cast(),
                    &mut unsupported,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
        let mut scalar_bytes = [0_u8; size_of::<usize>() + 1];
        assert_eq!(
            unsafe {
                float_view_refloat_control_encode(
                    OPERATION_PACKAGE,
                    0,
                    0,
                    0,
                    1,
                    0,
                    output.as_mut_ptr(),
                    9,
                    scalar_bytes.as_mut_ptr().add(1).cast(),
                    &mut unsupported,
                )
            },
            STATUS_INVALID_ARGUMENT
        );
    }

    fn encode(
        operation: u8,
        profile: u8,
        fatal: u8,
        target: u8,
        enabled: u8,
        current: u8,
    ) -> Result<Vec<u8>, u32> {
        let mut output = [0_u8; MAX_CONTROL_PAYLOAD_BYTES];
        let mut length = usize::MAX;
        let mut unsupported = u8::MAX;
        let status = unsafe {
            float_view_refloat_control_encode(
                operation,
                profile,
                fatal,
                target,
                enabled,
                current,
                output.as_mut_ptr(),
                output.len(),
                &mut length,
                &mut unsupported,
            )
        };
        if status == STATUS_OK {
            assert!(length <= output.len());
            assert_eq!(unsupported, 0);
            Ok(output[..length].to_vec())
        } else {
            Err(status)
        }
    }

    fn encode_status(arguments: (u8, u8, u8, u8, u8, u8)) -> u32 {
        encode(
            arguments.0,
            arguments.1,
            arguments.2,
            arguments.3,
            arguments.4,
            arguments.5,
        )
        .unwrap_err()
    }

    fn assert_lighting(profile: u8, target: u8, enabled: u8, current: u8, mask: u8, flags: u8) {
        let actual = encode(OPERATION_LIGHTING, profile, 0, target, enabled, current).unwrap();
        let expected = if profile == LIGHTING_PROFILE_LEGACY {
            vec![36, 101, 202, mask, flags]
        } else {
            vec![36, 101, 20, 0, 0, 0, mask, flags]
        };
        assert_eq!(actual, expected);
    }
}
