## 1. Apple BMS values bridge

- [ ] 1.1 Add the `vesc-bms-values-apple-ffi` workspace crate with the fixed two-symbol ABI, expected-CAN sentinel, bounded versioned result record, fixed status mapping, disjoint caller-region validation, panic containment, and payload-safe failures.
- [ ] 1.2 Add Rust ABI tests for exact legacy/extended records, zero/maximum collections, every semantic status, invalid expected CAN, null/capacity/overlap/maximum regions, insufficient output, fixed record bytes, and contained panics.
- [ ] 1.3 Add the hand-written `CVescBmsValues` header/module and compiled C harness covering ABI agreement, exact records/statuses, caller buffers, invalid regions, output bounds, and exact symbol names.
- [ ] 1.4 Add the locked Apple bridge build helper and root dependency, source-layout, exact-symbol, target-architecture, and generated/committed-artifact audits.

## 2. Shared Apple packaging

- [ ] 2.1 Add the C module and local Rust archive to SwiftPM so shared macOS protocol tests link the production bridge from source.
- [ ] 2.2 Add Xcode source-build, input/output, include/search-path, and linkage configuration for macOS, iOS device, and iOS simulator without changing existing Apple bridges.
- [ ] 2.3 Extend clean macOS, iOS simulator, and unsigned iOS device packaging checks to prove the correct BMS bridge architecture and shared production source selection.

## 3. Shared Apple production cutover

- [ ] 3.1 Add the versioned `VescBmsValuesDecoder` adapter that validates and maps the bounded bridge record into `BmsSnapshot` while preserving observation time and established protocol errors.
- [ ] 3.2 Select the adapter from `VescPayloadDecoder.bmsValues` without changing request construction, callers, discovery, routing, scheduling, polling, availability, or application-state behavior.
- [ ] 3.3 Add focused Swift tests for exact legacy/extended and zero/maximum mapping, ABI/record rejection, unsupported and structural errors, finite numeric and boolean validation, optional-tail consistency, and payload-safe diagnostics.
- [ ] 3.4 Delete the superseded Swift command-96 structural parser and BMS-only reader helpers, and add exact no-fallback/source-selection audits while retaining generic reader behavior needed by realtime and current-limit decoding.
- [ ] 3.5 Prove Android sources and packaging remain on the existing Kotlin BMS values decoder with no Apple module, archive, symbol, or Rust call-path dependency.

## 4. Software verification

- [ ] 4.1 Run focused safe-Rust, compiled C ABI, Swift protocol, dependency, symbol, source-deletion, Android-isolation, canonical, and clean Apple packaging checks.
- [ ] 4.2 Run `just verify` after production cutover and Swift parser deletion.

## 5. Physical BMS verification

- [ ] 5.1 Run `bms-direct-controller`, `bms-express`, and `bms-standalone` against the ESP32 fixture on the connected Android device through unchanged Kotlin.
- [ ] 5.2 Run the same three scenarios on the connected iOS device through the shared Rust decoder.
- [ ] 5.3 Run the same three scenarios on the macOS host through the shared Rust decoder.
- [ ] 5.4 Record only bounded location-sample absence as deferred manual coverage; do not waive permission, capability, monotonic-boundary, BLE, BMS, routing, polling, state, session, or cleanup failures.
