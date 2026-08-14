## ADDED Requirements

### Requirement: Rust VESC BMS telemetry service candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated VESC BMS polling/application lifecycle while Apple and Android retain their selected native owners. The candidate SHALL directly compose Rust auxiliary discovery, command-96 decoding, and request-scheduler facts to own discovery execution facts, selected-route polling, concrete observations, completion-driven cadence, failure accounting, unavailability, recovery, generation isolation, reset, and ordered host effects.

#### Scenario: Execute discovery work as scheduler facts

- **WHEN** current auxiliary discovery requests CAN enumeration, a firmware probe, or initial values
- **THEN** the service SHALL emit exact read-only direct/CAN request bytes, expected response, auxiliary priority, context, timeout, and scheduler generation without executing host work

#### Scenario: Poll one selected route

- **WHEN** a direct or CAN BMS is selected and a values request completes as response or sent
- **THEN** the service SHALL retain at most one current values poll, replace one generation-qualified timer for 1,000 ms, and request the same still-selected route when that exact timer fires

#### Scenario: Account for one failed poll

- **WHEN** a values request times out, is rejected because the bounded queue is full, cannot be enqueued, or returns one malformed command-96 payload
- **THEN** the service SHALL count exactly one consecutive failure for that poll and continue bounded recovery polling without delaying controller telemetry

#### Scenario: Become unavailable and recover

- **WHEN** a selected BMS reaches three consecutive failures and later returns a valid current complete response
- **THEN** status SHALL become unavailable while retaining the prior observation, then reset failures and recover to attached with the newer observation

#### Scenario: Own the complete bounded observation

- **WHEN** valid direct or CAN command-96 bytes arrive with a current expected route and observation time
- **THEN** the service SHALL own every bounded decoded field, cell, balancing flag, temperature, optional version/status, route, and time without publishing private values through diagnostics

#### Scenario: Preserve standalone BMS isolation

- **WHEN** an exact direct BMS reaches ready and is polled through the service
- **THEN** it SHALL use the same direct read-only lifecycle while emitting no session, location, persistence, controller-control, configuration, bootloader, or firmware-write effect

#### Scenario: Reject stale or mismatched work

- **WHEN** timer, completion, CAN result, identity, payload, route, request kind, auxiliary generation, scheduler generation, or service generation is stale, repeated, invalid, or mismatched
- **THEN** it SHALL not alter current discovery, failure, observation, cadence, or publication state

#### Scenario: Remain total at public bounds

- **WHEN** routes, inventories, payload counts, observation collections, timestamps, timers, counters, and generations reach valid limits or invalid input exceeds them
- **THEN** the service SHALL remain bounded and deterministic, reject invalid input without partial mutation, and wrap only explicit source-equivalent identity counters

### Requirement: BMS telemetry service remains below runtime execution

The Rust VESC BMS telemetry-service candidate SHALL expose only safe Rust commands, state, owned observations, scheduler facts, and ordered effects. It MUST NOT own or call BLE, a clock, timer executor, callback, foreign handle, bridge record, persistence port, SQLite connection, session store, transport, local server, frontend, thread, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return runtime work without executing it

- **WHEN** a transition requires scheduler enqueue, timer replacement/cancellation, state or observation publication, failure, or diagnostic work
- **THEN** the candidate SHALL return bounded typed effects in source-equivalent order and retain no host reference

#### Scenario: Compose the unified runtime directly

- **WHEN** a later Rust runtime owns scheduler execution, protocol decoding, application publication, persistence, and loopback serving
- **THEN** it SHALL call this safe service directly without `vesc-bms-telemetry-service-apple-ffi` or `vesc-bms-telemetry-service-android-jni`
