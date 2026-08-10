## 1. Lock the candidate boundary

- [x] 1.1 Add the safe `float-view-vesc-bms-values` crate to the locked core workspace with an empty production dependency graph and exact candidate ownership documentation.
- [x] 1.2 Extend workspace-layout, unsafe-code, dependency, bridge-symbol, packaged-artifact, and production-call-site audits without changing any native bridge or package.

## 2. Implement strict command-96 decoding

- [x] 2.1 Implement the bounded platform-neutral values model and exact signed fixed-point, VESC auto-float, fixed-field, cell, balancing, temperature, reported-CAN, and pressure decoding.
- [x] 2.2 Implement exact legacy/extended tail validation, bounded UTF-8 status borrowing, complete remainder handling, finite-number checks, and closed payload-safe failure classes.

## 3. Prove fixture and adversarial parity

- [x] 3.1 Test both committed source-derived BMS values payloads and every decoded field, collection item, flag, scale, route value, and optional tail.
- [x] 3.2 Test zero/50 collection bounds, maximum status, every prefix truncation, excessive counts, invalid flags, route mismatch, wrong commands, malformed auto-floats, UTF-8/status/tail errors, unexpected remainders, bounded storage, and redacted diagnostics.
- [x] 3.3 Run focused Rust, native BMS protocol, conformance, ownership, dependency, and packaging checks and confirm every native bridge symbol and production call site remains unchanged.

## 4. Verify repository and physical regression

- [x] 4.1 Run locked Rust formatting, warnings-denied linting, and candidate tests followed by the complete `just verify` software gate.
- [x] 4.2 Run focused standalone, direct-controller, and Express BMS physical scenarios against the ESP32 fixture on the connected Android device and record native production-path regression evidence.
- [x] 4.3 Run the same focused BMS physical scenarios on the connected iOS device and record native production-path regression evidence.
- [x] 4.4 Run the same focused BMS physical scenarios on the macOS host and record native production-path regression evidence; defer only a missing bounded location sample.
