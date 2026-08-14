## ADDED Requirements

### Requirement: Rust Refloat telemetry service candidate parity

The repository SHALL provide one safe bounded Rust candidate for the complete duplicated Refloat telemetry application service while Apple and Android retain their selected native services. The candidate SHALL compose the existing Rust legacy/dynamic decoded telemetry models and request-scheduler facts to own layout retry, route-correct request construction, legacy/realtime/compact selection, pending-response attribution, timeout recovery, adaptive cadence, compact-cache merging, normalized samples, session sequence and elapsed time, and ordered host effects.

#### Scenario: Negotiate a dynamic layout
- **WHEN** layout negotiation begins and receives timer ticks without establishment
- **THEN** the candidate SHALL emit one immediate and at most four later route-correct command-32 requests at the established priority/deadline, then cancel retry work and report the stable timeout exactly once

#### Scenario: Select serialized telemetry work
- **WHEN** legacy or dynamic polling starts, repeats, times out, or makes compact data due
- **THEN** the candidate SHALL retain at most one semantic telemetry request pending and emit exact direct/CAN request bytes and scheduler facts for the next legacy, realtime, or compact request without a parallel lane or immediate timeout retry

#### Scenario: Adapt ride cadence
- **WHEN** accepted samples transition among idle, riding, and the inclusive post-ride tail
- **THEN** the candidate SHALL select the established 100 ms riding cadence or configured idle cadence and emit timer replacement plus immediate next-request effects in source-equivalent order

#### Scenario: Merge normalized telemetry
- **WHEN** legacy samples, dynamic compact samples, dynamic realtime samples, decreasing compact totals, bounded extensions, or active firmware faults are accepted
- **THEN** legacy publishable data SHALL normalize directly, compact data SHALL replace the route-scoped cache without publication, realtime data SHALL merge only the latest slow fields and observation time, and published samples SHALL preserve every semantic field with monotonic source-equivalent sequence and nonnegative elapsed time

#### Scenario: Reject stale or mismatched input
- **WHEN** a response kind, scheduler completion, timer token, service generation, route, or telemetry family does not match current state
- **THEN** the candidate SHALL not clear, publish, retry, or advance unrelated work and SHALL return only the stable bounded diagnostic where native behavior reports one

#### Scenario: Remain total at public bounds
- **WHEN** intervals, tail durations, monotonic or observation times, sequences, tokens, generations, routes, extensions, and decoded optional values reach valid limits or invalid input exceeds them
- **THEN** the candidate SHALL remain bounded and deterministic, reject invalid input without partial mutation, and use explicit wrapping only for source-equivalent counters

### Requirement: Telemetry candidate remains below host execution

The Rust Refloat telemetry-service candidate SHALL expose only safe Rust state, commands, snapshots, decoded-model input, normalized samples, scheduler facts, and ordered effects. It MUST NOT own or call a clock, timer, BLE writer, callback, foreign handle, bridge record, persistence port, SQLite connection, transport, local server, frontend, thread, filesystem, network socket, async runtime, or OS framework.

#### Scenario: Return host work without executing it
- **WHEN** a transition requires a timer change, scheduler enqueue, sample publication, failure, or diagnostic
- **THEN** the candidate SHALL return the bounded typed effects in required order and retain no host reference

#### Scenario: Compose a later unified runtime
- **WHEN** a later Rust runtime owns scheduler execution, protocol decoding, application behavior, persistence, and loopback serving
- **THEN** it SHALL call this safe API directly and SHALL NOT require `refloat-telemetry-service-apple-ffi` or `refloat-telemetry-service-android-jni`
