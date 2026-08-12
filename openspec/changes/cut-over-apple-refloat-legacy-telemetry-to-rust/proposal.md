## Why

The safe Rust Refloat legacy-telemetry candidate already has source-derived and adversarial parity, while the shared Apple backend still owns a duplicate command-10 decoder in Swift. Selecting that candidate for the sibling iOS/macOS backend is the smallest reversible production transfer and leaves Android independently testable for its later cutover.

## What Changes

- Extend `refloat-legacy-telemetry` with one canonical, bounded, versioned result record and closed decoder-to-record outcome so later host bridges do not duplicate serialization.
- Add a temporary, narrow, two-symbol Apple C bridge that publishes the shared record through caller-owned memory with fixed statuses, checked regions, panic containment, and payload-safe failures.
- Build and link that bridge from locked source for macOS, iOS device, and iOS simulator without generated bindings or committed native archives.
- Replace the shared Apple `RefloatLegacyTelemetryDecoder` byte parser with a mechanical Rust-record adapter while preserving its API, normalized models, errors, compatibility, polling, routing, application behavior, persistence, and frontend behavior.
- Delete the superseded Swift command-10 length, byte-reading, scaling, enum-mapping, fault-classification, and field-weakening logic after the Rust path is selected; add no runtime or source fallback.
- Prove exact record parity, boundary failures, packaging, source ownership, canonical behavior, and complete software verification, then run Android, iOS, and macOS physical regression against the ESP32 fixture. Only bounded-time location-sample absence is deferred.
- Treat the Apple bridge as migration scaffolding: a future Android cutover may consume the same safe record, and the eventual unified Rust application runtime will delete both per-protocol host bridges.
- Non-goals: cutting Android legacy telemetry over from Kotlin; adding the unified Rust runtime or moving the loopback server, application service, SQLite, BLE, location, background execution, files, theme, lifecycle, permissions, WebView integration, dynamic Refloat telemetry, compatibility, routing, polling, session behavior, persisted formats, or the frontend contract.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Make the safe Rust candidate and its canonical record the sole shared Apple owner of Refloat command-10 structural and semantic decoding while retaining higher-level native owners and Android's independent later cutover.
- `platform-hosts`: Define the temporary Apple bridge, shared iOS/macOS adapter and packaging boundary, Android isolation, canonical record ownership, and superseded Swift-decoder deletion.
- `backend-conformance`: Require focused record/ABI/adapter, clean packaging, no-fallback, complete software, and all-host ESP32 evidence for the Apple ownership transfer.

## Impact

This affects the locked `backends/core` workspace, the existing safe telemetry crate, one new Apple bridge crate, one hand-written Apple C module, shared bridge build metadata, SwiftPM and Xcode linkage, the shared Apple Refloat adapter, focused Rust/C/Swift tests, root build/audit recipes, and OpenSpec requirements. Android production sources and artifacts, frontend APIs, SQLite schemas and data, OS-capability ownership, and user-visible behavior remain unchanged.
