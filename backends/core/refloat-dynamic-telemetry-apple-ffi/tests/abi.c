#include "CVescRefloatDynamicTelemetry.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

static const uint8_t layout_payload[] = {
    36, 101, 32, 3,
    11, 'm', 'o', 't', 'o', 'r', '.', 's', 'p', 'e', 'e', 'd',
    18, 'm', 'o', 't', 'o', 'r', '.', 'b', 'a', 't', 't', '_', 'v', 'o', 'l', 't', 'a', 'g', 'e',
    12, 'f', 'u', 't', 'u', 'r', 'e', '.', 'v', 'a', 'l', 'u', 'e',
    0,
};

static const uint8_t realtime_payload[] = {
    36, 101, 31, 0, 0, 0, 0, 0, 9, 2, 0, 0, 0,
    0x3c, 0x00, 0x40, 0x00, 0x42, 0x00,
};

int main(void) {
  assert(float_view_refloat_dynamic_telemetry_abi_version() ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_ABI_VERSION);
  assert(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_ABI_VERSION == 1);
  assert(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_OK == 0);
  assert(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INTERNAL_FAILURE == 7);
  assert(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_LAYOUT_RECORD_BYTES == 4168);
  assert(FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_SAMPLE_RECORD_BYTES == 2592);

  uint8_t layout[FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_LAYOUT_RECORD_BYTES] = {0};
  size_t layout_length = SIZE_MAX;
  assert(float_view_refloat_dynamic_telemetry_decode_layout(
             layout_payload, sizeof(layout_payload), layout, sizeof(layout),
             &layout_length) == FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_OK);
  const uint8_t expected_layout_header[] = {1, 1, 3, 0, 0, 41, 0, 0};
  assert(memcmp(layout, expected_layout_header, sizeof(expected_layout_header)) == 0);

  uint8_t sample[FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_SAMPLE_RECORD_BYTES] = {0};
  size_t sample_length = SIZE_MAX;
  assert(float_view_refloat_dynamic_telemetry_decode_realtime(
             layout, layout_length, realtime_payload, sizeof(realtime_payload),
             sample, sizeof(sample), &sample_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_OK);
  const uint8_t expected_sample_header[] = {
      1, 2, 3, 1, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0,
      0, 0, 0, 9, 0, 0, 0, 65, 0, 0, 0, 0, 0, 0, 0, 0,
  };
  assert(memcmp(sample, expected_sample_header, sizeof(expected_sample_header)) == 0);
  assert(sample_length == FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_SAMPLE_FIXED_BYTES + 21);

  const uint8_t unsupported[] = {37, 101, 32};
  layout_length = SIZE_MAX;
  assert(float_view_refloat_dynamic_telemetry_decode_layout(
             unsupported, sizeof(unsupported), layout, sizeof(layout),
             &layout_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_UNSUPPORTED_ENVELOPE);
  assert(layout_length == 0);

  const uint8_t invalid_layout[] = {36, 101, 32, 0, 0};
  assert(float_view_refloat_dynamic_telemetry_decode_layout(
             invalid_layout, sizeof(invalid_layout), layout, sizeof(layout),
             &layout_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_LAYOUT);
  assert(layout_length == 0);

  uint8_t small[2] = {9, 9};
  assert(float_view_refloat_dynamic_telemetry_decode_layout(
             layout_payload, sizeof(layout_payload), small, sizeof(small),
             &layout_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_OUTPUT_TOO_SMALL);
  assert(layout_length > sizeof(small));
  assert(small[0] == 9 && small[1] == 9);

  assert(float_view_refloat_dynamic_telemetry_decode_layout(
             NULL, 1, layout, sizeof(layout), &layout_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_dynamic_telemetry_decode_layout(
             layout_payload, sizeof(layout_payload), layout, sizeof(layout),
             NULL) == FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_ARGUMENT);

  union {
    max_align_t alignment;
    uint8_t bytes[64];
  } overlap;
  assert(float_view_refloat_dynamic_telemetry_decode_layout(
             layout_payload, sizeof(layout_payload), overlap.bytes,
             sizeof(overlap.bytes), (size_t *)(void *)overlap.bytes) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_dynamic_telemetry_decode_layout(
             (const uint8_t *)1,
             FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_LAYOUT_PAYLOAD_BYTES + 1,
             NULL, 0, &layout_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_TRUNCATED_PAYLOAD);

  uint8_t corrupt_layout[FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_MAX_LAYOUT_RECORD_BYTES] = {0};
  memcpy(corrupt_layout, layout, layout_length);
  corrupt_layout[0] = 2;
  assert(float_view_refloat_dynamic_telemetry_decode_realtime(
             corrupt_layout, layout_length, realtime_payload,
             sizeof(realtime_payload), sample, sizeof(sample), &sample_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_LAYOUT);
  assert(sample_length == 0);

  const uint8_t unsupported_realtime[] = {37, 101, 31};
  sample_length = SIZE_MAX;
  assert(float_view_refloat_dynamic_telemetry_decode_realtime(
             layout, layout_length, unsupported_realtime,
             sizeof(unsupported_realtime), sample, sizeof(sample), &sample_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_UNSUPPORTED_ENVELOPE);
  assert(sample_length == 0);

  const uint8_t truncated_realtime[] = {36, 101, 31};
  assert(float_view_refloat_dynamic_telemetry_decode_realtime(
             layout, layout_length, truncated_realtime,
             sizeof(truncated_realtime), sample, sizeof(sample), &sample_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_TRUNCATED_PAYLOAD);
  assert(sample_length == 0);

  uint8_t transitional_realtime[sizeof(realtime_payload)];
  memcpy(transitional_realtime, realtime_payload, sizeof(realtime_payload));
  transitional_realtime[9] = 3;
  assert(float_view_refloat_dynamic_telemetry_decode_realtime(
             layout, layout_length, transitional_realtime,
             sizeof(transitional_realtime), sample, sizeof(sample),
             &sample_length) == FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_OK);
  assert(sample_length >= FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_SAMPLE_FIXED_BYTES);

  uint8_t invalid_realtime[sizeof(realtime_payload)];
  memcpy(invalid_realtime, realtime_payload, sizeof(realtime_payload));
  invalid_realtime[3] = 8;
  assert(float_view_refloat_dynamic_telemetry_decode_realtime(
             layout, layout_length, invalid_realtime,
             sizeof(invalid_realtime),
             sample, sizeof(sample), &sample_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_TELEMETRY);
  assert(sample_length == 0);

  uint8_t small_sample[2] = {7, 7};
  assert(float_view_refloat_dynamic_telemetry_decode_realtime(
             layout, layout_length, realtime_payload, sizeof(realtime_payload),
             small_sample, sizeof(small_sample), &sample_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_OUTPUT_TOO_SMALL);
  assert(sample_length > sizeof(small_sample));
  assert(small_sample[0] == 7 && small_sample[1] == 7);

  assert(float_view_refloat_dynamic_telemetry_decode_realtime(
             layout, layout_length, realtime_payload, sizeof(realtime_payload),
             layout, layout_length, &sample_length) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_dynamic_telemetry_decode_layout(
             layout_payload, sizeof(layout_payload), overlap.bytes,
             sizeof(overlap.bytes), (size_t *)(void *)(overlap.bytes + 1)) ==
         FLOAT_VIEW_REFLOAT_DYNAMIC_TELEMETRY_STATUS_INVALID_ARGUMENT);

  return 0;
}
