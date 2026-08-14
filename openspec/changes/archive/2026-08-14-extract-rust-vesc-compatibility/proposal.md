## Why

Controller discovery cannot move cleanly into Rust while exact VESC firmware admission, access-mode, and shutdown policy remain duplicated in Swift and Kotlin. Extracting this small prerequisite now gives the later discovery and unified-runtime slices one safe source of truth without adding another temporary host bridge.

## What Changes

- Add an unselected, unsafe-free `backends/core/vesc-compatibility` candidate that directly consumes the existing safe Rust firmware-identity model and owns the exact audited controller-version catalogue, read-only/read-write decision, read/write authorization, and shutdown availability.
- Prove parity for all hardware types, audited and adjacent versions, stable and test builds, operations, and shutdown combinations with source-derived fixtures and exhaustive boundary tests.
- Add focused dependency, source-selection, bridge-catalogue, package, symbol, and generated-artifact audits so the candidate cannot enter either production host or create a new Apple FFI/Android JNI surface.
- Keep the selected Swift and Kotlin compatibility implementations and all controller discovery, routing, scheduling, controls, persistence, loopback, and OS behavior unchanged until a later unified-runtime composition change.
- Run the complete software gate and the Android, iOS, and macOS ESP32 physical matrix; defer only bounded mobile location acquisition as already agreed.

Non-goals include host cutover or native compatibility deletion, controller-discovery extraction, protocol decoding changes, new supported versions, BLE, application publication, SQLite, the local server, frontend contracts, or any OS-owned capability.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `native-core-migration`: Define the bounded Rust VESC compatibility candidate and its direct composition boundary.
- `platform-hosts`: Locate the unselected candidate while preserving native ownership and the bridge-deletion path.
- `backend-conformance`: Require source parity, isolation audits, software verification, and all-host physical regression evidence.

## Impact

The change adds one small safe Cargo workspace member depending only on `vesc-firmware-identity`, focused tests/audits, root verification wiring, and OpenSpec artifacts. It changes no frontend or native API, persistent format, production dependency graph, bridge catalogue, packaged library, or runtime source selection. The following controller-discovery increment can depend directly on both Rust firmware identity and this compatibility policy rather than duplicating either.
