# Interactive fixture development

## Purpose

Define the development-only SvelteKit control surface, serial coordinator, source-faithful VESC/Refloat simulation, and software verification contract for manually exercising Float View against the programmable ESP32 fixture.

## Requirements

### Requirement: Development-only fixture entry point
The repository SHALL provide `just fixture dev [port]` as the public entry point for a SvelteKit development server at `http://127.0.0.1:11004` that controls the programmable ESP32 fixture. The application MUST run only through the Vite development loop and MUST NOT introduce a production adapter, build artifact, preview command, separate package, separate lockfile, deployment, or production runtime.

#### Scenario: Start without a selected port
- **WHEN** a developer runs `just fixture dev` without a serial path
- **THEN** the loopback control application starts detached and presents the bounded set of enumerated serial ports for explicit selection

#### Scenario: Start with a selected port
- **WHEN** a developer runs `just fixture dev <port>`
- **THEN** the control application starts and attempts a bounded attachment to exactly that port

#### Scenario: Development hot replacement
- **WHEN** fixture browser UI source changes or its page reloads
- **THEN** Svelte hot replacement updates the control surface without replacing the server-side serial coordinator or interrupting an active BLE session

#### Scenario: Server source changes
- **WHEN** coordinator or other server-side fixture source changes
- **THEN** the developer restarts `just fixture dev` rather than retaining a coordinator instance created from obsolete server code

#### Scenario: No production fixture application
- **WHEN** repository production artifacts are built
- **THEN** no fixture control application is built, copied, packaged, or deployed

### Requirement: Exclusive server-side serial coordination
One server-side fixture coordinator SHALL exclusively own at most one selected serial port, perform the bounded `RESET`/`READY` attachment handshake, consume fixture events continuously, and serialize UI actions with serial-event handling. Browser code MUST NOT access Web Serial or communicate with the ESP32 directly.

#### Scenario: Serial attachment
- **WHEN** a developer explicitly attaches an available serial port
- **THEN** the coordinator opens only that port, resets it, confirms `READY` within a bounded deadline, and publishes an attached powered-off state

#### Scenario: Duplicate attachment
- **WHEN** browser reload, hot replacement, or a second control page resolves the fixture coordinator
- **THEN** all callers observe the same coordinator and no second owner opens the selected serial port

#### Scenario: Attachment failure
- **WHEN** the selected port cannot open, does not complete the fixture handshake, or emits invalid bounded protocol data
- **THEN** the coordinator closes any resource it opened and exposes an actionable detached error state

#### Scenario: Active serial loss
- **WHEN** the owned serial port fails, disappears, or emits malformed or out-of-bounds fixture control data after attachment
- **THEN** the coordinator closes its ownership, publishes detached powered-off unavailable state, preserves the last saved simulated-device configuration, and requires explicit reattachment

#### Scenario: Developer interruption
- **WHEN** the fixture development process is interrupted
- **THEN** it best-effort resets the fixture, turns off its diagnostic LED, closes its owned serial port, and exits within a bounded deadline

### Requirement: High-level interactive fixture state
The controller SHALL expose one atomic configuration action plus idempotent power on, power off, start riding, and stop riding actions while maintaining saved simulated-device configuration, desired power, observed BLE connection, and ride mode as distinct authoritative state.

#### Scenario: Power on
- **WHEN** an attached powered-off fixture receives power on
- **THEN** it begins advertising the canonical VESC UART BLE service and remains logically powered on

#### Scenario: Powered-on peer disconnects
- **WHEN** a BLE peer disconnects while desired power remains on
- **THEN** the coordinator re-advertises without changing ride mode

#### Scenario: Power off while connected
- **WHEN** a connected fixture receives power off
- **THEN** the coordinator stops advertising, disconnects the peer through reset, confirms readiness, and publishes powered-off disconnected state

#### Scenario: Start and stop riding
- **WHEN** a developer starts riding while the fixture is attached and powered on and later stops riding
- **THEN** telemetry requests after each action use riding and idle responses respectively without requiring reconnection

#### Scenario: Ride while powered off
- **WHEN** a developer attempts to start riding while the fixture is detached or powered off
- **THEN** the action is rejected without changing ride state or issuing a serial command

#### Scenario: Duplicate ride action
- **WHEN** start riding is requested while already riding or stop riding is requested while already idle
- **THEN** the action is inert, and a repeated start does not reset the ride seed or sample index

