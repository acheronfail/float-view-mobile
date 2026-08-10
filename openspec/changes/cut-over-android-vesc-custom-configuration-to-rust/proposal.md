## Why

Android still encodes and structurally decodes VESC custom-configuration commands 92–95 in Kotlin even though the audited safe Rust codec is already the production owner on Apple. Cutting over this one duplicate envelope boundary now completes cross-platform ownership of the proven component without widening the migration into Refloat schema semantics, application behavior, or persistence.

## What Changes

- Add one focused Android JNI bridge crate over the existing safe Rust custom-configuration component, with locked offline host/four-ABI builds, fixed operations/status/result metadata, caller-owned primitive arrays, panic containment, and exact symbol/dependency audits.
- Replace Android's Kotlin command-92–95 construction and response-envelope parsing with a thin adapter that invokes Rust, validates bounded primitive results and opaque ranges, and preserves the existing `VescCustomConfigurationCodec` API, response types, and error classes.
- Package the source-built bridge for all four supported Android ABIs and load the exact source-built host artifact for JVM tests without changing the existing framing or firmware-identity bridges.
- Delete the superseded Kotlin command construction, signed big-endian integer, schema-chunk, values-envelope, and acknowledgement algorithms after focused, packaging, software, and physical verification succeeds; retain no Kotlin fallback.
- Run the complete software gate and the same direct/CAN ESP32 configuration/control checkpoint on connected Android, iOS, and macOS. Only absence of a location observation during its bounded dwell may remain deferred manual coverage.
- Non-goals: schema assembly/decompression/XML, typed Refloat values, signatures, compatibility or safety policy, mutation coordination, routing, scheduling, application behavior, SQLite, the local server, BLE, background work, location, files, theme, lifecycle, permissions, persistence, frontend contract, or Apple bridge changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe Rust custom-configuration codec as Android's sole commands 92–95 envelope implementation and forbid a Kotlin fallback.
- `platform-hosts`: Define the narrow Android JNI ownership, packaging, source-deletion, and Apple-isolation boundaries while retaining higher-level and OS capability owners.
- `backend-conformance`: Require focused JNI, clean packaging, no-fallback, complete software, and all-host physical configuration evidence for Android's ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one new Android JNI bridge and build helper, Android Gradle native build/test packaging, the existing Kotlin custom-configuration codec and tests, root verification/audits, and the three listed OpenSpec capabilities. The frontend/backend contract, SQLite schema and data, Refloat schema/value services, Android OS services, and Apple production artifacts do not change. The only new production dependency edge is the Android bridge to the existing safe codec plus the already locked minimal `jni-sys` binding; no generated binding or committed native binary is added.
