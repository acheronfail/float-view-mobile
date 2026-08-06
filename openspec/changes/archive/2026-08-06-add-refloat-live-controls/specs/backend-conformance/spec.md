## MODIFIED Requirements

### Requirement: Read-only physical simulation

Canonical physical conformance SHALL permit required VESC identity, discovery, telemetry, only the exact route-correct Refloat product-control requests defined by `refloat-live-controls`, and exact eligible VESC-core shutdown `[156, 0, 0]`. It SHALL reject unknown, forced-shutdown, restart, broadcast-shutdown, motor-control, firmware, arbitrary persistent-configuration, tuning, recording, history, Handtest, remote, and RC-move commands. Product controls SHALL mutate only disposable simulated Refloat runtime/configuration state; shutdown SHALL simulate target disappearance without powering off the ESP32; neither path may issue motor movement on the ESP32 or host.

#### Scenario: Allowlisted Refloat control

- **WHEN** a writable physical scenario receives a version-correct package, flywheel, lighting, or clear-fatal request through the selected route
- **THEN** conformance applies the bounded simulated state transition and supplies the exact source-defined response or later confirmation

#### Scenario: Mutating VESC command

- **WHEN** a backend sends a command capable of motor motion, firmware mutation, arbitrary configuration, or another mutation outside the product-control allowlist
- **THEN** conformance fails the scenario, supplies no response, and does not mutate simulated state

#### Scenario: Allowlisted controller shutdown

- **WHEN** an eligible direct or CAN-routed physical scenario receives exact `[156, 0, 0]`
- **THEN** conformance records one accepted shutdown intent and simulates selected-controller disappearance without sending a motor command or powering off the ESP32

## ADDED Requirements

### Requirement: Canonical Refloat control coverage

Canonical conformance SHALL cover `refloat.control.set`, `refloat.fatalError.clear`, and every `RefloatControls` field against mock, Apple, and Android. Software scenarios SHALL exercise representative Refloat 1.0.0, 1.1.0, and 1.2.3 profiles across direct and CAN routes; ready and reconnecting lifecycle; saved and guest tracking; normal, flywheel, and read-only modes; idle and riding state; capability absence; idempotence; competing requests; timeouts; malformed responses; stale generations; frontend recreation; and exact state revisions. The flywheel matrix SHALL prove that every device mutation except exact flywheel disable emits no packet and that flywheel disable remains reachable while active flywheel run state is normalized as riding. Physical coverage SHALL include one direct and one CAN-routed writable control sequence per native backend without requiring motor movement.

#### Scenario: Versioned software matrix

- **WHEN** the Refloat control matrix runs across representative audited packages and routes
- **THEN** both native backends emit the exact package, flywheel, legacy-or-stable lighting, and version-applicable fatal command bytes and publish equivalent confirmed state

#### Scenario: Safety rejection matrix

- **WHEN** conformance requests controls while read-only, reconnecting, riding, in an incompatible package mode, without capability, during an active fault, or while another action is pending
- **THEN** Apple and Android return the same stable error, emit no prohibited command, and preserve the last confirmed controls, except that exact flywheel disable remains permitted from active flywheel mode

#### Scenario: Flywheel mutation matrix

- **WHEN** conformance requests package, lighting, fatal-clear, or another mutating operation while flywheel mode is active and its run state is normalized as riding
- **THEN** mock, Apple, Android, and the fixture reject every other mutation without transmission, while flywheel disable emits only the exact command 22 bytes

#### Scenario: Frontend recovery

- **WHEN** conformance recreates the frontend during a pending action or after an external simulated lighting change
- **THEN** replacement state recovers the native pending or newly confirmed values without duplicate command transmission

#### Scenario: Control command bounds

- **WHEN** control responses are truncated, oversized, routed from the wrong CAN ID, delayed past timeout, or delivered after a generation change
- **THEN** the matching action fails or the response is discarded with bounded diagnostics and no stale state publication

### Requirement: Canonical controller shutdown coverage

Canonical conformance SHALL cover `controller.shutdown` and complete controller-action state against mock, Apple, and Android. It SHALL exercise VESC 6.02 exclusion; stable writable 6.05, 6.06, and 7.00 acceptance; test-build rejection; idle and non-idle state; normal and flywheel mode; direct and CAN routes; exact payload bounds; a competing Refloat mutation; controller or transport loss; continued connectivity after an unsupported-hardware no-op; stale generations; and exact state revisions. Physical validation SHALL separately verify actual shutdown and manual power recovery on supported controller hardware.

#### Scenario: Versioned shutdown matrix

- **WHEN** the shutdown matrix runs across audited VESC versions and direct or CAN routes
- **THEN** only eligible 6.05, 6.06, and 7.00 scenarios emit exactly one `[156, 0, 0]` payload through the selected route

#### Scenario: Shutdown safety matrix

- **WHEN** conformance requests shutdown while non-idle, read-only, reconnecting, in flywheel mode, stale, or competing with another mutation
- **THEN** mock, Apple, Android, and the fixture reject it equivalently without transmission

#### Scenario: Shutdown lifecycle matrix

- **WHEN** an accepted shutdown is followed either by target loss or continued connectivity
- **THEN** target loss suppresses automatic reconnect while continued connectivity clears pending intent without claiming power-off
