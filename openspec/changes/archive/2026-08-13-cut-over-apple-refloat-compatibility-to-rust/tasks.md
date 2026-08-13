## 1. Canonical safe compatibility boundary

- [x] 1.1 Add the version-1 fixed compatibility record, closed record outcomes, sole serializer/parser, and atomic INFO decode/profile selection to `refloat-compatibility` while preserving unsafe-free and third-party-dependency-free behavior.
- [x] 1.2 Admit the canonical Rust dynamic-layout record through a one-way safe dependency and remove no connection-lifetime ownership from the hosts.
- [x] 1.3 Add focused safe-Rust tests for exact request/records, every source identity/catalogue/profile, record corruption, unknown releases, layout groups/extras/order, every input/record/layout bound, and redacted diagnostics.

## 2. Temporary Apple bridge

- [x] 2.1 Add `refloat-compatibility-apple-ffi` with exactly ABI-version, INFO-request, INFO-decode, and layout-admission symbols, fixed statuses, bounded pairwise-disjoint regions, deterministic outputs, shared-record publication, and panic containment.
- [x] 2.2 Add Rust bridge tests and a compiled C harness covering exact bytes, all safe outcomes, ABI/status agreement, null/alignment/overlap/capacity bounds, output initialization, and contained failures.
- [x] 2.3 Add the hand-written `CVescRefloatCompatibility` module and Apple-only shared catalogue entry while preserving Android’s closed eight-bridge Gradle/task/artifact set.
- [x] 2.4 Extend root dependency, workspace-layout, exact-symbol, record-ownership, generated/committed-artifact, native-algorithm-deletion, and Android-isolation audits for the safe crate and temporary bridge.

## 3. Shared Apple packaging and selection

- [x] 3.1 Add the C module and source-built Rust archive to SwiftPM for shared macOS tests and production linkage.
- [x] 3.2 Add Xcode source inputs, build output, include/search paths, and linkage for iOS device and simulator without changing existing bridge artifacts.
- [x] 3.3 Replace the Swift INFO/catalogue/layout policy with a strict compatibility-record adapter and carry one accepted decision through protocol messages, controller discovery, control setup, and supported-state publication.
- [x] 3.4 Reuse the current-generation canonical dynamic-layout record for Rust admission, preserving reset/replacement behavior and existing field-list publication.
- [x] 3.5 Add focused Swift tests for exact requests and all INFO/profile/layout decisions, ABI/status/record mapping, discovery/control propagation, connection reset/replacement, and exhaustive malformed-record rejection.
- [x] 3.6 Select Rust on both Apple hosts, delete superseded Swift INFO offsets, catalogue/profile derivation, required identifiers, set admission, and all fallback paths, then update ownership documentation.

## 4. Software verification

- [x] 4.1 Run formatting, warnings-denied Rust tests/lints, compiled C ABI tests, focused Swift protocol/application tests, dependency/symbol/source audits, and strict change validation.
- [x] 4.2 Run clean macOS, iOS simulator, and unsigned iOS device packaging plus Android host/four-ABI regression packaging.
- [x] 4.3 Run the complete `just verify` software gate after production selection and Swift algorithm deletion.

## 5. Physical regression

- [x] 5.1 Run focused identity, direct/legacy telemetry, direct/CAN control, applicable BMS, ride, unexpected-disconnect, and background/session scenarios on the connected Android device against the ESP32 fixture through unchanged Kotlin.
- [x] 5.2 Run the same applicable focused scenarios on the connected iOS device through production Rust compatibility decisions.
- [x] 5.3 Run focused identity, direct/legacy telemetry, direct/CAN control, applicable BMS, ride, and unexpected-disconnect scenarios on the macOS host through production Rust compatibility decisions.
- [x] 5.4 Record only bounded location-sample absence as deferred manual coverage; require every non-location assertion and cleanup to pass, then rerun strict OpenSpec validation in the final implementation state.

### Physical evidence

- 2026-08-13: Android physical conformance passed all 17 scenarios against `/dev/cu.usbmodem111401` on connected device `1cfb7634120d7ece` through the unchanged Kotlin compatibility path. The fixture reported `ADVERTISE_FAILED` once during rapid suite re-advertisement before any app assertion, so the complete matrix was rerun successfully with harness-supported one-scenario process isolation. Background BLE/session assertions passed; location was authorized and available but produced no sample during the bounded 10-second dwell, so only location persistence remains deferred to manual coverage.
- 2026-08-13: iOS physical conformance passed all 17 scenarios against the same fixture on connected device `00008101-00195021268B001E` through the production Rust compatibility bridge. Background BLE/session assertions passed and two location rows were persisted during the background checkpoint.
- 2026-08-13: macOS physical conformance passed all 16 applicable scenarios against the same fixture through the production Rust compatibility bridge.
