#include "CVescFrame.h"

#include <assert.h>
#include <stdint.h>
#include <string.h>

static const uint8_t short_payload[] = {
    0, 6, 6, 84, 72, 79, 82, 51, 48, 48, 0,
};
static const uint8_t short_frame[] = {
    2, 11, 0, 6, 6, 84, 72, 79, 82, 51, 48, 48, 0, 42, 216, 3,
};

int main(void) {
  assert(float_view_vesc_frame_abi_version() ==
         FLOAT_VIEW_VESC_FRAME_ABI_VERSION);
  assert(FLOAT_VIEW_VESC_FRAME_ABI_VERSION == 2);
  assert(FLOAT_VIEW_VESC_FRAME_STATUS_OK == 0);
  assert(FLOAT_VIEW_VESC_FRAME_STATUS_INTERNAL_FAILURE == 6);
  assert(FLOAT_VIEW_VESC_FRAME_STATUS_NEED_MORE_INPUT == 7);

  uint8_t frame[FLOAT_VIEW_VESC_FRAME_MAX_FRAME_BYTES] = {0};
  size_t frame_length = 0;
  assert(float_view_vesc_frame_encode(
             short_payload, sizeof(short_payload), frame, sizeof(frame),
             &frame_length) == FLOAT_VIEW_VESC_FRAME_STATUS_OK);
  assert(frame_length == sizeof(short_frame));
  assert(memcmp(frame, short_frame, sizeof(short_frame)) == 0);

  uint8_t payload[FLOAT_VIEW_VESC_FRAME_MAX_PAYLOAD_BYTES] = {0};
  size_t payload_length = 0;
  assert(float_view_vesc_frame_decode_exact(
             short_frame, sizeof(short_frame), payload, sizeof(payload),
             &payload_length) == FLOAT_VIEW_VESC_FRAME_STATUS_OK);
  assert(payload_length == sizeof(short_payload));
  assert(memcmp(payload, short_payload, sizeof(short_payload)) == 0);

  uint8_t long_payload[256] = {4};
  assert(float_view_vesc_frame_encode(
             long_payload, sizeof(long_payload), frame, sizeof(frame),
             &frame_length) == FLOAT_VIEW_VESC_FRAME_STATUS_OK);
  assert(frame_length == 262);
  assert(frame[0] == 3 && frame[1] == 1 && frame[2] == 0);
  assert(frame[259] == 0xaa && frame[260] == 0xe9 && frame[261] == 3);

  uint8_t maximum[FLOAT_VIEW_VESC_FRAME_MAX_PAYLOAD_BYTES];
  memset(maximum, 0x33, sizeof(maximum));
  assert(float_view_vesc_frame_encode(maximum, sizeof(maximum), frame,
                                      sizeof(frame), &frame_length) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_OK);
  assert(frame_length == FLOAT_VIEW_VESC_FRAME_MAX_FRAME_BYTES);
  assert(float_view_vesc_frame_decode_exact(
             frame, frame_length, payload, sizeof(payload), &payload_length) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_OK);
  assert(payload_length == sizeof(maximum));
  assert(memcmp(payload, maximum, sizeof(maximum)) == 0);

  size_t required = 0;
  uint8_t small[3] = {0};
  assert(float_view_vesc_frame_encode(
             short_payload, sizeof(short_payload), small, sizeof(small),
             &required) == FLOAT_VIEW_VESC_FRAME_STATUS_OUTPUT_TOO_SMALL);
  assert(required == sizeof(short_frame));
  assert(float_view_vesc_frame_encode(NULL, 1, small, sizeof(small),
                                      &required) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_ARGUMENT);
  assert(float_view_vesc_frame_encode(short_payload, sizeof(short_payload),
                                      small, sizeof(small), NULL) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_ARGUMENT);

  uint8_t overlap[32] = {0};
  assert(float_view_vesc_frame_encode(overlap, 3, overlap, 10, &required) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_ARGUMENT);
  assert(float_view_vesc_frame_encode((const uint8_t *)1,
                                      FLOAT_VIEW_VESC_FRAME_MAX_PAYLOAD_BYTES +
                                          1,
                                      NULL, 0, &required) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_INPUT_TOO_LARGE);

  uint8_t corrupt[sizeof(short_frame)];
  memcpy(corrupt, short_frame, sizeof(corrupt));
  corrupt[13] ^= 1;
  assert(float_view_vesc_frame_decode_exact(
             corrupt, sizeof(corrupt), payload, sizeof(payload),
             &payload_length) == FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_CRC);
  corrupt[13] ^= 1;
  corrupt[sizeof(corrupt) - 1] = 4;
  assert(float_view_vesc_frame_decode_exact(
             corrupt, sizeof(corrupt), payload, sizeof(payload),
             &payload_length) == FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_FRAME);

  uint8_t stream[2 + sizeof(short_frame) * 2] = {0xff, 0};
  memcpy(stream + 2, short_frame, sizeof(short_frame));
  memcpy(stream + 2 + sizeof(short_frame), short_frame, sizeof(short_frame));
  uint8_t unchanged[sizeof(stream)];
  memcpy(unchanged, stream, sizeof(stream));
  size_t scan_offset = SIZE_MAX;
  size_t scan_length = SIZE_MAX;
  size_t consumed_length = SIZE_MAX;
  assert(float_view_vesc_frame_stream_scan_next(
             stream, sizeof(stream), &scan_offset, &scan_length,
             &consumed_length) == FLOAT_VIEW_VESC_FRAME_STATUS_OK);
  assert(scan_offset == 2);
  assert(scan_length == sizeof(short_frame));
  assert(consumed_length == 2 + sizeof(short_frame));
  assert(memcmp(stream, unchanged, sizeof(stream)) == 0);

  const uint8_t split_header[] = {0xff, 3, 0};
  assert(float_view_vesc_frame_stream_scan_next(
             split_header, sizeof(split_header), &scan_offset, &scan_length,
             &consumed_length) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_NEED_MORE_INPUT);
  assert(scan_offset == 0 && scan_length == 0 && consumed_length == 1);

  const uint8_t oversized_declaration[] = {0xff, 3, 0x10, 0x01};
  assert(float_view_vesc_frame_stream_scan_next(
             oversized_declaration, sizeof(oversized_declaration),
             &scan_offset, &scan_length, &consumed_length) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_INPUT_TOO_LARGE);
  assert(scan_offset == 0 && scan_length == 0 && consumed_length == 2);

  assert(float_view_vesc_frame_stream_scan_next(
             (const uint8_t *)1, FLOAT_VIEW_VESC_FRAME_MAX_STREAM_BYTES + 1,
             &scan_offset, &scan_length, &consumed_length) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_INPUT_TOO_LARGE);
  assert(scan_offset == 0 && scan_length == 0 &&
         consumed_length == FLOAT_VIEW_VESC_FRAME_MAX_STREAM_BYTES + 1);

  scan_offset = 11;
  scan_length = 22;
  consumed_length = 33;
  assert(float_view_vesc_frame_stream_scan_next(
             short_frame, sizeof(short_frame), NULL, &scan_length,
             &consumed_length) == FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_ARGUMENT);
  assert(scan_offset == 11 && scan_length == 22 && consumed_length == 33);
  assert(float_view_vesc_frame_stream_scan_next(
             short_frame, sizeof(short_frame), &scan_offset, &scan_offset,
             &consumed_length) == FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_ARGUMENT);
  assert(scan_offset == 11 && scan_length == 22 && consumed_length == 33);
  assert(float_view_vesc_frame_stream_scan_next(
             (const uint8_t *)&scan_offset, sizeof(scan_offset), &scan_offset,
             &scan_length, &consumed_length) ==
         FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_ARGUMENT);
  assert(scan_offset == 11 && scan_length == 22 && consumed_length == 33);

  union {
    max_align_t alignment;
    uint8_t bytes[sizeof(size_t) + 1];
  } misaligned;
  assert(float_view_vesc_frame_stream_scan_next(
             short_frame, sizeof(short_frame),
             (size_t *)(void *)(misaligned.bytes + 1), &scan_length,
             &consumed_length) == FLOAT_VIEW_VESC_FRAME_STATUS_INVALID_ARGUMENT);
  assert(scan_offset == 11 && scan_length == 22 && consumed_length == 33);

  return 0;
}
