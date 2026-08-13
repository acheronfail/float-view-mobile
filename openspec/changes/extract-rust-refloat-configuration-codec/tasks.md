## 1. Lock the candidate boundary

- [ ] 1.1 Add `float-view-refloat-configuration-codec` to the locked core workspace with the approved pure-Rust decompression, streaming XML, and SHA-256 dependencies and an unsafe-free host-independent public model.
- [ ] 1.2 Document exact candidate ownership and extend workspace-layout, dependency, unsafe-code, bridge-symbol, build-catalogue, packaging, native-package, and production-call-site audits without adding a host bridge or selected artifact.

## 2. Implement bounded schema semantics

- [ ] 2.1 Implement the exact Qt length envelope and bounded zlib decompression with complete stream, declared output, and trailing-input validation.
- [ ] 2.2 Implement streaming `ConfigParams` XML parsing with DTD/entity rejection and canonical depth, element, field, group, subgroup, choice, text, name, and description bounds.
- [ ] 2.3 Implement exact field/order/group validation, supported type and wire-encoding normalization, finite metadata/default parsing, description sanitization, raw expanded-XML SHA-256 fingerprinting, and CRC32C wire signature calculation.

## 3. Implement exact value semantics

- [ ] 3.1 Implement complete signature-prefixed decode for every supported integer, fixed-point, enum, bitfield, boolean, and bounded NUL-terminated UTF-8 string form with truncation and remainder rejection.
- [ ] 3.2 Implement deterministic complete-value encoding with exact field/type ownership, finite/bounds/scale checks, representable wire ranges, choice and string validation, and missing/extra value rejection.

## 4. Prove source and adversarial parity

- [ ] 4.1 Test all nine immutable Refloat schema fixtures against exact manifest fingerprints, signatures, field/group counts, representative normalized metadata, and deterministic results.
- [ ] 4.2 Test every release's default, current, accepted-write, and expected-readback value fixture for exact decode and byte-for-byte encode parity.
- [ ] 4.3 Test compressed envelope and zlib boundaries, prohibited/malformed XML, resource limits, duplicate/missing/unsupported/invalid metadata, grouping and text behavior, and payload-free deterministic failures.
- [ ] 4.4 Test every value wire type and extrema plus signature mismatch, truncation, remainder, invalid UTF-8/terminators, finite/bounds/scale faults, wrong types, and missing/extra fields.
- [ ] 4.5 Run focused Rust tests, warnings-denied linting, unchanged native codec tests, and ownership/dependency/isolation audits; confirm Swift and Kotlin production sources and artifacts remain selected and unchanged.

## 5. Verify repository and physical regression

- [ ] 5.1 Run locked Rust formatting, candidate tests, strict OpenSpec validation, and the complete `just verify` software gate.
- [ ] 5.2 Run focused identity, direct/CAN control, applicable BMS, configuration schema/current/default/write/readback, telemetry, ride, unexpected-disconnect, and background/session scenarios on the connected Android device against the ESP32 fixture; defer only a missing bounded location sample.
- [ ] 5.3 Run the same applicable focused scenarios on the connected iOS device through its unchanged selected Swift codec; defer only a missing bounded location sample.
- [ ] 5.4 Run focused identity, direct/CAN control, applicable BMS, configuration schema/current/default/write/readback, telemetry, ride, and unexpected-disconnect scenarios on the macOS host through its unchanged selected Swift codec.
- [ ] 5.5 Record physical evidence, rerun focused checks and `just verify` in the final implementation state, and require every non-location assertion and cleanup to pass before sync and archive.
