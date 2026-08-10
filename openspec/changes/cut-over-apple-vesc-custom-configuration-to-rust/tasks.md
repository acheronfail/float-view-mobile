## 1. Apple custom-configuration bridge

- [ ] 1.1 Add the `vesc-custom-configuration-apple-ffi` workspace crate with the fixed three-symbol ABI, encode operations, decode result record, status mapping, bounded disjoint caller regions, panic containment, and no payload-bearing diagnostics.
- [ ] 1.2 Add exhaustive Rust ABI tests for exact requests/responses, every operation and response kind, unsupported and malformed payloads, null/capacity/overlap/maximum failures, fixed record bytes, and contained failures.
- [ ] 1.3 Add the hand-written `CVescCustomConfiguration` header/module and compiled C harness covering ABI agreement, caller buffers, fixed statuses and records, invalid regions, and exact symbol names.
- [ ] 1.4 Add the locked Apple bridge build helper and root dependency, source-layout, exact-symbol, target-architecture, and generated/committed-artifact audits.

## 2. Shared Apple packaging

- [ ] 2.1 Add the C module and local Rust archive to SwiftPM so shared macOS codec tests link the production bridge from source.
- [ ] 2.2 Add Xcode source-build, input/output, search-path, and linkage configuration for macOS, iOS device, and iOS simulator without changing the existing framing or firmware-identity bridges.
- [ ] 2.3 Add clean macOS, iOS simulator, and unsigned iOS device packaging checks that prove the correct bridge architecture and production source selection.

## 3. Shared Apple production cutover

- [ ] 3.1 Replace `VescCustomConfigurationCodec` internals with the versioned Rust bridge adapter while preserving its public request methods, maximum chunk size, response variants, and established protocol errors.
- [ ] 3.2 Add focused Swift tests for exact request and response mapping, opaque range copying, unsupported and structural errors, ABI and record rejection, output bounds, and payload-safe diagnostics.
- [ ] 3.3 Delete the superseded Swift command construction, signed-integer encoding, schema-chunk, values-envelope, and acknowledgement parsing, and add no-fallback/source-selection audits.
- [ ] 3.4 Prove Android sources and packaging remain on the existing Kotlin custom-configuration codec with no Apple module, archive, or symbol dependency.

## 4. Canonical and physical configuration coverage

- [ ] 4.1 Extend canonical direct/CAN physical control orchestration to open the Refloat schema, read current and default values, write the same safe current values, verify readback, and retain established no-motor-movement controls and cleanup.
- [ ] 4.2 Add or update deterministic conformance tests for the strengthened physical request sequence and its direct/CAN route behavior.
- [ ] 4.3 Run the focused direct and CAN configuration/control checkpoint on connected Android through unchanged Kotlin against the ESP32 fixture.
- [ ] 4.4 Run the focused direct and CAN configuration/control checkpoint on connected iOS through the shared Rust bridge against the ESP32 fixture.
- [ ] 4.5 Run the focused direct and CAN configuration/control checkpoint on macOS through the shared Rust bridge against the ESP32 fixture.

## 5. Verification

- [ ] 5.1 Run focused safe-Rust, compiled C ABI, Swift codec/service, Android isolation, dependency, symbol, source-deletion, canonical, and clean Apple packaging checks.
- [ ] 5.2 Run `just verify` after production cutover and deletion, with only bounded location sample absence deferred and every other physical/software assertion retained.
