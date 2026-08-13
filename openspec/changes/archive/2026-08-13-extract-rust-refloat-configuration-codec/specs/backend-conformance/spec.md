## ADDED Requirements

### Requirement: Rust Refloat configuration codec candidate evidence

Candidate acceptance SHALL combine exhaustive source-derived schema/value parity, adversarial bounded-input evidence, repository ownership and production-isolation audits, the complete software gate, and unchanged selected-path physical regression on Android, iOS, and macOS. Physical runs during this unselected extraction SHALL prove regression freedom only and MUST NOT be reported as Rust production selection. Only absence of a location sample during the bounded acquisition window MAY remain deferred manual coverage; every other assertion and cleanup MUST pass.

#### Scenario: Exercise source-derived parity

- **WHEN** focused candidate tests consume all nine immutable Refloat schema families and their default, current, accepted-write, and expected-readback values
- **THEN** exact manifest fingerprints, signatures, counts, normalized records, typed values, and byte-for-byte encodings match the committed native and conformance evidence

#### Scenario: Exercise adversarial bounds

- **WHEN** focused tests cover compressed and expanded extents, zlib completion and trailing input, XML constructs and limits, metadata and ordering faults, every supported wire type, signature/truncation/remainder faults, numeric extrema, string validity, and missing/extra/wrong values
- **THEN** the candidate returns deterministic payload-free failures without panic, partial publication, unbounded work, or broadened schema acceptance

#### Scenario: Prove software isolation

- **WHEN** focused checks, native parity tests, dependency and unsafe-code audits, bridge/build/package/symbol ownership audits, strict OpenSpec validation, and `just verify` run
- **THEN** all gates pass while both production host codecs and every selected artifact remain unchanged

#### Scenario: Verify all-host physical regression

- **WHEN** the connected Android device, iOS device, and macOS host run their applicable identity, direct/CAN control, BMS, configuration schema/current/default/write/readback, telemetry, ride, unexpected-disconnect, and background/session scenarios against the ESP32 fixture
- **THEN** every non-location assertion and cleanup passes through the unchanged selected host paths and the evidence is recorded as regression-only

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile device has authorized background location capability but produces no sample within the harness acquisition window
- **THEN** only that sample assertion is recorded for later manual verification while BLE, logging, lifecycle, persistence, and all other scenario requirements remain mandatory