#### Scenario: Action without serial
- **WHEN** a power or ride action requires an unavailable serial fixture
- **THEN** no serial command is issued and the complete state reports an actionable validation error

### Requirement: Composable source-faithful controller variants
The controller SHALL compose one stable THOR300 identity, one of the controller VESC firmware variants 6.02, 6.05, and 6.06, and one of the exact Refloat codecs 1.0.0, 1.1.0, and 1.2.3 at runtime. It MUST NOT implement the combinations as a registry of nine or eighteen declared board profiles. Each selected controller/package pair SHALL reproduce its source version's read-only wire behavior rather than changing display labels alone.

#### Scenario: Variant matrix is available
- **WHEN** the control surface loads a complete state snapshot
- **THEN** it offers exactly the three supported controller firmware values and exact Refloat values and identifies THOR300 VESC 6.06 with Refloat 1.2.3 as the default

#### Scenario: Configuration saved while powered off
- **WHEN** a developer saves any supported firmware/Refloat pair together with topology and CAN ID while the fixture is powered off and disconnected
- **THEN** the coordinator atomically applies the complete configuration, resets configuration-scoped framing, layout, telemetry, and counters, advances the state revision once, and uses only the new composed identity on the next connection

#### Scenario: Configuration save while active
- **WHEN** a developer attempts to save any changed configuration field while advertising or connected
- **THEN** the server rejects the whole save without changing identity, forwarding, or protocol state

#### Scenario: VESC firmware response shape
- **WHEN** the native backend requests controller firmware from a composed 6.02, 6.05, or 6.06 variant
- **THEN** the response reports that exact version and uses the source-version `COMM_FW_VERSION` layout, including the 6.06 hardware-CRC tail and no invented tail for 6.02 or 6.05

#### Scenario: Stable controller model identity
- **WHEN** any selected firmware variant answers controller firmware
- **THEN** it reports the deliberate THOR300 hardware name and VESC hardware type while only source-derived layout, version, and optional-tail fields vary

#### Scenario: All combinations remain read-only
- **WHEN** any of the nine runtime controller/package combinations receives an unknown or mutating request
- **THEN** it supplies no response regardless of whether that upstream version could implement the command

### Requirement: Selectable direct and VESC Express topology
The controller SHALL expose each composed controller/package combination either as a directly connected BLE VESC or behind a VESC Express 6.05 BLE transport with exactly one CAN-connected motor controller. Express mode SHALL accept one canonical decimal CAN ID in the inclusive range `0` through `253`, defaulting to `1`.

#### Scenario: Direct controller discovery
- **WHEN** direct topology is selected and the native backend requests firmware
- **THEN** the BLE peer reports the selected controller firmware with VESC hardware type and accepts applicable Refloat requests without CAN forwarding

#### Scenario: Express controller discovery
- **WHEN** Express topology is selected with CAN ID `42`
- **THEN** the BLE peer first reports VESC Express 6.05 transport identity, CAN discovery returns exactly `42`, and the selected controller identity and applicable Refloat responses are available through CAN-forward command `34` targeting `42`

#### Scenario: Forward to another CAN ID
- **WHEN** Express topology is selected and a forwarded request targets an ID other than the configured controller
- **THEN** the simulator returns no controller response and records only a bounded high-level diagnostic

#### Scenario: Invalid CAN ID
- **WHEN** a developer submits `254`, `255`, a negative or fractional number, padded text, a non-decimal representation, or nonnumeric input
- **THEN** the server rejects the value without changing the retained CAN ID or any simulator state

#### Scenario: Direct topology retains CAN input
- **WHEN** direct topology is selected
- **THEN** the CAN-ID input is disabled, no CAN device is advertised, and the last valid Express CAN ID remains visible and available for a later Express selection

#### Scenario: Topology configuration saved while powered off
- **WHEN** a developer includes changed topology or a valid CAN ID in the complete configuration saved while powered off and disconnected
- **THEN** the coordinator applies it with the firmware and Refloat selections and atomically resets topology-scoped framing, discovery, layout, telemetry, and counters before the next advertisement

#### Scenario: Topology configuration saved while active
- **WHEN** a developer attempts to save changed topology or CAN ID while advertising or connected
- **THEN** the server rejects the complete save without changing the identity or forwarding path visible to the BLE peer

