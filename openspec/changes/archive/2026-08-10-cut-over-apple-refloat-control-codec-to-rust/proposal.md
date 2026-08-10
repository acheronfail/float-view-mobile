## Why

The safe Rust Refloat control codec now has exhaustive source-derived and adversarial parity, while the shared Apple backend still duplicates the same safety-sensitive wire encoding and lighting-response validation in Swift. Selecting the proven component for both sibling Apple hosts is the smallest next production ownership transfer and leaves Android independently reversible for a later change.

## What Changes

- Add a narrow, versioned, stateless Apple C ABI around `refloat-control`, using fixed operations and statuses, caller-owned bounded buffers and primitive results, panic containment, payload-safe diagnostics, and an exact three-symbol allowlist.
- Build and link that bridge from locked source for macOS, iOS device, and iOS simulator through the shared bridge catalogue and existing SwiftPM/Xcode conventions, without generated bindings or committed native archives.
- Replace the shared Apple package, flywheel, legacy/stable lighting, zero-mask query, fatal-clear, and lighting-response Swift algorithms with a thin Rust-backed adapter while preserving the existing Swift models, protocol errors, compatibility policy, routing, framing, mutation authorization, scheduling, confirmation, telemetry, and application state.
- Delete the superseded Swift Refloat control wire algorithm after focused bridge, packaging, canonical, and physical verification proves production selection; retain no runtime or source fallback.
- Keep Android on its current Kotlin codec and outside the Apple bridge/package graph until a separate Android cutover.
- Run the complete software gate and focused direct/CAN Refloat control physical regression on the connected Android device, iOS device, and macOS host with the ESP32 fixture. Only absence of a location sample within its bounded dwell is deferred manual coverage.
- Non-goals: Android production selection; Refloat compatibility lookup, telemetry, configuration, mutation coordination, routing, framing, application behavior, SQLite, the loopback server, BLE, background work, location, files, theme, lifecycle, permissions, persistence, or frontend-contract changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe Rust Refloat control codec as the sole shared Apple wire implementation while retaining native policy/application owners and Android's independent later cutover.
- `platform-hosts`: Define the focused Apple bridge, shared iOS/macOS adapter and packaging boundary, Android isolation, and superseded Swift-algorithm deletion requirements.
- `backend-conformance`: Require compiled ABI, clean packaging, no-fallback, canonical, software, and all-host physical evidence for the Apple ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one focused Apple bridge crate, one hand-written `CVescRefloatControl` module, the shared bridge catalogue, SwiftPM and Xcode linkage, the shared Apple Refloat control adapter and tests, root build/audit recipes, OpenSpec requirements, and the existing ESP32 control checkpoints. The safe codec keeps its empty production dependency graph; the bridge depends only on that codec and the Rust standard library. Android production code and packages, frontend APIs, persisted data, SQLite schemas, OS capabilities, and user-visible behavior do not change.
