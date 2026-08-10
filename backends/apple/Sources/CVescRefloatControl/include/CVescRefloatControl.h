#ifndef FLOAT_VIEW_C_VESC_REFLOAT_CONTROL_H
#define FLOAT_VIEW_C_VESC_REFLOAT_CONTROL_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_VIEW_REFLOAT_CONTROL_ABI_VERSION UINT32_C(1)
#define FLOAT_VIEW_REFLOAT_CONTROL_MAX_PAYLOAD_BYTES ((size_t)9)

typedef uint32_t FloatViewRefloatControlStatus;
enum {
  FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OK = 0,
  FLOAT_VIEW_REFLOAT_CONTROL_STATUS_INVALID_ARGUMENT = 1,
  FLOAT_VIEW_REFLOAT_CONTROL_STATUS_INVALID_PAYLOAD = 2,
  FLOAT_VIEW_REFLOAT_CONTROL_STATUS_UNSUPPORTED_COMMAND = 3,
  FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OUTPUT_TOO_SMALL = 4,
  FLOAT_VIEW_REFLOAT_CONTROL_STATUS_INTERNAL_FAILURE = 5,
};

enum {
  FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_PACKAGE = 1,
  FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_FLYWHEEL = 2,
  FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_LIGHTING = 3,
  FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_FATAL_CLEAR = 4,
};

enum {
  FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_NONE = 0,
  FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_LEGACY = 1,
  FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_STABLE = 2,
};

enum {
  FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_NONE = 0,
  FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_LEDS = 1,
  FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_HEADLIGHTS = 2,
};

uint32_t float_view_refloat_control_abi_version(void);

FloatViewRefloatControlStatus float_view_refloat_control_encode(
    uint8_t operation,
    uint8_t lighting_profile,
    uint8_t fatal_clear_available,
    uint8_t target,
    uint8_t enabled,
    uint8_t current_flags,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length,
    uint8_t *unsupported_command);

FloatViewRefloatControlStatus float_view_refloat_control_decode_lighting(
    const uint8_t *payload,
    size_t payload_length,
    uint8_t lighting_profile,
    uint8_t *lighting_flags);

#ifdef __cplusplus
}
#endif

#endif
