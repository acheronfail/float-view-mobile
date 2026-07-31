## ADDED Requirements

### Requirement: Canonical VBMS simulation ownership
`conformance/` SHALL own source-faithful local and CAN-routed VBMS identity, values encoding, topology composition, request classification, timeouts, malformed responses, and deterministic scenario behavior. The programmable ESP32 firmware MUST remain a bounded byte and event bridge and MUST NOT interpret BMS state or choose responses.

#### Scenario: Source-faithful VBMS fixtures
- **WHEN** software or physical conformance answers VBMS firmware and values requests
- **THEN** it uses immutable payload fixtures derived from the pinned upstream VBMS encoder and validates Swift, Kotlin, and simulator decoding against the same bytes

#### Scenario: Routed BMS request
- **WHEN** a scenario places a VBMS at CAN ID `42`
- **THEN** conformance responds only to a command-96 request forwarded exactly once to ID `42` and records other routes as bounded unexpected requests

#### Scenario: Unsafe BMS command
- **WHEN** a backend sends a BMS mutation, bootloader, configuration, or firmware-write command
- **THEN** canonical conformance fails the scenario and the fixture performs no mutation

### Requirement: Complete VBMS topology and lifecycle matrix
Canonical conformance SHALL exercise direct-controller-plus-VBMS, gateway-plus-controller-and-VBMS, and direct-VBMS-plus-CAN-controller compositions with zero, one, configured, omitted-from-ping, and multiple BMS nodes. It SHALL verify that BMS behavior never controls controller readiness or session continuity.

#### Scenario: Unique auxiliary BMS
- **WHEN** each supported controller topology contains one live VBMS
- **THEN** both backends establish the same controller route, become ready, select the expected independent BMS route, and publish equivalent decoded values

#### Scenario: Multiple BMS nodes without preference
- **WHEN** two valid BMS identities are present and `preferredBmsCanId` is null
- **THEN** both backends publish the same sorted ambiguity candidates, send no BMS values request, and continue controller telemetry

#### Scenario: Multiple BMS nodes with preference
- **WHEN** two valid BMS identities are present and `preferredBmsCanId` names one of them
- **THEN** both backends directly probe and poll only that exact CAN route without requiring it in ping results

#### Scenario: BMS loss during ride
- **WHEN** a BMS stops responding during a persisted riding session
- **THEN** conformance observes attached-to-unavailable BMS state after the bounded threshold while controller samples and the same session continue

#### Scenario: BMS recovery and reconnect
- **WHEN** the BMS later responds or the BLE link reconnects to the same controller and BMS
- **THEN** conformance observes direct revalidation and a fresh attached snapshot without replacing or interrupting the controller session

### Requirement: Bounded BMS failure and contract assertions
Conformance SHALL cover maximum and excessive cell and temperature counts, invalid balancing flags, truncation at every variable section, non-finite values, CAN-ID mismatch, legacy and extended tails, status termination and bounds, response timeouts, candidate overflow, state-event bounds, privacy, and live-only persistence.

#### Scenario: Malformed values table
- **WHEN** table-driven cases supply each malformed count, value, flag, route, or remainder
- **THEN** Apple and Android reject the same cases without publishing partial values or blocking controller work

#### Scenario: Maximum complete values
- **WHEN** a valid response contains 50 cells, 50 temperatures, and the maximum bounded extended status
- **THEN** both backends publish equivalent complete state within the contract message bound and the frontend renders it without an unbounded collection

#### Scenario: Live-only verification
- **WHEN** multiple BMS snapshots occur during a ride followed by review, diagnostics, export, and deletion
- **THEN** conformance verifies state revisions and latest live replacement while session sample shapes, counts, aggregates, stored rows, diagnostics, and export contents contain no BMS additions

#### Scenario: Privacy verification
- **WHEN** a VBMS identity contains a meaningful UUID
- **THEN** canonical state, errors, bounded diagnostics, ordinary logs, and exports contain no UUID bytes or textual encoding

