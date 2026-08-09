## ADDED Requirements

### Requirement: Complete live BMS contract state

`ActiveConnection` SHALL contain one mandatory bounded `bms` value with status `discovering`, `not_found`, `attached`, `unavailable`, or `ambiguous`; nullable direct-or-CAN connection; at most 16 display-safe candidates; and nullable complete latest snapshot. The snapshot SHALL contain fixed platform-neutral fields for observation time, voltages, currents, session counters, cell voltages and balancing, temperatures, humidity, state of charge and health, reported CAN ID, lifetime counters, pressure, data version, and status.

#### Scenario: Attached BMS

- **WHEN** the backend receives and completely decodes values directly from the selected VBMS
- **THEN** a newer authoritative state reports `attached`, the verified route, no ambiguity candidates, and one complete snapshot with bounded arrays

#### Scenario: Ambiguous BMS

- **WHEN** multiple unconfigured VBMS identities are detected
- **THEN** authoritative state reports `ambiguous`, a null selected connection and snapshot, and CAN-ID-sorted bounded candidates with canonical UUIDs when meaningful

#### Scenario: Unavailable after a prior sample

- **WHEN** a previously attached VBMS exceeds its consecutive failure threshold
- **THEN** authoritative state reports `unavailable` while retaining the last snapshot and its original observation time

#### Scenario: Frontend recreation

- **WHEN** a disposable frontend reconnects during BMS discovery, ambiguity, attachment, unavailability, or controller reconnect
- **THEN** `state.get` supplies the complete current BMS value without another platform-specific request

### Requirement: Live-only BMS event semantics

Every changed BMS status, route, candidate set, or successful complete BMS snapshot SHALL advance the application-state revision and publish the newest conflated `state.changed` snapshot. BMS snapshots MUST NOT be persisted as session samples or diagnostics and MUST NOT change session counts, aggregates, integrity, review, or exports.

#### Scenario: Successful BMS poll

- **WHEN** a complete BMS response differs by observation time or value from authoritative state
- **THEN** the backend publishes it through a newer complete application snapshot without appending a session sample

#### Scenario: Slow frontend

- **WHEN** BMS polls produce state faster than the frontend consumes events
- **THEN** pending delivery converges on the newest complete bounded BMS snapshot without retaining an unbounded history

#### Scenario: Session export

- **WHEN** a session recorded while live BMS values were visible is reviewed, diagnosed, or exported
- **THEN** its persisted shapes and results contain no BMS snapshot arrays or BMS-only counters introduced by this change

### Requirement: Preferred BMS CAN profile field

Every Board Profile SHALL accept nullable or absent `preferredBmsCanId` for automatic BMS discovery or an integer from 0 through 253 for one authoritative CAN route. The field SHALL be revision checked, SHALL NOT contribute to profile completeness, and SHALL remain independent from the controller's `preferredCanId`.

#### Scenario: Valid BMS preference

- **WHEN** the frontend updates `preferredBmsCanId` to `42` at the current Board Profile revision
- **THEN** both backends persist exactly `42`, advance the profile revision once, preserve unrelated fields, and apply it to the current auxiliary generation and subsequent connections

#### Scenario: Cleared BMS preference

- **WHEN** the frontend clears `preferredBmsCanId`
- **THEN** the profile returns to bounded automatic BMS discovery without clearing or changing `preferredCanId`

#### Scenario: Invalid BMS preference

- **WHEN** a patch supplies a negative, fractional, nonnumeric, or greater-than-253 `preferredBmsCanId`
- **THEN** every backend rejects it with the standard invalid-request semantics and leaves persisted profile and active BMS routing unchanged

#### Scenario: Active BMS-only reconfiguration

- **WHEN** a valid `preferredBmsCanId` update targets the active saved board
- **THEN** controller route, connection readiness, and active session remain unchanged while a newer auxiliary generation resolves the requested BMS route
