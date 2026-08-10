#include "CVescCustomConfiguration.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

int main(void) {
  assert(float_view_vesc_custom_configuration_abi_version() ==
         FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_ABI_VERSION);
  assert(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_ABI_VERSION == 1);
  assert(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_OK == 0);
  assert(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_INTERNAL_FAILURE == 5);
  assert(FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESULT_RECORD_BYTES == 28);

  uint8_t output[64] = {0};
  size_t output_length = 0;
  assert(float_view_vesc_custom_configuration_encode(
             FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_SCHEMA, 7, 400, 3,
             NULL, 0, output, sizeof(output), &output_length) ==
         FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_OK);
  const uint8_t schema_request[] = {92, 7, 0, 0, 1, 144, 0, 0, 0, 3};
  assert(output_length == sizeof(schema_request));
  assert(memcmp(output, schema_request, sizeof(schema_request)) == 0);

  const uint8_t values[] = {1, 2, 3, 4};
  assert(float_view_vesc_custom_configuration_encode(
             FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_WRITE, 9, 0, 0,
             values, sizeof(values), output, sizeof(output), &output_length) ==
         FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_OK);
  const uint8_t write_request[] = {95, 9, 1, 2, 3, 4};
  assert(output_length == sizeof(write_request));
  assert(memcmp(output, write_request, sizeof(write_request)) == 0);

  const uint8_t schema_response[] = {92, 7, 0, 0, 0, 4, 0, 0, 0, 0, 1, 2, 3, 4};
  uint8_t record[FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESULT_RECORD_BYTES] = {0};
  uint8_t unsupported = UINT8_MAX;
  assert(float_view_vesc_custom_configuration_decode(
             schema_response, sizeof(schema_response), record, sizeof(record),
             &output_length, &unsupported) ==
         FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_OK);
  const uint8_t expected_record[] = {
      1, 1, 92, 7, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 4,
  };
  assert(output_length == sizeof(expected_record));
  assert(memcmp(record, expected_record, sizeof(expected_record)) == 0);
  assert(unsupported == 0);

  const uint8_t wrong[] = {36};
  assert(float_view_vesc_custom_configuration_decode(
             wrong, sizeof(wrong), record, sizeof(record), &output_length,
             &unsupported) ==
         FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_UNSUPPORTED_COMMAND);
  assert(output_length == 0 && unsupported == 36);

  uint8_t small[2] = {0};
  assert(float_view_vesc_custom_configuration_decode(
             schema_response, sizeof(schema_response), small, sizeof(small),
             &output_length, &unsupported) ==
         FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_OUTPUT_TOO_SMALL);
  assert(output_length == FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_RESULT_RECORD_BYTES);
  assert(float_view_vesc_custom_configuration_encode(
             FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_OPERATION_CURRENT, 0, 1, 0,
             NULL, 0, output, sizeof(output), &output_length) ==
         FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_INVALID_ARGUMENT);
  assert(float_view_vesc_custom_configuration_decode(
             NULL, 1, record, sizeof(record), &output_length, &unsupported) ==
         FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_INVALID_ARGUMENT);

  union {
    max_align_t alignment;
    uint8_t bytes[64];
  } overlap;
  assert(float_view_vesc_custom_configuration_decode(
             schema_response, sizeof(schema_response), overlap.bytes, 28,
             (size_t *)(void *)overlap.bytes, &unsupported) ==
         FLOAT_VIEW_VESC_CUSTOM_CONFIGURATION_STATUS_INVALID_ARGUMENT);
  return 0;
}
