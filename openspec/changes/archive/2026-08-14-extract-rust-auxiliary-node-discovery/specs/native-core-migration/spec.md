## ADDED Requirements

### Requirement: Rust auxiliary-node discovery candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated auxiliary VBMS discovery decision state machine while Apple and Android retain their selected native owners. The candidate SHALL directly compose the safe controller-discovery route and inventory model to own exact BMS admission, direct/preferred/automatic route selection, inventory reuse, bounded fallback enumeration, availability, observation retention, reset, and stable failure decisions without host execution.

#### Scenario: Select an exact direct BMS

- **WHEN** the local routed identity is exact stock VESC BMS 6.00 or VBMS32 custom-module 6.06
- **THEN** the candidate SHALL select the direct route and request values without CAN discovery

#### Scenario: Reject an unaudited identity

- **WHEN** a local or CAN identity has another hardware type, hardware name, version, or malformed route attribution
- **THEN** the candidate SHALL NOT retain or poll it as a BMS and SHALL expose no private identity field

#### Scenario: Reuse a complete controller inventory

- **WHEN** current controller discovery supplies a complete routed inventory containing zero, one, or multiple exact BMS identities
- **THEN** the candidate SHALL classify that inventory without another CAN ping or firmware probe and resolve not-found, unique selection, or ambiguity deterministically

#### Scenario: Enumerate an incomplete inventory

- **WHEN** no complete reusable inventory exists and no direct BMS is selected
- **THEN** the candidate SHALL request one CAN ping, normalize at most 64 unique IDs in `0...253`, and probe each route sequentially at most once

#### Scenario: Honor an exact preferred route

- **WHEN** a valid preferred BMS CAN ID is configured
- **THEN** the candidate SHALL use a matching complete-inventory identity or probe that exact route directly, select it only when compatible, and fail unavailable without falling back when it is missing, incompatible, or timed out

#### Scenario: Enforce discovery bounds

- **WHEN** normalized discovery exceeds 64 CAN IDs or classification finds a seventeenth exact BMS
- **THEN** the candidate SHALL become unavailable with a stable bounded issue, select no partial route, and retain no unbounded raw input

#### Scenario: Retain and recover observation state

- **WHEN** a selected route receives a valid current observation, becomes unavailable, and later receives another valid current observation
- **THEN** it SHALL become attached, retain the prior observation as stale while unavailable, and replace it while recovering without inspecting or logging the observation payload

#### Scenario: Isolate generations and reset

- **WHEN** begins, CAN results, firmware results, timeouts, observations, failures, and resets are repeated, reordered, stale, invalid, or reach counter rollover
- **THEN** every public transition SHALL remain deterministic and panic-free, stale work SHALL NOT alter current state, and reset SHALL clear all auxiliary state while invalidating old work

### Requirement: Auxiliary discovery remains below runtime execution

The Rust auxiliary-node discovery candidate SHALL expose only safe bounded routed facts, state, generic owned observation retention, stable issues, and ordered effects. It MUST NOT own or call BLE, clocks, timers, schedulers, request bytes, protocol decoders, callbacks, bridges, persistence, Board Profile JSON, SQLite, transport, local serving, threads, filesystem, sockets, async runtime, frontend serialization, or OS frameworks.

#### Scenario: Return host work without executing it

- **WHEN** discovery requires a CAN ping, firmware probe, values request, publication, or failure
- **THEN** it SHALL return or expose the corresponding bounded typed effect and state while retaining no host reference

#### Scenario: Compose the unified runtime directly

- **WHEN** a later Rust runtime owns request scheduling, BMS decoding, application publication, and persistence
- **THEN** it SHALL call this safe auxiliary-discovery API directly without adding auxiliary-discovery Apple FFI or Android JNI crates
