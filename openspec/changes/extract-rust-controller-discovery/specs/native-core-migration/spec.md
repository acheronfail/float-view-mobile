## ADDED Requirements

### Requirement: Rust controller-discovery candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated direct/CAN controller-discovery decision state machine while Apple and Android retain their selected native owners. The candidate SHALL directly compose safe firmware identity, VESC compatibility, and Refloat compatibility models to own route admission, configured and learned recovery, enumeration, continuity, candidate selection, readiness, reset, and stable failure decisions without host execution.

#### Scenario: Select a direct controller
- **WHEN** the transport identity is a motor controller at an exactly supported stable or test firmware version
- **THEN** the candidate SHALL select the direct route, retain the identity and access decision, and request Refloat identification without CAN discovery

#### Scenario: Reject unsupported direct firmware
- **WHEN** a direct motor identity has an unsupported version or a non-motor endpoint cannot establish a controller route
- **THEN** the candidate SHALL return the exact stable unsupported/no-controller failure without admitting a different identity or exposing private fields

#### Scenario: Recover configured and learned routes
- **WHEN** a gateway has an authoritative preferred CAN ID or a meaningful learned UUID hint
- **THEN** the candidate SHALL probe the exact route first, enforce configured UUID continuity, accept a matching learned hint, and clear then enumerate after a stale or timed-out learned hint without falling back from an authoritative preference

#### Scenario: Enumerate a bounded CAN network
- **WHEN** CAN discovery supplies duplicates, IDs outside `0...253`, mixed hardware, missing responses, and valid controller identities
- **THEN** the candidate SHALL normalize and sort at most 64 unique valid IDs, probe each sequentially at most once, retain a complete bounded classified inventory, and never select a non-motor node

#### Scenario: Enforce enumeration bounds
- **WHEN** normalized discovery exceeds 64 CAN IDs or probing finds a seventeenth motor controller
- **THEN** the candidate SHALL fail with `too_many_can_devices` or `too_many_controllers`, publish no truncated successful result, and retain no unbounded raw input

#### Scenario: Resolve continuity and ambiguity
- **WHEN** enumeration completes with zero, one, or multiple controllers during a fresh connection or active-session recovery
- **THEN** the candidate SHALL reproduce UUID-first continuity, supported prior-route fallback only when UUID is absent, controller-changed failures, sole-controller admission/rejection, and sorted display-safe candidate publication

#### Scenario: Select only a current supported candidate
- **WHEN** selection names a current supported candidate, an unsupported candidate, an unknown ID, or stale state
- **THEN** the candidate SHALL resume Refloat identification only for the valid candidate and return a stable invalid-state result without mutation for every invalid selection

#### Scenario: Complete Refloat readiness
- **WHEN** an admitted controller receives an audited legacy or dynamic Refloat profile and a dynamic-layout admission result
- **THEN** legacy SHALL become ready directly, dynamic SHALL request layout then become ready only after successful admission, and unsupported input SHALL fail closed

#### Scenario: Isolate generations and reset
- **WHEN** begins, responses, probe timeouts, identification timeouts, selections, failures, and resets are repeated, reordered, stale, or reach counter rollover
- **THEN** every public transition SHALL remain deterministic and panic-free, stale facts SHALL NOT alter current state, and reset SHALL clear all connection state while invalidating old work

### Requirement: Controller discovery remains below runtime execution

The Rust controller-discovery candidate SHALL expose only safe bounded facts, state, snapshots, issues, and ordered effects. It MUST NOT own or call BLE, clocks, timers, schedulers, request-byte codecs, callbacks, foreign records, bridges, persistence, Board Profile JSON, SQLite, transport, local serving, threads, filesystem, sockets, async runtime, or OS frameworks.

#### Scenario: Return host work without executing it
- **WHEN** discovery requires a probe, CAN ping, routing-hint clear/store, Refloat request, layout request, readiness publication, candidate selection, or failure
- **THEN** it SHALL return the corresponding bounded typed effect and retain no host reference

#### Scenario: Compose the unified runtime directly
- **WHEN** a later Rust runtime owns request scheduling, persistence, application publication, and protocol execution
- **THEN** it SHALL call this safe discovery API directly without adding controller-discovery Apple FFI or Android JNI crates
