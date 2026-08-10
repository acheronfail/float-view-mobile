## 1. Lock the candidate boundary

- [ ] 1.1 Add the safe `float-view-refloat-legacy-telemetry` crate to the locked core workspace with an empty production dependency graph and exact candidate ownership documentation.
- [ ] 1.2 Extend workspace-layout, unsafe-code, dependency, bridge-symbol, packaged-artifact, and production-call-site audits without changing any native bridge, package, or bridge-build catalog entry.

## 2. Implement complete legacy compact decoding

- [ ] 2.1 Implement exact custom-app-data/Refloat/command-10 validation, detail levels 1 through 4, fixed-size normalized protocol values, optional tails, state and flag semantics, and exact speed conversion.
- [ ] 2.2 Implement the exact active-firmware-fault form, raw and stable fault classification, field-weakening sentinel absence, finite auto-float distance validation, and closed payload-safe errors.
- [ ] 2.3 Keep output allocation-free and fixed-size with redacted diagnostics and no host, bridge, application, scheduler, persistence, or transport dependency.

## 3. Prove fixture and adversarial parity

- [ ] 3.1 Test every committed command-10 detail-level and active-fault fixture, every present/absent optional field, exact numeric units, and source-faithful sentinel behavior.
- [ ] 3.2 Test every envelope and length boundary, detail/state/mode/footpad/stop/adjustment/alert/fault mapping, signed/scaled extreme, non-finite distance, unknown bounded code, deterministic output, size bound, and redacted diagnostic.
- [ ] 3.3 Run focused Rust, native Refloat telemetry, conformance, ownership, dependency, and packaging checks and confirm every native bridge symbol and production call site remains unchanged.

## 4. Verify repository and physical regression

- [ ] 4.1 Run locked Rust formatting, warnings-denied linting, and candidate tests followed by the complete `just verify` software gate.
- [ ] 4.2 Run focused identity, direct/CAN control, and background/session physical scenarios against the ESP32 fixture on the connected Android device and record native production-path regression evidence; defer only a missing bounded location sample.
- [ ] 4.3 Run the same focused controller physical scenarios on the connected iOS device and record native production-path regression evidence; defer only a missing bounded location sample.
- [ ] 4.4 Run focused identity and direct/CAN control scenarios on the macOS host and record native production-path regression evidence.
