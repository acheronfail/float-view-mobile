## 1. Lock the candidate boundary

- [ ] 1.1 Add the safe `float-view-refloat-dynamic-telemetry` crate to the locked core workspace with an empty production dependency graph and exact candidate ownership documentation.
- [ ] 1.2 Extend workspace-layout, unsafe-code, dependency, bridge-symbol, packaged-artifact, bridge-catalogue, and production-call-site audits without changing any native bridge or package.

## 2. Implement bounded dynamic telemetry decoding

- [ ] 2.1 Implement complete command-32 envelope, UTF-8 identifier, ordering, combined-count, per-name, aggregate-name, uniqueness, and full-consumption validation into an owned bounded layout.
- [ ] 2.2 Implement command-31 envelope, closed-mask, exact layout-derived extent, complete finite source float16, source-tick, semantic header, running-only, charging, and 1.2 alert-tail decoding.
- [ ] 2.3 Implement exact typed known-field projection, bounded ordered unknown extensions, fatal-latch and raw/classified firmware-fault behavior, closed errors, and redacted diagnostics with no partial output.

## 3. Prove source and adversarial parity

- [ ] 3.1 Test the committed shared command-32 layout and every 1.1/1.2 running, charging, and active-firmware-fault command-31 fixture with exact identifiers, values, semantics, ticks, and optional groups.
- [ ] 3.2 Test zero, subnormal, normal, exponent-31, saturation, sign, and every structural envelope/mask/extent/tail boundary required by the source format.
- [ ] 3.3 Test exact and excessive field, UTF-8 byte, aggregate-name, duplicate, extension, semantic-code, alert/fault-relation, deterministic output, and payload-safe diagnostic boundaries.
- [ ] 3.4 Run focused Rust, native Refloat telemetry, conformance, ownership, dependency, and packaging checks and confirm every native bridge symbol and production call site remains unchanged.

## 4. Verify repository and physical regression

- [ ] 4.1 Run locked Rust formatting, warnings-denied linting, candidate tests, strict OpenSpec validation, and the complete `just verify` software gate.
- [ ] 4.2 Run focused identity, direct/CAN control, applicable BMS, dynamic telemetry, ride, unexpected-disconnect, and background/session physical scenarios against the ESP32 fixture on the connected Android device; defer only a missing bounded location sample.
- [ ] 4.3 Run the same applicable focused scenarios on the connected iOS device through its unchanged native dynamic decoder; defer only a missing bounded location sample.
- [ ] 4.4 Run focused identity, direct/CAN control, applicable BMS, dynamic telemetry, ride, and unexpected-disconnect scenarios on the macOS host through its unchanged native dynamic decoder.
