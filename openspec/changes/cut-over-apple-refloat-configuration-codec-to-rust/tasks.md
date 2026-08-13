## 1. Canonical safe records

- [ ] 1.1 Add bounded versioned schema and value record models, serializers, parsers, and crate-owned maxima to `refloat-configuration-codec`.
- [ ] 1.2 Add schema-record decode, values-record decode, and values-record encode entry points that validate canonicality and schema/value consistency before invoking the existing safe codec.
- [ ] 1.3 Add exhaustive record round-trip, golden-fixture, malformed-record, mismatch, and maximum-bound Rust tests while preserving the existing upstream fingerprint, signature, value-byte, and checksum evidence.

## 2. Temporary Apple bridge

- [ ] 2.1 Add a stateless `refloat-configuration-codec-apple-ffi` crate with exactly four exported symbols, fixed statuses, caller-owned buffers, disjoint-region validation, deterministic output lengths, panic containment, and no retained state.
- [ ] 2.2 Add Rust ABI tests and a compiled-C harness covering exact record bytes, all status mappings, output-too-small behavior, null/oversize/misaligned/overlapping regions, determinism, and repeated independent calls.
- [ ] 2.3 Add the hand-written Apple C header/module and register one Apple-only bridge in the shared typed build catalogue without changing Android's bridge set, packages, Gradle tasks, or artifacts.
- [ ] 2.4 Extend bridge/catalogue/source audits to require the four-symbol surface, safe-crate protocol ownership, Apple-only isolation, and absence of generated bindings or committed native archives.

## 3. Apple packaging and production selection

- [ ] 3.1 Build and link the locked bridge source through SwiftPM for macOS, iOS device, and iOS simulator with complete source and lockfile invalidation.
- [ ] 3.2 Build and link the same bridge and C module through the iOS/macOS Xcode project and archive phases without prebuilt or committed artifacts.
- [ ] 3.3 Replace the Swift Refloat configuration codec with a strict ABI/canonical-record adapter that projects existing schema/value models and mechanically writes value records for Rust validation.
- [ ] 3.4 Retain the exact canonical schema record with the active configuration-service generation and use it for current/default decode and write encode, clearing schema and record together.
- [ ] 3.5 Add focused Swift tests for record projection/construction, ABI and malformed-record rejection, fixed error mapping, stale-generation handling, service cache/schema/current/default/write/readback behavior, and no fallback.
- [ ] 3.6 Select Rust in Apple production, delete superseded Swift decompression/XML/fingerprint/signature/scalar/framing/CRC algorithms and comparison-only paths, and update ownership documentation.

## 4. Software and packaging verification

- [ ] 4.1 Run the safe-crate, bridge, compiled-C, focused Swift, typed-catalogue, source-audit, clean SwiftPM, and clean Xcode checks for macOS, iOS device, and iOS simulator.
- [ ] 4.2 Prove Android's production Kotlin selection and exact JNI/Gradle/package/artifact surface remain unchanged.
- [ ] 4.3 Run formatting and diff checks followed by the complete `just verify` software gate.

## 5. Physical regression and completion

- [ ] 5.1 Run the production Android physical checkpoint against the ESP32 fixture and prove the retained Kotlin codec plus direct/CAN configuration schema/current/default/write/readback, BLE, routing, reconnect, files, theme, and mobile background behavior.
- [ ] 5.2 Run the production iOS physical checkpoint against the ESP32 fixture and prove the selected Rust codec plus direct/CAN configuration schema/current/default/write/readback, BLE, routing, reconnect, files, theme, and mobile background behavior.
- [ ] 5.3 Run the production macOS physical checkpoint against the ESP32 fixture and prove the selected Rust codec plus direct/CAN configuration schema/current/default/write/readback, BLE, routing, reconnect, files, and theme behavior.
- [ ] 5.4 Record any bounded unavailable location sample for later manual verification, require every non-location checkpoint to pass, and rerun the final software gate after physical evidence is captured.
