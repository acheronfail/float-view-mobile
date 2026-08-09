#ifndef FLOAT_VIEW_C_VESC_FRAME_H
#define FLOAT_VIEW_C_VESC_FRAME_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_VIEW_VESC_FRAME_ABI_VERSION UINT32_C(1)
#define FLOAT_VIEW_VESC_FRAME_MAX_PAYLOAD_BYTES ((size_t)4096)
#define FLOAT_VIEW_VESC_FRAME_MAX_FRAME_BYTES ((size_t)4102)

typedef uint32_t FloatViewVescFrameStatus;

enum {
  FLOAT_VIEW_VESC_FRAME_STATUS_OK = 0,
  FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_ARGUMENT = 1,
  FLOAT_VIEW_VESC_FRAME_STATUS_INPUT_TOO_LARGE = 2,
  FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_FRAME = 3,
  FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_CRC = 4,
  FLOAT_VIEW_VESC_FRAME_STATUS_OUTPUT_TOO_SMALL = 5,
  FLOAT_VIEW_VESC_FRAME_STATUS_INTERNAL_FAILURE = 6,
};

/*
 * All non-empty byte regions must be valid for the supplied lengths. The
 * output_length pointer must be valid and aligned for one size_t. Input,
 * output, and output_length regions must not overlap. A byte pointer may be
 * null only when its corresponding length or capacity is zero. Rust never
 * retains a pointer or transfers allocator ownership across this boundary.
 */
uint32_t float_view_vesc_frame_abi_version(void);

FloatViewVescFrameStatus float_view_vesc_frame_encode(
    const uint8_t *payload,
    size_t payload_length,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);

FloatViewVescFrameStatus float_view_vesc_frame_decode_exact(
    const uint8_t *frame,
    size_t frame_length,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);

#ifdef __cplusplus
}
#endif

#endif
