## Why

The Rust VESC framing candidate now has component parity, but no shipped backend executes it, so the highest remaining migration risk is the native build and language boundary rather than framing behavior. The next increment should prove that boundary on one platform family with a stateless production slice before introducing stateful FFI or moving more protocol behavior.

## What Changes

- Make the existing Rust CRC-16, frame encoder, and exact frame decoder the only production implementation used by the shared Apple backend on both iOS and macOS.
- Add one minimal Apple-only C ABI bridge with fixed status codes, primitive pointer/length arguments, caller-owned bounded buffers, panic containment, and an explicit exported-symbol allowlist.
- Build and link the Rust static library from locked source for macOS, iOS device, and iOS simulator builds without committing native binaries or running a code generator.
- Keep the public Swift `VescFrameCodec` surface as a thin hand-written adapter so existing protocol call sites and error behavior remain stable.
- Move the shared `VescProtocolError` model out of the current framing implementation, retain Swift stream boundary/reassembly state, and delete the Swift CRC, encode, and exact-decode algorithms after the Apple cutover gates pass.
- Prove through focused ABI/Swift tests, `just verify`, canonical macOS/iOS execution, and the physical iOS BLE checkpoint that Apple production traffic uses Rust without a Swift fallback.
- Retain Android's Kotlin framing implementation and the Rust stream decoder as their current owners/candidates for later independent changes.
- Non-goals: moving stream reassembly state across FFI; adding JNI or Android packaging; moving VESC payload decoding/routing, BMS, Refloat, Bluetooth, application behavior, SQLite, the loopback server, or frontend behavior; adding callbacks, opaque runtime handles, `cbindgen`, generated APIs, committed native binaries, or third-party Rust dependencies.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define a partial host cutover for the stateless Apple frame codec while stream reassembly and Android remain explicitly owned by their existing implementations.
- `platform-hosts`: Record Rust as the Apple production owner for CRC/encode/exact-decode beneath a thin Swift adapter without moving OS-owned platform responsibilities.
- `native-development-loop`: Require deterministic locked Rust static-library builds and linkage for macOS, iOS device, and iOS simulator development and verification.
- `backend-conformance`: Require Apple software and physical gates to demonstrate that selected production framing executes Rust with no native algorithm fallback.

## Impact

This affects the `backends/core` workspace, a new narrow Apple ABI crate and C header, Apple SwiftPM and Xcode linkage, the Swift framing/error source layout, root `justfile` build/audit recipes, Apple ownership documentation, and OpenSpec requirements. It adds no third-party dependency, frontend-contract change, persistence change, database migration, Android change, user-visible behavior, native callback, allocator-sharing contract, or committed build artifact. The Swift CRC/encode/decode implementation is removed only after both Apple hosts and the required physical iOS BLE path pass.
