## 1. Lock the candidate boundary

- [ ] 1.1 Add the safe `float-view-vesc-controller-data` crate to the locked core workspace with an empty production dependency graph and exact candidate ownership documentation.
- [ ] 1.2 Extend workspace-layout, unsafe-code, dependency, bridge-symbol, packaged-artifact, and production-call-site audits without changing any native bridge or package.

## 2. Implement exact controller-data decoding

- [ ] 2.1 Implement bounded command-4 realtime decoding for the consumed fixed-point fields, signed ERPM, raw fault code, stable fault classification, and permitted response tail.
- [ ] 2.2 Implement bounded command-14 signature, VESC auto-float current-limit, finite/nonzero, and optional firmware-offset series-cell decoding.
- [ ] 2.3 Implement closed payload-safe unsupported-command and truncated/invalid-data failures with no partial output or decoded-value diagnostics.

## 3. Prove fixture and adversarial parity

- [ ] 3.1 Test both committed realtime fixtures, exact consumed-prefix behavior, permitted tails, signed/scaled boundaries, and every fault classification.
- [ ] 3.2 Test all five supported motor-configuration signatures and offsets, unknown/missing/zero/positive series-cell behavior, auto-float zero/sign/range cases, every consumed truncation, wrong commands, zero/non-finite limits, deterministic output, and redacted diagnostics.
- [ ] 3.3 Run focused Rust, native controller-data, conformance, ownership, dependency, and packaging checks and confirm every native bridge symbol and production call site remains unchanged.

## 4. Verify repository and physical regression

- [ ] 4.1 Run locked Rust formatting, warnings-denied linting, and candidate tests followed by the complete `just verify` software gate.
- [ ] 4.2 Run focused identity, direct/CAN control, and background/session physical scenarios against the ESP32 fixture on the connected Android device and record native production-path regression evidence; defer only a missing bounded location sample.
- [ ] 4.3 Run the same focused controller physical scenarios on the connected iOS device and record native production-path regression evidence; defer only a missing bounded location sample.
- [ ] 4.4 Run the same focused identity and direct/CAN control scenarios on the macOS host and record native production-path regression evidence.
