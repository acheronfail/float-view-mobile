## Why

Android production framing is still split: Kotlin owns notification-stream boundary parsing while Rust owns CRC, encoding, and exact frame decoding. The next safe increment should remove that last duplicated Android framing algorithm through the already-proven Rust scanner and existing JNI/NDK path before broader VESC payload, BMS, Refloat, application, SQLite, or loopback-server ownership moves.

## What Changes

- Make Rust the sole Android production implementation of VESC stream boundary interpretation, including leading-noise disposal, short/long header parsing, declared-length bounds, incomplete-frame retention decisions, complete-frame extent, ordered multiple-frame progress, malformed-frame consumption, and aggregate overflow behavior.
- Extend and version the hand-written Android JNI contract with one stateless next-frame scanner operation that returns fixed status and caller-owned primitive lengths; retain Kotlin-owned bounded byte storage without Rust-owned handles, callbacks, returned allocations, or generated bindings.
- Keep the public Kotlin `VescFrameStreamDecoder` surface as a thin storage adapter that delegates every boundary decision to Rust and every complete frame to the existing Rust exact decoder.
- Preserve source-built host-JVM and four-ABI Android packaging while keeping the complete Apple framing path unchanged.
- Delete the Kotlin VESC stream header/length/noise/extent/overflow algorithm after focused, packaging, canonical, and representative physical Android fragmentation/background checkpoints pass, then make repository audits reject its return.
- Non-goals: moving Rust-owned stream lifetime, VESC command or payload interpretation, firmware identity, BMS, Refloat, Bluetooth, application behavior, SQLite, the local server, the frontend contract, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Complete Android framing-algorithm ownership by selecting Rust stream boundary scanning while retaining Kotlin-owned bounded storage.
- `platform-hosts`: Narrow the retained Kotlin framing role to caller-owned byte storage and mechanical consumption beneath the Android backend.
- `native-development-loop`: Require the versioned scanner JNI contract and its source-built host/four-ABI linkage to remain deterministic across existing build paths.
- `backend-conformance`: Require software, packaging, canonical, and focused physical evidence before deleting Kotlin stream parsing.

## Impact

This affects the safe `backends/core/vesc-frame` API only as an already-selected dependency, the Android JNI bridge and Kotlin adapter, host-JVM bridge tests, Android native-library symbol/version audits, focused protocol tests, root `justfile` verification, backend ownership documentation, and OpenSpec requirements. It adds no dependency, Gradle module, generated binding, committed native artifact, frontend-contract change, database or stored-data change, Apple runtime change, user-visible behavior change, or OS-capability ownership change.
