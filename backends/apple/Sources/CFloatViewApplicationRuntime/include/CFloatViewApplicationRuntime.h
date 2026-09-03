#ifndef FLOAT_VIEW_C_APPLICATION_RUNTIME_H
#define FLOAT_VIEW_C_APPLICATION_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FLOAT_VIEW_APPLICATION_RUNTIME_ABI_VERSION UINT32_C(1)
#define FLOAT_VIEW_APPLICATION_RUNTIME_MAX_COMMAND_BYTES ((size_t)1048576)
#define FLOAT_VIEW_APPLICATION_RUNTIME_MAX_RESULT_BYTES ((size_t)4194304)

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

FloatViewApplicationRuntimeStatus float_view_application_runtime_create(
    uint64_t *out_handle);

FloatViewApplicationRuntimeStatus float_view_application_runtime_destroy(
    uint64_t handle);

/* Thread-safe cooperative interruption for an active reader or pruner. */
FloatViewApplicationRuntimeStatus float_view_application_runtime_interrupt(
    uint64_t handle);

FloatViewApplicationRuntimeStatus float_view_application_runtime_apply(
    uint64_t handle,
    const uint8_t *command,
    size_t command_length,
    size_t *out_required);

FloatViewApplicationRuntimeStatus float_view_application_runtime_copy_result(
    uint64_t handle,
    uint8_t *output,
    size_t output_capacity,
    size_t *out_written);

#ifdef __cplusplus
}
#endif

#endif
