## MODIFIED Requirements

### Requirement: Conformance-owned VESC simulation
`/conformance` SHALL own the serial fixture client, VESC frame reassembly and classification, typed BLE-transport and one-or-more CAN-controller identity/topology/Refloat configurations, telemetry-layout fields and phases, fragmentation, cadence assertions, disconnect injection, CAN discovery omissions and failures, and bounded failure diagnostics. Exact response encoding SHALL be checked against immutable protocol fixtures. The ESP32 firmware MUST NOT duplicate those behaviors.

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

### Requirement: Canonical physical scenario matrix
Physical conformance SHALL run identity, ride, and unexpected-disconnect/reconnect scenarios on Android, iOS, and macOS. Android and macOS SHALL each run the complete representative topology set: a direct dynamic controller, direct legacy controller, automatically discovered Express controller, explicitly configured Express controller whose ping does not identify it, and Express connection requiring user selection. iOS SHALL run direct dynamic and Express discovery plus mobile background continuity without repeating every shared-Apple topology case. Android SHALL also run mobile background continuity. macOS backgrounding SHALL NOT be required because mobile OS suspension is the behavior under test.

#### Scenario: Identity
- **WHEN** any physical target scans and connects to the advertised fixture
- **THEN** it reaches `ready`, exposes the configured direct or CAN route and supported controller and Refloat identity, sends only allowed startup requests, and completes an explicit disconnect

#### Scenario: Ride
- **WHEN** any physical target receives idle, riding, and post-ride telemetry phases with configured fragmentation
- **THEN** conformance verifies route-correct request cadence, automatic durable-session state, review, diagnostics, export, and deletion through the shared contract

#### Scenario: Unexpected disconnect and reconnect
- **WHEN** any physical target loses the fixture connection after telemetry begins and the fixture advertises again
- **THEN** the backend reconnects, revalidates its controller route, resumes identity and telemetry, preserves the same active session, and later completes it through explicit disconnect

#### Scenario: Reconnect finds another controller
- **WHEN** the fixture returns a different meaningful controller UUID during an active-session reconnect
- **THEN** the backend interrupts the old session, reports `controller_changed`, and does not append replacement-controller telemetry

#### Scenario: Explicit CAN ID without ping
- **WHEN** the Board Profile configures the simulated controller ID and Express CAN ping omits that ID
- **THEN** the target probes the configured route directly and reaches `ready` without selecting a different controller

#### Scenario: Multiple-controller selection
- **WHEN** Express discovery reports two VESC motor controllers
- **THEN** conformance observes both candidates, selects one through the shared contract, verifies the chosen route and persisted preference, and verifies its reuse after reconnect

#### Scenario: Mobile background continuity
- **WHEN** Android or iOS is backgrounded while a physical ride session is active
- **THEN** native polling and session work continue where the OS permits, and a foregrounded replacement frontend recovers the same connection and session with additional samples

#### Scenario: macOS physical coverage
- **WHEN** macOS physical conformance runs
- **THEN** it runs identity, ride, reconnect, and the complete representative topology set through the shared Apple backend and CoreBluetooth without a synthetic mobile-background scenario

#### Scenario: iOS physical coverage
- **WHEN** iOS physical conformance runs
- **THEN** it runs identity, ride, reconnect, direct dynamic, Express discovery, and background continuity without repeating all macOS shared-backend topology cases

## ADDED Requirements

### Requirement: Exhaustive connection compatibility coverage
Software conformance SHALL cover the Cartesian product of direct and Express topology, VESC 6.02/6.05/6.06, and Refloat 1.0.0/1.1.0/1.2.3. It SHALL additionally cover configured, absent, invalid, duplicate, zero, one, and multiple CAN candidates without requiring a separate simulator implementation per combination.

#### Scenario: Supported version matrix
- **WHEN** the software compatibility matrix runs
- **THEN** all eighteen topology/controller/package combinations establish the expected route and protocol family, report legacy source package 1.0.0 as wire version `1.0`, and produce route-correct telemetry

#### Scenario: Wrong configured ID
- **WHEN** a configured CAN ID is absent, times out, or reports non-VESC hardware
- **THEN** conformance verifies bounded failure and no fallback to another candidate

#### Scenario: Ambiguous selection recovery
- **WHEN** the frontend connection is recreated while multiple candidates await selection
- **THEN** conformance verifies the candidates recover through `state.get` and a current selection resumes the connection exactly once

#### Scenario: Candidate ordering and support
- **WHEN** firmware probes discover supported and unsupported VESC motor controllers out of CAN-ID order
- **THEN** conformance verifies all motor controllers appear sorted by CAN ID, unsupported entries cannot be selected, and no supported entry is silently chosen

#### Scenario: Discovery bounds
- **WHEN** simulation exceeds 64 normalized CAN IDs or 16 motor controllers
- **THEN** conformance verifies the matching stable issue, no misleading truncated candidate state, and no unbounded probing

#### Scenario: Selection profile merge
- **WHEN** another Board Profile field changes after candidates are published and a selection is accepted
- **THEN** conformance verifies only `preferredCanId` is merged into the latest profile and its revision advances once

#### Scenario: Stable selection errors
- **WHEN** conformance sends malformed, wrong-device, unsupported-candidate, stale, or duplicate controller selections
- **THEN** both backends return `invalid_request` only for malformed input and `invalid_state` for every well-formed invalid-state selection

#### Scenario: Failed same-device retry
- **WHEN** a failed connection's preferred CAN ID is corrected and `device.connect` names the same device
- **THEN** conformance verifies a new generation clears the old issue and uses the latest Board Profile

#### Scenario: Same controller moves CAN ID
- **WHEN** a private automatic hint fails during active-session reconnect and discovery finds the same meaningful UUID at another CAN ID
- **THEN** conformance verifies the hint and route update, the same session resumes, and no ambiguity prompt is shown

### Requirement: BLE setup and stream coverage
Software backend tests SHALL cover VESC Tool-style saved-identifier advertisement allowlisting, default and enlarged BLE write payloads, supported write response modes, fragmented frames and notifications, missing setup elements, setup timeouts, bounded queues, and stale lifecycle callbacks.

#### Scenario: Default payload fragmentation
- **WHEN** an outbound VESC frame exceeds the default BLE payload
- **THEN** tests verify ordered safe chunks reassemble to exactly one original frame

#### Scenario: Setup timeout
- **WHEN** service discovery or notification enablement never completes
- **THEN** tests verify deterministic failure within the setup deadline and bounded retained diagnostics

#### Scenario: Stale notification
- **WHEN** an old connection generation supplies bytes after reconnect begins
- **THEN** tests verify those bytes cannot complete identification or mutate the new connection state

#### Scenario: Saved advertisement exception
- **WHEN** unfiltered BLE observation receives one exact saved identifier and one unknown identifier without UART service advertisements
- **THEN** tests verify only the saved device is published and both still require actual UART service validation before connection setup succeeds

#### Scenario: Saved identity before session
- **WHEN** a first-time peer supplies valid local VESC firmware and later requires controller selection
- **THEN** tests verify saved identity and profile exist during selection while no durable session exists until application `ready`
