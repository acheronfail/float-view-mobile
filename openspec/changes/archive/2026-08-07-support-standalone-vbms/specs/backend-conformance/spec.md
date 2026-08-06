## MODIFIED Requirements

### Requirement: Complete VBMS topology and lifecycle matrix

Canonical conformance SHALL exercise standalone direct VBMS, direct-controller-plus-VBMS, and gateway-plus-controller-and-VBMS compositions with zero, one, configured, omitted-from-ping, and multiple auxiliary BMS nodes. It SHALL verify that a direct exact VBMS becomes BMS-only without controller work, while auxiliary BMS behavior never controls board-controller readiness or session continuity.

#### Scenario: Standalone direct VBMS

- **WHEN** the BLE transport itself presents an exact supported VBMS identity
- **THEN** both backends reach ready with BMS device kind, poll values directly, publish equivalent decoded values, and create no controller route, Refloat state, telemetry sample, location, or session

#### Scenario: Unique auxiliary BMS

- **WHEN** each supported controller topology contains one live CAN-routed VBMS
- **THEN** both backends establish the same controller route, become ready, select the expected independent BMS route, and publish equivalent decoded values

#### Scenario: Multiple BMS nodes without preference

- **WHEN** two valid auxiliary BMS identities are present and `preferredBmsCanId` is null
- **THEN** both backends publish the same sorted ambiguity candidates, send no BMS values request, and continue controller telemetry

#### Scenario: Multiple BMS nodes with preference

- **WHEN** two valid auxiliary BMS identities are present and `preferredBmsCanId` names one of them
- **THEN** both backends directly probe and poll only that exact CAN route without requiring it in ping results

#### Scenario: BMS loss during ride

- **WHEN** an auxiliary BMS stops responding during a persisted riding session
- **THEN** conformance observes attached-to-unavailable BMS state after the bounded threshold while controller samples and the same session continue

#### Scenario: BMS recovery and reconnect

- **WHEN** a standalone or auxiliary BMS later responds or its BLE transport reconnects
- **THEN** conformance observes generation-safe revalidation and a fresh attached snapshot without creating or replacing a session for standalone BMS mode

## ADDED Requirements

### Requirement: Standalone BMS negative-work verification

The canonical suite SHALL verify Apple and Android BMS-only connections against the same source-faithful direct VBMS fixture and SHALL reject any CAN discovery, forwarding, controller, Refloat, mutation, or durable-session work from that connection.

#### Scenario: Direct BMS request sequence

- **WHEN** software conformance connects to the standalone VBMS scenario
- **THEN** its bounded request transcript contains direct firmware identification and direct read-only BMS values requests but no CAN ping, forwarded packet, controller telemetry, Refloat, motor configuration, shutdown, or BMS mutation command

#### Scenario: BMS-only generation cancellation

- **WHEN** standalone BMS polling completes after explicit disconnect or a replacement generation begins
- **THEN** conformance observes no stale BMS snapshot, session, or navigation-driving device kind from the cancelled generation
