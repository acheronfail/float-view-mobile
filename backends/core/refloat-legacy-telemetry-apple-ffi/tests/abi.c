#include "CVescRefloatLegacyTelemetry.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

static const uint8_t detail_one[] = {
    36, 101, 10, 1, 0, 0, 0, 0, 0, 0, 1, 2, 50, 100, 128, 128,
    128, 128, 128, 128, 0, 0, 128, 1, 244, 0, 0, 0, 0, 0, 0, 0,
    0, 128, 222,
};

int main(void) {
  assert(float_view_refloat_legacy_telemetry_abi_version() ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_ABI_VERSION);
  assert(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_ABI_VERSION == 1);
  assert(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_OK == 0);
  assert(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_INTERNAL_FAILURE == 6);
  assert(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_MAX_PAYLOAD_BYTES == 59);
  assert(FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_MAX_RECORD_BYTES == 264);

  uint8_t record[FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_MAX_RECORD_BYTES] = {0};
  size_t record_length = SIZE_MAX;
  uint8_t unsupported_command = UINT8_MAX;
  assert(float_view_refloat_legacy_telemetry_decode(
             detail_one, sizeof(detail_one), record, sizeof(record),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_OK);
  assert(record_length == FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_SAMPLE_RECORD_BYTES);
  const uint8_t expected_header[] = {
      1, 1, 1, 2, 1, 4, 1, 1, 0, 1, 0, 0, 0, 0, 0, 0,
  };
  assert(memcmp(record, expected_header, sizeof(expected_header)) == 0);
  assert(unsupported_command == 0);

  const uint8_t fault[] = {36, 101, 10, 69, 1};
  assert(float_view_refloat_legacy_telemetry_decode(
             fault, sizeof(fault), record, sizeof(record),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_OK);
  assert(record_length == FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_ACTIVE_FAULT_RECORD_BYTES);
  const uint8_t expected_fault[] = {
      1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0,
  };
  assert(memcmp(record, expected_fault, sizeof(expected_fault)) == 0);

  const uint8_t unsupported[] = {37, 101, 10, 1};
  record_length = SIZE_MAX;
  unsupported_command = 0;
  assert(float_view_refloat_legacy_telemetry_decode(
             unsupported, sizeof(unsupported), record, sizeof(record),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_UNSUPPORTED_ENVELOPE);
  assert(record_length == 0 && unsupported_command == 37);

  const uint8_t truncated[] = {36, 101, 10, 1};
  record_length = SIZE_MAX;
  unsupported_command = UINT8_MAX;
  assert(float_view_refloat_legacy_telemetry_decode(
             truncated, sizeof(truncated), record, sizeof(record),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_TRUNCATED_PAYLOAD);
  assert(record_length == 0 && unsupported_command == 0);

  const uint8_t invalid[] = {36, 101, 10, 0};
  assert(float_view_refloat_legacy_telemetry_decode(
             invalid, sizeof(invalid), record, sizeof(record),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_INVALID_PAYLOAD);
  assert(record_length == 0 && unsupported_command == 0);

  uint8_t small[2] = {0};
  assert(float_view_refloat_legacy_telemetry_decode(
             detail_one, sizeof(detail_one), small, sizeof(small),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_OUTPUT_TOO_SMALL);
  assert(record_length == FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_SAMPLE_RECORD_BYTES);

  assert(float_view_refloat_legacy_telemetry_decode(
             NULL, 1, record, sizeof(record), &record_length,
             &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_legacy_telemetry_decode(
             detail_one, sizeof(detail_one), record, sizeof(record), NULL,
             &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_INVALID_ARGUMENT);

  union {
    max_align_t alignment;
    uint8_t bytes[64];
  } overlap;
  assert(float_view_refloat_legacy_telemetry_decode(
             detail_one, sizeof(detail_one), overlap.bytes,
             sizeof(overlap.bytes), (size_t *)(void *)overlap.bytes,
             &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_legacy_telemetry_decode(
             detail_one, sizeof(detail_one), record,
             FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_MAX_RECORD_BYTES + 1,
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_legacy_telemetry_decode(
             (const uint8_t *)1,
             FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_MAX_PAYLOAD_BYTES + 1,
             NULL, 0, &record_length, &unsupported_command) ==
         FLOAT_VIEW_REFLOAT_LEGACY_TELEMETRY_STATUS_TRUNCATED_PAYLOAD);

  return 0;
}
