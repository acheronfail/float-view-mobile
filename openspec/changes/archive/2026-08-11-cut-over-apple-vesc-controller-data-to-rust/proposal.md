## Why

The safe Rust controller-data decoder now has fixture and adversarial parity, while the shared Apple backend still duplicates command-4 realtime and command-14 current-limit parsing in Swift. Selecting the proven component for both sibling Apple hosts is the smallest next production ownership transfer and leaves Android independently reversible for a later change.

## What Changes

- Add a narrow, versioned, two-symbol Apple C ABI around `vesc-controller-data`, using one tagged caller-owned result record, fixed statuses, bounded regions, panic containment, and payload-safe diagnostics.
- Build and link that bridge from locked source for macOS, iOS device, and iOS simulator through the shared allowlisted bridge tooling, without generated bindings or committed native archives.
- Replace shared Apple `VescPayloadDecoder.realtime` and `currentLimits` structural decoding with a thin Rust-backed adapter while preserving public models, errors, routing, framing, scheduling, telemetry, fault response, and application state.
- Delete the superseded Swift byte reader, fixed-point/auto-float parsing, serialization-signature table, series-cell lookup, and fault classification after production selection is verified; add no runtime or source fallback.
- Extend focused Rust/C/Swift, dependency, symbol, packaging, source-selection, canonical, and complete software verification for exact records and every bounded failure class.
- Run focused identity, direct/CAN control, and mobile background physical regressions on the connected Android device, iOS device, and macOS host against the ESP32 fixture. Only absence of a location sample within its bounded dwell is deferred.
- Non-goals: Android's later controller-data cutover; request construction, compatibility, routing, framing, scheduling, telemetry polling, fault policy, state publication, application behavior, SQLite, loopback transport, BLE, background work, location, files, theme, lifecycle, persistence, or frontend-contract changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe Rust command-4/command-14 decoder as the sole shared Apple structural controller-data implementation while retaining higher-level native owners and Android's independent later cutover.
- `platform-hosts`: Define the focused Apple bridge, shared iOS/macOS adapter and packaging boundary, Android isolation, and superseded Swift-parser deletion requirements.
- `backend-conformance`: Require compiled ABI, clean packaging, no-fallback, canonical, software, and all-host physical evidence for the Apple ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one new Apple bridge crate, one hand-written Apple C module, the shared bridge catalog, SwiftPM and Xcode linkage, the shared Apple VESC protocol adapter, focused Rust/C/Swift tests, root build/audit recipes, OpenSpec requirements, and the existing ESP32 controller checkpoints. The safe decoder keeps its empty production dependency graph; the bridge depends only on that decoder and the Rust standard library. Android production code and packages, frontend APIs, persisted data, SQLite schemas, OS capabilities, and user-visible behavior do not change.
