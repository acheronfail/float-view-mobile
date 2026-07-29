# Backend Conformance Specification

## Purpose

Define the single canonical software and physical acceptance suite required of every supported backend.

## Requirements

### Requirement: One canonical suite
The repository SHALL contain one top-level `/conformance` TypeScript black-box suite that imports the hand-written frontend contract and is not copied into Swift or Kotlin implementations.

#### Scenario: Common execution
- **WHEN** conformance runs against mock, Android, iOS, or macOS
- **THEN** the same scenario and assertion implementations are used

### Requirement: Complete contract coverage
The canonical suite SHALL cover every mandatory method, result shape, state event, stable error, authentication rule, ordering invariant, lifecycle behavior, persistence behavior, and resource bound visible to the frontend. A required scenario MUST create its prerequisite device, Board Profile, session, or session group and MUST fail rather than silently skip its assertions when that state is absent.

#### Scenario: Contract method added
- **WHEN** a new frontend-visible method is introduced
- **THEN** the change is incomplete until canonical success and relevant failure scenarios cover it

#### Scenario: Backend drift
- **WHEN** a backend returns a structurally or behaviorally incompatible result
- **THEN** canonical conformance fails with the first actionable mismatch

#### Scenario: Missing scenario resource
- **WHEN** a success scenario requires a device, Board Profile, session, or session group that setup did not create
- **THEN** canonical conformance fails with an actionable setup error instead of omitting the operation

#### Scenario: Destructive product operations
- **WHEN** canonical conformance creates a disposable session and linked session group
- **THEN** it exercises session deletion and group unlinking and verifies their authoritative state results

### Requirement: Minimal target adapters
Each target adapter SHALL only build or locate its application, prepare isolated test state, launch it, expose or refresh bootstrap readiness, apply uniform foreground/background controls where relevant, and stop it. Contract requests and assertions MUST remain in the shared suite.

#### Scenario: Platform launch
- **WHEN** conformance selects a target
- **THEN** platform-specific code returns a usable endpoint without duplicating scenario logic

#### Scenario: Mobile frontend recreation
- **WHEN** a shared scenario backgrounds and foregrounds a mobile target
- **THEN** its adapter performs only the OS lifecycle control and endpoint refresh while shared TypeScript performs every assertion

### Requirement: Software and hardware execution
The suite SHALL support deterministic software execution and physical execution through the programmable VESC fixture. Hardware scenarios SHALL exercise production native Bluetooth, protocol, application-service, and storage paths.

#### Scenario: Fast software gate
- **WHEN** `just verify` runs without physical hardware
- **THEN** the frontend, mock, native code, and deterministic software conformance are verified

#### Scenario: Physical BLE gate
- **WHEN** a hardware conformance command runs with a configured ESP32 fixture
- **THEN** the same relevant contract scenarios drive the backend through its real OS Bluetooth stack

#### Scenario: Production path
- **WHEN** physical conformance launches a Debug application with a test-only endpoint seam
- **THEN** Bluetooth, VESC protocol, application-service, session, and SQLite behavior still use the production native implementations

### Requirement: Deterministic state synchronization
The shared conformance client SHALL wait for bounded predicates over monotonic authoritative state revisions and SHALL be able to bootstrap a replacement WebSocket after mobile frontend recreation without restarting the native backend or fixture runner.

#### Scenario: Asynchronous connection transition
- **WHEN** a connection request is accepted before the backend reaches `ready`
- **THEN** conformance waits for a newer state satisfying the expected transition rather than using an unbounded sleep

#### Scenario: iOS foreground recovery
- **WHEN** iOS stops its loopback frontend server while backgrounded and starts it again in the foreground
- **THEN** conformance obtains current bootstrap credentials, opens a replacement WebSocket, and verifies the preserved native state

### Requirement: Conformance-owned VESC simulation
`/conformance` SHALL own the serial fixture client, VESC frame reassembly and classification, typed BLE-transport and CAN-controller identity/topology/Refloat configurations, telemetry-layout fields and phases, fragmentation, cadence assertions, disconnect injection, and bounded failure diagnostics. Exact response encoding SHALL be checked against immutable protocol fixtures. The ESP32 firmware MUST NOT duplicate those behaviors.

