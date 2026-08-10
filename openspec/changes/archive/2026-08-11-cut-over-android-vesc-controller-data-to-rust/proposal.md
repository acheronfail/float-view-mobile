## Why

Android still structurally decodes VESC command-4 realtime values and command-14 current limits in Kotlin even though the bounded Rust decoder is already proven and selected by both Apple hosts. Cutting over this isolated duplicate now completes cross-platform ownership of the existing component without widening the migration into controller scheduling, application behavior, persistence, or OS services.

## What Changes

- Add one focused Android JNI bridge over the existing safe `vesc-controller-data` component, using the shared allowlisted build tooling, a fixed two-symbol ABI, caller-owned primitive arrays, panic containment, and exact dependency/symbol audits.
- Add a thin Kotlin controller-data adapter that validates and maps the bounded tagged result into the existing `RealtimeValues`, `VescFault`, and `ControllerCurrentLimits` models while preserving established errors.
- Build the bridge from locked offline source for host JVM tests and all four supported Android ABIs in Debug and Release, without generated bindings or committed native libraries.
- Select Rust exactly once from Android's existing VESC payload decoder, then delete the superseded Kotlin byte reader, fixed-point and auto-float parsing, serialization-signature/cell-offset lookup, optional-cell logic, and fault classification.
- Preserve the existing Apple Rust path unchanged and run focused software, clean packaging, `just verify`, and ESP32 controller regressions on connected Android, iOS, and macOS. Only bounded-time location sample absence may remain deferred manual coverage.
- Non-goals: changing request construction, firmware compatibility, VESC framing, direct/CAN routing, scheduling, telemetry polling, fault response, application behavior, SQLite, the local server, BLE, lifecycle/background work, location, files, theme, permissions, the frontend contract, Apple bridges, or introducing a shared FFI runtime/code generator in this step.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe Rust controller-data decoder as Android's sole command-4/14 structural implementation while retaining all higher-level native owners and the existing Apple selection.
- `platform-hosts`: Define the narrow Android JNI bridge, packaging, source-deletion, and Apple-isolation boundaries while keeping OS and application capabilities native.
- `backend-conformance`: Require focused JNI/Kotlin, clean packaging, no-fallback, complete software, and all-host physical controller evidence for Android's ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one new Android JNI bridge crate, the shared bridge catalog, Android Gradle native build/test packaging, the existing Kotlin VESC decoder and tests, root verification/audits, the three listed OpenSpec capabilities, and existing ESP32 identity/control/background checkpoints. The frontend/backend API, application models, SQLite schema and data, controller scheduling and policy, Android OS services, and Apple production artifacts do not change. The new production dependency edge is limited to the Android bridge depending on the existing safe decoder and locked minimal `jni-sys` binding.
