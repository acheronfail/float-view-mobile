## 1. Apple controller-data bridge

- [ ] 1.1 Add the `vesc-controller-data-apple-ffi` workspace crate with the fixed two-symbol ABI, closed decode operations, tagged bounded records, fixed status mapping, disjoint caller-region validation, panic containment, and payload-safe failures.
- [ ] 1.2 Add Rust ABI tests for exact realtime/current-limit records, every safe-decoder status, invalid operation, null/capacity/overlap/maximum regions, insufficient output, fixed record bytes, and contained panics.
- [ ] 1.3 Add the hand-written `CVescControllerData` header/module and compiled C harness covering ABI agreement, exact records/statuses, caller buffers, invalid regions, output bounds, and exact symbol names.
- [ ] 1.4 Add the bridge to the shared allowlisted build catalog and root dependency, source-layout, exact-symbol, target-architecture, and generated/committed-artifact audits.

## 2. Shared Apple packaging

- [ ] 2.1 Add the C module and local Rust archive to SwiftPM so shared macOS protocol tests link the production bridge from source.
- [ ] 2.2 Add Xcode source-build, input/output, include/search-path, and linkage configuration for macOS, iOS device, and iOS simulator without changing existing Apple bridges.
- [ ] 2.3 Extend clean macOS, iOS simulator, and unsigned iOS device packaging checks to prove the correct controller-data bridge architecture and shared production source selection.

## 3. Shared Apple production cutover

- [ ] 3.1 Add the versioned `VescControllerDataDecoder` adapter that validates and maps realtime and current-limit tagged records into the existing Swift models and errors.
- [ ] 3.2 Select the adapter from `VescPayloadDecoder.realtime` and `currentLimits` without changing request construction, routing, framing, scheduling, telemetry, fault response, or application-state behavior.
- [ ] 3.3 Add focused Swift tests for exact normal/faulted realtime mapping, current-limit signatures and optional cells, ABI/record rejection, unsupported and structural errors, finite numeric validation, and payload-safe diagnostics.
- [ ] 3.4 Delete the superseded Swift byte reader, fixed-point and auto-float parsing, signature/offset lookup, optional-cell logic, and fault classification, and add exact no-fallback/source-selection audits.
- [ ] 3.5 Prove Android sources and packaging remain on the existing Kotlin controller-data decoder with no Apple module, archive, symbol, or Rust call-path dependency.

## 4. Software verification

- [ ] 4.1 Run focused safe-Rust, compiled C ABI, Swift protocol, dependency, symbol, source-deletion, Android-isolation, canonical, and clean Apple packaging checks.
- [ ] 4.2 Run `just verify` after production cutover and Swift parser deletion.

## 5. Physical controller verification

- [ ] 5.1 Run focused identity, direct/CAN control, and background/session scenarios against the ESP32 fixture on the connected Android device through unchanged Kotlin.
- [ ] 5.2 Run the same focused controller and background scenarios on the connected iOS device through the shared Rust decoder.
- [ ] 5.3 Run focused identity and direct/CAN control scenarios on the macOS host through the shared Rust decoder.
- [ ] 5.4 Record only bounded location-sample absence as deferred manual coverage; do not waive permission, capability, monotonic-boundary, BLE, controller-data, routing, framing, telemetry, state, session, control, or cleanup failures.
