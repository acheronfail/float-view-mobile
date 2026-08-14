## 1. Safe Compatibility Candidate

- [ ] 1.1 Add `float-view-vesc-compatibility` to the locked Rust workspace with unsafe code forbidden and `float-view-vesc-firmware-identity` as its only production dependency.
- [ ] 1.2 Implement the exact four-version motor-controller catalogue and total typed access-mode, support, operation-permission, and shutdown decisions over `FirmwareIdentity`.
- [ ] 1.3 Keep unrelated identity fields and private values out of compatibility keys and redacted diagnostics.

## 2. Parity and Boundary Evidence

- [ ] 2.1 Compose the four immutable THOR300 firmware frame fixtures through the safe frame, identity, and compatibility components and assert exact stable decisions.
- [ ] 2.2 Exhaust every major/minor pair plus hardware-type, stable/nonzero test-marker, read/write, shutdown, adjacent-version, and unrelated-field cases against the selected source policy.
- [ ] 2.3 Cover catalogue ordering/uniqueness, total public behavior, and diagnostic privacy without adding third-party dependencies or generated fixtures.

## 3. Isolation and Repository Integration

- [ ] 3.1 Add focused root test and audit recipes and wire the candidate into workspace verification.
- [ ] 3.2 Audit the exact dependency graph, safe source surface, absence of FFI/JNI/callback/runtime code, unchanged ten-entry bridge catalogues, and absence from Apple/Android packages and production sources.
- [ ] 3.3 Document the candidate's direct Rust composition boundary, unchanged native owners, and the next controller-discovery migration step.

## 4. Verification

- [ ] 4.1 Run focused tests/audits, warnings-denied locked workspace linting, strict OpenSpec validation, and `git diff --check`.
- [ ] 4.2 Run `just verify` with the selected native compatibility implementations unchanged.
- [ ] 4.3 Run the canonical ESP32 physical matrix on the connected Android device, connected iOS device, and macOS host; require every non-location checkpoint and defer only bounded location acquisition per the agreed manual follow-up.
