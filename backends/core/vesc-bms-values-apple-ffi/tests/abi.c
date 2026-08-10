#include "CVescBmsValues.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

int main(void) {
  assert(float_view_vesc_bms_values_abi_version() ==
         FLOAT_VIEW_VESC_BMS_VALUES_ABI_VERSION);
  assert(FLOAT_VIEW_VESC_BMS_VALUES_ABI_VERSION == 1);
  assert(FLOAT_VIEW_VESC_BMS_VALUES_STATUS_OK == 0);
  assert(FLOAT_VIEW_VESC_BMS_VALUES_STATUS_INTERNAL_FAILURE == 5);
  assert(FLOAT_VIEW_VESC_BMS_VALUES_MAX_PAYLOAD_BYTES == 350);
  assert(FLOAT_VIEW_VESC_BMS_VALUES_MAX_RECORD_BYTES == 1034);

  const uint8_t minimum[] = {
      96,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0,
      0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      42,
      0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0,
      0, 0,
  };
  assert(sizeof(minimum) == 58);
  uint8_t record[FLOAT_VIEW_VESC_BMS_VALUES_MAX_RECORD_BYTES] = {0};
  size_t record_length = 0;
  uint8_t unsupported = UINT8_MAX;
  assert(float_view_vesc_bms_values_decode(
             minimum, sizeof(minimum), 42, record, sizeof(record),
             &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_BMS_VALUES_STATUS_OK);
  const uint8_t expected_header[] = {1, 0, 0, 0, 42, 0, 0, 0};
  assert(record_length == 144);
  assert(memcmp(record, expected_header, sizeof(expected_header)) == 0);
  for (size_t index = sizeof(expected_header); index < record_length; index++) {
    assert(record[index] == 0);
  }
  assert(unsupported == 0);

  assert(float_view_vesc_bms_values_decode(
             minimum, sizeof(minimum), FLOAT_VIEW_VESC_BMS_VALUES_ANY_CAN_ID,
             record, sizeof(record), &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_BMS_VALUES_STATUS_OK);
  assert(float_view_vesc_bms_values_decode(
             minimum, sizeof(minimum), 41, record, sizeof(record),
             &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_BMS_VALUES_STATUS_TRUNCATED_PAYLOAD);
  assert(float_view_vesc_bms_values_decode(
             minimum, sizeof(minimum), 254, record, sizeof(record),
             &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_BMS_VALUES_STATUS_INVALID_ARGUMENT);

  const uint8_t wrong[] = {37};
  assert(float_view_vesc_bms_values_decode(
             wrong, sizeof(wrong), 42, record, sizeof(record), &record_length,
             &unsupported) == FLOAT_VIEW_VESC_BMS_VALUES_STATUS_UNSUPPORTED_COMMAND);
  assert(record_length == 0 && unsupported == 37);

  uint8_t small[2] = {0};
  assert(float_view_vesc_bms_values_decode(
             minimum, sizeof(minimum), 42, small, sizeof(small),
             &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_BMS_VALUES_STATUS_OUTPUT_TOO_SMALL);
  assert(record_length == 144);
  assert(float_view_vesc_bms_values_decode(
             NULL, 1, 42, record, sizeof(record), &record_length,
             &unsupported) == FLOAT_VIEW_VESC_BMS_VALUES_STATUS_INVALID_ARGUMENT);

  union {
    max_align_t alignment;
    uint8_t bytes[160];
  } overlap;
  assert(float_view_vesc_bms_values_decode(
             minimum, sizeof(minimum), 42, overlap.bytes, sizeof(overlap.bytes),
             (size_t *)(void *)overlap.bytes, &unsupported) ==
         FLOAT_VIEW_VESC_BMS_VALUES_STATUS_INVALID_ARGUMENT);
  return 0;
}
