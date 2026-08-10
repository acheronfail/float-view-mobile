## ADDED Requirements

### Requirement: Rust custom-configuration candidate evidence

The Rust VESC custom-configuration candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, every-release fixture parity, adversarial structural tests, the complete `just verify` software gate, and focused physical regression on Android, iOS, and macOS. Component evidence SHALL cover all Refloat 1.0.0–1.2.3 schema/current/default/write/readback fixture bytes, every command 92–95 request and response variant, index and signed-integer boundaries, chunk bounds and arithmetic, value minimum/bounds, exact acknowledgements, recognized and unrecognized wrong commands, redacted failures, and unchanged native bridge symbols and production call sites.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, Refloat, and native custom-configuration paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, direct/CAN readiness and control, or cleanup. Later host-cutover changes SHALL rerun focused configuration operations through the selected Rust codec before native codec deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, session, routing, control, or cleanup failures.

#### Scenario: Verify every-release component parity

- **WHEN** focused Rust tests encode and decode envelopes around every committed Refloat schema, current, default, accepted-write, and readback fixture
- **THEN** every request byte, response variant, extent, index, and opaque byte range agrees with selected native behavior without adding schema or value semantics

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every fixed-field truncation, integer boundary, chunk inconsistency, values boundary, acknowledgement tail, and wrong-command class
- **THEN** the candidate fails closed with stable bounded errors and no payload-bearing diagnostic

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbol allowlists and production calls remain unchanged

#### Scenario: Verify all-host physical regression

- **WHEN** focused identity and direct/CAN Refloat physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes BLE identity, direct/CAN readiness and safe control, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical configuration execution, and superseded-code deletion independently
