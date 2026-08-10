#ifndef FLOAT_VIEW_C_VESC_FIRMWARE_IDENTITY_H
#define FLOAT_VIEW_C_VESC_FIRMWARE_IDENTITY_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_ABI_VERSION UINT32_C(1)
#define FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_MAX_PAYLOAD_BYTES ((size_t)157)
#define FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_RECORD_HEADER_BYTES ((size_t)26)
#define FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_MAX_ENCODED_NAME_BYTES ((size_t)192)
#define FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_MAX_RECORD_BYTES ((size_t)410)

#define FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_FLAG_FIRMWARE_NAME_PRESENT UINT8_C(1)
#define FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_FLAG_HARDWARE_CRC_PRESENT UINT8_C(2)

typedef uint32_t FloatViewVescFirmwareIdentityStatus;

enum {
  FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_OK = 0,
  FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_INVALID_ARGUMENT = 1,
  FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_UNSUPPORTED_COMMAND = 2,
  FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_TRUNCATED_PAYLOAD = 3,
  FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_OUTPUT_TOO_SMALL = 4,
  FLOAT_VIEW_VESC_FIRMWARE_IDENTITY_STATUS_INTERNAL_FAILURE = 5,
};

/*
 * Successful version-1 record layout:
 *   0 major, 1 minor, 2 test version, 3 hardware type,
 *   4 custom configuration count, 5 option flags,
 *   6..7 hardware-name UTF-8 length (big endian),
 *   8..9 firmware-name UTF-8 length (big endian),
 *   10..21 private UUID, 22..25 optional CRC value (big endian),
 *   26.. replacement-decoded hardware name followed by firmware name.
 */
uint32_t float_view_vesc_firmware_identity_abi_version(void);

/*
 * All non-empty byte regions must be valid for their supplied lengths.
 * output_length must be size_t-aligned and unsupported_command must be valid
 * for one byte. All regions must be pairwise disjoint. Rust retains nothing.
 */
FloatViewVescFirmwareIdentityStatus float_view_vesc_firmware_identity_decode(
    const uint8_t *payload,
    size_t payload_length,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length,
    uint8_t *unsupported_command);

#ifdef __cplusplus
}
#endif

#endif
