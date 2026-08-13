## Why

The safe Rust Refloat dynamic-telemetry candidate now has source-derived, adversarial, repository, and all-host physical evidence, while the shared Apple backend still duplicates command-31/32 decoding in Swift. Selecting it for sibling iOS/macOS is the smallest reversible production transfer and keeps Android independently testable for its later cutover.

## What Changes

- Extend `refloat-dynamic-telemetry` with canonical bounded versioned records for negotiated command-32 layouts and command-31 samples, including ordered extensions and complete source float16 behavior.
- Add a temporary, narrow, three-symbol Apple C bridge: ABI version, layout decode, and realtime decode. It uses caller-owned buffers, fixed statuses, checked disjoint regions, panic containment, and no opaque Rust handle or generated binding.
- Build and link the bridge from locked source for macOS, iOS device, and iOS simulator without committed native archives.
- Replace the shared Apple `RefloatDynamicTelemetryDecoder` wire parser with a strict mechanical record adapter. The existing connection generation retains the validated canonical layout record and resets it on reconnect.
- Delete superseded Swift command-31/32 envelope, identifier, extent, float16, semantic, alert/fault, and extension parsing after Rust is selected; add no source or runtime fallback.
- Preserve frontend contracts, normalized telemetry models, compatibility, direct/CAN routing, framing, polling, compact-cache merging, application behavior, SQLite, sessions, and all OS capabilities.
- Prove safe-record, compiled C-ABI, Swift-adapter, packaging, ownership, complete software, and Android/iOS/macOS ESP32 regression evidence. Only bounded location-sample absence may be deferred.
- Treat this bridge as migration scaffolding to be deleted when complete protocol/application ownership moves behind the final Rust runtime.
- Non-goals: cutting Android dynamic telemetry over; moving the loopback server, application service, SQLite, BLE, location, background execution, files, theme, lifecycle, permissions, WebViews, routing, polling, persistence formats, or the frontend contract.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe dynamic-telemetry component as Apple’s sole command-31/32 decoder while retaining native orchestration and an independent Android cutover.
- `platform-hosts`: Define the temporary three-symbol Apple bridge, canonical record ownership, sibling-host packaging, Android isolation, and deletion of superseded Swift parsing.
- `backend-conformance`: Require focused record/ABI/adapter, clean packaging, no-fallback, complete software, and all-host ESP32 evidence for the ownership transfer.

## Impact

This affects the locked `backends/core` workspace, the safe dynamic-telemetry crate, one new Apple bridge crate and hand-written C module, shared bridge build metadata, SwiftPM/Xcode linkage, the shared Apple Refloat adapter and connection-scoped layout state, focused Rust/C/Swift tests, audits, and OpenSpec requirements. Android production sources and artifacts, frontend APIs, SQLite schemas/data, persisted telemetry formats, and user-visible behavior remain unchanged.
