#ifndef FLOAT_VIEW_C_VESC_CUSTOM_CONFIGURATION_H
#define FLOAT_VIEW_C_VESC_CUSTOM_CONFIGURATION_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_ABI_VERSION UINT32_C(1)
#define FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_MAX_PAYLOAD_BYTES ((size_t)65536)
#define FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESULT_RECORD_BYTES ((size_t)28)

typedef uint32_t FloatViewVescCustomConfigurationStatus;
enum {
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_OK = 0,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_INVALID_ARGUMENT = 1,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_INVALID_PAYLOAD = 2,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_UNSUPPORTED_COMMAND = 3,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_OUTPUT_TOO_SMALL = 4,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_INTERNAL_FAILURE = 5,
};

enum {
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_SCHEMA = 1,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_CURRENT = 2,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_DEFAULT = 3,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_WRITE = 4,
};

enum {
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESPONSE_SCHEMA = 1,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESPONSE_CURRENT = 2,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESPONSE_DEFAULT = 3,
  FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESPONSE_WRITE_ACKNOWLEDGEMENT = 4,
};

uint32_t float_view_vesc_custom_configuration_abi_version(void);

FloatViewVescCustomConfigurationStatus float_view_vesc_custom_configuration_encode(
    uint8_t operation,
    uint8_t index,
    int64_t length,
    int64_t offset,
    const uint8_t *values,
    size_t values_length,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);

/*
 * Successful version-1 decode record:
 *   0 record version, 1 response kind, 2 command, 3 index,
 *   4..11 total length (u64 big endian), 12..19 offset (u64 big endian),
 *   20..23 opaque payload offset (u32 big endian),
 *   24..27 opaque payload length (u32 big endian).
 * All non-empty regions must be valid, bounded, and pairwise disjoint.
 * Rust retains nothing.
 */
FloatViewVescCustomConfigurationStatus float_view_vesc_custom_configuration_decode(
    const uint8_t *payload,
    size_t payload_length,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length,
    uint8_t *unsupported_command);

#ifdef __cplusplus
}
#endif

#endif
