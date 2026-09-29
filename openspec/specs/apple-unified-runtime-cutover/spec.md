# apple-unified-runtime-cutover Specification

## Purpose

Define the shared Apple backend's selection of the unified Rust runtime, bounded native integration, and required ownership and verification evidence.

## Verification scope

Use the current [impact-based verification policy](../../../AGENTS.md#verification)
for ongoing changes. Full-suite, all-platform packaging and physical acceptance
gates below apply only to the original completed migration, refactor or feature
change. They do not impose recurring gates on maintenance. Preserve behavioral
coverage, selecting relevant tests and hardware checkpoints by actual impact.

## Requirements

### Requirement: Apple selects one coarse runtime boundary

iOS and macOS SHALL use one process-owned Rust application runtime as the sole production owner of VESC stream/framing, connection discovery, request scheduling, controller/BMS/Refloat interpretation, telemetry normalization/cadence, mutation/configuration coordination, active-session writes, bounded history reduction, and prune decisions. Swift MUST NOT retain a production fallback for those decisions.

#### Scenario: Prepare an Apple transport

- **WHEN** CoreBluetooth reports the selected VESC transport ready
- **THEN** Swift begins the Rust runtime generation and executes its ordered framed-write, deadline, timer, cache, activation, persistence, publication, failure, and diagnostic effects

#### Scenario: Receive notification fragments

- **WHEN** Apple receives fragmented, noisy, multiple, malformed, or oversized BLE notification bytes
- **THEN** the bytes enter the coarse runtime exactly once and Swift performs no framing or protocol interpretation fallback

#### Scenario: Reset or reconnect

- **WHEN** transport loss, explicit disconnect, radio/permission loss, timeout, replacement, or reconnect resets the connection
- **THEN** Swift cancels the returned OS work, advances the runtime generation, and stale host firings or BLE events cannot mutate current state

### Requirement: Apple retains OS and contract ownership

Swift SHALL retain Bluetooth execution, clocks/timers, lifecycle/background work, permissions, location acquisition, native file presentation, theme/UI integration, WebView ownership, storage-root protection, and export/import presentation. Shared Rust SHALL own loopback transport, authentication, frontend dispatch, and serialization. The shared backend service SHALL own product/catalog persistence, archive formats and portable application policy. Swift SHALL supply typed OS facts and execute bounded OS effects without duplicating domain records, frontend dispatch or portable decisions. Runtime state and durable session data SHALL survive disposable WebView replacement and remain exposed only through the existing authenticated frontend contract.

#### Scenario: Execute an effect

- **WHEN** Rust emits a bounded host effect
- **THEN** Swift executes only the corresponding OS capability work in order and reports later facts without adding application policy

#### Scenario: Replace the WebView

- **WHEN** iOS or macOS destroys and recreates its WebView
- **THEN** the new frontend receives the same native-process runtime snapshot and authenticated monotonic state stream

#### Scenario: Acquire a location

- **WHEN** the native location provider supplies an authorized finite observation for an attached session
- **THEN** Swift supplies it to the shared service, which applies capture policy and publishes location state only after the Rust session marker advances

### Requirement: Apple coarse ABI is bounded and callback-free

The Apple runtime boundary SHALL use one versioned, closed, strictly bounded caller-owned command/result envelope. It MUST contain panics, reject invalid handles/tokens/UTF-8/tags/extents/non-finite values atomically, retain no caller buffer or Swift object, invoke no callback, and expose no private payload in diagnostics.

#### Scenario: Submit a valid command

- **WHEN** Swift submits a valid command to a live handle with sufficient result capacity
- **THEN** Rust mutates exactly once and returns the complete ordered effect envelope through caller-owned memory

#### Scenario: Reject hostile boundary input

- **WHEN** a handle, token, envelope, buffer region, command, number, path, or result capacity violates the ABI contract
- **THEN** the boundary returns one fixed payload-free failure without partial runtime mutation, caller-memory corruption, callback, or panic escape

#### Scenario: Destroy a runtime

- **WHEN** the Apple service destroys or replaces its runtime handle
- **THEN** later commands and stale timer/deadline tokens fail closed and destruction is safe to repeat

### Requirement: Apple session publication remains atomic

For saved tracking, the Rust runtime SHALL commit normalized samples and accepted locations to the canonical session database before Apple publishes the corresponding live state or commit marker. Historical reads and pruning SHALL use the Rust reader/pruner with existing published-boundary, interruption, preview, stale-plan, and atomic-commit behavior.

#### Scenario: Publish live telemetry

- **WHEN** one valid notification produces a normalized sample for an attached saved session
- **THEN** its database transaction and marker advance before the sample appears in the Apple state stream

#### Scenario: Continue guest telemetry

- **WHEN** a guest connection produces valid normalized telemetry without an attached session
- **THEN** Apple publishes live state without inventing a durable session or marker

#### Scenario: Read and prune history

- **WHEN** the frontend requests a bounded historical page/reduction or commits a current prune preview
- **THEN** Apple returns the existing contract projection of the Rust operation and does not execute native SQL reduction or row-selection logic

### Requirement: Apple cutover deletes superseded ownership

The completed Apple cutover MUST delete all selected Swift protocol/application/session decision implementations, all ten Apple per-protocol FFI crates and native modules, Apple FFI support, per-family build/link/ABI machinery, and obsolete private tests. The repository SHALL contain one Apple runtime bridge artifact, ten Android protocol JNI bridges, and no Apple production fallback, and combined Rust-plus-Apple production source SHALL be below the pre-cutover 51,994-line baseline.

#### Scenario: Audit the completed cutover

- **WHEN** repository audits inspect source, workspace members, bridge crates, native modules, linked archives, and forbidden imports/symbols
- **THEN** only the coarse Apple runtime boundary is selected on Apple, Android remains unchanged, and the measured source/build inventory has decreased

### Requirement: Apple ownership requires software and physical evidence

Apple production ownership SHALL transfer only after focused Rust/Swift tests, packaging for macOS/simulator/device, strict OpenSpec validation, the complete `just verify` gate, macOS/iOS canonical software conformance, and connected iOS physical conformance against the ESP32 fixture all pass.

#### Scenario: Complete Apple verification

- **WHEN** the deletion-bearing implementation is ready to commit
- **THEN** every software gate passes and the connected iOS device completes the canonical physical matrix through the coarse runtime with the fixture

#### Scenario: Defer Android hardware

- **WHEN** Apple physical verification completes
- **THEN** Android hardware verification remains explicitly deferred while Android software checks and production behavior remain unchanged
