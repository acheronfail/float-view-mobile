## Why

Apple now proves that the unified Rust runtime can replace a native application/protocol stack while reducing source and build surface. Android still carries the equivalent 2,208-line Kotlin service, duplicated decision owners, ten protocol JNI adapters, ten Rust JNI crates, a JNI support crate, thirty Gradle build tasks, and eighty packaged bridge artifacts. Completing the Android cutover is the decisive test of the migration: both mobile hosts must share one runtime without retaining the temporary bridge matrix or a native fallback.

## What Changes

- Extract the already-proven versioned command/effect mapping and registry from the Apple ABI into one safe platform-neutral host-boundary component, leaving thin hand-written Apple C and Android JNI adapters.
- Make Android own one process-lifetime Rust runtime and execute its ordered effects while retaining Android Bluetooth, timers/execution, lifecycle/background service, permissions, location acquisition, native file presentation, WebView, loopback transport/authentication, product catalogue, and import/export responsibilities.
- **BREAKING (internal only):** delete the superseded Kotlin connection/discovery/scheduler/telemetry/configuration/mutation/protocol/session decisions and tests with no production fallback.
- Delete all ten protocol-specific Android JNI crates and Kotlin bridge classes, `android-jni-support`, the bridge catalog/helper, per-family Gradle tasks/source sets/test properties, and eighty Debug/Release ABI artifacts; replace them with one runtime JNI library per ABI/profile and one host-test library.
- Simplify Apple and Android runtime build recipes around the two thin host bindings and install final ownership, artifact-count, source-count, and forbidden-path audits.
- Preserve the frontend contract and SQL schema; run focused boundary/adapter/persistence checks, `just verify`, Android software conformance, and a connected iOS/ESP32 regression. Android physical conformance remains explicitly skipped.

## Capabilities

### New Capabilities

- `android-unified-runtime-cutover`: Android runtime selection, retained OS ownership, deletion, packaging, software parity, iOS physical regression, and final convergence requirements.

### Modified Capabilities

- `unified-rust-runtime`: Record both mobile hosts as selectors and require final removal of all temporary protocol bridges and build-matrix scaffolding.
- `rust-android-jni-support`: Retire the temporary protocol-JNI support contract once its consumers are deleted.
- `rust-host-bridge-build-tooling`: Replace the per-protocol bridge catalog/build matrix with one runtime build path per host.

## Impact

Affected areas are `backends/core`, `backends/android`, the shared Apple runtime binding/build references, `justfile`, conformance evidence, and OpenSpec ownership documentation. The frontend wire contract and canonical SQL schema remain unchanged. Android hardware is not exercised; the connected iOS device and ESP32 fixture provide the requested physical regression after shared-boundary changes.
