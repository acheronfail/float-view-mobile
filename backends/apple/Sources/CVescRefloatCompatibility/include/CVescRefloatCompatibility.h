#ifndef FLOAT_VIEW_C_VESC_REFLOAT_COMPATIBILITY_H
#define FLOAT_VIEW_C_VESC_REFLOAT_COMPATIBILITY_H

#include <stddef.h>
#include <stdint.h>

#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_ABI_VERSION 1u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_INFO_REQUEST_BYTES 5u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_RECORD_BYTES 12u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_MAX_PAYLOAD_BYTES 4096u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_MAX_LAYOUT_RECORD_BYTES 4168u

#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OK 0u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_ARGUMENT 1u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_UNSUPPORTED_ENVELOPE 2u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_TRUNCATED_PAYLOAD 3u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_UNSUPPORTED_PROTOCOL 4u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_UNSUPPORTED_RELEASE 5u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INVALID_LAYOUT 6u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_OUTPUT_TOO_SMALL 7u
#define FLOAT_VIEW_REFLOAT_COMPATIBILITY_STATUS_INTERNAL_FAILURE 8u

uint32_t float_view_refloat_compatibility_abi_version(void);
uint32_t float_view_refloat_compatibility_info_request(
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);
uint32_t float_view_refloat_compatibility_decode_info(
    const uint8_t *payload,
    size_t payload_length,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);
uint32_t float_view_refloat_compatibility_admit_layout(
    const uint8_t *layout_record,
    size_t layout_record_length);

#endif
