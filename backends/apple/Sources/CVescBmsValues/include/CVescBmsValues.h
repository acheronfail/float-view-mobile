#ifndef FLOAT_VIEW_C_VESC_BMS_VALUES_H
#define FLOAT_VIEW_C_VESC_BMS_VALUES_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_VIEW_VESC_BMS_VALUES_ABI_VERSION UINT32_C(1)
#define FLOAT_VIEW_VESC_BMS_VALUES_ANY_CAN_ID UINT8_C(255)
#define FLOAT_VIEW_VESC_BMS_VALUES_MAX_PAYLOAD_BYTES ((size_t)350)
#define FLOAT_VIEW_VESC_BMS_VALUES_RECORD_HEADER_BYTES ((size_t)8)
#define FLOAT_VIEW_VESC_BMS_VALUES_FIXED_VALUE_COUNT ((size_t)17)
#define FLOAT_VIEW_VESC_BMS_VALUES_MAX_RECORD_BYTES ((size_t)1034)
#define FLOAT_VIEW_VESC_BMS_VALUES_FLAG_EXTENDED_TAIL_PRESENT UINT8_C(1)

typedef uint32_t FloatViewVescBmsValuesStatus;
enum {
  FLOAT_VIEW_VESC_BMS_VALUES_STATUS_OK = 0,
  FLOAT_VIEW_VESC_BMS_VALUES_STATUS_INVALID_ARGUMENT = 1,
  FLOAT_VIEW_VESC_BMS_VALUES_STATUS_UNSUPPORTED_COMMAND = 2,
  FLOAT_VIEW_VESC_BMS_VALUES_STATUS_TRUNCATED_PAYLOAD = 3,
  FLOAT_VIEW_VESC_BMS_VALUES_STATUS_OUTPUT_TOO_SMALL = 4,
  FLOAT_VIEW_VESC_BMS_VALUES_STATUS_INTERNAL_FAILURE = 5,
};

uint32_t float_view_vesc_bms_values_abi_version(void);

/*
 * Successful version-1 record:
 *   0 record version, 1 option flags, 2 cell count, 3 temperature count,
 *   4 reported CAN ID, 5 optional data version value,
 *   6 status UTF-8 byte length, 7 reserved zero,
 *   8..143 seventeen f64 IEEE-754 bit patterns (u64 big endian),
 *   then counted cell f64 values, counted balancing bytes, counted
 *   temperature f64 values, and counted status UTF-8 bytes.
 * All non-empty regions must be valid, bounded, and pairwise disjoint.
 * Rust retains nothing.
 */
FloatViewVescBmsValuesStatus float_view_vesc_bms_values_decode(
    const uint8_t *payload,
    size_t payload_length,
    uint8_t expected_can_id,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length,
    uint8_t *unsupported_command);

#ifdef __cplusplus
}
#endif

#endif
