## 1. Lock the candidate boundary

- [ ] 1.1 Add the safe `float-view-vesc-firmware-identity` crate to the locked core workspace with an empty production dependency graph and exact ownership documentation.
- [ ] 1.2 Extend workspace-layout, unsafe-code, dependency, and bridge-symbol audits for the candidate while proving Apple and Android production integration remains unchanged.

## 2. Implement strict firmware identity decoding

- [ ] 2.1 Implement the bounded platform-neutral identity model and typed unsupported-command and structural errors without logging or formatting private identity data.
- [ ] 2.2 Implement exact common-prefix, stock VESC BMS 6.00, motor/custom-module, bounded replacement-name, and optional big-endian CRC decoding behavior.

## 3. Prove source-derived and adversarial parity

- [ ] 3.1 Test every committed THOR300, VESC Express, stock VBMS, and VBMS32 identity fixture through the Rust candidate using only workspace-local test support.
- [ ] 3.2 Test test-version markers, empty and maximum-length names, invalid UTF-8 replacement, UUID/count preservation, absent/present CRCs, every structural truncation class, terminator bounds, malformed BMS identities, invalid extended tails, wrong commands, and redacted failures.
- [ ] 3.3 Run focused Rust, native protocol, conformance, ownership, dependency, and packaging checks and confirm bridge symbols and production call sites remain unchanged.

## 4. Verify repository and physical regression

- [ ] 4.1 Run locked Rust formatting, warnings-denied linting, and candidate tests followed by the complete `just verify` software gate.
- [ ] 4.2 Run the focused physical identity scenario against the ESP32 fixture on the connected Android device and record production-path regression evidence.
- [ ] 4.3 Run the focused physical identity scenario against the ESP32 fixture on the connected iOS device and record production-path regression evidence.
- [ ] 4.4 Run the focused physical identity scenario against the ESP32 fixture on the macOS host and record production-path regression evidence; defer only a missing bounded location sample.
