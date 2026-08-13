#include "CVescRefloatConfigurationCodec.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint8_t schema_record[FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_MAX_SCHEMA_RECORD_BYTES];
static uint8_t value_record[FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_MAX_VALUE_RECORD_BYTES];
static uint8_t encoded[FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_MAX_VALUE_PAYLOAD_BYTES];

static uint8_t *read_fixture(const char *path, size_t *length) {
  FILE *file = fopen(path, "rb");
  assert(file != NULL);
  assert(fseek(file, 0, SEEK_END) == 0);
  long extent = ftell(file);
  assert(extent >= 0);
  assert(fseek(file, 0, SEEK_SET) == 0);
  uint8_t *bytes = malloc((size_t)extent);
  assert(bytes != NULL);
  assert(fread(bytes, 1, (size_t)extent, file) == (size_t)extent);
  assert(fclose(file) == 0);
  *length = (size_t)extent;
  return bytes;
}

int main(void) {
  assert(float_view_refloat_configuration_codec_abi_version() ==
         FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_ABI_VERSION);
  assert(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_ABI_VERSION == 1);
  assert(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_OK == 0);
  assert(FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_INVALID_VALUE == 30);

  size_t compressed_length = 0;
  size_t payload_length = 0;
  uint8_t *compressed = read_fixture(
      "backends/fixtures/refloat-configuration/schemas/1.2.3.qcompress",
      &compressed_length);
  uint8_t *payload = read_fixture(
      "backends/fixtures/refloat-configuration/values/1.2.3-current.bin",
      &payload_length);

  size_t schema_length = SIZE_MAX;
  assert(float_view_refloat_configuration_codec_decode_schema(
             compressed, compressed_length, schema_record, sizeof(schema_record),
             &schema_length) == FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_OK);
  const uint8_t expected_schema_header[] = {1, 1, 0, 0};
  assert(memcmp(schema_record, expected_schema_header, sizeof(expected_schema_header)) == 0);

  size_t values_length = SIZE_MAX;
  assert(float_view_refloat_configuration_codec_decode_values(
             schema_record, schema_length, payload, payload_length, value_record,
             sizeof(value_record), &values_length) ==
         FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_OK);
  const uint8_t expected_values_header[] = {1, 2, 0, 0};
  assert(memcmp(value_record, expected_values_header, sizeof(expected_values_header)) == 0);

  size_t encoded_length = SIZE_MAX;
  assert(float_view_refloat_configuration_codec_encode_values(
             schema_record, schema_length, value_record, values_length, encoded,
             sizeof(encoded), &encoded_length) ==
         FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_OK);
  assert(encoded_length == payload_length);
  assert(memcmp(encoded, payload, payload_length) == 0);

  uint8_t small[2] = {9, 9};
  size_t required = SIZE_MAX;
  assert(float_view_refloat_configuration_codec_decode_schema(
             compressed, compressed_length, small, sizeof(small), &required) ==
         FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_OUTPUT_TOO_SMALL);
  assert(required == schema_length);
  assert(small[0] == 9 && small[1] == 9);

  uint8_t corrupt_schema[FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_MAX_SCHEMA_RECORD_BYTES];
  memcpy(corrupt_schema, schema_record, schema_length);
  corrupt_schema[2] = 1;
  values_length = SIZE_MAX;
  assert(float_view_refloat_configuration_codec_decode_values(
             corrupt_schema, schema_length, payload, payload_length, value_record,
             sizeof(value_record), &values_length) ==
         FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_INVALID_SCHEMA_RECORD);
  assert(values_length == 0);

  assert(float_view_refloat_configuration_codec_decode_schema(
             NULL, 1, schema_record, sizeof(schema_record), &schema_length) ==
         FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_configuration_codec_decode_schema(
             compressed, compressed_length, schema_record, sizeof(schema_record),
             NULL) == FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_configuration_codec_decode_schema(
             compressed, compressed_length, schema_record, sizeof(schema_record),
             (size_t *)(void *)schema_record) ==
         FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_configuration_codec_decode_schema(
             compressed, compressed_length, NULL, 0,
             (size_t *)(void *)(schema_record + 1)) ==
         FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_configuration_codec_decode_schema(
             (const uint8_t *)1,
             FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_MAX_COMPRESSED_BYTES + 1,
             NULL, 0, &schema_length) ==
         FLOAT_VIEW_REFLOAT_CONFIGURATION_CODEC_STATUS_COMPRESSED_TOO_LARGE);

  free(payload);
  free(compressed);
  return 0;
}
