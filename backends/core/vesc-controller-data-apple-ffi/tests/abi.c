#include "CVescControllerData.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

static uint64_t read_u64(const uint8_t *bytes) {
  uint64_t value = 0;
  for (size_t index = 0; index < 8; index++) {
    value = (value << 8) | bytes[index];
  }
  return value;
}

static double read_f64(const uint8_t *bytes) {
  const uint64_t bits = read_u64(bytes);
  double value = 0;
  memcpy(&value, &bits, sizeof(value));
  return value;
}

int main(void) {
  assert(float_view_vesc_controller_data_abi_version() ==
         FLOAT_VIEW_VESC_CONTROLLER_DATA_ABI_VERSION);
  assert(FLOAT_VIEW_VESC_CONTROLLER_DATA_ABI_VERSION == 1);
  assert(FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_OK == 0);
  assert(FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_INTERNAL_FAILURE == 5);
  assert(FLOAT_VIEW_VESC_CONTROLLER_DATA_MAX_PAYLOAD_BYTES == 4096);
  assert(FLOAT_VIEW_VESC_CONTROLLER_DATA_MAX_RECORD_BYTES == 60);

  const uint8_t realtime[30] = {
      4, 0, 250, 255, 201, 0, 0, 4, 210, 255, 255, 255, 6,
      0, 0, 0, 0, 0, 0, 0, 0, 1, 244, 255, 255, 251, 46, 1, 248, 1,
  };
  uint8_t record[FLOAT_VIEW_VESC_CONTROLLER_DATA_MAX_RECORD_BYTES] = {0};
  size_t record_length = 0;
  uint8_t unsupported = UINT8_MAX;
  assert(float_view_vesc_controller_data_decode(
             FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_REALTIME,
             realtime, sizeof(realtime), record, sizeof(record),
             &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_OK);
  const uint8_t realtime_header[8] = {1, 1, 0, 1, 1, 0, 0, 0};
  assert(record_length == FLOAT_VIEW_VESC_CONTROLLER_DATA_REALTIME_RECORD_BYTES);
  assert(memcmp(record, realtime_header, sizeof(realtime_header)) == 0);
  assert(read_f64(record + 8) == 25.0);
  assert(read_f64(record + 16) == -5.5);
  assert(read_f64(record + 24) == 12.34);
  assert(read_f64(record + 32) == -2.5);
  assert(read_f64(record + 40) == 0.5);
  assert(read_f64(record + 48) == 50.4);
  const uint8_t expected_erpm[4] = {255, 255, 251, 46};
  assert(memcmp(record + 56, expected_erpm, sizeof(expected_erpm)) == 0);
  assert(unsupported == 0);

  const uint8_t limits[25] = {
      14, 0x57, 0xAD, 0x8F, 0x53, 0, 0, 0, 0,
      0x42, 0xC8, 0, 0, 0xC2, 0xC8, 0, 0,
      0x42, 0x48, 0, 0, 0xC1, 0xA0, 0, 0,
  };
  assert(float_view_vesc_controller_data_decode(
             FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_CURRENT_LIMITS,
             limits, sizeof(limits), record, sizeof(record),
             &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_OK);
  const uint8_t limits_header[8] = {1, 2, 0, 0, 0, 0, 0, 0};
  assert(record_length ==
         FLOAT_VIEW_VESC_CONTROLLER_DATA_CURRENT_LIMITS_RECORD_BYTES);
  assert(memcmp(record, limits_header, sizeof(limits_header)) == 0);
  assert(read_f64(record + 8) == 100.0);
  assert(read_f64(record + 16) == -100.0);
  assert(read_f64(record + 24) == 50.0);
  assert(read_f64(record + 32) == -20.0);

  const uint8_t wrong[] = {14};
  assert(float_view_vesc_controller_data_decode(
             FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_REALTIME,
             wrong, sizeof(wrong), record, sizeof(record),
             &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_UNSUPPORTED_COMMAND);
  assert(record_length == 0 && unsupported == 14);

  uint8_t small[8] = {0};
  assert(float_view_vesc_controller_data_decode(
             FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_REALTIME,
             realtime, sizeof(realtime), small, sizeof(small),
             &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_OUTPUT_TOO_SMALL);
  assert(record_length == 60);
  assert(float_view_vesc_controller_data_decode(
             99, realtime, sizeof(realtime), record, sizeof(record),
             &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_INVALID_ARGUMENT);
  assert(float_view_vesc_controller_data_decode(
             FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_REALTIME,
             NULL, 1, record, sizeof(record), &record_length, &unsupported) ==
         FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_INVALID_ARGUMENT);

  union {
    max_align_t alignment;
    uint8_t bytes[80];
  } overlap;
  assert(float_view_vesc_controller_data_decode(
             FLOAT_VIEW_VESC_CONTROLLER_DATA_OPERATION_REALTIME,
             realtime, sizeof(realtime), overlap.bytes, 60,
             (size_t *)(void *)overlap.bytes, &unsupported) ==
         FLOAT_VIEW_VESC_CONTROLLER_DATA_STATUS_INVALID_ARGUMENT);
  return 0;
}
