## ADDED Requirements

### Requirement: Rust VESC controller-data candidate evidence

The Rust VESC controller-data candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, upstream-derived fixture parity, adversarial structural and numeric tests, the complete `just verify` software gate, and focused physical controller regression on Android, iOS, and macOS. Component evidence SHALL cover normal and faulted realtime payloads; exact consumed prefixes and permitted tails; every selected signed fixed-point field; signed ERPM; all raw fault classifications; every supported motor-configuration signature and series-cell offset; unknown, absent, zero, and positive cell counts; VESC auto-float zero, sign, range, and invalid forms; every consumed-prefix truncation; wrong commands; zero and non-finite current limits; bounded storage; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native controller-data decoder, scheduling, telemetry, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, direct/CAN readiness, current-limit publication, telemetry, sessions, controls, background continuity, or cleanup. Later host-cutover changes SHALL rerun focused controller reads through the selected Rust component before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, controller-data, routing, state, session, control, or cleanup failures.

#### Scenario: Verify upstream-derived component parity

- **WHEN** focused Rust tests decode the committed realtime fixtures and source-derived current-limit vectors
- **THEN** every selected scalar, scale, signed value, fault, limit, signature, offset, and optional cell count agrees with selected native behavior

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every consumed truncation, wrong command, numeric boundary, invalid auto-float, zero limit, unknown signature, and cell-offset boundary
- **THEN** the candidate either returns the exact bounded value or fails closed with a stable payload-safe error and no partial result

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical controller regression

- **WHEN** focused identity, direct/CAN control, and mobile background physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE controller reads, routing, telemetry, state, session/control behavior, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical Rust controller-data execution, and superseded-code deletion independently
