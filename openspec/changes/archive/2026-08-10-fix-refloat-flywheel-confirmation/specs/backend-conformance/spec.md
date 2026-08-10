## ADDED Requirements

### Requirement: Cross-platform flywheel confirmation regression

Canonical verification SHALL exercise equivalent Apple and Android application-service lifecycles for flywheel enable, repeated flywheel telemetry during pending disable, normal-mode disable confirmation, prohibited competing mutation cancellation, and absent/stale failure callbacks. The complete software gate and focused direct/CAN physical Refloat control sequences on Android, iOS, and macOS SHALL pass without confirmation timeout, duplicate completion, mutation safety relaxation, or motor movement.

#### Scenario: Native service parity

- **WHEN** focused native tests inject the same command transmission and authoritative telemetry sequence
- **THEN** Apple and Android publish equivalent pending and confirmed states and return the same successful enable/disable results

#### Scenario: Complete software regression

- **WHEN** `just verify` runs
- **THEN** native, conformance, frontend, Rust, packaging, and ownership gates pass without a new warning or fallback path

#### Scenario: All-host physical control regression

- **WHEN** focused direct and CAN Refloat control scenarios run against the ESP32 fixture on Android, iOS, and macOS
- **THEN** each host completes enable and disable confirmation without fixture rejection, timeout, or motor movement