#### Scenario: Complete topology matrix
- **WHEN** software verification exercises every runtime-composed controller/package pair through direct and Express topology
- **THEN** table-driven Cartesian tests cover all eighteen effective configurations without requiring eighteen simulator implementations or golden fixtures

### Requirement: Version-specific Refloat protocol
Each Refloat codec SHALL implement the source-derived INFO and read-only telemetry protocol of exact package identity 1.0.0, 1.1.0, or 1.2.3. The simulator MUST NOT claim that selecting an older profile expands Float View's current package compatibility.

#### Scenario: Refloat 1.0.0 INFO
- **WHEN** a client requests Refloat INFO from the 1.0.0 codec
- **THEN** the codec returns the legacy version-1 short INFO response encoding major/minor as `10` for Refloat 1.0.0 even when the request asks for INFO version 2

#### Scenario: Refloat 1.0.0 telemetry
- **WHEN** a client sends an allowlisted legacy realtime or compact all-data request to the 1.0.0 codec
- **THEN** the response uses the corresponding fixed legacy layout and the codec does not advertise dynamic commands 31 or 32

#### Scenario: Refloat 1.1.0 protocol
- **WHEN** a client requests INFO, realtime IDs, or realtime data from the 1.1.0 codec
- **THEN** the codec returns INFO version 2 with package version 1.1.0 and uses the 1.1 dynamic ID and float16 data layouts without the 1.2 alert tail

#### Scenario: Refloat 1.2.3 protocol
- **WHEN** a client requests INFO, realtime IDs, or realtime data from the 1.2.3 codec
- **THEN** the codec returns INFO version 2 with package version 1.2.3 and uses the 1.2 dynamic layout, mask, and bounded alert/fault tail

#### Scenario: Current product compatibility remains authoritative
- **WHEN** the current native backend connects to a Refloat 1.0.0 or 1.1.0 selection
- **THEN** the fixture returns only source-faithful responses to requests actually received, does not initiate telemetry, and preserves that backend's actual unsupported or connected-without-telemetry behavior rather than forcing a synthetic failure

#### Scenario: Current direct telemetry limitation
- **WHEN** the current native backend connects directly to the THOR300 identity
- **THEN** the fixture answers direct firmware and Refloat INFO requests but does not claim frontend riding telemetry because the backend currently requires a CAN identity for telemetry layout and polling

### Requirement: Read-only Refloat device simulation
While powered on, the coordinator SHALL answer the composed configuration's allowlisted VESC transport identity, optional CAN discovery, controller identity, Refloat identity, and applicable read-only telemetry requests through real BLE. It MUST NOT answer motor-control, firmware-write, persistent-configuration, unknown, or malformed requests.

#### Scenario: Native backend connects
- **WHEN** a production native backend discovers and connects to the advertising ESP32
- **THEN** the coordinator services its identity, topology, Refloat layout, and telemetry requests using the existing conformance-owned framing and fixtures

#### Scenario: Unsafe request
- **WHEN** the native peer sends an unknown or potentially mutating VESC request
- **THEN** interactive mode supplies no response, records a bounded actionable diagnostic without terminating its serial loop, and does not weaken canonical conformance's fail-fast request allowlist

#### Scenario: Canonical conformance remains deterministic
- **WHEN** automated physical conformance uses its existing fixture scenarios
- **THEN** their scripted telemetry phases, exact fixture responses, cadence assertions, and cleanup behavior remain deterministic

### Requirement: Plausible interactive telemetry
The interactive simulator SHALL encode the selected Refloat codec's legacy or dynamic telemetry layout from one normalized sample model and generate bounded, seeded, smoothly varying values whose related fields remain coherent. Ride mode SHALL control the next telemetry response rather than running an unsolicited telemetry stream.

#### Scenario: Riding telemetry
- **WHEN** ride mode is riding and the native backend requests telemetry repeatedly
- **THEN** responses report a riding board state, both footpads active, nonzero speed and ERPM, correlated bounded load values, plausible pack voltage, and slowly changing temperatures

#### Scenario: Idle telemetry
- **WHEN** ride mode is idle and the native backend requests telemetry
- **THEN** the response reports idle state, inactive footpads, zero speed, ERPM, duty-related motion, and current while retaining plausible bounded voltage and temperature

