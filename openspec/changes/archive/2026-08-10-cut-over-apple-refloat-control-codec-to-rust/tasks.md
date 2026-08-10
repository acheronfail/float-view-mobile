## 1. Apple Refloat control bridge

- [x] 1.1 Add the `refloat-control-apple-ffi` workspace crate with the fixed three-symbol ABI, canonical operation/profile/target inputs, bounded caller-owned outputs, fixed status mapping, pairwise region validation, panic containment, and payload-safe failures.
- [x] 1.2 Add Rust ABI tests for every operation and lighting state, exact source-derived bytes, invalid primitive combinations, malformed responses, unavailable fatal clear, null/capacity/overlap/alignment regions, insufficient output, deterministic scalar outputs, and contained panics.
- [x] 1.3 Add the hand-written `CVescRefloatControl` header/module and compiled C harness covering ABI agreement, exact payloads/results/statuses, caller regions, invalid inputs, output bounds, and exact symbol names.
- [x] 1.4 Add `refloat-control` to the shared bridge catalogue and extend root dependency, source-layout, exact-symbol, ABI-version, target-architecture, generated-artifact, and unavailable-toolchain audits without adding another build helper.

## 2. Shared Apple packaging

- [x] 2.1 Add the C module and source-built Rust archive to SwiftPM so shared macOS protocol tests link the production bridge.
- [x] 2.2 Add the generic bridge build phase, inputs/outputs, include/search paths, and linkage to the iOS Xcode project for iOS device and simulator without changing existing bridges.
- [x] 2.3 Extend clean macOS, iOS simulator, and unsigned iOS device packaging checks to prove the correct Refloat control bridge architecture and shared production source selection.

## 3. Shared Apple production cutover

- [x] 3.1 Replace Swift control-wire construction and lighting-response parsing with a versioned `CVescRefloatControl` adapter that validates statuses, lengths, primitive results, and maps failures to established protocol errors.
- [x] 3.2 Propagate closed package/flywheel bridge failures through the existing throwing service and mutation-coordinator boundaries without changing compatibility, routing, framing, authorization, scheduling, confirmation, telemetry, or application state.
- [x] 3.3 Add focused Swift tests for every fixture request/response, both profiles, all current/target/enabled/query combinations, all defined response flags, ABI/status/length/result rejection, unavailable fatal clear, and payload-safe diagnostics.
- [x] 3.4 Delete the superseded Swift package, flywheel, lighting, query, fatal-clear, and lighting-response algorithms and add exact no-fallback/source-selection audits while retaining shared Refloat helpers used outside controls.
- [x] 3.5 Prove Android source and packaging remain on the Kotlin Refloat control codec with no Apple module, archive, symbol, or Rust call-path dependency.

## 4. Software verification

- [x] 4.1 Run focused safe-Rust, bridge Rust tests, compiled C ABI, Swift control tests, conformance controls, dependency, symbol, source-deletion, Android-isolation, and clean Apple packaging checks.
- [x] 4.2 Run `just verify` after production cutover and Swift algorithm deletion.

## 5. Physical Refloat control verification

- [x] 5.1 Run `control-direct`, `control-can`, and mobile background regression against the ESP32 fixture on the connected Android device through unchanged Kotlin.
- [x] 5.2 Run the same applicable scenarios on the connected iOS device through the shared Rust control codec.
- [x] 5.3 Run `control-direct` and `control-can` on the macOS host through the shared Rust control codec.
- [x] 5.4 Record only bounded location-sample absence as deferred manual coverage; do not waive permission, capability, monotonic-boundary, BLE, routing, control, confirmation, state, background/session, no-motor-movement, or cleanup failures.
