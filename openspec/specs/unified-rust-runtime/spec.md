# Unified Rust Runtime Specification

## Purpose

Define the safe coarse application boundary that composes proven portable backend owners and the evidence and convergence gates required before native hosts select it.

## Requirements

### Requirement: One portable application runtime owner

The core workspace SHALL contain one safe application-runtime component that directly composes the existing VESC framing, connection/application coordinator, normalized telemetry, and canonical session-SQLite components. It MUST own connection-scoped stream and application state without owning an OS service, callback, executor, thread, loopback transport, catalog database, filesystem discovery, or platform object.

#### Scenario: Construct the runtime

- **WHEN** a caller creates a runtime with valid bounded settings and no active connection or session
- **THEN** it exposes one disconnected snapshot, retains no host reference, and creates no background work

#### Scenario: Reject another sideways extraction

- **WHEN** portable framing, protocol, scheduling, discovery, telemetry, configuration, mutation, or session behavior is needed by the runtime
- **THEN** it composes the existing safe owner directly instead of copying the behavior or introducing another protocol-specific bridge

### Requirement: Closed command and ordered effect boundary

The runtime SHALL accept a closed set of bounded platform-neutral commands carrying explicit monotonic and epoch time facts and SHALL return a bounded ordered list of owned effects. Effects SHALL cover framed BLE writes, timer/deadline replacement and cancellation, routing-hint and configuration-cache work, activation, state/telemetry/BMS/session-marker publication, diagnostics, and closed failures without invoking host callbacks.

#### Scenario: Process a fragmented notification stream

- **WHEN** bounded BLE notification bytes contain noise, fragments, multiple valid frames, or a recoverable malformed frame
- **THEN** the runtime retains only the bounded incomplete suffix, submits complete payloads to application coordination in order, and returns the same ordered effects as individual valid payload delivery

#### Scenario: Schedule a BLE write

- **WHEN** application coordination emits a bounded VESC payload write
- **THEN** the runtime emits exactly one fully framed byte effect and the host need not call a protocol codec

#### Scenario: Reject invalid input atomically

- **WHEN** a command has stale generation, invalid time, invalid identifier, oversized bytes, impossible primitive value, or would exceed the effect bound
- **THEN** the runtime returns a closed payload-free failure without partial state mutation or host work

### Requirement: Commit telemetry before publication

The runtime SHALL optionally own one caller-selected canonical session writer. When attached, every normalized telemetry sample and location observation SHALL commit through the shared SQLite owner before the corresponding publication and marker effects become visible.

#### Scenario: Persist normalized telemetry

- **WHEN** a valid response produces a normalized telemetry sample while a session writer is attached
- **THEN** the runtime commits the sample, emits its advanced marker, and only then emits the sample/public-state effects in deterministic order

#### Scenario: Persistence fails

- **WHEN** SQLite rejects, blocks, fills, or otherwise fails a sample or location transaction
- **THEN** the runtime emits a closed persistence failure, exposes no uncommitted sample or marker, and remains boundedly resettable

#### Scenario: No session is attached

- **WHEN** normalized live telemetry is produced for a connection that intentionally has no durable session
- **THEN** live publication continues without fabricating a durable marker or database

### Requirement: Reuse bounded historical and prune ownership

The runtime component SHALL make the existing session reader and pruner operations available through the same safe component boundary with their published-boundary, pagination, reduction, interruption, preview, stale-plan, and atomic-commit semantics unchanged.

#### Scenario: Read historical data

- **WHEN** a caller supplies a valid session path, published boundary, fields, range, and bound
- **THEN** the runtime obtains the deterministic result directly from the shared session-SQLite owner without native SQL or reduction logic

#### Scenario: Preview and commit pruning

- **WHEN** a caller previews and commits a still-current bounded prune plan
- **THEN** the runtime uses the existing shared pruner and returns its exact atomic result without copying selection or deletion behavior

### Requirement: Platform responsibilities remain outside the runtime

The runtime MUST express Bluetooth I/O, clocks, timers, persistence paths, cache access, routing-hint storage, and activation as caller facts or returned effects. Bluetooth adapters, location acquisition, lifecycle/background execution, permissions, native file presentation, theme, and WebViews SHALL remain platform-owned. Shared Rust transport SHALL own loopback authentication and the frontend API above the application runtime. The shared backend service SHALL satisfy portable cache, routing-hint, storage and activation effects directly, forwarding only OS effects to native adapters. The application runtime MUST remain independent of HTTP, sockets, and server executors.

#### Scenario: Execute runtime effects

- **WHEN** the runtime returns writes, timers, cache work, activation, or publication effects
- **THEN** the shared backend service executes portable work, dispatches OS work to native adapters and reports resulting facts through later runtime commands

#### Scenario: Replace a WebView

- **WHEN** a platform destroys and recreates its WebView while the native process and runtime remain alive
- **THEN** the replacement frontend observes the same process-owned authoritative state without transferring runtime ownership to the view

### Requirement: Candidate evidence precedes host selection

The unselected runtime SHALL have exact composition, framing, persistence, ordering, adversarial, reset, and redaction tests plus repository audits proving no production host, host bridge, generated binding, or OS dependency selects it. `just verify` SHALL pass before a host-cutover proposal may select the runtime.

#### Scenario: Complete candidate verification

- **WHEN** the runtime change is ready to archive
- **THEN** focused tests, workspace lint, strict OpenSpec validation, the full software gate, and production-isolation audits pass while Apple and Android continue using their existing selected paths

### Requirement: Migration economics are a release gate

The repository SHALL retain a reproducible baseline for production source lines, workspace and bridge counts, selected families, and host build artifacts. Every later host cutover SHALL name its deletion inventory and the migration SHALL NOT be considered complete until both hosts use the coarse runtime boundary and all superseded per-protocol bridges and native decision owners are removed.

#### Scenario: Add a host cutover

- **WHEN** Apple or Android selects the runtime
- **THEN** the same completed change deletes that host's superseded connection, protocol, and persistence decision code and reports the baseline delta

#### Scenario: Finish both host cutovers

- **WHEN** Apple and Android have passed their required gates through the runtime boundary
- **THEN** a final change removes every obsolete per-protocol Apple FFI and Android JNI crate, native adapter, header/module, and per-family build entry rather than retaining fallback scaffolding

### Requirement: Apple selects the unified runtime independently

The shared iOS/macOS backend SHALL select the unified Rust runtime through one coarse audited Apple boundary only after deleting its superseded native application/protocol/session decisions and passing complete Apple software and physical gates. Android SHALL remain the production owner of its current application behavior and continue using its temporary JNI bridges until a separate deletion-bearing cutover.

#### Scenario: Complete Apple selection

- **WHEN** the Apple deletion, packaging, canonical conformance, and connected iOS fixture gates pass
- **THEN** Apple becomes the first production selector of the unified runtime while Android ownership is unchanged

#### Scenario: Preserve the final convergence gate

- **WHEN** only Apple has selected the runtime
- **THEN** the migration remains incomplete and bridge retirement cannot remove Android scaffolding until Android independently passes its cutover

### Requirement: Both mobile hosts complete unified runtime selection

Apple and Android SHALL select the same safe application runtime through thin platform bindings, with no protocol-specific bridge or native application/protocol/session-decision fallback remaining.

#### Scenario: Complete the migration

- **WHEN** Android passes its deletion, packaging, software, convergence, and requested iOS physical-regression gates
- **THEN** both mobile hosts share the unified runtime and the temporary protocol bridge/build matrix is fully retired