#### Scenario: Real BLE request and deterministic response
- **WHEN** the application sends a VESC request over real BLE and the firmware reports `WRITE <HEX>`
- **THEN** canonical conformance classifies the request, selects the configured deterministic response, and supplies exact `NOTIFY <HEX>` bytes to the firmware

#### Scenario: Hardware variant
- **WHEN** a conformance scenario selects a different typed board configuration
- **THEN** the same fixture firmware presents the selected identity and topology through conformance-supplied VESC responses

#### Scenario: Express controller discovery
- **WHEN** a backend queries an Express transport followed by its CAN-connected VESC controller
- **THEN** conformance supplies distinct transport and controller firmware, Refloat identity, telemetry layout, and telemetry values in the expected request order

### Requirement: Concurrent fixture orchestration
One bounded fixture runner SHALL own serial setup, the `RESET`/`READY` handshake, diagnostic LEDs, advertising, GATT-write processing, simulator responses, notification fragmentation, disconnect injection, reconnect advertising, cadence verification, and cleanup while shared contract requests execute concurrently.

#### Scenario: Connect requires fixture responses
- **WHEN** `device.connect` waits for native firmware and Refloat discovery
- **THEN** the fixture runner continues processing `WRITE` events and returning `NOTIFY` bytes until the backend reaches its expected state

#### Scenario: Reconnect injection
- **WHEN** a scenario commands `DISCONNECT` after telemetry has begun
- **THEN** the runner restarts advertising, services the reconnect identity sequence, and preserves bounded diagnostics across the transition

#### Scenario: Runner failure
- **WHEN** serial, BLE, framing, command safety, cadence, or scenario behavior fails
- **THEN** the runner records the first useful violation, sets the failure LED, and performs bounded target, fixture, serial, and LED cleanup

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

### Requirement: Canonical physical scenario matrix
Physical conformance SHALL run identity, ride, and unexpected-disconnect/reconnect scenarios on Android, iOS, and macOS. It SHALL additionally run background continuity on Android and iOS. macOS backgrounding SHALL NOT be required because mobile OS suspension is the behavior under test.

#### Scenario: Identity
- **WHEN** any physical target scans and connects to the advertised fixture
- **THEN** it reaches `ready`, exposes the configured controller and Refloat identity, sends only allowed startup requests, and completes an explicit disconnect

#### Scenario: Ride
- **WHEN** any physical target receives idle, riding, and post-ride telemetry phases with configured fragmentation
- **THEN** conformance verifies request cadence, automatic durable-session state, review, diagnostics, export, and deletion through the shared contract

#### Scenario: Unexpected disconnect and reconnect
- **WHEN** any physical target loses the fixture connection after telemetry begins and the fixture advertises again
- **THEN** the backend reconnects, resumes identity and telemetry, preserves the same active session, and later completes it through explicit disconnect

#### Scenario: Mobile background continuity
- **WHEN** Android or iOS is backgrounded while a physical ride session is active
- **THEN** native polling and session work continue where the OS permits, and a foregrounded replacement frontend recovers the same connection and session with additional samples

#### Scenario: macOS physical coverage
- **WHEN** macOS physical conformance runs
- **THEN** it runs identity, ride, and reconnect through CoreBluetooth without a synthetic mobile-background scenario

### Requirement: Bounded diagnostics
Conformance SHALL bound execution time, captured messages, hardware transcripts, retries, and output. Passing runs SHALL not retain historical evidence artifacts; failures SHALL report the first useful violation and bounded diagnostic context.

#### Scenario: Physical progress
- **WHEN** a physical conformance suite runs
- **THEN** it reports the active scenario, meaningful classified fixture and lifecycle milestones, bounded telemetry progress, scenario timing, and cleanup without dumping every raw packet

#### Scenario: Scenario failure
- **WHEN** a backend or fixture violates an expectation
- **THEN** the runner stops within its deadline and reports the first violation with bounded context, and a secondary cleanup failure does not replace or indefinitely delay it
