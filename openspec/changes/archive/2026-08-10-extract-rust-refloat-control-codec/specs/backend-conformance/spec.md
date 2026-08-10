## ADDED Requirements

### Requirement: Rust Refloat control codec candidate evidence

The Rust Refloat control codec candidate SHALL NOT be treated as a production backend owner until it passes locked formatting, warnings-denied linting, dependency and workspace-layout audits, source-derived fixture parity, exhaustive lighting-state and adversarial response tests, the complete `just verify` software gate, and focused physical Refloat control regression on Android, iOS, and macOS. Component evidence SHALL cover every committed package, flywheel, legacy/stable lighting, and fatal-clear request; both lighting profiles; zero-mask queries; all current, target, and enabled lighting combinations; exact payload capacities; all defined lighting response states; unavailable fatal clear; empty, short, long, wrong-prefix, wrong-package, wrong-command, wrong-profile, and undefined-flag failures; bounded storage; and payload-safe diagnostics.

Physical evidence in this candidate-only change SHALL use the production Bluetooth, routing, framing, native Refloat codec, mutation policy, confirmation, and application-state paths selected by each host and SHALL prove only that introducing the isolated Rust workspace component did not regress live package, flywheel, lighting, fatal-clear, or cleanup behavior. Later host-cutover changes SHALL rerun focused controls through the selected Rust component before native codec deletion. A missing location sample within a bounded automated dwell SHALL remain deferred manual coverage and SHALL not conceal permission, capability, monotonic-boundary, BLE, control, routing, state, or cleanup failures.

#### Scenario: Verify source-derived and exhaustive component parity

- **WHEN** focused Rust tests encode the committed controls, exhaust both lighting profiles and state combinations, and decode all defined lighting response states
- **THEN** every command, parameter, mask, flag, payload extent, and returned boolean agrees with selected native behavior

#### Scenario: Verify bounded adversarial behavior

- **WHEN** focused Rust tests exercise unavailable fatal clear and every response extent, prefix, package, command, profile, and flag failure class
- **THEN** the candidate fails closed with stable bounded errors, fixed resource use, no partial result, and no payload-bearing diagnostic

#### Scenario: Verify software isolation

- **WHEN** `just verify` runs with the candidate present
- **THEN** the Rust component and every existing frontend, conformance, Apple, Android, packaging, canonical, dependency, and ownership check pass while native bridge symbols and production call sites remain unchanged

#### Scenario: Verify all-host physical Refloat control regression

- **WHEN** focused Refloat live-control physical conformance runs against the ESP32 fixture on the connected Android device, connected iOS device, and macOS host
- **THEN** each selected native path completes real-BLE control encoding, routing, framing, response decoding, confirmation, state, and cleanup without executing or depending on the Rust candidate

#### Scenario: Defer production cutover evidence

- **WHEN** candidate parity and all-host regression checks pass
- **THEN** the change records component readiness only and requires later Apple and Android changes to prove bridge mapping, packaging, production selection, physical Rust control execution, and superseded-code deletion independently
