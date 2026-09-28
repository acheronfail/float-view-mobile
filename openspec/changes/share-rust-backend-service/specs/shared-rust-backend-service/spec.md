## Purpose

Define a complete portable backend service that lets native hosts supply only OS capabilities while sharing product persistence, application policy and frontend behavior.

## ADDED Requirements

### Requirement: One portable backend service per process

Each supported native backend SHALL select one process-owned shared service for product state, persistence and application policy, composing the existing application runtime and shared frontend server. The service SHALL own exactly one application runtime and SHALL remain independent of disposable views and listener lifetime. A host MUST NOT implement a parallel portable product service or fallback.

#### Scenario: Construct another host

- **WHEN** a host supplies the required OS capabilities and storage root
- **THEN** the same shared service provides settings, saved boards, profiles, Tunes, sessions, archives, connection policy and state publication without host product implementations

#### Scenario: Replace a frontend or listener

- **WHEN** a WebView is replaced or the listener stops and restarts during permitted recording
- **THEN** the service retains the same runtime, durable session, revisions and launch context without replaying accepted mutations

### Requirement: Hosts execute OS capabilities only

The native boundary SHALL contain closed bounded OS facts, effects and correlated completions. Hosts SHALL own Bluetooth execution, clock/timer execution, permissions, location acquisition, background execution, protected storage-root setup, external file access/presentation, theme and UI integration. Shared code SHALL own portable validation, identities, product CRUD, SQL, archive formats, reconnect policy, recording eligibility and state projection. OS adapters MUST NOT depend on frontend method names or duplicate domain snapshots.

#### Scenario: Complete a native effect

- **WHEN** the shared service needs a permission prompt, BLE operation or native file presentation
- **THEN** the host executes only that OS effect and returns its correlated outcome without making product decisions

#### Scenario: Stale or repeated completion

- **WHEN** an OS completion is duplicated or arrives after its service lifetime or operation has ended
- **THEN** it cannot produce another result, mutate current state or affect a replacement service

#### Scenario: Unavailable platform capability

- **WHEN** an OS capability is denied or unavailable
- **THEN** shared policy returns the canonical platform-appropriate state/error without fallback product logic in the host

### Requirement: Shared product persistence preserves semantics

The shared service SHALL own settings, saved boards, profiles, private routing associations, schema caches and Tunes. It SHALL preserve canonical validation/defaults, optimistic revisions, identity compatibility, bounded pagination, cache integrity and eviction, immutable session snapshots and separation of private local identifiers from portable board identity. Invalid updates MUST fail atomically with the existing error semantics.

#### Scenario: Conflicting settings or profile update

- **WHEN** two clients update the same product record using its old revision
- **THEN** only the first valid update commits and the other receives the canonical conflict without lost fields or extra revision advancement

#### Scenario: Save and apply a Tune

- **WHEN** a Tune is created, changed or applied
- **THEN** shared code validates its exact package/schema/wire identity, complete values and current revisions before persistence or any board mutation

#### Scenario: Corrupt schema cache

- **WHEN** cached data has the wrong identity, digest or size
- **THEN** the service rejects that cache entry and follows the existing bounded discovery behavior without treating it as authoritative configuration

#### Scenario: Portable board association

- **WHEN** an archive introduces a known board identity without a local Bluetooth association
- **THEN** its history and profile are available while connecting still requires a valid locally discovered association

### Requirement: Shared session catalog and recovery

The shared service SHALL own session creation, catalog summaries, keyset queries, close/retention, recovery, pending deletion and persisted maintenance coordination. The session commit marker SHALL remain authoritative across catalog/session failures. Existing schemas and formats SHALL remain unchanged, and ordinary interruption MUST NOT invoke incompatible-schema deletion. Recovery SHALL be bounded, restartable and complete required reconciliation before exposing affected sessions.

#### Scenario: Catalog update fails after session commit

- **WHEN** a sample commits but the catalog summary update fails
- **THEN** no uncommitted boundary is published and the next permitted read or append reconciles the catalog from the durable marker without duplicating the sample

#### Scenario: Recover interrupted application work

- **WHEN** the process restarts with active-session metadata, a pending deletion or interrupted maintenance work
- **THEN** the service applies the existing recovery and retention rules using bounded scans, preserves committed history and resumes or terminates the persisted work deterministically

#### Scenario: Forget an active board

- **WHEN** a request attempts to forget the actively connected board
- **THEN** it fails without deleting its board, profile, routing metadata or session history

#### Scenario: Delete during a reader or export

