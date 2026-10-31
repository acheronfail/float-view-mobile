# Unified Rust Runtime Specification

## Purpose

Define the safe coarse application runtime selected by the shared service on both native host families.

Original cutover requirements and evidence remain in [archived changes](../../changes/archive/).
Ongoing checks follow the [verification policy](../../../AGENTS.md#verification).
The capability identifier remains stable for historical references.

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

The application runtime SHALL remain independent of HTTP, sockets, server executors and OS integration. Bluetooth, clocks, timers, paths, cache access, routing-hint storage and activation SHALL cross its boundary as facts/effects. The shared service executes portable work; hosts execute OS work.

#### Scenario: Execute runtime effects

- **WHEN** the runtime returns writes, timers, cache work, activation or publication effects
- **THEN** the shared service executes portable cache, routing-hint, storage and activation work and forwards only OS effects to adapters, reporting outcomes through later commands
- **AND** native hosts retain Bluetooth adapters, location acquisition, lifecycle/background execution, permissions, native file presentation, theme and WebViews; shared transport owns loopback authentication and the frontend API

#### Scenario: Replace a WebView

- **WHEN** a platform destroys and recreates its WebView while the process and runtime remain alive
- **THEN** the replacement frontend observes the same authoritative state without transferring runtime ownership to the view

### Requirement: Both mobile hosts select the same runtime

Apple and Android SHALL select the same safe application runtime through thin coarse platform bindings. No protocol-specific bridge, duplicate native application/protocol/session decision owner or fallback SHALL remain.

#### Scenario: Inspect production selection

- **WHEN** source, dependencies and native build inputs are audited
- **THEN** both host families select the existing shared runtime and no superseded per-protocol bridge or build matrix is present

#### Scenario: Verify changed composition

- **WHEN** runtime composition changes
- **THEN** affected tests cover framing, persistence, ordering, adversarial input, reset and redaction through the selected owner rather than an isolated candidate
