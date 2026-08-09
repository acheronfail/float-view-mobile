#ifndef FLOAT_VIEW_C_VESC_FRAME_H
#define FLOAT_VIEW_C_VESC_FRAME_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_VIEW_VESC_FRAME_ABI_VERSION UINT32_C(2)
#define FLOAT_VIEW_VESC_FRAME_MAX_PAYLOAD_BYTES ((size_t)4096)
#define FLOAT_VIEW_VESC_FRAME_MAX_FRAME_BYTES ((size_t)4102)
#define FLOAT_VIEW_VESC_FRAME_MAX_STREAM_BYTES ((size_t)8192)

typedef uint32_t FloatViewVescFrameStatus;

enum {
  FLOAT_VIEW_VESC_FRAME_STATUS_OK = 0,
  FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_ARGUMENT = 1,
  FLOAT_VIEW_VESC_FRAME_STATUS_INPUT_TOO_LARGE = 2,
  FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_FRAME = 3,
  FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_CRC = 4,
  FLOAT_VIEW_VESC_FRAME_STATUS_OUTPUT_TOO_SMALL = 5,
  FLOAT_VIEW_VESC_FRAME_STATUS_INTERNAL_FAILURE = 6,
  FLOAT_VIEW_VESC_FRAME_STATUS_NEED_MORE_INPUT = 7,
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

/*
 * Scan one borrowed notification-stream region without retaining it. On OK,
 * frame_offset and frame_length identify one complete candidate frame and
 * consumed_length covers leading noise plus that frame. NEED_MORE_INPUT may
 * report disposable leading noise in consumed_length while frame outputs are
 * zero. INPUT_TOO_LARGE reports the prefix to discard for an oversized
 * declaration, or the complete input length for aggregate overflow. All three
 * output pointers must be valid, size_t-aligned, pairwise disjoint, and
 * disjoint from the input region.
 */
FloatViewVescFrameStatus float_view_vesc_frame_stream_scan_next(
    const uint8_t *input,
    size_t input_length,
    size_t *frame_offset,
    size_t *frame_length,
    size_t *consumed_length);

#ifdef __cplusplus
}
#endif

#endif
