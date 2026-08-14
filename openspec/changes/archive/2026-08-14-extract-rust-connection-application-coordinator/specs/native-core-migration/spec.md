## ADDED Requirements

### Requirement: Rust connection application coordinator candidate parity

The repository SHALL provide one safe bounded Rust candidate for the duplicated connection-generation application spine while Apple and Android retain their selected native owners. The candidate SHALL directly compose Rust controller discovery, request scheduling, Refloat telemetry, and VESC BMS telemetry to own start/reset, request attribution, discovery sequencing, readiness activation, child lifecycle, and ordered runtime effects.

#### Scenario: Begin one atomic connection generation

- **WHEN** valid connection settings, route preferences, continuity facts, and current time begin a new generation
- **THEN** the coordinator SHALL cancel and reset prior work, advance coordinator and scheduler generations, start exact direct firmware identification, and publish ordered deadline/write work without retaining a host reference

#### Scenario: Discover and prepare one controller

- **WHEN** current scheduler-attributed direct, preferred, hinted, enumerated, Refloat INFO, and dynamic-layout responses establish one supported controller
- **THEN** the coordinator SHALL preserve UUID-first continuity and explicit-selection policy, remember or clear routing hints in source-equivalent order, and enter awaiting activation with the exact selected route, identity, compatibility profile, and classified inventory

#### Scenario: Activate application services only after confirmation

- **WHEN** the current awaiting-activation generation receives host confirmation with valid monotonic/session facts
- **THEN** the coordinator SHALL start route-correct Refloat telemetry and attached BMS discovery, publish ready after their initial ordered effects, and reject stale or repeated activation without starting duplicate work

#### Scenario: Preserve standalone BMS isolation

- **WHEN** the direct transport identity is an exact supported standalone BMS
- **THEN** the coordinator SHALL enter ready and start only the direct read-only BMS lifecycle without controller selection, Refloat telemetry, session, location, persistence, configuration, mutation, or control effects

#### Scenario: Attribute every scheduler result once

- **WHEN** writes receive responses, are sent, time out, encounter queue capacity, are cancelled, or belong to an old generation
- **THEN** the coordinator SHALL route each exact completion to only its owning discovery or child-service transition before pumping later work and SHALL not double-count failure or publish stale data

#### Scenario: Route every child timer exactly

- **WHEN** a current Refloat layout/poll or BMS poll timer fires
- **THEN** the coordinator SHALL deliver it only to the exact current child generation and route, enqueue resulting facts through the owned scheduler, and ignore stale, repeated, mismatched, or cancelled timers

#### Scenario: Reset all connection-scoped state

- **WHEN** the connection resets before, during, or after discovery/readiness/telemetry
- **THEN** scheduler requests, deadlines, child timers, layouts, observations, failures, selections, candidates, and public state SHALL clear in deterministic order and all prior-generation events SHALL become inert

#### Scenario: Remain total at public bounds

- **WHEN** payloads, CAN inventories, scheduler queues, timestamps, timers, counters, generations, nested effects, and decoded collections reach valid limits or invalid input exceeds them
- **THEN** the coordinator SHALL remain bounded and deterministic, reject invalid input without partial mutation, and wrap only explicitly source-equivalent counters

### Requirement: Connection coordinator remains below host runtime execution

The Rust connection application coordinator candidate SHALL expose only safe Rust commands, snapshots, owned protocol/application values, and ordered effects. It MUST NOT own or call BLE, frame streams, a physical clock, timer executor, callback, foreign handle, bridge record, persistence port, SQLite connection, session store, transport, local server, frontend, thread, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return all host work without executing it

- **WHEN** a transition requires BLE payload writing, deadline/timer work, routing-hint storage, application publication, persistence confirmation, failure, or diagnostic work
- **THEN** the candidate SHALL return bounded typed effects in source-equivalent order and retain no host or caller reference

#### Scenario: Compose the future runtime directly

- **WHEN** a later Rust runtime owns effect execution, persistence, sessions, application publication, and loopback serving
- **THEN** it SHALL call this coordinator and its safe dependencies directly without a connection-coordinator Apple FFI or Android JNI crate
