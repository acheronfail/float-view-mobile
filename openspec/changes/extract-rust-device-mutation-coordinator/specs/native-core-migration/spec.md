## ADDED Requirements

### Requirement: Rust device mutation coordinator candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated connection-scoped device mutation authorization and serialization owner while Apple and Android retain their selected native owners. The candidate SHALL directly use safe compatibility, control-codec, normalized-telemetry, firmware-identity, and scheduler-fact models to own current authoritative mutation state without a foreign bridge.

#### Scenario: Configure one exact generation

- **WHEN** a current checked controller route, structurally decoded firmware identity, admitted Refloat compatibility profile, and readiness facts configure the candidate
- **THEN** it SHALL derive writable and shutdown capability through safe Rust policy, replace all prior-generation state atomically, and publish one bounded loading control snapshot with no request

#### Scenario: Establish authoritative control state

- **WHEN** current monotonic normalized telemetry and exact profile-matched lighting observations arrive
- **THEN** package, flywheel, fatal-latch, active-fault, clearability, LED, and headlight state SHALL converge to the same ready/loading snapshot and confirmation order as both native owners

#### Scenario: Authorize every semantic control

- **WHEN** package, flywheel, lighting, or fatal-clear is requested against current readiness, compatibility, access, ride, package, footpad, fault, and pending state
- **THEN** the candidate SHALL either return an exact idempotent snapshot with no request or one generation-qualified mutation token and exact route-correct scheduler request facts matching the selected native behavior

#### Scenario: Preserve the flywheel safety exception

- **WHEN** authoritative package mode is flywheel
- **THEN** every mutation SHALL be rejected except exact eligible flywheel disable, which SHALL remain available while normalized ride state is riding and SHALL confirm only after later normal-mode telemetry

#### Scenario: Serialize controller shutdown

- **WHEN** shutdown is requested on a current ready idle exact supported writable controller with no competing mutation
- **THEN** the candidate SHALL reserve the sole pending slot and return one exact direct or once-forwarded accepted-only shutdown request while unsupported, moving, flywheel, stale, read-only, or competing states emit no request

#### Scenario: Serialize configuration and Tune work

- **WHEN** a configuration write or Tune apply is requested with current ready writable idle state, no footpad engagement, ready normal package state, and no competing mutation
- **THEN** the candidate SHALL return one exact generation/sequence-qualified configuration lease that excludes controls and shutdown until matching verified finish, failure, cancellation, or reset

#### Scenario: Confirm, fail, and reset exactly

- **WHEN** authoritative observations, request failure, lease completion, shutdown clearing, cancellation, or connection reset occur
- **THEN** only the exact current token and generation SHALL alter pending state, confirmations SHALL follow source order, and stale, repeated, mismatched, or prior-generation events SHALL remain inert

#### Scenario: Remain total at public bounds

- **WHEN** routes, identities, generations, sequences, sample sequences, payloads, profiles, observations, and transition order reach valid limits or invalid input exceeds them
- **THEN** the candidate SHALL remain bounded and deterministic, reject invalid input without partial mutation, wrap only the explicit token counter, and expose no private firmware or telemetry payload through diagnostics

### Requirement: Device mutation coordinator remains below runtime execution

The Rust device mutation coordinator candidate SHALL expose only safe Rust commands, bounded snapshots, mutation tokens, scheduler request facts, stable errors/diagnostics, and ordered effects. It MUST NOT own or call BLE, VESC framing, a scheduler instance, physical clock, timer executor, callback, foreign handle, bridge record, persistence, SQLite, sessions, local server, frontend, thread, filesystem, socket, async runtime, or OS framework.

#### Scenario: Return mutation work without executing it

- **WHEN** an authorized operation requires scheduler submission, application publication, pending completion, or diagnostic work
- **THEN** the candidate SHALL return the exact bounded owned values in deterministic order and retain no host or caller reference

#### Scenario: Compose the future coordinator directly

- **WHEN** a later Rust connection application coordinator owns configuration, request scheduling, response attribution, and effect publication
- **THEN** it SHALL call this safe candidate directly without a device-mutation Apple FFI or Android JNI crate