#### Scenario: Smooth reproducible samples
- **WHEN** the same seeded ride receives the same sequence of telemetry requests
- **THEN** it produces the same values and adjacent samples remain within defined change bounds

#### Scenario: Equivalent values across package encodings
- **WHEN** equivalent seeded ride samples are encoded through Refloat 1.0.0, 1.1.0, and 1.2.3 response formats
- **THEN** decoding each format yields equivalent core state, footpad, voltage, duty, current, ERPM, speed, and temperature values within each format's precision

#### Scenario: Stop takes effect
- **WHEN** stop riding completes before the next telemetry request
- **THEN** that next response is idle rather than consuming a remaining scripted riding phase

### Requirement: Loopback authenticated control surface
The fixture development server SHALL bind only to `127.0.0.1:11004` and SHALL require exact Origin `http://127.0.0.1:11004` and a random per-process launch token supplied to the server-rendered control page for every state-changing request. Requests, bodies, responses, polling, diagnostics, queues, and retained state MUST be bounded.

#### Scenario: Valid local action
- **WHEN** the control page submits a bounded action with the expected loopback Origin and launch token
- **THEN** the server applies the action once and returns a complete state snapshot with a monotonically increasing revision

#### Scenario: Unauthorized action
- **WHEN** a mutation omits or changes the Origin or launch token
- **THEN** the server rejects it without changing coordinator or serial state

#### Scenario: Development port occupied
- **WHEN** the fixed fixture development port is unavailable
- **THEN** startup fails with an actionable diagnostic instead of binding another port or a non-loopback interface

#### Scenario: Bounded diagnostics
- **WHEN** serial, BLE, request, or action events continue for an extended development session
- **THEN** the control surface retains only bounded high-level diagnostics and counters without logging full telemetry, launch tokens, or Bluetooth identifiers

### Requirement: Observable and accessible manual controls
The Svelte control surface SHALL present serial attachment, one saved-configuration form containing direct or Express topology, retained Express CAN ID, controller firmware, and exact Refloat package, plus desired power, observed BLE connection, ride mode, current synthetic telemetry, bounded diagnostics, and request counters from complete authoritative snapshots. Interactive controls MUST retain native button semantics and support primary pointer and keyboard activation without double activation.

#### Scenario: State refresh
- **WHEN** the coordinator revision advances because of an action or serial event
- **THEN** bounded polling replaces the displayed state with the complete newer snapshot

#### Scenario: Unavailable action
- **WHEN** current state makes a control unavailable
- **THEN** the control is visibly disabled and does not issue a mutation

#### Scenario: Active configuration controls
- **WHEN** the fixture is advertising or connected
- **THEN** topology, CAN-ID, firmware, Refloat, and Save controls are visibly disabled while displaying the identity and forwarding path currently presented over BLE

#### Scenario: Atomic configuration form
- **WHEN** the developer edits one or more configuration fields and activates Save while powered off
- **THEN** the client submits all topology, CAN-ID, firmware, and Refloat fields in one mutation and replaces its form from the returned authoritative snapshot

#### Scenario: Ride control availability
- **WHEN** the fixture is detached, powered off, already riding, or already idle
- **THEN** start and stop controls are enabled only for their valid powered-on transition and cannot issue duplicate ride mutations

#### Scenario: Keyboard activation
- **WHEN** a focused enabled high-level control is activated from the keyboard
- **THEN** its action occurs exactly once

### Requirement: Software-only verification
The complete software gate SHALL type-check and test the fixture development server, ownership/import boundaries, coordinator, composable variant catalog, direct and CAN-forwarded request paths, version-specific protocol encoders, telemetry encoder, and Svelte controls without requiring an ESP32, native device, production fixture build, or deployed server.

#### Scenario: Repository verification
- **WHEN** `just verify` runs without physical hardware
- **THEN** fake serial controls exercise attachment and active loss, the table-generated Cartesian matrix, configurable CAN forwarding, atomic configuration, powered-on ride transitions, reconnect, version-specific interactive and canonical request policies, direct/older-package compatibility limits, telemetry relationships, UI-only HMR coordinator reuse, and cleanup while no fixture web artifact is built

#### Scenario: Physical use remains explicit
- **WHEN** a developer wants to use interactive real BLE simulation
- **THEN** they explicitly run the development command with a flashed ESP32 near a Bluetooth-capable selected native target
