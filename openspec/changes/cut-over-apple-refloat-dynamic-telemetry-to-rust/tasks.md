## 1. Canonical safe dynamic records

- [ ] 1.1 Add version-1 bounded layout and sample records, exact constants, closed record outcomes, canonical layout parsing, and sole serializers to `refloat-dynamic-telemetry` without changing its dependency or unsafe-code policy.
- [ ] 1.2 Add focused safe-Rust tests for exact source layouts/samples, every semantic code/flag/option, all float16 bit patterns, ordered extensions, canonical absence, record corruption, maximum extents, and redacted diagnostics.

## 2. Temporary Apple bridge

- [ ] 2.1 Add `refloat-dynamic-telemetry-apple-ffi` with exactly the ABI-version, layout-decode, and realtime-decode symbols, fixed statuses, bounded pairwise-disjoint regions, shared-record publication, deterministic outputs, and panic containment.
- [ ] 2.2 Add Rust bridge tests and a compiled C harness covering exact records, all safe outcomes, ABI/status agreement, null/alignment/overlap/capacity bounds, output initialization, and contained failures.
- [ ] 2.3 Add the hand-written `CVescRefloatDynamicTelemetry` module and Apple-only shared catalog entry while preserving Android’s closed seven-bridge Gradle/task/artifact set.
- [ ] 2.4 Extend root dependency, workspace-layout, exact-symbol, record-ownership, generated/committed-artifact, native-parser-deletion, and Android-isolation audits for the safe crate and temporary bridge.

## 3. Shared Apple packaging and selection

- [ ] 3.1 Add the C module and source-built Rust archive to SwiftPM for shared macOS tests and production linkage.
- [ ] 3.2 Add Xcode source inputs, build output, include/search paths, and linkage for iOS device and simulator without changing existing bridge artifacts.
- [ ] 3.3 Replace `RefloatDynamicTelemetryDecoder` with a strict bridge-record adapter that exposes existing field lists, retains private canonical layout bytes, preserves normalized models/errors, and contains no command-31/32 parsing.
- [ ] 3.4 Add focused Swift tests for exact layouts, running/non-running/charging/alert/fault samples, complete source float16 values, extensions, ABI/status mapping, layout reset/replacement, and exhaustive malformed-record rejection.
- [ ] 3.5 Select the Rust adapter from Apple production, retain/clear the canonical layout in the existing connection generation, delete all superseded Swift dynamic wire algorithms and semantic mappings, and add no fallback.
- [ ] 3.6 Update Apple/core/Android ownership documentation to identify Apple’s selected decoder, Android’s unchanged Kotlin owner, and the bridge’s planned deletion with the unified Rust runtime.

## 4. Software verification

- [ ] 4.1 Run formatting, warnings-denied Rust tests/lints, compiled C ABI tests, focused Swift protocol tests, dependency/symbol/source audits, and strict change validation.
- [ ] 4.2 Run clean macOS, iOS simulator, and unsigned iOS device packaging plus Android host/four-ABI regression packaging.
- [ ] 4.3 Run the complete `just verify` software gate after production selection and Swift parser deletion.

## 5. Physical regression

- [ ] 5.1 Run focused identity, direct/legacy telemetry, direct/CAN control, applicable BMS, ride, unexpected-disconnect, and background/session scenarios on the connected Android device against the ESP32 fixture through unchanged Kotlin.
- [ ] 5.2 Run the same applicable focused scenarios on the connected iOS device through production Rust dynamic telemetry decoding.
- [ ] 5.3 Run focused identity, direct/legacy telemetry, direct/CAN control, applicable BMS, ride, and unexpected-disconnect scenarios on the macOS host through production Rust dynamic telemetry decoding.
- [ ] 5.4 Record only bounded location-sample absence as deferred manual coverage; require every non-location assertion and cleanup to pass, then rerun strict OpenSpec validation in the final implementation state.
