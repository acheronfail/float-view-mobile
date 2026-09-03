## Why

The safe application runtime is now proven but unselected, so Apple still pays for a 2,402-line application state machine, duplicated protocol/application/session decisions, ten per-protocol C bridges, and ten build artifacts. Selecting one coarse runtime boundary now is the first deletion-bearing test of whether the Rust migration improves maintainability and backend scalability rather than only adding code.

## What Changes

- Add one audited, callback-free Apple C boundary around a process-owned Rust application runtime and its session reader/pruner operations.
- Make the shared iOS/macOS service execute Rust commands and ordered effects while retaining CoreBluetooth, clocks/timers, lifecycle/background work, permissions, CoreLocation acquisition, native file presentation, WebView, loopback authentication/transport, product catalogue, and export/import ownership.
- Move active session writes and bounded historical/prune decisions to the existing Rust session owner while preserving the frontend contract and canonical schema.
- **BREAKING (internal only):** delete the selected Swift connection/discovery/scheduler/telemetry/configuration/mutation/protocol/session-decision implementations with no fallback.
- Delete all ten Apple per-protocol FFI crates, their support crate, ten `CVesc*` modules, loaders/adapters, build phases, linker entries, ABI tests, and per-family Apple artifacts; replace them with one coarse Apple runtime artifact.
- Record the before/after source, workspace, bridge, and artifact counts and require a net reduction across Rust plus Apple production source.
- Run focused bridge/service/persistence gates, full `just verify`, and connected-device iOS conformance against the ESP32 fixture. Android physical testing remains deferred and Android production code remains unchanged.

## Capabilities

### New Capabilities

- `apple-unified-runtime-cutover`: Apple selection, host responsibility boundary, deletion, parity, physical verification, and convergence requirements for the unified Rust runtime.

### Modified Capabilities

- `unified-rust-runtime`: Record Apple as the first production selector while retaining Android ownership and the final bridge-retirement gate.

## Impact

Affected areas are `backends/core` (one coarse Apple boundary and removal of eleven Apple-only bridge/support crates), `backends/apple` (application/protocol/storage composition and packaging), Apple-related `justfile` recipes/audits, conformance fixtures, and OpenSpec ownership documentation. The frontend wire contract and SQL schema do not change. Android keeps its current Kotlin application and ten JNI bridges until its separate cutover.
