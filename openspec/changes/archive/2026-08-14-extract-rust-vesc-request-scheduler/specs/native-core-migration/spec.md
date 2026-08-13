## ADDED Requirements

### Requirement: Rust VESC request scheduler candidate parity

The repository SHALL provide a safe, bounded Rust candidate for the complete connection-wide VESC request scheduling state machine while Apple and Android retain their selected native schedulers. The candidate SHALL preserve priority and FIFO arbitration, one response-bearing request in flight, one-way completion, typed response attribution, queue limits, generations, cancellation, explicit deadline expiry, deferred pumping, stable outcomes, and ordered host effects without depending on host execution.

#### Scenario: Arbitrate bounded work
- **WHEN** requests of every priority and equal-priority sequence are enqueued around one in-flight request
- **THEN** the candidate SHALL preserve the non-preemptive in-flight request, select queued work by highest priority then FIFO order, and reject only work beyond the exact bounded queue capacity

#### Scenario: Attribute a response exactly
- **WHEN** a response, route, Refloat command, or typed operation context does not match the current request and generation
- **THEN** the candidate SHALL leave that request in flight and SHALL NOT complete or advance unrelated work

#### Scenario: Replace a connection generation
- **WHEN** a new generation begins with queued or in-flight work and stale responses or deadlines arrive afterward
- **THEN** the candidate SHALL cancel the old work exactly once, reject old-generation enqueue attempts, and prevent stale events from affecting new-generation work

#### Scenario: Complete one-way and response work in order
- **WHEN** one-way, response-bearing, cancelled, timed-out, queue-full, and deferred requests transition
- **THEN** the candidate SHALL emit the same stable completion outcomes and ordered deadline, completion, diagnostic, and next-write effects as the source native behavior

#### Scenario: Remain total at adversarial bounds
- **WHEN** payload, queue, route, timing, sequence, generation, and operation values reach valid boundaries or malformed values exceed them
- **THEN** the candidate SHALL remain bounded and deterministic, use explicit wrapping only for source-equivalent counters, and reject invalid facts without panicking or partially mutating accepted state

### Requirement: Scheduler candidate remains below host execution

The Rust scheduler candidate SHALL expose only safe Rust facts, commands, snapshots, and ordered effects. It MUST NOT own or call a clock, timer, Bluetooth writer, callback, foreign handle, protocol codec, persistence port, transport, frontend, thread, filesystem, network socket, or OS framework.

#### Scenario: Emit host work without executing it
- **WHEN** a transition requires bytes to be written, a deadline to change, a diagnostic to be recorded, or a request to complete
- **THEN** the candidate SHALL return bounded typed effects in the required order and retain no host reference

#### Scenario: Add a later unified runtime
- **WHEN** later changes compose protocol and application services around the scheduler
- **THEN** they SHALL use the safe Rust state API directly and SHALL NOT require a protocol-specific Apple FFI or Android JNI scheduler bridge
