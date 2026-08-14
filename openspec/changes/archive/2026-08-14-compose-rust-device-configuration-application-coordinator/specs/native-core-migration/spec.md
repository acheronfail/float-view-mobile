## ADDED Requirements

### Requirement: Composed Rust device configuration application lifecycle

The safe Rust connection application coordinator SHALL directly own and compose the Rust device mutation coordinator and Refloat configuration service with its existing generation, scheduler, discovery, telemetry, and BMS owners. The composition SHALL preserve the selected native authorization, serialization, request attribution, response handling, cache intent, confirmation, failure, and reset behavior without a foreign bridge.

#### Scenario: Activate one selected controller generation

- **WHEN** current discovery reaches a supported controller and the future persistence owner activates the exact awaiting generation with an optional exact cache value
- **THEN** the coordinator SHALL configure mutation and configuration children from the selected checked route, firmware identity, Refloat profile, writability, package version, flywheel state, and cache facts before starting ready application behavior

#### Scenario: Exclude standalone BMS application behavior

- **WHEN** the direct transport is a standalone supported BMS
- **THEN** the coordinator SHALL start only BMS behavior and SHALL expose no controller mutation, Refloat configuration, telemetry-session, cache, or controller-ready work

#### Scenario: Submit semantic mutations through one scheduler

- **WHEN** a current control, fatal-clear, lighting, or shutdown command is authorized
- **THEN** the mutation owner SHALL produce the sole exact token/request facts and the connection coordinator SHALL submit them through its existing scheduler without copying authorization or wire-codec behavior

#### Scenario: Observe authoritative confirmation in source order

- **WHEN** current normalized telemetry or an attributed lighting response arrives
- **THEN** the coordinator SHALL deliver the observation to the mutation owner before publishing resulting application state, preserving exact confirmation and flywheel-prohibition order

#### Scenario: Read configuration without a mutation lease

- **WHEN** current/default values or schema loading is requested without a device write
- **THEN** the configuration service SHALL produce commands 92–94 work through the existing scheduler while controls and shutdown remain governed only by their current pending state

#### Scenario: Hold one lease through verified configuration write

- **WHEN** a configuration write or Tune apply is authorized
- **THEN** the coordinator SHALL acquire the sole generation/sequence-qualified mutation lease before command-95 submission and retain it through acknowledgement and command-93 verified readback until exact success or failure

#### Scenario: Attribute every configuration completion

- **WHEN** a commands 92–95 response or scheduler completion arrives
- **THEN** exact scheduler context, route, index, operation, generation, command, and response bounds SHALL select one configuration transition, while stale, mismatched, malformed, repeated, or unrelated input SHALL alter no current operation

#### Scenario: Return cache and publication work

- **WHEN** configuration state admits, rejects, stores, removes, loads, writes, verifies, conflicts, or fails
- **THEN** the coordinator SHALL return deterministic bounded cache/publication/failure effects without executing persistence or exposing schema/value bytes through diagnostics

#### Scenario: Fail and reset all composed work exactly

- **WHEN** encoding, enqueue, timeout, queue-full, cancellation, stale generation, decode, service, flywheel, disconnect, reconnect, or reset terminates work
- **THEN** scheduler, configuration operation, mutation token, in-memory schema/value state, and published snapshots SHALL converge atomically for the exact current generation with no stuck lease or stale completion

#### Scenario: Remain total at composition bounds

- **WHEN** generations, counters, routes, payloads, schema pages, values, cache bytes, nested effects, transition ordering, and public commands reach valid limits or exceed invalid bounds
- **THEN** the coordinator SHALL remain deterministic and panic-free for public input, reject invalid work before partial publication, enforce the finite effect bound, and expose no private firmware, schema, configuration, or telemetry payload in diagnostics

### Requirement: Composed application coordinator remains below runtime execution

The composed Rust coordinator SHALL expose only safe Rust commands, snapshots, owned work values, and ordered effects. It MUST NOT own or invoke BLE, VESC framing, a physical clock, timer executor, callback, foreign handle, bridge record, cache implementation, persistence port, SQLite, session storage, local server, frontend, thread, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return execution work without performing it

- **WHEN** a transition requires a BLE write, deadline/timer action, cache mutation, persistence/publication action, or diagnostic
- **THEN** the coordinator SHALL return the exact bounded owned effect in deterministic order and retain no host or caller reference

#### Scenario: Compose the future runtime directly

- **WHEN** a later platform-neutral Rust runtime owns effect execution, SQLite/sessions, application publication, and loopback serving
- **THEN** it SHALL call this safe coordinator and its safe children directly without connection, mutation, configuration, or protocol Apple FFI/Android JNI boundaries
