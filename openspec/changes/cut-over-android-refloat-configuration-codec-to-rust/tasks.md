## 1. Temporary Android JNI bridge

- [ ] 1.1 Add `refloat-configuration-codec-android-jni` to the locked workspace with exactly ABI-version, schema-decode, values-decode, and values-encode methods, fixed payload-free statuses, bounded distinct primitive arrays, exact two-pass output publication, shared JNI support, canonical safe-crate delegation, and panic containment.
- [ ] 1.2 Add focused Rust tests covering exact audited schema/value records and write payloads, every safe failure mapping, insufficient capacity, null/exception/reference/array/result/distinctness bounds, deterministic repeated calls, output initialization, and contained failures.

## 2. Android build and ownership integration

- [ ] 2.1 Extend the existing Refloat configuration bridge catalogue entry with exact Android package, library, JNI-prefix, and four-symbol metadata while preserving every Apple field and artifact unchanged.
- [ ] 2.2 Add the tenth typed Android Gradle bridge specification with host and four-ABI Debug/Release tasks, generated JNI merge inputs, host-JVM property, complete dependency invalidation, and exact task/input/output/consumer audits.
- [ ] 2.3 Extend workspace, dependency, symbol, record-ownership, generated-artifact, Kotlin-source-selection, and Apple-isolation audits for the temporary bridge.
- [ ] 2.4 Update Android/core ownership documentation to identify safe Rust as both hosts' selected configuration codec and both platform bridges as planned deletions with the unified Rust runtime.

## 3. Kotlin adapter and production selection

- [ ] 3.1 Replace the Kotlin Refloat configuration protocol implementation with a narrow JNI loader and strict canonical schema/value record reader/writer while preserving existing models, codec entry points, bounded errors, and payload-safe diagnostics.
- [ ] 3.2 Retain and clear the exact canonical schema record atomically with each generation-scoped cached or freshly assembled schema, and require it for current/default decode and write encode.
- [ ] 3.3 Add focused host-JVM tests for every audited release/schema/value type, exact record projection/construction, two-pass calls, ABI/status mapping, malformed records, stale generations, current/default/write/readback behavior, and service cache/schema parity.
- [ ] 3.4 Select Rust in Android production, delete every superseded Kotlin Qt/XML/fingerprint/signature/scalar/framing/scale/CRC algorithm and comparison/fallback path, and prove stateful configuration/application/OS ownership remains unchanged.

## 4. Software and packaging verification

- [ ] 4.1 Run formatting, warnings-denied safe-crate/JNI tests and lints, focused host-JVM tests, dependency/symbol/source audits, strict change validation, and diff checks.
- [ ] 4.2 Run clean host and four-ABI Android Debug/Release packaging and unchanged macOS, iOS simulator, and unsigned iOS device packaging.
- [ ] 4.3 Run the complete `just verify` software gate after production selection and Kotlin algorithm deletion.

## 5. Physical regression

- [ ] 5.1 Run the complete production Android physical scenario matrix against the ESP32 fixture through the selected Rust codec, including direct/CAN configuration schema/current/default/write/readback, BLE, routing, BMS, ride, reconnect, files, theme, persistence, and mobile background behavior.
- [ ] 5.2 Run the complete production iOS physical scenario matrix against the ESP32 fixture through its unchanged selected Rust codec and bridge.
- [ ] 5.3 Run the complete production macOS physical scenario matrix against the ESP32 fixture through its unchanged selected Rust codec and bridge.
- [ ] 5.4 Record only bounded location-sample absence as deferred manual coverage; require every non-location assertion and cleanup to pass, then rerun `just verify`, formatting/diff checks, and strict OpenSpec validation in the final implementation state.
