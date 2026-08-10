## Why

Android still structurally decodes `COMM_BMS_GET_VALUES = 96` in Kotlin even though the audited safe Rust decoder is already the production owner on Apple. Cutting over this one remaining host duplicate now completes cross-platform ownership of the proven BMS-values component without widening the migration into BMS lifecycle, application behavior, or persistence.

## What Changes

- Add one focused Android JNI bridge crate over the existing safe Rust BMS-values component, with locked offline host/four-ABI builds, a fixed bounded record/status ABI, panic containment, and exact symbol/dependency audits.
- Replace Android's Kotlin command-96 structural parser with a thin adapter that invokes Rust once, validates and maps a bounded caller-owned record into the existing `BmsSnapshot`, attaches the existing observation time, and preserves established protocol errors.
- Package the source-built bridge for all four supported Android ABIs and load the exact source-built host artifact for JVM tests without changing Android's framing, firmware-identity, or custom-configuration bridges.
- Delete the superseded Kotlin command-96 field, collection, balancing, auto-float, reported-CAN, status-tail, and remainder algorithms after focused, packaging, software, and physical verification succeeds; retain no Kotlin fallback.
- Run the complete software gate and the existing standalone, direct-controller, and Express ESP32 BMS checkpoints on connected Android, iOS, and macOS. Only absence of a location observation during its bounded dwell may remain deferred manual coverage.
- Non-goals: BMS compatibility, discovery, request construction, direct/CAN routing, scheduling, polling, timestamps, availability, application state, SQLite, the local server, BLE, background work, location, files, theme, lifecycle, permissions, persistence, frontend-contract behavior, or Apple bridge changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe Rust command-96 decoder as Android's sole structural BMS-values implementation while retaining higher-level native owners and the existing Apple path.
- `platform-hosts`: Define the narrow Android JNI ownership, packaging, source-deletion, and Apple-isolation boundaries while keeping OS and application capabilities native.
- `backend-conformance`: Require focused JNI, clean packaging, no-fallback, complete software, and all-host physical BMS evidence for Android's ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one new Android JNI bridge and build helper, Android Gradle native build/test packaging, the existing Kotlin VESC BMS decoder and tests, root verification/audits, the three listed OpenSpec capabilities, and the existing ESP32 BMS checkpoint. The frontend/backend API, `BmsSnapshot`, SQLite schema and persisted data, BMS application policy, Android OS services, and Apple production artifacts do not change. The only new production dependency edge is the Android bridge to the existing safe decoder plus the already locked minimal `jni-sys` binding; no generated binding or committed native binary is added.
