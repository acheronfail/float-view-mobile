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
  assert(FLOAT_VIEW_VESC_FRAME_STATUS_OK == 0);
  assert(FLOAT_VIEW_VESC_FRAME_STATUS_INTERNAL_FAILURE == 6);

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

  return 0;
}
