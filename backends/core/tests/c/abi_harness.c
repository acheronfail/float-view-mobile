#include "float_view_core.h"

#include <assert.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <time.h>

_Static_assert(sizeof(FloatViewStatus) == 4, "status width");
_Static_assert(sizeof(FloatViewCommandHeader) == 40, "command header size");
_Static_assert(_Alignof(FloatViewCommandHeader) == 8, "command header alignment");
_Static_assert(offsetof(FloatViewCommandHeader, runtime_generation) == 16,
               "generation offset");
_Static_assert(offsetof(FloatViewCommandHeader, payload_len) == 32,
               "payload length offset");

typedef struct {
  FloatViewRuntime *runtime;
  uint64_t generation;
  FloatViewStatus result;
} WaitContext;

static void *wait_until_stopped(void *raw_context) {
  WaitContext *context = raw_context;
  FloatViewCommandHeader header = {0};
  size_t required = 0;
  uint8_t payload[1] = {0};
  context->result = float_view_runtime_wait_command(
      context->runtime, context->generation, 30000, &header, payload,
      sizeof(payload), &required);
  return NULL;
}

static void test_identity_and_lifecycle(void) {
  assert(float_view_abi_version() == FLOAT_VIEW_ABI_VERSION);

  uint8_t short_output[4] = {0xA5, 0xA5, 0xA5, 0xA5};
  size_t required = 0;
  assert(float_view_build_id(short_output, sizeof(short_output), &required) ==
         FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL);
  assert(required == FLOAT_VIEW_BUILD_ID_CAPACITY);
  assert(short_output[0] == 0xA5);

  uint8_t build_id[FLOAT_VIEW_BUILD_ID_CAPACITY] = {0};
  assert(float_view_build_id(build_id, sizeof(build_id), &required) ==
         FLOAT_VIEW_STATUS_OK);
  assert(memcmp(build_id, "float-view-0.1.0", sizeof(build_id)) == 0);

  assert(float_view_runtime_create(NULL) == FLOAT_VIEW_STATUS_INVALID_ARGUMENT);
  assert(float_view_runtime_destroy(NULL) == FLOAT_VIEW_STATUS_INVALID_HANDLE);
  assert(float_view_runtime_destroy((FloatViewRuntime *)(uintptr_t)999999) ==
         FLOAT_VIEW_STATUS_INVALID_HANDLE);

  FloatViewRuntime *runtime = NULL;
  assert(float_view_runtime_create(&runtime) == FLOAT_VIEW_STATUS_OK);
  assert(runtime != NULL);
  uint64_t generation = 0;
  assert(float_view_runtime_start(runtime, &generation) == FLOAT_VIEW_STATUS_OK);
  assert(generation != 0);
  assert(float_view_runtime_start(runtime, &generation) ==
         FLOAT_VIEW_STATUS_WRONG_STATE);

  uint8_t input[] = {1, 2, 3, 4, 5};
  assert(float_view_runtime_submit_event(runtime, generation + 1, 42, input,
                                         sizeof(input)) ==
         FLOAT_VIEW_STATUS_STALE_GENERATION);
  assert(float_view_runtime_submit_event(runtime, generation, 42, input,
                                         sizeof(input)) ==
         FLOAT_VIEW_STATUS_OK);
  memset(input, 0xEE, sizeof(input));

  FloatViewCommandHeader header;
  memset(&header, 0xA5, sizeof(header));
  uint8_t short_payload[2] = {0xA5, 0xA5};
  required = 0;
  assert(float_view_runtime_wait_command(
             runtime, generation, 0, &header, short_payload,
             sizeof(short_payload), &required) ==
         FLOAT_VIEW_STATUS_BUFFER_TOO_SMALL);
  assert(required == 5);
  assert(short_payload[0] == 0xA5);
  assert(header.abi_version == UINT32_C(0xA5A5A5A5));

  uint8_t payload[5] = {0};
  assert(float_view_runtime_wait_command(runtime, generation, 0, &header,
                                         payload, sizeof(payload), &required) ==
         FLOAT_VIEW_STATUS_OK);
  const uint8_t expected[] = {1, 2, 3, 4, 5};
  assert(memcmp(payload, expected, sizeof(expected)) == 0);
  assert(header.abi_version == FLOAT_VIEW_ABI_VERSION);
  assert(header.kind == 42);
  assert(header.runtime_generation == generation);
  assert(header.payload_len == sizeof(payload));

  assert(float_view_runtime_complete_operation(
             runtime, generation, header.operation_id, 0, NULL, 0) ==
         FLOAT_VIEW_STATUS_OK);
  assert(float_view_runtime_complete_operation(
             runtime, generation, header.operation_id, 0, NULL, 0) ==
         FLOAT_VIEW_STATUS_DUPLICATE_OPERATION);

  WaitContext context = {.runtime = runtime,
                         .generation = generation,
                         .result = FLOAT_VIEW_STATUS_INTERNAL};
  pthread_t waiter;
  assert(pthread_create(&waiter, NULL, wait_until_stopped, &context) == 0);
  const struct timespec pause = {.tv_sec = 0, .tv_nsec = 20000000};
  assert(nanosleep(&pause, NULL) == 0);
  assert(float_view_runtime_stop(runtime) == FLOAT_VIEW_STATUS_OK);
  assert(pthread_join(waiter, NULL) == 0);
  assert(context.result == FLOAT_VIEW_STATUS_STOPPED);
  assert(float_view_runtime_destroy(runtime) == FLOAT_VIEW_STATUS_OK);
  assert(float_view_runtime_destroy(runtime) == FLOAT_VIEW_STATUS_INVALID_HANDLE);
}

static void test_panic_containment(void) {
  FloatViewRuntime *runtime = NULL;
  uint64_t generation = 0;
  assert(float_view_runtime_create(&runtime) == FLOAT_VIEW_STATUS_OK);
  assert(float_view_runtime_start(runtime, &generation) == FLOAT_VIEW_STATUS_OK);
  assert(float_view_runtime_submit_event(runtime, generation, UINT32_MAX, NULL,
                                         0) == FLOAT_VIEW_STATUS_PANIC);
  assert(float_view_runtime_stop(runtime) == FLOAT_VIEW_STATUS_PANIC);
  assert(float_view_runtime_destroy(runtime) == FLOAT_VIEW_STATUS_OK);
}

int main(void) {
  test_identity_and_lifecycle();
  test_panic_containment();
  return 0;
}
