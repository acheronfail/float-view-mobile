#ifndef FLOAT_VIEW_C_APPLICATION_RUNTIME_H
#define FLOAT_VIEW_C_APPLICATION_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_VIEW_APPLICATION_RUNTIME_ABI_VERSION UINT32_C(1)
#define FLOAT_VIEW_APPLICATION_RUNTIME_MAX_COMMAND_BYTES ((size_t)1048576)
#define FLOAT_VIEW_APPLICATION_RUNTIME_MAX_RESULT_BYTES ((size_t)1048576)

typedef int32_t FloatViewApplicationRuntimeStatus;

enum {
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_OK = 0,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_INVALID_ARGUMENT = 1,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_INVALID_HANDLE = 2,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_INVALID_ENVELOPE = 3,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_RUNTIME_FAILURE = 4,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_RESULT_PENDING = 5,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_BUFFER_TOO_SMALL = 6,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_BUSY = 7,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_INTERNAL = 8,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_INVALID_STATE = 9,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_CONFLICT = 10,
  FLOAT_VIEW_APPLICATION_RUNTIME_STATUS_UNAVAILABLE = 11,
};

/*
 * Handles and tokens are opaque scalar identities, not pointers. Commands and
 * effects are closed version-1 UTF-8 JSON envelopes. Rust never retains caller
 * memory or invokes a host callback. One successful apply stores exactly one
 * result until copy_result publishes it into caller-owned memory.
 */
uint32_t float_view_application_runtime_abi_version(void);

/* Listener commands borrow the process owner and use 1-MiB caller buffers. */
int32_t float_view_application_runtime_server_destroy(uint64_t handle);
int32_t float_view_application_runtime_server_command(uint64_t handle, const uint8_t *input, size_t length, uint8_t *output, size_t capacity, size_t *out_written);
int32_t float_view_application_runtime_service_create(const uint8_t *root, size_t length, uint64_t *out_handle);
int32_t float_view_application_runtime_service_destroy(uint64_t handle);
int32_t float_view_application_runtime_service_apply(uint64_t handle, const uint8_t *command, size_t command_length, size_t *out_required);
int32_t float_view_application_runtime_service_copy_result(uint64_t handle, uint8_t *output, size_t capacity, size_t *out_written);
int32_t float_view_application_runtime_service_server_create(uint64_t service, uint32_t platform, const uint8_t *html, size_t length, uint64_t *out_handle);

#ifdef __cplusplus
}
#endif

#endif
