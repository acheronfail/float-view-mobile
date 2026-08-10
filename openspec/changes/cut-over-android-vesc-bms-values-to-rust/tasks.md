## 1. Android BMS values bridge

- [ ] 1.1 Add the `vesc-bms-values-android-jni` workspace crate with the fixed two-symbol ABI, expected-CAN sentinel, bounded versioned result record, fixed status mapping, panic containment, and exact safe-decoder dependency.
- [ ] 1.2 Implement bounded JNI null, shape, alias, expected-CAN, extent, pending-exception, copy, capacity, and primitive-result handling over the locked minimal `jni-sys` binding.
- [ ] 1.3 Add focused Rust/raw-JNI tests for exact legacy/extended and zero/maximum records, every semantic status, invalid callers and expected CAN, insufficient output, fixed record bytes, redaction, and contained panics.
- [ ] 1.4 Add the pinned offline host/four-ABI build helper with exact two-symbol and artifact-architecture audits and no committed native output.

## 2. Android packaging and production selection

- [ ] 2.1 Wire disjoint BMS host/debug/release outputs into Gradle unit tests and all four Android ABI packages without changing the framing, firmware-identity, or custom-configuration bridges.
- [ ] 2.2 Add the thin Kotlin bridge loader and versioned `VescBmsValuesDecoder` adapter with one-call status/result/record validation and `BmsSnapshot` mapping while preserving observation time and established errors.
- [ ] 2.3 Select the adapter exactly once from `VescPayloadDecoder.bmsValues` without changing request construction, callers, discovery, routing, scheduling, polling, availability, or application-state behavior.
- [ ] 2.4 Add focused JVM production-path and raw-bridge tests for exact legacy/extended and zero/maximum mapping, malformed input, ABI/status/result corruption, finite numeric and boolean validation, optional-tail consistency, bounds, redaction, and absence of a Kotlin fallback.
- [ ] 2.5 Delete the superseded Kotlin command-96 field, collection, balancing, reported-CAN, BMS auto-float, status-tail, and remainder algorithms while retaining generic reader behavior required by realtime and current-limit decoding.
- [ ] 2.6 Extend root dependency, symbol, packaging, ABI, ownership, generated-artifact, source-selection, and source-deletion audits to require the Android BMS bridge and preserve Apple isolation.

## 3. Software verification

- [ ] 3.1 Pass locked Rust formatting, warnings-denied linting, workspace tests, focused Android JNI/Kotlin tests, exact audits, and clean debug/release four-ABI packaging.
- [ ] 3.2 Pass the complete `just verify` software gate after Android selects Rust and the Kotlin algorithms are deleted.

## 4. Physical cutover and final gate

- [ ] 4.1 Run `bms-direct-controller`, `bms-express`, and `bms-standalone` through production BLE on the connected Android device using the Rust JNI decoder.
- [ ] 4.2 Run the same three focused scenarios on the connected iOS device through the unchanged Apple Rust bridge.
- [ ] 4.3 Run the same three focused scenarios on the macOS host through the unchanged Apple Rust bridge.
- [ ] 4.4 Confirm required cleanup and every non-location assertion on all hosts; defer only bounded-time location sample absence.
- [ ] 4.5 Confirm the Kotlin structural parser is absent and rerun the complete `just verify` gate after physical cutover evidence succeeds.
