## MODIFIED Requirements

### Requirement: Conformance-owned VESC simulation

`/conformance` SHALL own the serial fixture client, VESC frame reassembly and classification, typed BLE-transport and one-or-more CAN-controller identity/topology/Refloat configurations, telemetry-layout fields and phases, fragmentation, cadence assertions, disconnect injection, CAN discovery omissions and failures, and bounded failure diagnostics. Exact response encoding SHALL be checked against immutable protocol fixtures. The ESP32 firmware MUST NOT duplicate those behaviors. Software-host VESC simulation SHALL also be owned by conformance and reused through thin test adapters; native hosts MUST NOT retain their own protocol response models. Software selection SHALL require an explicit conformance build and authenticated run configuration, and MUST remain unavailable in ordinary and physical builds.

#### Scenario: Real BLE request and deterministic response

- **WHEN** the application sends a VESC request over real BLE and the firmware reports `WRITE <HEX>`
- **THEN** canonical conformance classifies the request, selects the configured deterministic response, and supplies exact `NOTIFY <HEX>` bytes to the firmware

#### Scenario: Hardware variant

- **WHEN** a conformance scenario selects a different typed board configuration
- **THEN** the same fixture firmware presents the selected identity and topology through conformance-supplied VESC responses

#### Scenario: Express controller discovery

- **WHEN** a backend queries an Express transport followed by its CAN-connected VESC controllers
- **THEN** conformance supplies distinct transport and controller firmware, Refloat identity, telemetry layout, and telemetry values in the expected routed request order

#### Scenario: Multiple controller candidates

- **WHEN** an Express configuration contains multiple VESC motor controllers and optional non-VESC CAN nodes
- **THEN** conformance returns the configured ping and per-ID firmware responses and can verify that only display-safe VESC candidates reach authoritative state

#### Scenario: Software host parity

- **WHEN** a software host runs canonical conformance
- **THEN** it uses the shared independent simulation models while the selected production service and protocol implementation handle application requests

#### Scenario: Explicit software build

- **WHEN** an emulator or simulator launches an ordinary build without conformance authorization
- **THEN** its device type alone cannot select a simulated peripheral
