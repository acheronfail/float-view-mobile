#include "CVescRefloatCompatibility.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

static const char *const fields[] = {
    "motor.speed", "motor.erpm", "motor.current", "motor.dir_current",
    "motor.filt_current", "motor.duty_cycle", "motor.batt_voltage",
    "motor.batt_current", "motor.mosfet_temp", "motor.motor_temp",
    "imu.pitch", "imu.balance_pitch", "imu.roll", "footpad.adc1",
    "footpad.adc2", "remote.input",
};
static const char *const runtime_fields[] = {
    "setpoint", "atr.setpoint", "brake_tilt.setpoint", "torque_tilt.setpoint",
    "turn_tilt.setpoint", "remote.setpoint", "balance_current", "atr.accel_diff",
    "atr.speed_boost", "booster.current",
};

static size_t append_names(uint8_t *record, size_t cursor,
                           const char *const *names, size_t count,
                           size_t *aggregate) {
  for (size_t index = 0; index < count; ++index) {
    const size_t length = strlen(names[index]);
    assert(length > 0 && length <= UINT8_MAX);
    record[cursor++] = (uint8_t)length;
    memcpy(record + cursor, names[index], length);
    cursor += length;
    *aggregate += length;
  }
  return cursor;
}

static size_t make_layout(uint8_t *record) {
  size_t cursor = 8;
  size_t aggregate = 0;
  cursor = append_names(record, cursor, fields, sizeof(fields) / sizeof(fields[0]),
                        &aggregate);
  cursor = append_names(record, cursor, runtime_fields,
                        sizeof(runtime_fields) / sizeof(runtime_fields[0]),
                        &aggregate);
  record[0] = 1;
  record[1] = 1;
  record[2] = (uint8_t)(sizeof(fields) / sizeof(fields[0]));
  record[3] = (uint8_t)(sizeof(runtime_fields) / sizeof(runtime_fields[0]));
  record[4] = (uint8_t)(aggregate >> 8);
  record[5] = (uint8_t)aggregate;
  record[6] = 0;
  record[7] = 0;
  return cursor;
}

int main(void) {
  assert(float_view_refloat_compatibility_abi_version() ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_ABI_VERSION);
  assert(FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OK == 0);
  assert(FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INTERNAL_FAILURE == 8);

  uint8_t request[FLOAT_VIEW_REFLOAT_COMPATIBILITY_INFO_REQUEST_BYTES] = {0};
  size_t length = SIZE_MAX;
  assert(float_view_refloat_compatibility_info_request(
             request, sizeof(request), &length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OK);
  const uint8_t expected_request[] = {36, 101, 0, 2, 0};
  assert(length == sizeof(expected_request));
  assert(memcmp(request, expected_request, sizeof(expected_request)) == 0);

  const uint8_t legacy[] = {36, 101, 0, 10};
  uint8_t record[FLOAT_VIEW_REFLOAT_COMPATIBILITY_RECORD_BYTES] = {0};
  length = SIZE_MAX;
  assert(float_view_refloat_compatibility_decode_info(
             legacy, sizeof(legacy), record, sizeof(record), &length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OK);
  const uint8_t expected_legacy[] = {1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0};
  assert(length == sizeof(expected_legacy));
  assert(memcmp(record, expected_legacy, sizeof(expected_legacy)) == 0);

  uint8_t stable[61] = {36, 101, 0, 2, 0};
  stable[25] = 1;
  stable[26] = 2;
  stable[27] = 3;
  assert(float_view_refloat_compatibility_decode_info(
             stable, sizeof(stable), record, sizeof(record), &length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OK);
  const uint8_t expected_stable[] = {1, 1, 1, 2, 1, 3, 3, 2, 2, 1, 0, 0};
  assert(memcmp(record, expected_stable, sizeof(expected_stable)) == 0);

  const uint8_t unsupported_envelope[] = {35, 101, 0, 10};
  assert(float_view_refloat_compatibility_decode_info(
             unsupported_envelope, sizeof(unsupported_envelope), record,
             sizeof(record), &length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_UNSUPPORTED_ENVELOPE);
  assert(length == 0);
  const uint8_t unsupported_protocol[] = {36, 101, 0, 1};
  assert(float_view_refloat_compatibility_decode_info(
             unsupported_protocol, sizeof(unsupported_protocol), record,
             sizeof(record), &length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_UNSUPPORTED_PROTOCOL);
  const uint8_t unsupported_release[] = {36, 101, 0, 13};
  assert(float_view_refloat_compatibility_decode_info(
             unsupported_release, sizeof(unsupported_release), record,
             sizeof(record), &length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_UNSUPPORTED_RELEASE);

  uint8_t small[2] = {9, 9};
  assert(float_view_refloat_compatibility_decode_info(
             legacy, sizeof(legacy), small, sizeof(small), &length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OUTPUT_TOO_SMALL);
  assert(length == FLOAT_VIEW_REFLOAT_COMPATIBILITY_RECORD_BYTES);
  assert(small[0] == 9 && small[1] == 9);
  assert(float_view_refloat_compatibility_decode_info(
             NULL, 1, record, sizeof(record), &length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_compatibility_info_request(
             request, sizeof(request), NULL) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_ARGUMENT);

  union {
    max_align_t alignment;
    uint8_t bytes[64];
  } overlap;
  assert(float_view_refloat_compatibility_decode_info(
             legacy, sizeof(legacy), overlap.bytes, sizeof(record),
             (size_t *)(void *)overlap.bytes) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_ARGUMENT);

  uint8_t layout[FLOAT_VIEW_REFLOAT_COMPATIBILITY_MAX_LAYOUT_RECORD_BYTES] = {0};
  const size_t layout_length = make_layout(layout);
  assert(float_view_refloat_compatibility_admit_layout(layout, layout_length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OK);
  layout[0] = 2;
  assert(float_view_refloat_compatibility_admit_layout(layout, layout_length) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_LAYOUT);
  assert(float_view_refloat_compatibility_admit_layout(NULL, 1) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_compatibility_admit_layout(
             (const uint8_t *)1,
             FLOAT_VIEW_REFLOAT_COMPATIBILITY_MAX_LAYOUT_RECORD_BYTES + 1) ==
         FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_LAYOUT);
  return 0;
}
