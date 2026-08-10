## 1. Shared Android JNI support

- [ ] 1.1 Add the `android-jni-support` workspace crate with only locked `jni-sys = 0.4.1`, re-exported signature types, protocol-neutral errors, and no JNI exports.
- [ ] 1.2 Implement generic panic containment plus a non-retaining per-call environment wrapper for exception, array length, alias, bounded byte read/write, and exact integer publication operations.
- [ ] 1.3 Add deterministic fake-JNI-table tests for null environment/table, pending/new exceptions, null/aliased references, zero/exact/excessive lengths, bounded copies/publication, and panics.
- [ ] 1.4 Add workspace audits enforcing the support dependency boundary, absence of protocol/host/export ownership, and exclusive ownership of raw JNI function-table operations.

## 2. Android bridge migration

- [ ] 2.1 Migrate firmware identity, BMS values, and controller data JNI bridges to shared support while preserving exact statuses, records, capacities, validation order, and adversarial behavior.
- [ ] 2.2 Migrate VESC framing, custom configuration, and Refloat control JNI bridges to shared support while preserving every encode/decode/scan/control symbol and result shape.
- [ ] 2.3 Delete bridge-local environment, exception, array, alias, copy/publication, and containment helpers plus duplicate primitive-only tests.
- [ ] 2.4 Remove direct `jni-sys` dependencies from all six consumers and verify each depends only on its domain crate and `android-jni-support` while retaining its independent library/export set.

## 3. Build integration and documentation

- [ ] 3.1 Add the support manifest/source to all six Android Gradle bridge task input sets and audit exact incremental invalidation coverage.
- [ ] 3.2 Update shared builder, packaging, symbol, architecture, dependency, raw-JNI-ownership, and generated-artifact audits without changing any task, package, library, or symbol identity.
- [ ] 3.3 Update core and Android documentation to describe six independent JNI bridges over shared primitive-array safety support and why macros/code generation remain out of scope.

## 4. Software verification

- [ ] 4.1 Pass formatting plus warnings-denied focused support/workspace tests and linting.
- [ ] 4.2 Pass all six focused Android host/JNI/adversarial suites and exact symbol/dependency/raw-JNI ownership audits.
- [ ] 4.3 Pass clean Android host/four-ABI Debug/Release packaging and unchanged macOS, iOS device, and iOS simulator Debug/Release bridge/package builds.
- [ ] 4.4 Pass strict OpenSpec validation and the complete `just verify` software gate with no generated or committed native artifacts.

## 5. Physical regression and final gate

- [ ] 5.1 Run identity, direct/CAN control, BMS, and background scenarios as applicable on the connected Android device against the ESP32 fixture.
- [ ] 5.2 Run the same applicable identity, direct/CAN control, BMS, and background scenarios on the connected iOS device against the ESP32 fixture.
- [ ] 5.3 Run identity, direct/CAN control, and BMS scenarios on the macOS host against the ESP32 fixture.
- [ ] 5.4 Confirm cleanup and every non-location assertion on all hosts; defer only bounded-time location sample absence.
- [ ] 5.5 Rerun structural audits and the complete `just verify` gate after physical evidence succeeds.
