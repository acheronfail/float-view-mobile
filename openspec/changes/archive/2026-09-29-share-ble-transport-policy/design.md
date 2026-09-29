## Context

Both native `VescBluetoothPolicy` files contain discovery eligibility, write-mode selection, chunking, queue admission and callback checks. `CoreBluetoothAdapter` and `AndroidBluetoothAdapter` also own logical queues and setup timers. The shared application service already owns reconnect, activation and a 10-second connection-attempt deadline. The existing C/JNI exchange can carry closed OS facts/effects; the workspace is already at its 28-member ceiling.

Apple prefers write-with-response, otherwise uses `canSendWriteWithoutResponse`, and caps the negotiated write payload at 244 bytes. Android deliberately retains 20-byte writes after recorded Samsung/ESP32 trouble with larger payloads, and paces no-response writes at 20 ms. Android's absent-CCCD compatibility path is a supported device behavior, not obsolete application-version compatibility.

## Goals / Non-Goals

**Goals:** A Linux adapter supplies capabilities and performs GATT actions without reimplementing portable filtering, buffering, ordering or stale-work protection. Preserve existing hardware behavior and current frontend errors/states.

**Non-Goals:** OS Bluetooth bindings in Rust; changing firmware support, MTU policy or descriptor interoperability; new reconnect decisions; a general native scheduler rewrite; new public API or persistent state.

## Decisions

### One cohesive transport state machine, no helper-call FFI

Add a focused transport module inside the existing `backend-service` crate. Keep its reducer independent of HTTP, UI and OS objects; compose it under application policy. It owns one active connection's logical setup/write state and consumes normalized capabilities/results. This avoids a new crate or independent transport runtime and lets the existing process lifetime enforce teardown. A standalone helper library would leave queue/generation state duplicated; a cross-platform Bluetooth dependency would replace working OS integration and is out of scope.

```text
application policy (connect/reconnect/activation)
                  |
shared BLE policy (profile decisions, queue, write correlation)
                  |
closed OS facts/effects through existing C/JNI families
                  |
native GATT executor (objects, permissions, capabilities, readiness)
```

### Normalize capabilities without erasing OS differences

Native code reports advertised UART presence, opaque device identity, GATT service/characteristic availability, write properties, notification setup result, effective payload ceiling and flow-control availability. Rust selects the profile and preferred write mode and admits advertisements using the shared saved-ID set. Bound candidate observations to the existing discovery limit (50); native object registries remain bounded independently, as retaining an OS object is platform responsibility.

Android reports a 20-byte ceiling and supplies a generation-qualified availability event after its existing 20 ms pacing delay. Apple reports the negotiated ceiling capped at 244 and supplies real write-without-response readiness. Required service/characteristic decisions are shared; descriptor operations and absent-CCCD handling remain native capability outcomes. Do not treat a successful local API submission as a remote write acknowledgement.

### Ordered writes with admission before allocation

Queue complete outbound bytes in Rust, compute chunk count with checked arithmetic before copying/splitting, and emit at most one chunk in flight. Total pending plus in-flight work is limited to 128 chunks and 65,536 payload bytes; per chunk is at most the reported ceiling and never above 244 bytes. Rejection is atomic: no prefix is transmitted for a newly rejected frame. These totals intentionally close the old accounting gap that excluded an already dispatched chunk. The current 1 MiB OS control-envelope and 64 normal callback-admission ceilings remain upper limits, not excuses to widen BLE buffers.

Attach service lifetime, connection generation and write operation ID to every action/result. With-response completion releases exactly the matching chunk. Without-response advances only after native submission acceptance and the platform's readiness/pacing condition. Native code holds at most the current OS operation and bounded callback delivery, not a second logical frame queue. Duplicate, foreign or late results cannot free new capacity or transmit replacement-connection data. A write error clears the affected queue and reports failure once to existing application reconnect policy; never replay an uncertain mutation.

### One owner for setup and stalled-write deadlines

The shared service owns the existing 10-second overall setup deadline; native adapters execute its OS cancellation and report terminal outcomes. Remove duplicate native logical setup-deadline decisions once parity is proven. OS cancellation/object cleanup can still need platform-local bounded waits, which cannot initiate reconnect. Use a 10-second per-in-flight write progress deadline, checked in shared policy through the existing bounded timer exchange. This bounds missing completion/readiness without introducing a new scheduler. Cancel deadlines on terminal completion and connection replacement. Never extend the overall setup window with per-stage retries.

### Keep API and binding shape small

Extend closed Bluetooth facts/effects in the existing service exchange, not one exported symbol per helper. Native callbacks must capture operation/generation identity at issuance rather than label old callbacks with the latest active generation. Keep malformed facts bounded and reserve disconnect/cancel cleanup under saturated admission. Raw identifiers and byte dumps remain excluded from default logs. Shared changes do not alter frontend contract, SQL or archives.

## Risks / Trade-offs

- Apple and Android acknowledge writes differently → model submission, acknowledgement and readiness separately; cover each ordering and the Android pacing workaround.
- Moving queues can strand native callbacks → generation-qualified ownership and tests for old callbacks arriving after reconnect to the same device.
- Stricter total accounting rejects a formerly admitted boundary case → document the internal behavior change and test exact limits/+1 before any emitted write.
- Helper extraction grows more plumbing than it removes → measure production source and bridge symbols, require net source reduction across touched production paths and remove native duplicate state in the selection commits.
- Test simulation bypasses real GATT → physical Apple/Android ESP32 acceptance remains mandatory; shared testing is useful but not a prerequisite for this change.

## Migration Plan

Inventory existing behavior and upstream/fixture evidence first, implement the pure reducer and fault tests, then integrate Apple and Android in buildable slices. Delete each native policy and superseded queue when selected; leave no production fallback. Add physical write-mode/payload/failure scenarios where fixture capabilities permit; unexercisable hardware cases remain explicit rather than counted as passed. Run software gates and real macOS/iOS/Android BLE checkpoints, record source/symbol/dependency deltas, then sync/archive after `just verify`. No data migration. Rollback is source reversion of a verified slice.
