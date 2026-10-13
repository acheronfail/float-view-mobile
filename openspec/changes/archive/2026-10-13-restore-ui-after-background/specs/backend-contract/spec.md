# Spec Delta

## MODIFIED Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement the WebSocket control methods `state.get`, `uiState.get`, `uiState.set`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `controller.shutdown`, `refloat.control.set`, `refloat.fatalError.clear`, `savedDevice.update`, `savedDevice.delete`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.list`, `session.get`, `session.marker.add`, `session.export`, `session.delete`, `session.delete.cancel`, `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`, plus the authenticated session-data request kinds `series`, `samples`, `events`, `locations`, and `markers`. Session-group methods SHALL NOT remain; `session.getDiagnostics` SHALL NOT remain; exact retained historical reads use `samples`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory control method and session-data request kind is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method or session-data kind outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

## ADDED Requirements

### Requirement: Authenticated checkpoint operations

Every supported backend and mock SHALL implement `uiState.get` with empty parameters returning a typed checkpoint or null, and `uiState.set` with exactly `checkpoint` returning `Accepted`. Null SHALL clear the checkpoint; a valid record SHALL atomically replace it. These methods SHALL use the existing authenticated envelopes, origin checks and stable errors, with no platform branches or additional endpoint.

#### Scenario: Read replace and clear
- **WHEN** an authenticated client reads before any save, saves a valid checkpoint, reads again and then saves null
- **THEN** it observes null, the exact accepted checkpoint and finally null through bounded terminal results

#### Scenario: Invalid authentication
- **WHEN** a checkpoint operation has an invalid origin or token
- **THEN** existing transport rules reject it without exposing or modifying the checkpoint

### Requirement: Closed bounded checkpoint shape

A checkpoint SHALL contain format version 1, a closed route descriptor, optional Live presentation and at most one supported surface descriptor. Each identifier SHALL be nonempty and at most 512 UTF-8 bytes; the complete encoded record SHALL be at most 4,096 UTF-8 bytes. Unknown versions, fields, kinds, invalid identifiers and incompatible route/surface combinations SHALL fail with `invalid_request` without changing the previous checkpoint.

#### Scenario: Route descriptors
- **WHEN** a client supplies a checkpoint route
- **THEN** its kind is exactly Home, Sessions, Settings, Live, BMS, board history with saved-board ID, or session detail with session ID and nullable saved-board return context

#### Scenario: Presentation descriptors
- **WHEN** a client supplies Live presentation or an open surface
- **THEN** Live presentation has only normal/detailed mode and target device identity, and the surface uses the fixed supported kind and applicable target fields without nested dialog stacks, draft maps or arbitrary URLs

#### Scenario: Malformed record
- **WHEN** a request exceeds a checkpoint bound, supplies an unknown version/field, combines a session route with a Live panel or includes an unsaved draft
- **THEN** it returns `invalid_request` and a subsequent get returns the previous accepted checkpoint

#### Scenario: Structurally valid stale target
- **WHEN** a well-formed checkpoint refers to a target that subsequently disappears
- **THEN** checkpoint retrieval still returns the record and restoration validates the target through current authoritative state and normal detail reads

### Requirement: Checkpoint isolation and lifetime

Checkpoint operations SHALL retain only one record in the process-owned service, survive listener restart and frontend replacement, and have no persistence, BLE, recording or product mutation effects. They SHALL neither increment application/product/catalog revisions nor publish `state.changed`; checkpoints SHALL be absent from snapshots, archives and exports. Process recreation SHALL start with no checkpoint.

#### Scenario: Listener and token replacement
- **WHEN** the listener stops and restarts over the same service
- **THEN** a client authenticated with the current bootstrap token retrieves the accepted checkpoint while obsolete authentication remains rejected

#### Scenario: Native process replacement
- **WHEN** a new native service process is constructed
- **THEN** `uiState.get` returns null without reading any persisted UI state

#### Scenario: No product side effects
- **WHEN** a checkpoint is saved or cleared while telemetry and recording continue
- **THEN** the save causes no BLE effects, durable writes, state events or revision increments, and no snapshot or export contains the checkpoint
