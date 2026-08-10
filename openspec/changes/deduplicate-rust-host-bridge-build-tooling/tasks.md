## 1. Shared build tooling

- [ ] 1.1 Add the closed four-entry bridge catalog with exact package, artifact, diagnostic, JNI-prefix, and sorted JNI-symbol metadata plus unknown-identifier failure.
- [ ] 1.2 Add the shared Apple builder with preserved SDK/architecture/configuration mapping, pinned target checks, locked offline Cargo invocation, copy/`lipo` publication, and actionable invalid-input failures.
- [ ] 1.3 Add the shared Android builder with preserved host/four-ABI mapping, pinned Rust/NDK/API/linker behavior, exact artifact installation, architecture checks, catalogued symbol auditing, and invalid-input failures.
- [ ] 1.4 Add focused catalog/helper checks covering all four identities, unknown identifiers, invalid platform/profile/ABI inputs, missing pinned NDK behavior, and exact executable/non-executable file modes.

## 2. Host integration and deletion

- [ ] 2.1 Rewire root Apple and Android Rust recipes to pass explicit bridge identifiers to the shared helpers without changing recipe names, outputs, ABI tests, or symbol audits.
- [ ] 2.2 Rewire every Android Gradle host/debug/release task to the shared Android helper and catalog while preserving task names, bridge-specific inputs, disjoint outputs, library properties, and packaging dependencies.
- [ ] 2.3 Rewire all four iOS Xcode Rust phases to the shared Apple helper/catalog while preserving phase names, bridge-specific source inputs, archive outputs, and incremental invalidation.
- [ ] 2.4 Delete all eight bridge-local helpers and update structural, ownership, generated-artifact, helper-count, catalog-completeness, missing-toolchain/NDK, and source-build audits to reject drift or reintroduction.
- [ ] 2.5 Update core/host documentation to distinguish independent bridge crates and artifacts from their shared allowlisted build mechanics.

## 3. Software verification

- [ ] 3.1 Pass shell syntax and focused invalid-input checks, all four Apple host builds and C ABI tests/audits, all four Android host JNI tests/audits, and exact symbol/architecture comparisons.
- [ ] 3.2 Pass clean Apple macOS/iOS device/iOS simulator and Android four-ABI Debug/Release packaging with no generated or committed native artifacts.
- [ ] 3.3 Pass strict OpenSpec validation and the complete `just verify` software gate after every caller uses shared tooling and copied helpers are absent.

## 4. Physical regression and final gate

- [ ] 4.1 Run `control-direct`, `bms-express`, and `background` as applicable through production BLE on the connected Android device using shared-tool-built artifacts.
- [ ] 4.2 Run the same applicable focused scenarios on the connected iOS device using shared-tool-built artifacts.
- [ ] 4.3 Run `control-direct` and `bms-express` on the macOS host using shared-tool-built artifacts.
- [ ] 4.4 Confirm cleanup and every non-location assertion on all hosts; defer only bounded-time location sample absence.
- [ ] 4.5 Rerun structural audits and the complete `just verify` gate after physical evidence succeeds.
