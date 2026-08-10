## ADDED Requirements

### Requirement: Rust firmware identity candidate evidence

The Rust firmware identity candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, source-derived parity, adversarial structural tests, the complete `just verify` software gate, and focused physical identity regression on Android, iOS, and macOS. Component evidence SHALL cover every audited THOR300 release, VESC Express, stock VBMS 6.00, VBMS32 6.06, nonzero test markers, bounded names, private UUIDs, configuration counts, absent and present CRC tails, wrong commands, truncation at every field class, terminator bounds, invalid stock-BMS tails, incomplete extended tails, invalid CRC-tail lengths, unexpected trailing bytes, and redacted failures.

Physical evidence in this candidate-only change SHALL use the production Bluetooth and native firmware identity paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress discovery, fragmented framing, direct/CAN identity, or cleanup. Later host-cutover changes SHALL rerun focused physical identity through the selected Rust decoder before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, session, or identity failures.

#### Scenario: Verify source-derived component parity

- **WHEN** focused Rust tests decode the committed motor, gateway, stock-BMS, and custom-module identity evidence
- **THEN** every accepted field and rejected structural class agrees with the current native behavior without adding a Rust-only compatibility rule

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while bridge symbol allowlists and production call sites remain unchanged

#### Scenario: Verify all-host physical regression

- **WHEN** focused physical identity conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native parser completes real-BLE identity, route, and cleanup behavior without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical execution, and superseded-code deletion independently
