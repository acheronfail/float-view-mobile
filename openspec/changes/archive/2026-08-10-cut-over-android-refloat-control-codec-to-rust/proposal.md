## Why

The safe Rust Refloat control codec is now the verified production owner on Apple, but Android still duplicates the same package, flywheel, lighting, query, fatal-clear, and lighting-response wire rules in Kotlin. Cutting over Android now completes this bounded protocol slice across all hosts before broader Refloat application behavior moves into Rust.

## What Changes

- Add one focused Android JNI bridge for the existing safe `refloat-control` crate, using the shared bridge catalogue and generic locked Android builder rather than another build helper.
- Build the bridge from source for host JVM tests and all four supported Android ABIs in debug and release, with an exact three-symbol allowlist and no generated bindings or committed native binaries.
- Replace Kotlin control-payload construction and lighting-response structural parsing with a thin ABI/status/result adapter, then delete the superseded Kotlin wire algorithms and prohibit fallback paths.
- Preserve Android compatibility lookup, VESC framing and routing, mutation authorization, scheduling, confirmation, telemetry, application state, SQLite, loopback transport, BLE, background work, location, lifecycle, files, theme, permissions, WebViews, and frontend behavior unchanged.
- Preserve the existing Apple Rust codec and C bridge unchanged and prove host isolation through software, packaging, and ESP32 physical regression.
- Make no frontend contract, persisted-data, database-schema, export, or user-visible behavior change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Transfer Android Refloat live-control wire encoding and lighting-response structural decoding from Kotlin to the existing safe Rust component.
- `platform-hosts`: Define the focused Android JNI bridge, source-built packaging, production adapter, and Apple/platform isolation boundary.
- `backend-conformance`: Require compiled-boundary, source-deletion, packaging, complete software, and all-host physical evidence before Android ownership transfers.

## Impact

Affected areas are `backends/core` workspace/catalogue metadata, one new Android JNI bridge crate, Android Gradle source-build/package wiring, the Kotlin `protocol/refloat` codec adapter and tests, root verification/audits, and OpenSpec requirements. The existing `jni-sys` dependency and generic Android bridge builder are reused; no new third-party dependency or runtime service is introduced. The Kotlin wire algorithm is removed after verification, while Apple code and every OS-owned capability remain unchanged.