- **WHEN** deletion races with a historical reader, export or maintenance lease
- **THEN** existing deferred-delete/cancel semantics protect active resources and final cleanup updates catalog state exactly once

### Requirement: Shared portable lifecycle policy

The shared service SHALL decide discovery expiry, saved/guest activation, board identity, reconnect timing, session attach/close/retention and location capture eligibility from ordered OS facts. Native code SHALL execute the resulting effects. Guest connections MUST NOT create durable session/catalog entries, and saved activation MUST await successful storage preparation before telemetry and ready publication.

#### Scenario: Reconnect with stale timers

- **WHEN** a board disconnects unexpectedly and an old reconnect timer fires after reset or replacement
- **THEN** shared policy enforces the configured retry/backoff budget and the old timer performs no connect, session close or state mutation

#### Scenario: Storage preparation fails

- **WHEN** saved activation cannot create or reconcile its durable session
- **THEN** no ready publication or recording begins and the canonical failure is reported without a partially activated runtime

#### Scenario: Background location eligibility

- **WHEN** ride state, settings, permissions or OS background capability change
- **THEN** shared policy starts or stops permitted capture, rejects stale observations, assigns the session clock and publishes accepted locations only after durable commit

### Requirement: Shared archive and export behavior

The shared service SHALL own archive validation/import, manifest generation, CSV export, staging and cleanup. It SHALL preserve existing file formats, board identity, session metadata, GPS export policy, CSV columns and matching rules. Native code SHALL only acquire authorized external file access or present service-issued files. File bytes MUST remain bounded streams rather than control envelopes, and client input MUST NOT authorize arbitrary filesystem paths.

#### Scenario: Import a large valid archive

- **WHEN** an authorized archive within the existing 128 MiB bound exceeds the control envelope size
- **THEN** bounded streaming validates and attaches it with the existing local-session and portable-board identity semantics without allocating an archive-sized buffer

#### Scenario: Reject hostile archive input

- **WHEN** an archive has malformed SQLite/schema/metadata, a path escape, or a stale or foreign transfer handle
- **THEN** validation fails without altering existing product records or accessing files outside the authorized root and partial staging is reclaimed

#### Scenario: Export while frontend disconnects

- **WHEN** an OS save/share presentation still owns a generated export and the frontend disconnects
- **THEN** the file remains valid for that presentation and is reclaimed after completion or bounded abandonment without retaining unlimited staging

#### Scenario: Preserve location export policy

- **WHEN** a completed session is exported with GPS export disabled or enabled
- **THEN** shared encoding preserves the existing privacy policy, column shape and nearest-location matching behavior on every host

### Requirement: Bounded shared execution and failure

Every command queue, effect batch, waiter, worker pool, collection, cache, transfer and shutdown wait SHALL have a documented numeric bound enforced before admission or allocation. Existing stricter public limits SHALL remain unchanged. History, archive, recovery and maintenance work MUST NOT wait on UI or obstruct permitted recording through long service locks. Cancellation SHALL release resources only when work has actually ended, and cleanup SHALL remain possible under saturated admission.

#### Scenario: Saturated service

- **WHEN** clients fill command or worker capacity during recording
- **THEN** additional work fails boundedly before side effects, recording continues within its existing guarantees and cancellation/cleanup still progresses

#### Scenario: Stop under contention

- **WHEN** listener stop or service shutdown races with storage locks, an OS completion or archive work
- **THEN** each obeys its documented bound, accepted connected requests receive at most one terminal response and no late completion reaches freed state

#### Scenario: Storage exhaustion

- **WHEN** available storage crosses the existing stop threshold or a write fails
- **THEN** shared policy exposes the canonical failure and preserves already committed data without a host-specific retention decision

### Requirement: Existing hosts prove complete portable ownership

Completion SHALL select the shared service on Apple and Android and delete superseded native stores, portable policy, domain dispatch and domain snapshot projection. A direct Rust test host SHALL exercise the same selected service while implementing only synthetic OS capabilities. Audits SHALL require a net production-source reduction below 49,922 lines, at most 28 workspace members, two existing native binding families and no temporary bridges or permanent native fallback.

#### Scenario: Inspect a future host's responsibilities

- **WHEN** the direct test host runs canonical product and lifecycle scenarios
- **THEN** it provides no settings/profile/Tune/session CRUD, catalog SQL, archive encoding, identity derivation, reconnect policy or product snapshot implementation

#### Scenario: Inspect selected native packages

- **WHEN** source, symbols, dependencies and clean host packages are audited
- **THEN** both host families select one shared service through their existing artifacts and contain no superseded portable implementation
