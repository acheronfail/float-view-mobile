## 1. Android Refloat control JNI bridge

- [x] 1.1 Add the `refloat-control-android-jni` workspace crate with the fixed three-symbol ABI, canonical operation/profile/target inputs, bounded JVM-owned arrays, fixed status/result mapping, alias and pending-exception validation, panic containment, and payload-safe failures.
- [x] 1.2 Add Rust JNI tests for every operation and lighting state, exact source-derived bytes, invalid primitive combinations, malformed responses, unavailable fatal clear, null/shape/alias/capacity regions, insufficient output, deterministic results, pending exceptions, and contained panics.
- [x] 1.3 Fill the Android `refloat-control` shared-catalogue metadata and extend root dependency, source-layout, exact-symbol, ABI-version, generated-artifact, and unavailable-toolchain audits without adding another build helper.

## 2. Android source-built packaging

- [x] 2.1 Add disjoint host, debug, and release Refloat-control JNI output directories and Gradle build tasks using the generic Android bridge helper.
- [x] 2.2 Add the hand-written `RustRefloatControlBridge` loader, host JVM library property, unit-test dependency, and generated JNI source packaging for all four supported Android ABIs.
- [x] 2.3 Extend clean host tests and debug/release packaging checks to prove exact bridge symbols, ABI agreement, all four architectures, and no generated binding or committed native library.

## 3. Android production cutover

- [x] 3.1 Replace Kotlin control-wire construction and lighting-response parsing with a versioned JNI adapter that validates statuses, lengths, unsupported-command values, lighting flags, and maps failures to established protocol exceptions.
- [x] 3.2 Add focused Kotlin tests for every fixture request/response, both profiles, all current/target/enabled/query combinations, all defined response flags, ABI/status/length/result rejection, unavailable fatal clear, and payload-safe diagnostics.
- [x] 3.3 Delete the superseded Kotlin package, flywheel, lighting, query, fatal-clear, and lighting-response algorithms and add exact no-fallback/source-selection audits while retaining shared Refloat helpers used outside controls.
- [x] 3.4 Prove Apple source and packaging remain on the existing C bridge with no Android JNI artifact, symbol, or call-path dependency.

## 4. Software verification

- [x] 4.1 Run focused safe-Rust, JNI bridge, linked JVM control, conformance-control, dependency, symbol, source-deletion, Apple-isolation, and clean Android packaging checks.
- [x] 4.2 Run `just verify` after production cutover and Kotlin algorithm deletion.

## 5. Physical Refloat control verification

- [x] 5.1 Run `control-direct`, `control-can`, and mobile background regression against the ESP32 fixture on the connected Android device through the Rust control codec.
- [x] 5.2 Run the same applicable scenarios on the connected iOS device through the unchanged Apple Rust control codec.
- [x] 5.3 Run `control-direct` and `control-can` on the macOS host through the unchanged Apple Rust control codec.
- [x] 5.4 Record only bounded location-sample absence as deferred manual coverage; do not waive permission, capability, monotonic-boundary, BLE, routing, framing, control, confirmation, state, background/session, no-motor-movement, or cleanup failures.
