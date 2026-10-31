# Apple Shared Runtime Integration

## Purpose

Define the current iOS/macOS OS adapter and coarse Rust binding responsibilities.

Original cutover requirements and evidence remain in [archived changes](../../changes/archive/).
Ongoing checks follow the [verification policy](../../../AGENTS.md#verification).
The capability identifier remains stable for historical references.

## Requirements

### Requirement: Apple selects one coarse runtime boundary

iOS and macOS SHALL use one process-owned Rust application runtime as the sole production owner of VESC stream/framing, connection discovery, request scheduling, controller/BMS/Refloat interpretation, telemetry normalization/cadence, mutation/configuration coordination, active-session writes, bounded history reduction, and prune decisions. Swift MUST NOT retain a production fallback for those decisions.

#### Scenario: Prepare an Apple transport

- **WHEN** CoreBluetooth reports the selected VESC transport ready
- **THEN** the shared service begins the runtime generation, executes portable effects and forwards ordered Bluetooth/timer work to Swift

#### Scenario: Receive notification fragments

- **WHEN** Apple receives fragmented, noisy, multiple, malformed, or oversized BLE notification bytes
- **THEN** the bytes enter the coarse runtime exactly once and Swift performs no framing or protocol interpretation fallback

#### Scenario: Reset or reconnect

- **WHEN** transport loss, explicit disconnect, radio/permission loss, timeout, replacement, or reconnect resets the connection
- **THEN** Swift cancels the returned OS work, advances the runtime generation, and stale host firings or BLE events cannot mutate current state

### Requirement: Apple retains OS ownership

Swift SHALL own Bluetooth, clocks/timers, lifecycle/background execution, permissions, location, protected storage-root setup, external file access/presentation, theme and WebViews. The shared service/server SHALL own portable product/catalog storage, archives, policy, authentication, dispatch and serialization without duplicate Swift domain records or snapshots.

#### Scenario: Execute an effect

- **WHEN** Rust emits a bounded host effect
- **THEN** Swift executes only that OS capability work in order and reports later facts without adding application policy

#### Scenario: Replace the WebView

- **WHEN** iOS or macOS destroys and recreates its WebView
- **THEN** runtime state and durable data survive and the new frontend receives the same process-owned snapshot and authenticated monotonic state stream

#### Scenario: Acquire a location

- **WHEN** the authorized native provider supplies a finite observation for an attached session
- **THEN** the shared service applies capture policy and publishes location state only after the Rust session marker advances

### Requirement: Apple coarse ABI is bounded and callback-free

The Apple boundary SHALL use the versioned closed command/result envelope and C safety rules in [rust-apple-ffi-support](../rust-apple-ffi-support/spec.md). It SHALL retain no caller buffer or Swift object, invoke no callback, contain panics and expose only payload-free failures.

#### Scenario: Submit and retrieve a result

- **WHEN** Swift submits a valid bounded service command to a live handle
- **THEN** Rust applies it once, reports the required result length and exposes the stored complete result through the separate bounded copy operation
- **AND** insufficient copy capacity does not replay the command

#### Scenario: Destroy a service

- **WHEN** the Apple owner destroys or replaces its service handle
- **THEN** later commands and stale tokens fail closed without affecting replacement state

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

### Requirement: Apple retains only the coarse binding

Apple production and package inputs SHALL contain one coarse runtime binding and no superseded Swift portable decision owners, per-protocol FFI crates/modules, support crate, build/link machinery or fallback imports.

#### Scenario: Audit selected Apple packages

- **WHEN** source, dependencies, headers, linked archives or exported symbols are checked
- **THEN** the coarse binding is selected and no obsolete protocol artifact or production fallback is present
