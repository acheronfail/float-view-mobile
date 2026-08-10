#ifndef FLOAT_VIEW_C_VESC_CONTROLLER_DATA_H
#define FLOAT_VIEW_C_VESC_CONTROLLER_DATA_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_VIEW_VESC_CONTROLLER_DATA_ABI_VERSION UINT32_C(1)
#define FLOAT_VIEW_VESC_CONTROLLER_DATA_MAX_PAYLOAD_BYTES ((size_t)4096)
#define FLOAT_VIEW_VESC_CONTROLLER_DATA_RECORD_HEADER_BYTES ((size_t)8)
#define FLOAT_VIEW_VESC_CONTROLLER_DATA_REALTIME_RECORD_BYTES ((size_t)60)
#define FLOAT_VIEW_VESC_CONTROLLER_DATA_CURRENT_LIMITS_RECORD_BYTES ((size_t)40)
#define FLOAT_VIEW_VESC_CONTROLLER_DATA_MAX_RECORD_BYTES ((size_t)60)

typedef uint32_t FloatViewVescControllerDataStatus;
enum {
  FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_OK = 0,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_INVALID_ARGUMENT = 1,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_UNSUPPORTED_COMMAND = 2,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_TRUNCATED_PAYLOAD = 3,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_OUTPUT_TOO_SMALL = 4,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_INTERNAL_FAILURE = 5,
};

typedef uint32_t FloatViewVescControllerDataOperation;
enum {
  FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_REALTIME = 1,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_CURRENT_LIMITS = 2,
};

enum {
  FLOAT_VIEW_VESC_CONTROLLER_DATA_RESULT_REALTIME = 1,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_RESULT_CURRENT_LIMITS = 2,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_FLAG_SERIES_CELLS_PRESENT = 1,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_FAULT_NONE = 0,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_FAULT_OVER_VOLTAGE = 1,
  FLOAT_VIEW_VESC_CONTROLLER_DATA_FAULT_UNKNOWN = 2,
};

uint32_t float_view_vesc_controller_data_abi_version(void);

/*
 * Successful version-1 record:
 *   0 version, 1 result kind, 2 option flags, 3 fault kind,
 *   4 raw fault code, 5 optional series-cell value, 6..7 reserved zero.
 * Realtime appends six f64 IEEE-754 bit patterns (u64 big endian), then one
 * signed i32 ERPM (big endian), for 60 bytes. Current limits append four f64
 * values for 40 bytes. All regions are bounded and pairwise disjoint.
 */
FloatViewVescControllerDataStatus float_view_vesc_controller_data_decode(
    uint32_t operation,
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
