## ADDED Requirements

### Requirement: Rust Refloat legacy telemetry candidate evidence

The Rust Refloat legacy telemetry candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, pinned-source fixture parity, adversarial structural and numeric tests, the complete `just verify` software gate, and focused physical controller regression on Android, iOS, and macOS. Component evidence SHALL cover detail levels 1 through 4; the active-firmware-fault form; every present and absent optional tail; package state, mode, footpad, stop-condition, setpoint-adjustment, alert, and firmware-fault mappings; signed and scaled numeric boundaries; speed conversion; finite distance; sentinel field weakening; exact and inexact lengths; wrong envelopes and commands; deterministic fixed-size output; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native Refloat telemetry decoder, scheduling, telemetry, session, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, direct/CAN readiness, telemetry, sessions, controls, background continuity, or cleanup. Later host-cutover changes SHALL rerun focused command-10 reads through the selected Rust component before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, telemetry, routing, state, session, control, or cleanup failures.

#### Scenario: Verify source-derived component parity

- **WHEN** focused Rust tests decode every committed command-10 detail-level and fault fixture
- **THEN** every envelope, scalar, scale, enum, flag, nullable tail, fault, and sentinel behavior agrees with pinned Refloat source and the stable normalized telemetry requirements

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every input-length boundary, invalid form, wrong identifier, numeric extreme, non-finite distance, sentinel value, and unknown bounded code
- **THEN** the candidate either returns the exact bounded value or fails closed with a stable payload-safe error and no partial result

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical telemetry regression

- **WHEN** focused identity, direct/CAN control, and mobile background/session physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE discovery, routing, telemetry, state, session/control behavior, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical Rust command-10 execution, sentinel normalization, and superseded-code deletion independently
