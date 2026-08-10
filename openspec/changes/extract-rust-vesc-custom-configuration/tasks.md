## 1. Lock the candidate boundary

- [x] 1.1 Add the safe `float-view-vesc-custom-configuration` crate to the locked core workspace with an empty production dependency graph and exact ownership documentation.
- [x] 1.2 Extend workspace-layout, unsafe-code, dependency, bridge-symbol, and production-call-site audits for the candidate while correcting completed firmware-identity ownership documentation.

## 2. Implement strict custom-configuration envelopes

- [x] 2.1 Implement bounded command-92 schema, command-93 current, command-94 default, and command-95 write request encoding with typed failures and no payload diagnostics.
- [x] 2.2 Implement borrowed schema-chunk, current/default values, and exact write-acknowledgement response decoding with signed big-endian and structural/unsupported error parity.

## 3. Prove fixture and adversarial parity

- [x] 3.1 Test request and response envelopes around every committed Refloat 1.0.0–1.2.3 schema, current, default, accepted-write, and readback fixture without adding schema/value dependencies.
- [x] 3.2 Test all index and signed-integer boundaries, fixed-field truncations, chunk extents/arithmetic, value minimum/bounds, write bounds, acknowledgement tails, wrong commands, opaque-byte preservation, and redacted failures.
- [x] 3.3 Run focused Rust, native protocol, conformance, ownership, dependency, and packaging checks and confirm every native bridge symbol and production call site remains unchanged.

## 4. Verify repository and physical regression

- [x] 4.1 Run locked Rust formatting, warnings-denied linting, and candidate tests followed by the complete `just verify` software gate.
- [x] 4.2 Run focused identity and direct/CAN Refloat physical scenarios against the ESP32 fixture on the connected Android device and record production-path regression evidence.
- [x] 4.3 Run the same focused physical scenarios on the connected iOS device and record production-path regression evidence.
- [x] 4.4 Run the same focused physical scenarios on the macOS host and record production-path regression evidence; defer only a missing bounded location sample.
