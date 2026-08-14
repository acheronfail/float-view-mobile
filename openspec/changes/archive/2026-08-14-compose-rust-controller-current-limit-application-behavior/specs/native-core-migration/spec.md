## ADDED Requirements

### Requirement: Composed Rust controller current-limit application lifecycle

The safe Rust connection application coordinator SHALL directly own the complete one-shot controller current-limit application lifecycle by composing the safe VESC controller-data decoder with its current activation generation and request scheduler. The composition SHALL preserve native request order, direct/CAN routing, response attribution, optional live-state publication, soft failure, and reset behavior without a foreign bridge.

#### Scenario: Request current limits after controller activation

- **WHEN** an exact supported controller generation is activated
- **THEN** the coordinator SHALL enqueue one route-correct command-14 request with controller-work priority, the native-equivalent timeout, exact current scheduler generation, and current-limit response context after initial configuration work and before telemetry/BMS startup completes

#### Scenario: Exclude standalone BMS

- **WHEN** the direct transport is an exact supported standalone BMS
- **THEN** the coordinator SHALL publish no current-limit value and SHALL emit no command-14 request

#### Scenario: Decode one attributed direct or CAN response

- **WHEN** the exact current scheduled direct/CAN current-limit request receives a valid command-14 response
- **THEN** the safe controller-data owner SHALL decode all four finite nonzero limits and optional positive battery-series-cell count, and the coordinator SHALL retain and publish exactly that current ready value

#### Scenario: Keep optional read failures non-fatal

- **WHEN** current-limit work is malformed, timed out, queue-full, cancelled, stale, repeated, mismatched, or reset
- **THEN** the coordinator SHALL expose no stale or partial current-limit value, SHALL leave connection readiness unchanged, and SHALL emit at most stable payload-free diagnostic work

#### Scenario: Clear limits with the connection generation

- **WHEN** begin, reconnect, disconnect, or reset replaces the current connection generation
- **THEN** retained current limits and every in-flight completion from the prior generation SHALL become absent and inert before new application state is published

#### Scenario: Preserve bounded private state

- **WHEN** valid or adversarial current-limit values, payload extents, routes, generations, completions, and transition orders reach their bounds
- **THEN** the coordinator SHALL remain deterministic and panic-free for public input, retain at most one bounded value, and expose no numeric limits or payload bytes through diagnostics or debug formatting

### Requirement: Current-limit composition remains below runtime execution

The composed current-limit lifecycle SHALL expose only safe Rust snapshot values and ordered effects. It MUST NOT own or invoke BLE, VESC framing, a physical clock, timer executor, callback, foreign handle, bridge record, persistence, SQLite, sessions, local serving, frontend code, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return request and publication work without executing it

- **WHEN** activation or an attributed completion requires BLE request execution or application publication
- **THEN** the coordinator SHALL return exact bounded owned effects in deterministic order and retain no host or caller reference

#### Scenario: Compose the future runtime directly

- **WHEN** a later platform-neutral Rust runtime executes coordinator effects and serves application state
- **THEN** it SHALL consume the safe current-limit value directly without a controller-data or coordinator Apple FFI/Android JNI boundary
