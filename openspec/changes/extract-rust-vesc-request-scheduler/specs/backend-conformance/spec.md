## ADDED Requirements

### Requirement: Rust VESC request scheduler candidate evidence

The Rust request scheduler extraction SHALL be backed by source-derived parity, adversarial bounds, production-isolation audits, the complete software gate, and unchanged Android, iOS, and macOS physical BLE behavior with the ESP32 fixture before the candidate may be treated as ready for later composition.

#### Scenario: Replay source-derived scheduler behavior
- **WHEN** the focused Rust suite runs vectors derived from the complete Swift and Kotlin scheduler tests and implementations
- **THEN** every priority, FIFO, route/context match, generation, deadline, timeout, cancellation, one-way, deferred, diagnostic, and completion transition SHALL match the selected behavior

#### Scenario: Exercise adversarial scheduler bounds
- **WHEN** focused tests reach maximum queue and payload bounds, invalid facts, stale or repeated events, cancellation races, deferred pumping, and counter rollover
- **THEN** the candidate SHALL remain bounded, deterministic, panic-free for public inputs, and free of duplicate or misattributed completions

#### Scenario: Prove production isolation
- **WHEN** dependency, symbol, package, build-catalogue, source, and generated-artifact audits run
- **THEN** neither host SHALL select or package the candidate and no new Apple FFI, Android JNI, callback, or opaque runtime boundary SHALL exist

#### Scenario: Pass complete software verification
- **WHEN** the extraction is ready for implementation commit
- **THEN** `just verify` and strict OpenSpec validation SHALL pass with the native schedulers still selected

#### Scenario: Preserve all-host physical behavior
- **WHEN** the connected Android device, connected iOS device, and macOS host run the canonical ESP32 fixture matrix
- **THEN** every non-location checkpoint SHALL pass with unchanged BLE, connection, protocol, session, background, import/export, and theme behavior, while bounded location acquisition MAY be recorded for later manual verification
