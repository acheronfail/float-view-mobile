## 1. Canonical safe telemetry record

- [x] 1.1 Add the allocation-free version-1 legacy telemetry record, exact constants, closed record outcome, and sole serializer to `refloat-legacy-telemetry` without changing its dependency or unsafe-code policy.
- [x] 1.2 Add focused safe-Rust tests for exact detail-level 1–4 and active-fault records, every closed code/flag/option, sentinel absence, numeric and length boundaries, impossible internal results, and redacted diagnostics.

## 2. Temporary Apple bridge

- [x] 2.1 Add `refloat-legacy-telemetry-apple-ffi` with the exact two-symbol ABI, fixed statuses, bounded/disjoint caller regions, shared-record publication, panic containment, and no bridge-local serializer.
- [x] 2.2 Add Rust bridge tests and a compiled C harness covering exact records, all safe outcomes, ABI/status agreement, null/alignment/overlap/capacity bounds, output initialization, and contained failures.
- [x] 2.3 Add the hand-written `CVescRefloatLegacyTelemetry` module and an Apple-only shared catalog entry while making the Android builder reject unavailable JNI metadata and preserving Android's closed six-bridge integration set.
- [x] 2.4 Extend root dependency, workspace-layout, exact-symbol, record-ownership, generated/committed-artifact, and Android-isolation audits for the safe crate and temporary Apple bridge.

## 3. Shared Apple packaging and selection

- [x] 3.1 Add the C module and source-built Rust archive to SwiftPM for shared macOS tests and production linkage.
- [x] 3.2 Add Xcode source inputs, build output, include/search paths, and linkage for macOS, iOS device, and iOS simulator without changing existing bridge artifacts.
- [x] 3.3 Replace `RefloatLegacyTelemetryDecoder` with a strict bridge-record adapter that preserves its API, normalized models, established errors, and payload-safe diagnostics.
- [x] 3.4 Add focused Swift tests for all detail levels, active known/unknown faults, option and sentinel behavior, ABI/status mapping, and exhaustive malformed-record rejection.
- [x] 3.5 Select the Rust adapter from Apple production, delete every superseded Swift command-10 wire algorithm and fallback, and prove compatibility, routing, polling, application, persistence, and frontend ownership remain unchanged.
- [x] 3.6 Update Apple/core ownership documentation to identify the selected decoder and the bridge's planned deletion with the unified Rust application runtime; preserve Android documentation and Kotlin production ownership.

## 4. Software verification

- [x] 4.1 Run formatting, warnings-denied Rust tests/lints, compiled C ABI tests, focused Swift protocol tests, dependency/symbol/source audits, and strict change validation.
- [x] 4.2 Run clean macOS, iOS simulator, and unsigned iOS device packaging plus Android host/four-ABI regression packaging.
- [x] 4.3 Run the complete `just verify` software gate after production selection and Swift parser deletion.

## 5. Physical regression

- [x] 5.1 Run focused identity, direct/CAN control, applicable BMS, telemetry, and background/session scenarios on the connected Android device against the ESP32 fixture through unchanged Kotlin.
- [x] 5.2 Run the same applicable focused scenarios on the connected iOS device through production Rust legacy telemetry decoding.
- [x] 5.3 Run focused identity, direct/CAN control, applicable BMS, and telemetry scenarios on the macOS host through production Rust legacy telemetry decoding.
- [x] 5.4 Record only bounded location-sample absence as deferred manual coverage; require every non-location assertion and cleanup to pass, then rerun `just verify` and strict OpenSpec validation in the final implementation state.
