## ADDED Requirements

### Requirement: Rust VESC BMS values candidate evidence

The Rust VESC BMS values candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, source-derived fixture parity, adversarial structural tests, the complete `just verify` software gate, and focused physical BMS regression on Android, iOS, and macOS. Component evidence SHALL cover committed legacy and extended command-96 payloads; every decoded field and unit; signed fixed-point and auto-float behavior; zero and 50 cell/temperature counts; balancing flags; reported-CAN validation; absent and 40-byte status tails; every prefix truncation; excessive counts; wrong commands; malformed numeric, UTF-8, tail, and remainder classes; bounded storage; and payload-safe failures.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, native BMS decoder, polling, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress standalone direct BMS, direct-controller/CAN-BMS, Express/controller/CAN-BMS, or cleanup behavior. Later host-cutover changes SHALL rerun focused command-96 decoding through the selected Rust component before native parser deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, BMS, routing, polling, state, or cleanup failures.

#### Scenario: Verify source-derived component parity

- **WHEN** focused Rust tests decode the committed legacy and extended BMS values payloads plus exact minimum and maximum bounded forms
- **THEN** every fixed value, collection item, flag, route field, optional tail, scale, and unit agrees with selected native behavior

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise every truncation, count, flag, route, numeric, status, tail, remainder, and wrong-command failure class
- **THEN** the candidate fails closed with stable bounded errors, fixed resource use, no partial result, and no payload-bearing diagnostic

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical BMS regression

- **WHEN** standalone, direct-controller, and Express BMS physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE identity, direct/CAN values decoding, polling, authoritative state, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical command-96 execution, and superseded-code deletion independently
