## ADDED Requirements

### Requirement: Rust Refloat compatibility candidate evidence

The Rust Refloat compatibility candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, pinned-source and canonical fixture parity, adversarial identity/catalogue/layout tests, the complete `just verify` software gate, and focused physical controller regression on Android, iOS, and macOS. Component evidence SHALL cover exact INFO request bytes; minimal and source legacy response forms; compact major/minor and complete version-2 identities; legacy patch absence; every audited source/wire catalogue entry; telemetry, lighting, and fatal-clear profiles; unknown and prerelease-equivalent rejection; all required identifier names and groups; bounded extras, ordering, counts, lengths, aggregate bytes, UTF-8, uniqueness, missing/wrong-group names; wrong envelopes, unsupported protocol versions, truncation, excessive payloads; deterministic complete results; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native Refloat INFO/compatibility, telemetry, scheduling, session, control, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust component did not regress discovery, direct/CAN readiness, profile selection, telemetry, rides, controls, background continuity, unexpected-disconnect recovery, or cleanup. Later host-cutover changes SHALL rerun focused INFO/profile selection through Rust before native deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, protocol, telemetry, route, state, session, background, control, or cleanup failures.

#### Scenario: Verify source-derived compatibility parity

- **WHEN** focused Rust tests exercise every source/canonical INFO shape, audited release, profile, and required layout
- **THEN** request bytes, structural identities, catalogue/profile results, and layout admission agree exactly with pinned source and canonical compatibility requirements

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every envelope, protocol byte, extent, identity, catalogue miss, field count, name length, aggregate length, UTF-8, uniqueness, ordering, group, and missing-name boundary
- **THEN** the candidate either returns the exact bounded decision or fails closed with a stable payload-safe error and no partial result

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols, build catalogues, packaged artifacts, and production call sites remain unchanged

#### Scenario: Verify all-host physical compatibility regression

- **WHEN** focused identity, direct/CAN control, applicable BMS, legacy/dynamic telemetry, ride, unexpected-disconnect, and mobile background/session physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE discovery, INFO/profile admission, routing, telemetry, state, session/control behavior, background continuity where applicable, recovery, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical Rust INFO/profile execution, and superseded-code deletion independently

#### Scenario: Defer only bounded location acquisition

- **WHEN** a mobile physical run does not receive a location sample within its bounded automated dwell
- **THEN** only that sample observation is deferred and permission, capability, monotonic-boundary, BLE, protocol, telemetry, state, session, background, and cleanup failures remain blocking
