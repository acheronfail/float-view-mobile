## ADDED Requirements

### Requirement: One canonical suite
The repository SHALL contain one top-level `/conformance` TypeScript black-box suite that imports the hand-written frontend contract and is not copied into Swift or Kotlin implementations.

#### Scenario: Common execution
- **WHEN** conformance runs against mock, Android, iOS, or macOS
- **THEN** the same scenario and assertion implementations are used

### Requirement: Complete contract coverage
The canonical suite SHALL cover every mandatory method, result shape, state event, stable error, authentication rule, ordering invariant, lifecycle behavior, persistence behavior, and resource bound visible to the frontend.

#### Scenario: Contract method added
- **WHEN** a new frontend-visible method is introduced
- **THEN** the change is incomplete until canonical success and relevant failure scenarios cover it

#### Scenario: Backend drift
- **WHEN** a backend returns a structurally or behaviorally incompatible result
- **THEN** canonical conformance fails with the first actionable mismatch

### Requirement: Minimal target adapters
Each target adapter SHALL only build or locate its application, launch it, expose bootstrap readiness, and stop it. Contract requests and assertions MUST remain in the shared suite.

#### Scenario: Platform launch
- **WHEN** conformance selects a target
- **THEN** platform-specific code returns a usable endpoint without duplicating scenario logic

### Requirement: Software and hardware execution
The suite SHALL support deterministic software execution and physical execution through the programmable VESC fixture. Hardware scenarios SHALL exercise production native Bluetooth, protocol, application-service, and storage paths.

#### Scenario: Fast software gate
- **WHEN** `just verify` runs without physical hardware
- **THEN** the frontend, mock, native code, and deterministic software conformance are verified

#### Scenario: Physical BLE gate
- **WHEN** a hardware conformance command runs with a configured ESP32 fixture
- **THEN** the same relevant contract scenarios drive the backend through its real OS Bluetooth stack

### Requirement: Conformance-owned VESC simulation
`/conformance` SHALL own the serial fixture client, VESC frame reassembly and classification, typed board identity/topology/Refloat configurations, telemetry phases, fragmentation, cadence assertions, disconnect injection, and bounded failure diagnostics. The ESP32 firmware MUST NOT duplicate those behaviors.

#### Scenario: Real BLE request and deterministic response
- **WHEN** the application sends a VESC request over real BLE and the firmware reports `WRITE <HEX>`
- **THEN** canonical conformance classifies the request, selects the configured deterministic response, and supplies exact `NOTIFY <HEX>` bytes to the firmware

#### Scenario: Hardware variant
- **WHEN** a conformance scenario selects a different typed board configuration
- **THEN** the same fixture firmware presents the selected identity and topology through conformance-supplied VESC responses

### Requirement: Read-only physical simulation
Canonical conformance SHALL permit only required VESC identity, discovery, and telemetry requests and SHALL reject unknown, motor-control, firmware, and persistent-configuration commands.

#### Scenario: Mutating VESC command
- **WHEN** a backend sends a command capable of motion or persistent mutation
- **THEN** conformance fails the scenario and supplies no mutating response

### Requirement: Conformance owns hardware dependencies
Any host serial dependency required to control the physical fixture SHALL belong to `/conformance` and MUST NOT be installed by `/frontend` or implemented inside the ESP-IDF project.

#### Scenario: Frontend setup
- **WHEN** a developer installs or runs the frontend without hardware conformance
- **THEN** no hardware serial dependency is required by the frontend application

### Requirement: Supported-backend gate
Android, iOS, and macOS SHALL NOT be considered supported unless their current build passes the canonical suite.

#### Scenario: Release verification
- **WHEN** a supported platform build is prepared
- **THEN** its required canonical conformance result is successful

### Requirement: Bounded diagnostics
Conformance SHALL bound execution time, captured messages, hardware transcripts, retries, and output. Passing runs SHALL not retain historical evidence artifacts; failures SHALL report the first useful violation and bounded diagnostic context.

#### Scenario: Scenario failure
- **WHEN** a backend or fixture violates an expectation
- **THEN** the runner stops within its deadline and reports the first violation with bounded context
