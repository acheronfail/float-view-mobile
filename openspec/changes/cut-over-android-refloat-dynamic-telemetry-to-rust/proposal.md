## Why

The safe Rust Refloat dynamic-telemetry component and its canonical layout/sample records are already selected by the shared Apple backend, while Android still duplicates command-31/32 decoding in Kotlin. Selecting the same component on Android is the smallest reversible ownership transfer, completes this protocol slice across every host, and removes the remaining native wire parser before moving higher-level behavior into Rust.

## What Changes

- Add a temporary, narrow Android JNI bridge that publishes the existing canonical layout and sample records through JVM-owned primitive arrays with fixed statuses, call-local validation, and panic containment.
- Build and package the bridge from locked source for the host JVM and all four Android ABIs through the existing typed bridge catalogue and Gradle registration.
- Replace `RefloatDynamicTelemetryDecoder` with a strict Rust-record adapter while preserving its API, connection-scoped layout lifetime, normalized models, established errors, compatibility, routing, polling, application behavior, persistence, and frontend behavior.
- Delete the superseded Kotlin command-31/32 envelope, layout, extent, float16, semantic, alert/fault, and extension algorithms after the Rust path is selected; add no runtime or source fallback.
- Preserve the selected Apple bridge and production behavior unchanged while proving exact JNI records, adversarial boundaries, clean packaging, complete software verification, and Android/iOS/macOS ESP32 regression.
- Treat both platform bridges as migration scaffolding that will be removed when the local server, application behavior, protocols, and SQLite run inside one Rust application runtime.
- Non-goals: changing the canonical records or Apple ABI; moving compatibility, routing, scheduling, application behavior, SQLite, loopback transport, BLE, location, background execution, files, theme, lifecycle, permissions, WebView integration, or the frontend contract.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Make safe Rust the sole Android owner of Refloat command-31/32 structural and semantic decoding while retaining higher-level Kotlin owners.
- `platform-hosts`: Define the temporary Android JNI bridge, Kotlin record adapter, packaging boundary, Apple isolation, and superseded Kotlin-decoder deletion.
- `backend-conformance`: Require focused JNI/adapter, clean packaging, no-fallback, complete software, and all-host ESP32 evidence for the Android ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one new Android JNI bridge crate, shared bridge catalogue metadata, Android Gradle packaging, one Kotlin native loader/record adapter, focused Rust/JVM tests, root build/audit recipes, and OpenSpec requirements. Apple production sources and artifacts, frontend APIs, SQLite schemas and data, OS-capability ownership, and user-visible behavior remain unchanged.
