## 1. Lock the candidate boundary

- [ ] 1.1 Add the safe `float-view-refloat-compatibility` crate to the locked core workspace with an empty production dependency graph and exact candidate ownership documentation.
- [ ] 1.2 Extend workspace-layout, unsafe-code, dependency, bridge-symbol, packaged-artifact, build-catalogue, and production-call-site audits without changing any native bridge or package.

## 2. Implement bounded compatibility decisions

- [ ] 2.1 Implement the exact unframed INFO request, bounded envelope/response decoding, structural legacy-versus-patched wire identity, and closed payload-safe failures.
- [ ] 2.2 Implement the exact nine-entry audited source/wire catalogue and closed telemetry, lighting, and fatal-clear profile selection without ranges or free-form semantic-version parsing.
- [ ] 2.3 Implement bounded always-present/runtime identifier validation, uniqueness, required-group admission, and tolerance for ordered or unordered bounded future identifiers.

## 3. Prove source and adversarial parity

- [ ] 3.1 Test exact INFO request bytes, minimal/source legacy forms, compact major/minor forms, complete version-2 forms, legacy patch absence, envelope/extent/version boundaries, deterministic output, and redacted errors.
- [ ] 3.2 Test every audited source/wire entry and profile plus unknown, patch-mismatched, range-like, and prerelease-equivalent rejection.
- [ ] 3.3 Test exact required layouts, every missing or wrong-group identifier, bounded extras/order, count/name/aggregate boundaries, UTF-8, empty names, and duplicates.
- [ ] 3.4 Run focused Rust and native compatibility checks plus ownership, dependency, bridge-symbol, build-catalogue, packaging, and production-isolation audits and confirm selected host code remains unchanged.

## 4. Verify repository and physical regression

- [ ] 4.1 Run locked Rust formatting, warnings-denied linting, candidate tests, strict OpenSpec validation, and the complete `just verify` software gate.
- [ ] 4.2 Run focused identity, direct/CAN control, applicable BMS, legacy/dynamic telemetry, ride, unexpected-disconnect, and background/session scenarios against the ESP32 fixture on the connected Android device; defer only a missing bounded location sample.
- [ ] 4.3 Run the same applicable focused scenarios on the connected iOS device through its unchanged native compatibility path; defer only a missing bounded location sample.
- [ ] 4.4 Run focused identity, direct/CAN control, applicable BMS, legacy/dynamic telemetry, ride, and unexpected-disconnect scenarios on the macOS host through its unchanged native compatibility path.
