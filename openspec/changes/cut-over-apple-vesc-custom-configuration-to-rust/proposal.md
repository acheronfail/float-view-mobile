## Why

The strict Rust commands 92–95 envelope codec now matches every committed Refloat configuration fixture and malformed boundary, but Apple production still duplicates that structure in Swift. The smallest useful next increment is to select the proven Rust codec beneath the shared Apple backend, removing one Swift implementation from both iOS and macOS without combining the cutover with Android, schema semantics, or application behavior.

## What Changes

- Add one Apple-specific, versioned, caller-owned-buffer C ABI around the safe `vesc-custom-configuration` crate, with fixed operations/statuses, panic containment, bounded input/output regions, payload-safe diagnostics, and an exact exported-symbol allowlist.
- Add locked source builds and a hand-written Apple C module for macOS, iOS device, and iOS simulator without generated bindings or committed native archives.
- Replace shared Apple `VescCustomConfigurationCodec` internals with a thin Rust bridge adapter while preserving its Swift API, `VescCustomConfigurationResponse`, `VescProtocolError`, routing, scheduling, schema/value owners, and complete frontend behavior.
- Delete the superseded Swift commands 92–95 encoding, signed big-endian extent, chunk-validation, values-envelope, and acknowledgement-decoding implementation after verification proves Rust production selection.
- Extend Rust/C/Swift, dependency, symbol, source-selection, packaging, and canonical checks for exact request/response mapping, ABI and region failures, opaque-byte bounds, and absence of a Swift fallback.
- Strengthen the focused direct/CAN physical control checkpoint to exercise configuration schema/current/default/write/readback operations, then run it on the connected iOS device and macOS host through Rust and on Android through unchanged Kotlin ownership.
- Non-goals: Android JNI or codec cutover; schema decompression/XML, typed values, safety policy, mutation coordination, routing, scheduling, application behavior, SQLite, the loopback server, BLE, background work, location, files, theme, lifecycle, persistence, or frontend contract changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe Rust custom-configuration envelope codec for shared Apple production use while retaining higher-level native owners and Android's later independent cutover.
- `platform-hosts`: Define the focused Apple bridge, shared iOS/macOS adapter and packaging boundary, Android isolation, and superseded Swift-code deletion requirements.
- `backend-conformance`: Require compiled ABI, clean packaging, no-fallback, canonical, and all-host physical configuration evidence for the Apple ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one new Apple bridge crate and build helper, one hand-written Apple C module, SwiftPM and Xcode linkage, the shared Apple custom-configuration adapter, focused Rust/C/Swift tests, root build/audit recipes, canonical physical control coverage, OpenSpec requirements, and the ESP32 physical checkpoint. The safe codec keeps its empty production dependency graph; the bridge depends only on that codec and the standard library. No frontend API, persisted data, SQLite schema, Android production source, OS capability, or user-visible behavior changes.
