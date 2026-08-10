## Why

The strict Rust `COMM_FW_VERSION` decoder now matches every audited identity and malformed boundary, but Apple production still duplicates that parser in Swift. The smallest useful next increment is to select the proven Rust decoder beneath the shared Apple backend, so one bounded bridge removes the Swift implementation from both iOS and macOS without combining the cutover with Android or another protocol layer.

## What Changes

- Add one Apple-specific, versioned, caller-owned-buffer C ABI around the safe `vesc-firmware-identity` crate, with fixed statuses, panic containment, bounded payload/result regions, private-identity-safe diagnostics, and an exact exported-symbol allowlist.
- Add deterministic locked source builds and hand-written C module integration for macOS, iOS device, and iOS simulator architectures without committing native archives or adding generated bindings.
- Replace the shared Apple `VescFirmwareIdentityDecoder` internals with a thin bridge adapter that preserves `FirmwareInfo`, replacement UTF-8 behavior, `VescProtocolError`, compatibility policy, routing, and all public protocol behavior.
- Delete the superseded Swift common-prefix, terminated-name, hardware-tail, and CRC-tail parser after focused, packaging, canonical, and physical checks prove Rust production selection.
- Extend Rust/C/Swift verification and repository audits to cover ABI version/status/buffer failures, immutable motor/Express/BMS identities, malformed input, private-data redaction, clean Apple packaging, and absence of a Swift fallback.
- Run `just verify` plus focused ESP32 physical identity regression on the connected iOS device, macOS host, and Android device. iOS and macOS SHALL exercise Rust identity decoding; Android SHALL remain a Kotlin-owned regression target for a later independent cutover.
- Non-goals: changing the frontend contract, firmware compatibility admission, CAN routing, command scheduling, VESC framing, BMS values, Refloat, BLE, background work, location, files, theme, lifecycle, application behavior, SQLite, the loopback server, Android JNI, or Android identity ownership.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the Rust firmware identity decoder for shared Apple production use through a bounded bridge while preserving Android's later independent cutover.
- `platform-hosts`: Define the Apple bridge location, shared iOS/macOS ownership, native representation mapping, packaging, and superseded Swift-code deletion requirements.
- `backend-conformance`: Require focused ABI, packaging, canonical, all-host physical, and no-fallback evidence for the Apple identity ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one new Apple bridge crate and build helper, one hand-written Apple C module, SwiftPM and Xcode linkage, the shared Apple firmware identity adapter, focused Rust/C/Swift tests, root build/audit recipes, OpenSpec requirements, and the physical identity checkpoint. The safe decoder keeps its empty production dependency graph; the bridge depends only on that decoder and the standard library. No frontend API, persisted data, SQLite schema, Android production source, OS capability, application behavior, or user-visible behavior changes.
