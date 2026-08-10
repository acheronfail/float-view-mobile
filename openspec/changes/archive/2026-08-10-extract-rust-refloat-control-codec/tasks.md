## 1. Lock the candidate boundary

- [x] 1.1 Add the safe `float-view-refloat-control` crate to the locked core workspace with an empty production dependency graph and exact candidate ownership documentation.
- [x] 1.2 Extend workspace-layout, unsafe-code, dependency, bridge-symbol, packaged-artifact, and production-call-site audits without changing any native bridge or package.

## 2. Implement exact Refloat control encoding and decoding

- [x] 2.1 Implement typed bounded profiles, lighting controls/states, fixed-capacity payloads, and exact package, flywheel, legacy/stable lighting, query, and fatal-clear request encoding.
- [x] 2.2 Implement exact legacy/stable lighting response validation, two-bit state decoding, and closed payload-safe malformed and unsupported-command failures.

## 3. Prove fixture and adversarial parity

- [x] 3.1 Test every committed source-derived Refloat product-control request and lighting response against the immutable fixture.
- [x] 3.2 Exhaust both lighting profiles, all current/target/enabled states, zero-mask queries, all response states, exact capacities, unavailable fatal clear, malformed extents/prefixes/commands/flags, deterministic bounded output, and redacted diagnostics.
- [x] 3.3 Run focused Rust, native Refloat control, conformance, ownership, dependency, and packaging checks and confirm every native bridge symbol and production call site remains unchanged.

## 4. Verify repository and physical regression

- [x] 4.1 Run locked Rust formatting, warnings-denied linting, and candidate tests followed by the complete `just verify` software gate.
- [x] 4.2 Run focused Refloat live-control physical scenarios against the ESP32 fixture on the connected Android device and record native production-path regression evidence.
- [x] 4.3 Run the same focused Refloat live-control physical scenarios on the connected iOS device and record native production-path regression evidence.
- [x] 4.4 Run the same focused Refloat live-control physical scenarios on the macOS host and record native production-path regression evidence; defer only a missing bounded location sample.
