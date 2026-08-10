## 1. Android controller-data bridge

- [ ] 1.1 Add the `vesc-controller-data-android-jni` workspace crate with the fixed two-symbol ABI, closed decode operations, tagged bounded records, fixed status mapping, panic containment, and exact safe-decoder/JNI dependencies.
- [ ] 1.2 Implement bounded JNI null, shape, alias, operation, extent, pending-exception, copy, capacity, and primitive-result handling over the locked minimal `jni-sys` binding.
- [ ] 1.3 Add focused Rust/raw-JNI tests for exact realtime/current-limit records, every safe-decoder status, invalid operation and caller arrays, insufficient output, fixed record bytes, redaction, and contained panics.
- [ ] 1.4 Add the bridge to the shared allowlisted catalog and root workspace/dependency/symbol/source-layout/generated-artifact audits.

## 2. Android source packaging

- [ ] 2.1 Wire disjoint controller-data host/debug/release outputs and bridge-specific inputs into Gradle unit tests and all four Android ABI packages without changing existing bridge outputs.
- [ ] 2.2 Extend clean host, Debug, and Release packaging checks to prove exact controller-data symbols, architectures, source selection, and absence of committed/generated native artifacts.

## 3. Android production selection and deletion

- [ ] 3.1 Add the thin Kotlin bridge loader and versioned `VescControllerDataDecoder` adapter with one-call status/result/record validation and existing model/error mapping.
- [ ] 3.2 Select the adapter exactly once from `VescPayloadDecoder.realtime` and `currentLimits` without changing request construction, routing, framing, scheduling, telemetry, fault response, or application behavior.
- [ ] 3.3 Add focused JVM production-path and raw-record tests for normal/faulted realtime values, current-limit signatures and optional cells, ABI/status/result corruption, finite numeric validation, fault/cell consistency, bounds, and payload-safe diagnostics.
- [ ] 3.4 Delete the superseded Kotlin byte reader, fixed-point and auto-float parsing, signature/offset lookup, optional-cell logic, `VescFault.fromCode` classification, and runtime fallback paths.
- [ ] 3.5 Extend root ownership/source-deletion/source-selection/package-content audits to require the Android controller-data bridge while preserving Apple bridge isolation.

## 4. Software verification

- [ ] 4.1 Pass locked Rust formatting, warnings-denied linting, workspace tests, focused Android JNI/Kotlin tests, exact audits, canonical conformance, and clean four-ABI Debug/Release packaging.
- [ ] 4.2 Pass strict OpenSpec validation and the complete `just verify` software gate after Android selects Rust and the Kotlin algorithms are absent.

## 5. Physical cutover and final gate

- [ ] 5.1 Run focused identity, direct/CAN control, and background/session scenarios against the ESP32 fixture on the connected Android device through the production Rust JNI decoder.
- [ ] 5.2 Run the same applicable focused controller and background scenarios on the connected iOS device through the unchanged Apple Rust decoder.
- [ ] 5.3 Run focused identity and direct/CAN control scenarios on the macOS host through the unchanged Apple Rust decoder.
- [ ] 5.4 Confirm cleanup and every non-location assertion on all hosts; defer only bounded-time location sample absence, and rerun the complete software gate if physical evidence requires any implementation change.
