## 1. Apple custom-configuration bridge

- [x] 1.1 Add the `vesc-custom-configuration-apple-ffi` workspace crate with the fixed three-symbol ABI, encode operations, decode result record, status mapping, bounded disjoint caller regions, panic containment, and no payload-bearing diagnostics.
- [x] 1.2 Add exhaustive Rust ABI tests for exact requests/responses, every operation and response kind, unsupported and malformed payloads, null/capacity/overlap/maximum failures, fixed record bytes, and contained failures.
- [x] 1.3 Add the hand-written `CVescCustomConfiguration` header/module and compiled C harness covering ABI agreement, caller buffers, fixed statuses and records, invalid regions, and exact symbol names.
- [x] 1.4 Add the locked Apple bridge build helper and root dependency, source-layout, exact-symbol, target-architecture, and generated/committed-artifact audits.

## 2. Shared Apple packaging

- [x] 2.1 Add the C module and local Rust archive to SwiftPM so shared macOS codec tests link the production bridge from source.
- [x] 2.2 Add Xcode source-build, input/output, search-path, and linkage configuration for macOS, iOS device, and iOS simulator without changing the existing framing or firmware-identity bridges.
- [x] 2.3 Add clean macOS, iOS simulator, and unsigned iOS device packaging checks that prove the correct bridge architecture and production source selection.

## 3. Shared Apple production cutover

- [x] 3.1 Replace `VescCustomConfigurationCodec` internals with the versioned Rust bridge adapter while preserving its public request methods, maximum chunk size, response variants, and established protocol errors.
- [x] 3.2 Add focused Swift tests for exact request and response mapping, opaque range copying, unsupported and structural errors, ABI and record rejection, output bounds, and payload-safe diagnostics.
- [x] 3.3 Delete the superseded Swift command construction, signed-integer encoding, schema-chunk, values-envelope, and acknowledgement parsing, and add no-fallback/source-selection audits.
- [x] 3.4 Prove Android sources and packaging remain on the existing Kotlin custom-configuration codec with no Apple module, archive, or symbol dependency.

## 4. Canonical and physical configuration coverage

- [x] 4.1 Extend canonical direct/CAN physical control orchestration to open the Refloat schema, read current and default values, write the audited disposable configuration mutation, verify readback, and retain established no-motor-movement controls and cleanup.
- [x] 4.2 Add or update deterministic conformance tests for the strengthened physical request sequence and its direct/CAN route behavior.
- [x] 4.3 Run the focused direct and CAN configuration/control checkpoint on connected Android through unchanged Kotlin against the ESP32 fixture.
- [x] 4.4 Run the focused direct and CAN configuration/control checkpoint on connected iOS through the shared Rust bridge against the ESP32 fixture.
- [x] 4.5 Run the focused direct and CAN configuration/control checkpoint on macOS through the shared Rust bridge against the ESP32 fixture.

## 5. Verification

- [x] 5.1 Run focused safe-Rust, compiled C ABI, Swift codec/service, Android isolation, dependency, symbol, source-deletion, canonical, and clean Apple packaging checks.
- [x] 5.2 Run `just verify` after production cutover and deletion, with only bounded location sample absence deferred and every other physical/software assertion retained.
