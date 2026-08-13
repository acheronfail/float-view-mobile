## 1. Lock the candidate boundary

- [x] 1.1 Add the safe `float-view-refloat-compatibility` crate to the locked core workspace with an empty production dependency graph and exact candidate ownership documentation.
- [x] 1.2 Extend workspace-layout, unsafe-code, dependency, bridge-symbol, packaged-artifact, build-catalogue, and production-call-site audits without changing any native bridge or package.

## 2. Implement bounded compatibility decisions

- [x] 2.1 Implement the exact unframed INFO request, bounded envelope/response decoding, structural legacy-versus-patched wire identity, and closed payload-safe failures.
- [x] 2.2 Implement the exact nine-entry audited source/wire catalogue and closed telemetry, lighting, and fatal-clear profile selection without ranges or free-form semantic-version parsing.
- [x] 2.3 Implement bounded always-present/runtime identifier validation, uniqueness, required-group admission, and tolerance for ordered or unordered bounded future identifiers.

## 3. Prove source and adversarial parity

- [x] 3.1 Test exact INFO request bytes, minimal/source legacy forms, compact major/minor forms, complete version-2 forms, legacy patch absence, envelope/extent/version boundaries, deterministic output, and redacted errors.
- [x] 3.2 Test every audited source/wire entry and profile plus unknown, patch-mismatched, range-like, and prerelease-equivalent rejection.
- [x] 3.3 Test exact required layouts, every missing or wrong-group identifier, bounded extras/order, count/name/aggregate boundaries, UTF-8, empty names, and duplicates.
- [x] 3.4 Run focused Rust and native compatibility checks plus ownership, dependency, bridge-symbol, build-catalogue, packaging, and production-isolation audits and confirm selected host code remains unchanged.

## 4. Verify repository and physical regression

- [x] 4.1 Run locked Rust formatting, warnings-denied linting, candidate tests, strict OpenSpec validation, and the complete `just verify` software gate.
- [x] 4.2 Run focused identity, direct/CAN control, applicable BMS, legacy/dynamic telemetry, ride, unexpected-disconnect, and background/session scenarios against the ESP32 fixture on the connected Android device; defer only a missing bounded location sample.
- [x] 4.3 Run the same applicable focused scenarios on the connected iOS device through its unchanged native compatibility path; defer only a missing bounded location sample.
- [x] 4.4 Run focused identity, direct/CAN control, applicable BMS, legacy/dynamic telemetry, ride, and unexpected-disconnect scenarios on the macOS host through its unchanged native compatibility path.

### Verification evidence (2026-08-13)

- Software: focused candidate tests, warnings-denied linting, native compatibility checks, ownership and isolation audits, Android and Apple packaging, strict OpenSpec validation, and `just verify` passed.
- Android: all 17 fixture scenarios passed across clean isolated runs after the continuous runner encountered a reset/re-advertisement startup timeout between its first two scenarios. Background BLE, lifecycle, logging, and persistence assertions passed; location was authorized and available but produced no sample during the bounded dwell, so only that sample remains deferred to manual verification.
- iOS: all 17 fixture scenarios passed in one continuous run, including background BLE logging and two persisted location samples (`rows 0->2`, `sequence 0->2`).
- macOS: all 16 applicable fixture scenarios passed in one continuous run.
