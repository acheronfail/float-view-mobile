## ADDED Requirements

### Requirement: Rust Refloat dynamic telemetry candidate evidence

The Rust Refloat dynamic telemetry candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, pinned-source fixture parity, adversarial layout/structural/numeric tests, the complete `just verify` software gate, and focused physical controller regression on Android, iOS, and macOS. Component evidence SHALL cover the exact shared command-32 layout; 1.1.x and 1.2.x running responses; running-only absence and presence; charging and active-alert tails; source ticks; package state, mode, footpad, stop, adjustment, alert, fatal-latch, and firmware-fault mappings; all known fields; unknown bounded extensions; every field/name/aggregate/extension boundary; the complete finite source float16 domain; exact and inexact extents; wrong envelopes, commands, masks, and tail relations; deterministic complete output; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native Refloat telemetry decoder, scheduling, telemetry, session, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, direct/CAN readiness, dynamic telemetry, rides, controls, background continuity, unexpected disconnect recovery, or cleanup. Later host-cutover changes SHALL rerun focused command-31/32 reads through the selected Rust component before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, telemetry, routing, state, session, control, background, or cleanup failures.

#### Scenario: Verify source-derived component parity

- **WHEN** focused Rust tests decode every committed command-32 and command-31 fixture
- **THEN** every envelope, identifier list, mask, extent, source tick, scalar, semantic code, flag, optional group, alert, fault, and extension behavior agrees with pinned Refloat source and the stable normalized telemetry requirements

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every field count, identifier length, aggregate length, UTF-8, uniqueness, extension, mask, payload extent, float16, semantic code, flag, and tail-relation boundary
- **THEN** the candidate either returns the exact bounded layout/sample or fails closed with a stable payload-safe error and no partial result

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical telemetry regression

- **WHEN** focused identity, direct/CAN control, applicable BMS, dynamic telemetry, ride, unexpected-disconnect, and mobile background/session physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE discovery, routing, telemetry, state, session/control behavior, background continuity where applicable, recovery, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, variable record bounds, packaging, production selection, physical Rust command-31/32 execution, complete float16 behavior, and superseded-code deletion independently

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run does not receive a location sample within its bounded automated dwell
- **THEN** only that sample observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, telemetry, state, session, background, and cleanup failures remain blocking
