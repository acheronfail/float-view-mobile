## MODIFIED Requirements

### Requirement: Composable source-faithful controller variants

The controller SHALL compose one stable THOR300 identity, one audited VESC firmware variant from 6.02, 6.05, 6.06, or 7.00, and one exact Refloat codec from 1.0.0–1.0.1, 1.1.0–1.1.2, or 1.2.0–1.2.3 at runtime. It MUST NOT implement the combinations as a registry of declared board profiles. Each selected controller/package pair SHALL reproduce its source version's identity, read protocol, and exact product-control behavior rather than changing display labels alone.

#### Scenario: Variant matrix is available

- **WHEN** the control surface loads a complete state snapshot
- **THEN** it offers the audited controller firmware and Refloat values and identifies THOR300 VESC 6.06 with Refloat 1.2.3 as the default

#### Scenario: Configuration saved while powered off

- **WHEN** a developer saves any supported firmware/Refloat pair together with topology and CAN ID while the fixture is powered off and disconnected
- **THEN** the coordinator atomically applies the complete configuration, resets configuration-scoped framing, layout, telemetry, control state, and counters, advances the state revision once, and uses only the new composed identity on the next connection

#### Scenario: Configuration save while active

- **WHEN** a developer attempts to save any changed configuration field while advertising or connected
- **THEN** the server rejects the whole save without changing identity, forwarding, protocol, or control state

#### Scenario: VESC firmware response shape

- **WHEN** the native backend requests controller firmware from a composed audited variant
- **THEN** the response reports that exact version and uses the source-version `COMM_FW_VERSION` layout and version-applicable hardware-CRC tail

#### Scenario: Stable controller model identity

- **WHEN** any selected firmware variant answers controller firmware
- **THEN** it reports the deliberate THOR300 hardware name and VESC hardware type while only source-derived layout, version, and optional-tail fields vary

#### Scenario: Allowlisted controls only

- **WHEN** a normal stable runtime combination receives a source-correct Refloat product control
- **THEN** it applies the bounded semantic state change while unknown, motor-control, and unrelated mutation requests receive no response

#### Scenario: Test-version combinations remain read-only

- **WHEN** an audited test-version runtime combination receives any Refloat product-control request
- **THEN** it supplies no mutating response and preserves simulated state

### Requirement: Read-only Refloat device simulation

While powered on, the coordinator SHALL answer the composed configuration's allowlisted VESC transport identity, optional CAN discovery, controller identity, Refloat identity, applicable telemetry reads, and exact Refloat product controls for normal stable firmware through real BLE. It MUST NOT answer motor-control, firmware-write, arbitrary persistent-configuration, tuning, recording, history, Handtest, remote, RC-move, unknown, or malformed requests. Audited test-version firmware SHALL remain read-only.

#### Scenario: Native backend connects

- **WHEN** a production native backend discovers and connects to the advertising ESP32
- **THEN** the coordinator services its identity, topology, Refloat layout, telemetry, and eligible product-control requests using conformance-owned framing and fixtures

#### Scenario: Eligible control request

- **WHEN** the writable native peer sends a version-correct package, flywheel, lighting, or clear-fatal request
- **THEN** interactive mode updates its complete simulated control state and supplies the exact immediate or telemetry confirmation without commanding ESP32 motor hardware

#### Scenario: Unsafe request

- **WHEN** the native peer sends an unknown or prohibited mutating VESC request
- **THEN** interactive mode supplies no response, records a bounded actionable diagnostic without terminating its serial loop, and does not weaken canonical conformance's fail-fast request allowlist

#### Scenario: Canonical conformance remains deterministic

- **WHEN** automated physical conformance uses its fixture scenarios
- **THEN** scripted telemetry and control phases, exact fixture responses, cadence assertions, safety rejections, and cleanup behavior remain deterministic

