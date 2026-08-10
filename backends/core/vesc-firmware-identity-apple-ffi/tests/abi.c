#include "CVescFirmwareIdentity.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

static const uint8_t stock_bms[] = {
    0, 6, 0, 'V', 'B', 'M', 'S', 0,
    0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
    0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb,
    0, 0, 1, 1,
};

int main(void) {
  assert(float_view_vesc_firmware_identity_abi_version() ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_ABI_VERSION);
  assert(FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_ABI_VERSION == 1);
  assert(FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_OK == 0);
  assert(FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_INTERNAL_FAILURE == 5);
  assert(FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_MAX_PAYLOAD_BYTES == 157);
  assert(FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_MAX_RECORD_BYTES == 410);

  uint8_t record[FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_MAX_RECORD_BYTES] = {0};
  size_t record_length = 0;
  uint8_t unsupported_command = UINT8_MAX;
  assert(float_view_vesc_firmware_identity_decode(
             stock_bms, sizeof(stock_bms), record, sizeof(record),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_OK);
  assert(record_length ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_RECORD_HEADER_BYTES + 4);
  const uint8_t expected_header[] = {
      6, 0, 0, 1, 1, 0, 0, 4, 0, 0,
      0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
      0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb,
      0, 0, 0, 0,
  };
  assert(memcmp(record, expected_header, sizeof(expected_header)) == 0);
  assert(memcmp(record + sizeof(expected_header), "VBMS", 4) == 0);
  assert(unsupported_command == 0);

  const uint8_t unsupported[] = {37};
  record_length = SIZE_MAX;
  unsupported_command = 0;
  assert(float_view_vesc_firmware_identity_decode(
             unsupported, sizeof(unsupported), record, sizeof(record),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_UNSUPPORTED_COMMAND);
  assert(record_length == 0 && unsupported_command == 37);

  const uint8_t truncated[] = {0};
  record_length = SIZE_MAX;
  unsupported_command = UINT8_MAX;
  assert(float_view_vesc_firmware_identity_decode(
             truncated, sizeof(truncated), record, sizeof(record),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_TRUNCATED_PAYLOAD);
  assert(record_length == 0 && unsupported_command == 0);

  uint8_t small[2] = {0};
  assert(float_view_vesc_firmware_identity_decode(
             stock_bms, sizeof(stock_bms), small, sizeof(small),
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_OUTPUT_TOO_SMALL);
  assert(record_length ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_RECORD_HEADER_BYTES + 4);

  assert(float_view_vesc_firmware_identity_decode(
             NULL, 1, record, sizeof(record), &record_length,
             &unsupported_command) ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_INVALID_ARGUMENT);
  assert(float_view_vesc_firmware_identity_decode(
             stock_bms, sizeof(stock_bms), record, sizeof(record), NULL,
             &unsupported_command) ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_INVALID_ARGUMENT);

  union {
    max_align_t alignment;
    uint8_t bytes[64];
  } overlap;
  assert(float_view_vesc_firmware_identity_decode(
             stock_bms, sizeof(stock_bms), overlap.bytes,
             sizeof(overlap.bytes), (size_t *)(void *)overlap.bytes,
             &unsupported_command) ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_INVALID_ARGUMENT);
  assert(float_view_vesc_firmware_identity_decode(
             stock_bms, sizeof(stock_bms), record,
             FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_MAX_RECORD_BYTES + 1,
             &record_length, &unsupported_command) ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_INVALID_ARGUMENT);
  assert(float_view_vesc_firmware_identity_decode(
             (const uint8_t *)1,
             FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_MAX_PAYLOAD_BYTES + 1,
             NULL, 0, &record_length, &unsupported_command) ==
         FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_TRUNCATED_PAYLOAD);

  return 0;
}
