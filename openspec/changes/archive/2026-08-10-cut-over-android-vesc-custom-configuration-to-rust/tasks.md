## 1. Android custom-configuration bridge

- [x] 1.1 Add the `vesc-custom-configuration-android-jni` workspace crate with the fixed three-symbol ABI, request operations, decode metadata, status mapping, panic containment, and exact safe-codec dependency.
- [x] 1.2 Implement bounded JNI null, shape, alias, extent, pending-exception, copy, capacity, and primitive-result handling over the locked minimal `jni-sys` binding.
- [x] 1.3 Add focused Rust/raw-JNI tests for exact every-release requests and responses, malformed and unsupported payloads, all caller-contract failures, fixed results, redaction, and contained panics.
- [x] 1.4 Add the pinned offline host/four-ABI build helper with exact three-symbol and artifact-architecture audits and no committed native output.

## 2. Android packaging and production selection

- [x] 2.1 Wire disjoint custom-configuration host/debug/release outputs into Gradle unit tests and all four Android ABI packages without changing the framing or firmware-identity bridges.
- [x] 2.2 Add the thin Kotlin bridge loader and convert `VescCustomConfigurationCodec` to one-call ABI/status/result/range validation while preserving its public API, response variants, constants, and established errors.
- [x] 2.3 Add focused JVM production-path and raw-bridge tests for every request/response kind, every-release fixtures, malformed/unsupported input, ABI/status/result corruption, bounds, redaction, and absence of a Kotlin fallback.
- [x] 2.4 Delete the superseded Kotlin command construction, signed-integer encoding/decoding, chunk arithmetic, values-envelope, and acknowledgement parsing.
- [x] 2.5 Extend root dependency, symbol, packaging, ABI, ownership, generated-artifact, and source-deletion audits to require the Android custom-configuration bridge and preserve Apple isolation.

## 3. Software verification

- [x] 3.1 Pass locked Rust formatting, warnings-denied linting, workspace tests, focused Android JNI/Kotlin tests, exact audits, and clean debug/release four-ABI packaging.
- [x] 3.2 Pass the complete `just verify` software gate after Android selects Rust and the Kotlin algorithms are deleted.

## 4. Physical cutover and final gate

- [x] 4.1 Run the focused direct and CAN ESP32 configuration/control checkpoint through production BLE on the connected Android device using the Rust JNI codec.
- [x] 4.2 Run the same focused direct and CAN checkpoint on the connected iOS device through the unchanged Apple Rust bridge.
- [x] 4.3 Run the same focused direct and CAN checkpoint on the macOS host through the unchanged Apple Rust bridge.
- [x] 4.4 Confirm no motor movement, required cleanup, and all non-location assertions on every host; defer only bounded-time location sample absence.
- [x] 4.5 Confirm the Kotlin envelope algorithms are absent and rerun the complete `just verify` gate after physical cutover evidence succeeds.
