# Spec Delta

## ADDED Requirements

### Requirement: Audited beta3 telemetry and control compatibility

In addition to existing audited releases, the shared backend and simulator SHALL admit exact Refloat 1.3.1-beta3 using beta2's selectable telemetry, product controls, IMU-timeout alerts, history and mutation safety rules. The full beta3 identity SHALL survive live display, saved session context and reconnect. Other unaudited releases SHALL remain unsupported.

#### Scenario: Beta3 direct or CAN discovery

- **WHEN** a verified direct or CAN controller reports exact 1.3.1-beta3
- **THEN** discovery accepts it, polls command 33 and command 35 without command-32 negotiation or a duplicate command-10 fault probe, and preserves its exact identity

#### Scenario: Beta3 timeout authority

- **WHEN** beta3 reports active, unknown, expired or cleared IMU-timeout evidence
- **THEN** the existing beta2 alert/history handling and submission-time mutation gates apply, including queued mutations and the flywheel-disable escape path

#### Scenario: Neighboring unaudited release

- **WHEN** INFO reports beta1, beta4, stable 1.3.1 or a malformed suffix
- **THEN** discovery remains unsupported without write traffic
