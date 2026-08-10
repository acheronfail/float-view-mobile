## Why

The safe Rust command-96 BMS values decoder now has source-derived and adversarial parity, while the shared Apple backend still duplicates that structural parser in Swift. Selecting the proven component for both sibling Apple hosts is the smallest next production ownership transfer and keeps Android independently reversible for a later change.

## What Changes

- Add a narrow, versioned, caller-owned-buffer Apple C ABI around `vesc-bms-values`, with fixed statuses, bounded records, panic containment, payload-safe diagnostics, and an exact exported-symbol allowlist.
- Build and link that bridge from locked source for macOS, iOS device, and iOS simulator through the existing SwiftPM and Xcode conventions, without generated bindings or committed native archives.
- Replace shared Apple `VescPayloadDecoder.bmsValues` structural decoding with a thin Rust-backed adapter while preserving `BmsSnapshot`, observation time, errors, routing, discovery, polling, scheduling, application state, and frontend behavior.
- Delete the superseded Swift command-96 field, collection, flag, auto-float, route, UTF-8-tail, and remainder parser after production selection is verified; add no runtime or source fallback.
- Extend focused Rust/C/Swift, dependency, symbol, packaging, source-selection, and canonical checks for exact legacy/extended mapping and every bounded failure class.
- Run the existing focused BMS physical scenarios on the connected Android device, iOS device, and macOS host against the ESP32 fixture; only absence of a location sample within its bounded dwell remains deferred manual coverage.
- Non-goals: Android's later BMS decoder cutover; BMS compatibility, discovery, routing, scheduling, polling, timestamps, availability, or application state; other VESC/Refloat protocols; SQLite; the loopback server; BLE; background work; location; files; theme; lifecycle; persistence; or frontend-contract changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe Rust command-96 decoder as the sole shared Apple structural BMS-values implementation while retaining higher-level native owners and Android's independent later cutover.
- `platform-hosts`: Define the focused Apple bridge, shared iOS/macOS adapter and packaging boundary, Android isolation, and superseded Swift-parser deletion requirements.
- `backend-conformance`: Require compiled ABI, clean packaging, no-fallback, canonical, software, and all-host physical evidence for the Apple ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one new Apple bridge crate and build helper, one hand-written Apple C module, SwiftPM and Xcode linkage, the shared Apple VESC protocol adapter, focused Rust/C/Swift tests, root build/audit recipes, OpenSpec requirements, and the existing ESP32 BMS checkpoint. The safe decoder keeps its empty production dependency graph; the bridge depends only on that decoder and the standard library. Android production code and packages, frontend APIs, persisted data, SQLite schemas, OS capabilities, and user-visible behavior do not change.
