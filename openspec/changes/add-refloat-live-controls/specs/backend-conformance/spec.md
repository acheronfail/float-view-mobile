## MODIFIED Requirements

### Requirement: Read-only physical simulation

Canonical physical conformance SHALL permit required VESC identity, discovery, telemetry, and only the exact route-correct Refloat product-control requests defined by `refloat-live-controls`. It SHALL reject unknown, motor-control, firmware, arbitrary persistent-configuration, tuning, recording, history, Handtest, remote, and RC-move commands. Product controls SHALL mutate only disposable simulated Refloat runtime/configuration state and MUST NOT issue motor movement on the ESP32 or host.

#### Scenario: Allowlisted Refloat control

- **WHEN** a writable physical scenario receives a version-correct package, flywheel, lighting, or clear-fatal request through the selected route
- **THEN** conformance applies the bounded simulated state transition and supplies the exact source-defined response or later confirmation

#### Scenario: Mutating VESC command

- **WHEN** a backend sends a command capable of motor motion, firmware mutation, arbitrary configuration, or another mutation outside the product-control allowlist
- **THEN** conformance fails the scenario, supplies no response, and does not mutate simulated state

## ADDED Requirements

### Requirement: Canonical Refloat control coverage

Canonical conformance SHALL cover `refloat.control.set`, `refloat.fatalError.clear`, and every `RefloatControls` field against mock, Apple, and Android. Software scenarios SHALL exercise representative Refloat 1.0.0, 1.1.0, and 1.2.3 profiles across direct and CAN routes; ready and reconnecting lifecycle; saved and guest tracking; normal and read-only firmware; idle and riding state; capability absence; idempotence; competing requests; timeouts; malformed responses; stale generations; frontend recreation; and exact state revisions. Physical coverage SHALL include one direct and one CAN-routed writable control sequence per native backend without requiring motor movement.

#### Scenario: Versioned software matrix

- **WHEN** the Refloat control matrix runs across representative audited packages and routes
- **THEN** both native backends emit the exact package, flywheel, legacy-or-stable lighting, and version-applicable fatal command bytes and publish equivalent confirmed state

#### Scenario: Safety rejection matrix

- **WHEN** conformance requests controls while read-only, reconnecting, riding, in an incompatible package mode, without capability, during an active fault, or while another action is pending
- **THEN** Apple and Android return the same stable error, emit no prohibited command, and preserve the last confirmed controls

#### Scenario: Frontend recovery

- **WHEN** conformance recreates the frontend during a pending action or after an external simulated lighting change
- **THEN** replacement state recovers the native pending or newly confirmed values without duplicate command transmission

#### Scenario: Control command bounds

- **WHEN** control responses are truncated, oversized, routed from the wrong CAN ID, delayed past timeout, or delivered after a generation change
- **THEN** the matching action fails or the response is discarded with bounded diagnostics and no stale state publication

