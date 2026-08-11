## 1. Canonical safe record

- [ ] 1.1 Add one safe bounded firmware-identity record value, exact existing version/layout/bound constants, and closed decode-to-record outcome to `vesc-firmware-identity` without adding unsafe code or a production dependency.
- [ ] 1.2 Move exact normal, optional-name/CRC, replacement-UTF-8, maximum-bound, unsupported-command, malformed, truncated, and impossible-identity record evidence into exhaustive safe-component tests.
- [ ] 1.3 Prove the canonical record operation emits every existing Apple/Android byte vector and exposes no host status, pointer, JNI/C type, callback, generated binding, or private diagnostic.

## 2. Thin independent host bridges

- [ ] 2.1 Migrate `vesc-firmware-identity-apple-ffi` to the shared record outcome while preserving its explicit two exports, pointer-region validation order, status mapping, caller-owned publication, header, archive, and C/Swift behavior.
- [ ] 2.2 Migrate `vesc-firmware-identity-android-jni` to the shared record outcome while preserving its explicit two exports, environment/reference/array validation order, status mapping, metadata publication, shared library, and Kotlin behavior.
- [ ] 2.3 Delete both bridge-local record encoders, parallel flag/layout constants and outcome types, duplicate semantic fixtures/helpers, and any superseded imports while retaining every distinct platform-boundary failure test.

## 3. Ownership audits and documentation

- [ ] 3.1 Add focused cross-bridge assertions proving identical accepted/rejected shared outcomes, exact unchanged bytes/statuses/metadata, and complete Apple C plus raw-JNI adversarial boundary coverage.
- [ ] 3.2 Extend repository audits to require one record owner, forbid bridge-local identity record construction and generated/export-macro tooling, preserve the safe crate's empty production dependency graph/unsafe prohibition, and verify both bridge build inputs rebuild from shared source.
- [ ] 3.3 Update core and host ownership documentation to distinguish the canonical safe record from independent Apple/Android boundary mechanics and record a material combined bridge production-line reduction from the 418-line baseline.

## 4. Software verification

- [ ] 4.1 Pass locked formatting, warnings-denied linting, all safe-component and bridge Rust tests, and exact dependency/source-layout audits.
- [ ] 4.2 Pass compiled Apple C ABI, Swift adapter, Android host-JNI, and JVM production-path suites with unchanged symbols, ABI versions, records, models, and errors.
- [ ] 4.3 Pass clean macOS/iOS device/iOS simulator packaging and Android host plus four-ABI Debug/Release packaging without generated or committed native artifacts.
- [ ] 4.4 Pass strict OpenSpec validation, `git diff --check`, and the complete `just verify` software gate.

## 5. Physical regression and final gate

- [ ] 5.1 Run applicable identity, direct/CAN control, BMS, and background scenarios on the connected Android device against the ESP32 fixture.
- [ ] 5.2 Run the same applicable identity, direct/CAN control, BMS, and background scenarios on the connected iOS device against the ESP32 fixture.
- [ ] 5.3 Run applicable identity, direct/CAN control, and BMS scenarios on the macOS host against the ESP32 fixture.
- [ ] 5.4 Confirm cleanup and every non-location assertion on all hosts, deferring only bounded-time location sample absence, then rerun ownership audits and the complete `just verify` gate.
