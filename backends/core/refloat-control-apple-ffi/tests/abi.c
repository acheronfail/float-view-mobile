#include "CVescRefloatControl.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

int main(void) {
  assert(float_view_refloat_control_abi_version() ==
         FLOAT_VIEW_REFLOAT_CONTROL_ABI_VERSION);
  assert(FLOAT_VIEW_REFLOAT_CONTROL_ABI_VERSION == 1);
  assert(FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OK == 0);
  assert(FLOAT_VIEW_REFLOAT_CONTROL_STATUS_INTERNAL_FAILURE == 5);
  assert(FLOAT_VIEW_REFLOAT_CONTROL_MAX_PAYLOAD_BYTES == 9);

  uint8_t output[FLOAT_VIEW_REFLOAT_CONTROL_MAX_PAYLOAD_BYTES] = {0};
  size_t output_length = SIZE_MAX;
  uint8_t unsupported = UINT8_MAX;
  assert(float_view_refloat_control_encode(
             FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_PACKAGE,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_NONE, 0,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_NONE, 1, 0, output,
             sizeof(output), &output_length, &unsupported) ==
         FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OK);
  const uint8_t package_enable[] = {36, 101, 12, 0};
  assert(output_length == sizeof(package_enable));
  assert(memcmp(output, package_enable, sizeof(package_enable)) == 0);
  assert(unsupported == 0);

  assert(float_view_refloat_control_encode(
             FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_FLYWHEEL,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_NONE, 0,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_NONE, 0, 0, output,
             sizeof(output), &output_length, &unsupported) ==
         FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OK);
  const uint8_t flywheel_disable[] = {36, 101, 22, 0x80, 0, 0, 0, 0, 1};
  assert(output_length == sizeof(flywheel_disable));
  assert(memcmp(output, flywheel_disable, sizeof(flywheel_disable)) == 0);

  assert(float_view_refloat_control_encode(
             FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_LIGHTING,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_STABLE, 0,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_HEADLIGHTS, 0, 3,
             output, sizeof(output), &output_length, &unsupported) ==
         FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OK);
  const uint8_t stable_lighting[] = {36, 101, 20, 0, 0, 0, 2, 1};
  assert(output_length == sizeof(stable_lighting));
  assert(memcmp(output, stable_lighting, sizeof(stable_lighting)) == 0);

  assert(float_view_refloat_control_encode(
             FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_FATAL_CLEAR,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_NONE, 0,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_NONE, 0, 0, output,
             sizeof(output), &output_length, &unsupported) ==
         FLOAT_VIEW_REFLOAT_CONTROL_STATUS_UNSUPPORTED_COMMAND);
  assert(output_length == 0 && unsupported == 36);

  const uint8_t response[] = {36, 101, 20, 3};
  uint8_t flags = UINT8_MAX;
  assert(float_view_refloat_control_decode_lighting(
             response, sizeof(response),
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_STABLE, &flags) ==
         FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OK);
  assert(flags == 3);
  assert(float_view_refloat_control_decode_lighting(
             response, sizeof(response) - 1,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_STABLE, &flags) ==
         FLOAT_VIEW_REFLOAT_CONTROL_STATUS_INVALID_PAYLOAD);
  assert(flags == 0);

  assert(float_view_refloat_control_encode(
             FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_PACKAGE,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_NONE, 0,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_NONE, 1, 0, NULL, 0,
             &output_length, &unsupported) ==
         FLOAT_VIEW_REFLOAT_CONTROL_STATUS_OUTPUT_TOO_SMALL);
  assert(output_length == 4);
  assert(float_view_refloat_control_encode(
             FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_PACKAGE,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_NONE, 0,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_NONE, 2, 0, output,
             sizeof(output), &output_length, &unsupported) ==
         FLOAT_VIEW_REFLOAT_CONTROL_STATUS_INVALID_ARGUMENT);
  assert(float_view_refloat_control_decode_lighting(
             NULL, 1, FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_STABLE,
             &flags) == FLOAT_VIEW_REFLOAT_CONTROL_STATUS_INVALID_ARGUMENT);

  union {
    max_align_t alignment;
    uint8_t bytes[32];
  } overlap;
  assert(float_view_refloat_control_encode(
             FLOAT_VIEW_REFLOAT_CONTROL_OPERATION_PACKAGE,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_PROFILE_NONE, 0,
             FLOAT_VIEW_REFLOAT_CONTROL_LIGHTING_TARGET_NONE, 1, 0,
             overlap.bytes, 9, (size_t *)(void *)overlap.bytes, &unsupported) ==
         FLOAT_VIEW_REFLOAT_CONTROL_STATUS_INVALID_ARGUMENT);
  return 0;
}
