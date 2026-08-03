# Platform Hosts Specification

## Purpose

Define repository ownership and the minimal Android, iOS, and macOS host responsibilities around the shared frontend and native backends.
## Requirements
### Requirement: Lowercase ownership layout

The repository SHALL place the frontend under `/frontend`, the shared Apple implementation and its iOS/macOS hosts under `/backends/apple`, the Android implementation and host under `/backends/android`, shared backend SQL and immutable protocol fixtures under `/backends`, canonical acceptance under top-level `/conformance`, and supporting firmware under `/tools/vesc-test-device`.

#### Scenario: Repository navigation

- **WHEN** a developer inspects the repository root
- **THEN** lowercase top-level directories identify frontend, backend, conformance, tool, and planning ownership without generic package or shared-code layers

### Requirement: One shared Apple backend

iOS and macOS under `/backends/apple` SHALL use the same Apple backend implementation for application state, Bluetooth, VESC protocol, storage, loopback transport, and contract dispatch.

#### Scenario: Apple behavior change

- **WHEN** shared backend behavior changes
- **THEN** both iOS and macOS receive the same implementation and contract semantics

### Requirement: Thin Apple hosts

The iOS and macOS hosts SHALL contain only application entry, lifecycle, WebView, permission presentation, export presentation, and platform resource wiring not owned by the shared backend.

#### Scenario: WebView replacement

- **WHEN** an Apple host replaces its WebView
- **THEN** it reconnects to the same native backend state without reimplementing backend behavior

### Requirement: One Android application module

Android under `/backends/android` SHALL use one Gradle application module. Backend boundaries SHALL be represented by focused Kotlin packages and source files rather than separate `api-models`, `core`, `protocol`, `logging`, and `bluetooth` Gradle modules.

#### Scenario: Android build

- **WHEN** the Android application is assembled
- **THEN** all required backend and host code is built from the one application module

### Requirement: Native backend ownership

Each native process SHALL own Bluetooth state, application state, background work, durable logging, and the loopback server independently of the WebView. The WebView SHALL own no durable or Bluetooth state.

#### Scenario: Frontend termination

- **WHEN** the WebView process or view is terminated
- **THEN** native backend state remains authoritative and recoverable by a replacement frontend

### Requirement: Minimal dependencies

Platform builds SHALL remove dependencies used only by deleted generation, module boundaries, native UI scaffolding, or duplicate tests, while retaining focused maintained dependencies that replace substantial protocol or server code.

#### Scenario: Dependency audit

- **WHEN** implementation is complete
- **THEN** every remaining direct dependency is used by current build, runtime, or verification code

### Requirement: No generic shared-code directory

Cross-language shared content under `/backends` SHALL be limited to immutable SQL and protocol fixtures. Swift and Kotlin implementation code MUST remain owned by their backend.

#### Scenario: Shared backend resource

- **WHEN** both backends consume the same current SQL schema or protocol fixture
- **THEN** the immutable resource may reside under `/backends` without introducing a cross-language shared-code layer

### Requirement: Physical conformance host seams

Debug physical-conformance builds MAY provide only the endpoint and lifecycle control needed by the host TypeScript runner. Those seams MUST preserve loopback-only transport and MUST use the same production Bluetooth, VESC protocol, application-service, and storage implementations as the application.

#### Scenario: Android physical launch

- **WHEN** conformance installs a Debug Android build on a selected ADB device
- **THEN** a fixed Debug loopback port can be forwarded while the application uses `AndroidBluetoothAdapter` and normal process-owned backend state

#### Scenario: Android physical scenarios reuse one authorized application

- **WHEN** multiple physical conformance scenarios run in one Android suite
- **THEN** the Debug application is built and installed once, permissions and loopback forwarding are prepared once, and each launch resets only app-owned Float View storage before normal backend initialization

#### Scenario: iOS physical launch

- **WHEN** conformance installs and launches a signed Debug iOS build on a selected device
- **THEN** a launch-configured loopback port can be tunneled while the application uses `CoreBluetoothAdapter` and normal native backend state

#### Scenario: iOS physical scenarios reuse one authorized application

- **WHEN** multiple physical conformance scenarios run in one iOS suite
- **THEN** the signed Debug application is installed once, each launch resets only app-owned test storage before backend initialization, and the existing OS Bluetooth authorization is preserved

#### Scenario: iOS physical launch does not own the developer terminal

- **WHEN** conformance launches or terminates the signed Debug iOS application
- **THEN** it uses structured process identifiers without attaching the application console to the invoking TTY, and loopback proxy failures are bounded and reported at the point of failure

#### Scenario: iOS physical build output

- **WHEN** the signed Debug application and lifecycle driver build, install, or execute successfully
- **THEN** Xcode plumbing output remains hidden behind concise timed substep progress, while a failure returns bounded captured diagnostics and cleanup remains independently bounded

#### Scenario: iOS device is still preparing

- **WHEN** Xcode reports that the selected physical device is temporarily busy preparing for development
- **THEN** destination discovery and the total retry count remain bounded, retry progress is visible, and unrelated build, signing, or installation failures are not retried

#### Scenario: iOS lifecycle control

- **WHEN** deterministic physical background coverage requires OS Home and foreground actions
- **THEN** a minimal lifecycle-only XCUITest driver may perform those actions serially without containing contract assertions, VESC behavior, or persistence inspection, and each action completes before conformance proceeds

#### Scenario: macOS physical launch

- **WHEN** conformance launches the macOS test host without software-Bluetooth or seeded-state configuration
- **THEN** readiness is exposed while CoreBluetooth and isolated production backend paths are used

### Requirement: Stable native loopback port registry

Supported native backends SHALL bind only to loopback and SHALL use stable default ports: macOS `11010`, iOS `11011`, and Android `11012`. A newly supported backend MUST receive the next unassigned port beginning at `11013`.

#### Scenario: Supported backend starts

- **WHEN** a macOS, iOS, or Android backend starts
- **THEN** its loopback server listens on its assigned platform port

#### Scenario: Assigned port unavailable

- **WHEN** the assigned loopback address and port cannot be bound
- **THEN** the native host fails startup with an actionable diagnostic and does not silently select another port

#### Scenario: New backend allocation

- **WHEN** another native backend becomes supported
- **THEN** it receives one documented unused port from the consecutive registry beginning at `11013`

### Requirement: Browser-development native host seams

Each native platform SHALL provide only the launch, readiness, forwarding, process lifetime, and native-presentation seams required for host-browser development. Those seams MUST use the production Bluetooth, VESC protocol, application-service, storage, permission, lifecycle, and export implementations.

#### Scenario: macOS browser host

- **WHEN** macOS browser development starts
- **THEN** a native process listens on `127.0.0.1:11010` without creating a WKWebView and retains CoreBluetooth, SQLite, permission, and save-panel ownership

#### Scenario: iOS browser host

- **WHEN** iOS browser development starts for a selected device
- **THEN** the installed application listens on device loopback port `11011` and a monitored `iproxy` exposes host loopback port `11011`

#### Scenario: Android browser host

- **WHEN** Android browser development starts for a selected device
- **THEN** the installed application listens on device loopback port `11012` and ADB exposes host loopback port `11012`

#### Scenario: Production host behavior

- **WHEN** a native application runs outside browser-development orchestration
- **THEN** it continues serving the embedded frontend and owning native state with the same backend contract

### Requirement: Mirrored native connection ownership

The Apple and Android backends SHALL use matching ownership boundaries for application coordination, controller discovery, telemetry polling, Bluetooth stream setup, VESC framing and commands, firmware parsing, exact compatibility mapping, and Refloat protocol families. Apple SHALL place application owners under `backends/apple/Sources/FloatViewBackend/application`, VESC protocol owners under `protocol/vesc`, and Refloat owners under `protocol/refloat`. Android SHALL retain its established application owners under `backends/android/app/src/main/java/fail/acheron/floatview/backend` and place corresponding VESC and Refloat owners under `protocol/vesc` and `protocol/refloat`. Swift and Kotlin implementations MUST remain backend-owned, dependencies MUST point downward from application coordination to Bluetooth and protocol components, and superseded monolithic paths MUST be deleted after extraction.

#### Scenario: Locate controller discovery

- **WHEN** a developer needs to change direct-or-CAN route selection
- **THEN** each native backend has one clearly named controller-discovery owner rather than route decisions spread through transport, polling, and view code

#### Scenario: Locate a protocol family

- **WHEN** a developer needs to change VESC framing or a Refloat wire layout
- **THEN** the matching Swift and Kotlin file is found under the corresponding `protocol/vesc` or `protocol/refloat` subtree rather than a version-named application-service branch

#### Scenario: Add a compatible firmware version

- **WHEN** a later VESC or Refloat release uses an existing tested wire family
- **THEN** each backend adds a compatibility-catalog entry and fixtures without copying application-service, routing, or polling code

#### Scenario: Add a new wire family

- **WHEN** a later release changes a required wire layout
- **THEN** each backend adds or changes the focused codec family and maps exact versions to it while shared framing and routing remain unchanged

#### Scenario: Backend source documentation

- **WHEN** a developer reads the backend ownership documentation
- **THEN** it identifies the mirrored Swift/Kotlin locations, dependency direction, privacy boundary, and version-to-family extension rule

### Requirement: Mirrored detailed telemetry ownership

Apple and Android SHALL retain mirrored owners for Refloat family decoding, telemetry scheduling, semantic normalization, application state, durable persistence, session retrieval, and serialization. VESC command routing SHALL remain below Refloat polling, and neither protocol owner SHALL depend on Bluetooth, storage, loopback transport, or frontend code.

#### Scenario: Add a known telemetry field

- **WHEN** support for an existing negotiated Refloat field is added
- **THEN** the matching Swift and Kotlin decoder, normalized model, persistence mapping, serialization, fixtures, and tests are updated without adding platform-specific frontend logic

#### Scenario: Route ownership

- **WHEN** a detailed Refloat request targets a CAN controller
- **THEN** the Refloat owner supplies the command and the VESC routing owner applies CAN forwarding exactly once

### Requirement: Equivalent native detailed persistence

The Apple and Android current schemas SHALL durably store every stable numeric telemetry value in the matching dedicated nullable sample column, unknown negotiated numeric values in bounded `extension_json`, observed field presence in a bounded per-session table, and discrete transitions in `session_event` with an independent monotonic event sequence, source sample sequence, observation time, elapsed time, and single-session ownership. Multiple transitions from one sample SHALL be inserted in stable catalog order. Session metadata SHALL capture its positive bounded riding and idle persistence intervals. The per-session commit marker SHALL atomically record committed sample/event boundaries, recorded/stored counts, history revision, elapsed coverage, commit time, and the bounded riding/distance/maximum/source-sequence/last-value projection required to recover catalog summaries with those rows. The schema SHALL initialize incremental auto-vacuum and bounded prune metadata/job state. Denormalized catalog summaries SHALL converge before a boundary is published and SHALL be reconciled from the marker after interruption or an in-process catalog failure before that session is read or appended again. Incompatible pre-release databases SHALL be deleted and recreated as the canonical first production schema rather than migrated.

#### Scenario: Equivalent stored sample

- **WHEN** both native backends accept equivalent normalized complete-catalog telemetry
- **THEN** dedicated columns, field presence, session-data retrieval, and export produce semantically equivalent numeric records and transitions despite native implementation differences

#### Scenario: Atomic session marker

- **WHEN** a batch introduces samples, multiple transition events, and a late extension field
- **THEN** those rows and the corresponding sample/event sequences, source evidence, recorded/stored counts, history revision, elapsed boundary, field presence, and recovery summary projection become durable in one session-database transaction

#### Scenario: Old disposable schema

- **WHEN** development storage contains the previous `detail_json` session layout or another incompatible version
- **THEN** normal test-stage recreation installs the complete optimized schema without migration, legacy reads, partial columns, or backfill

### Requirement: Native lifecycle owns detailed polling

Detailed realtime polling, the one-second compact due time, normalization, and durable commits SHALL remain native when the WebView is absent or recreated. Each host SHALL retain one in-flight request and no separate telemetry work queue. Background and reconnect behavior SHALL follow the existing platform lifecycle rules and prevent stale responses or a replacement controller from contributing detailed samples or events to an active session.

#### Scenario: WebView replacement

- **WHEN** the frontend disappears while native detailed telemetry continues
- **THEN** accepted samples remain durably ordered and a replacement frontend recovers the latest committed sample

#### Scenario: Controller changes

- **WHEN** reconnect validation reports a replacement controller
- **THEN** detailed polling stops, the old session ends, route-scoped compact state is cleared, and no replacement-controller fields or events are appended to the old session

### Requirement: Native standalone session catalog and grouping removal

Apple and Android SHALL expose the same bounded `session.list` control operation over their existing native session stores. The query SHALL use the catalog's `(started_at_ms, session_id)` keyset order, return at most the requested page size, encode the query shape/catalog revision/last-key context in its opaque cursor, and avoid offset scans. Complete state snapshot construction SHALL not query or serialize session summaries during telemetry-driven publication; it SHALL expose only the bounded session-catalog revision marker. The canonical schema, settings store, application service, loopback handlers, and mock SHALL contain no session-group tables, group-member tables, automatic-linking fields, group mutation ports, or linked-ride payloads. Since storage remains disposable test-stage data, incompatible group-bearing schemas SHALL be recreated without migration.

#### Scenario: Indexed session page

- **WHEN** both hosts serve successive valid `session.list` requests over a catalog larger than one page
- **THEN** both use the checked started-time/session-ID keyset order, return identical standalone summaries and continuation behavior, and do not use offset pagination

#### Scenario: Catalog query absent from telemetry snapshot

- **WHEN** a persisted telemetry sample invalidates application state
- **THEN** neither host queries or serializes a session-summary or group collection for that state event, and the unchanged session-catalog revision remains available

#### Scenario: Grouping removed from native surfaces

- **WHEN** a backend is built from the canonical disposable schema and exposes settings or control methods
- **THEN** group tables, linked-ride records, automatic-linking options, and group/unlink operations are absent on Apple and Android

#### Scenario: Cursor context mismatch

- **WHEN** a session-list cursor is malformed, rebound to another query shape, or issued before a newer catalog revision
- **THEN** both hosts reject it with the same bounded error and do not return a mixed page

### Requirement: Mirrored native session-data adapters

Apple and Android SHALL implement the same authenticated session-data request kinds with small backend-owned adapters over their production SQLite session stores. Each adapter SHALL validate the shared hand-written contract, acquire the requested published history revision and boundary, select only validated columns/ranges plus internal ride phase, stream ordered rows through an equivalent bounded configurable-gap first/minimum/maximum/last reducer, serialize the same explicit columnar response, and apply standard gzip encoding using platform facilities. Swift and Kotlin implementation code SHALL remain backend-owned; shared behavior SHALL be enforced through the contract, canonical SQL, immutable vectors, mock, and conformance rather than a generated or cross-language runtime.

#### Scenario: Equivalent historical query

- **WHEN** Apple and Android query equivalent stored rows with the same series, samples, or events request
- **THEN** they return semantically identical snapshots, columnar values, extrema, continuity segments, ordering, and continuation behavior

#### Scenario: Platform compression

- **WHEN** the frontend accepts gzip from either host
- **THEN** each host uses maintained platform facilities and returns the same decoded contract without a shared runtime dependency

#### Scenario: Backend ownership

- **WHEN** the WebView is destroyed, recreated, or aborts a query
- **THEN** native storage, active logging, snapshot publication, cancellation, recovery, and resource cleanup remain owned by the host backend

### Requirement: Historical reads do not obstruct persistence

Each host SHALL resolve session metadata under only a short store synchronization boundary, then use a dedicated read-only/query-only connection, one per-session read lease, and an explicit bounded WAL read transaction for the scan. Historical work SHALL execute away from the loopback and session-writer executors, SHALL allow no more than two concurrent reads per backend, SHALL remain cancellable throughout query/reduction/serialization, and SHALL release cursors, transactions, connections, reducers, and compression buffers after success, cancellation, disconnect, or serialization failure. Apple SHALL provide connection-local SQLite interruption/progress cancellation and Android SHALL provide database cancellation tied to coroutine/request lifetime. Releasing cancelled work within 250 milliseconds and ordinary completion within two seconds on the reference devices SHALL be recorded performance targets, not correctness deadlines.

#### Scenario: Review an active long session

- **WHEN** a historical overview scans an active 150,000-sample session while another sample becomes due for persistence
- **THEN** the durable append remains within its existing bound and the query observes only its requested published snapshot

#### Scenario: Superseded zoom request

- **WHEN** repeated pan or pinch interactions cancel an older native query
- **THEN** the host requests immediate interruption, exposes no stale result, eventually releases every resource without accumulating jobs or silently pinning WAL, and records whether the cancellation-release target was met

#### Scenario: Loopback remains responsive

- **WHEN** a long query, gzip encoding, or client backpressure is active
- **THEN** control-plane requests and authoritative state publication do not wait for that historical work's executor

### Requirement: Historical readers coordinate with recovery, export, pruning, and deletion

Opening a historical request SHALL acquire a lease only while the catalog row is readable and not pending maintenance or deletion. Deletion and pruning SHALL prevent new leases before requesting cancellation from existing leases and SHALL mutate or remove SQLite files only after every reader releases them. If release is slow, the operation SHALL remain visibly pending and cancellable instead of deleting beneath an open connection or relying on a correctness timeout. Pending deletion cancellation SHALL restore readability only when it wins before irreversible file removal; otherwise deletion SHALL report its stable completed result. Export, compaction, and recovery SHALL use the same file-lifecycle boundary wherever their snapshot or file operations could race a reader. Startup SHALL perform bounded integrity checks and reconcile catalog counts/boundaries/revisions from the authoritative marker before any recovered session can be queried.

#### Scenario: Delete during a historical read

- **WHEN** deletion begins while a completed-session query holds a read lease
- **THEN** no new query opens, cancellation is requested from the existing query, and files are removed only after its connection closes even when release exceeds the performance target

#### Scenario: Interrupted catalog update

- **WHEN** startup finds a valid session marker newer than the denormalized catalog
- **THEN** recovery repairs the catalog before metadata or history exposes the session

#### Scenario: Catalog failure without process exit

- **WHEN** the session marker commits and the following catalog update fails
- **THEN** that session remains unpublished and dirty until marker reconciliation succeeds, and another append does not proceed against divergent metadata

#### Scenario: Failed reader cleanup

- **WHEN** a query fails after opening its WAL snapshot but before completing gzip output
- **THEN** a later append, export, deletion, and historical query all remain usable without leaked leases or SQLite resources

### Requirement: Indexed field-minimized historical access

Each host SHALL map every stable contract field to its dedicated SQL column and SHALL parse `extension_json` at most once per row only when requested extensions are present. Series scans, keyset sample/event pages, and exact predecessor/successor seeks SHALL constrain the published sequence and elapsed range, preserve `(elapsedMs, sequence)` order, and use representative checked query plans without an avoidable full-table sort. Metadata SHALL obtain available fields from bounded field-presence rows rather than scanning samples.

#### Scenario: Stable field query

- **WHEN** a series request asks for two stable fields from a complete-catalog session
- **THEN** each host selects those two numeric columns plus ordering fields without decoding stable per-row JSON or unrequested telemetry

#### Scenario: Extension query

- **WHEN** a request asks for several valid extension fields
- **THEN** each selected row's bounded extension object is decoded no more than once and only the requested values reach the reducer

#### Scenario: Indexed exact lookup

- **WHEN** a nearest-sample tooltip request targets the middle of a long session
- **THEN** the host compares indexed exact, predecessor, and successor candidates rather than sorting the full range by absolute distance

### Requirement: Mirrored native ride pruning

Each host SHALL implement the same cancellable one-ride and all-completed-rides prune operations over an exclusive per-session maintenance lease. The selector SHALL retain field-presence and semantic source samples plus no more than eight ordinary real source rows per elapsed-zero-aligned bucket using the shared deterministic coverage rule. It SHALL stream one bucket with bounded native memory and place retained sequences in an indexed file-backed SQLite keep-set with bounded cleanup rather than an in-memory ride-sized collection. A per-ride logical prune SHALL be transactional, increment history revision, preserve original available fields/summaries/events/recorded count, update stored count and policy, and invalidate older readers and exports. Batch progress SHALL use a single bounded persisted job, a captured upper session key, and stable keyset traversal over rides completed at job start; it SHALL resume safely after process interruption and SHALL be per-ride atomic. A cancellable WAL checkpoint and incremental compaction SHALL be resumable and SHALL NOT make a committed logical prune appear failed solely because all free pages have not yet been reclaimed.

#### Scenario: Equivalent retained rows

- **WHEN** Apple and Android prune equivalent complete-catalog fixtures with the same target resolution
- **THEN** they retain identical source sequences, protected evidence, counts, policy, and next history revision

#### Scenario: Cancel during one ride

- **WHEN** cancellation is observed before a ride's logical prune commits
- **THEN** its transaction rolls back, all maintenance resources eventually release, and the prior history revision remains queryable

#### Scenario: Resume interrupted batch

- **WHEN** the process exits after one batch ride commits and before the next ride completes
- **THEN** startup uses marker revision and persisted keyset progress to avoid double-pruning, then resumes or terminates the bounded job deterministically

#### Scenario: Interrupted compaction

- **WHEN** logical pruning commits but incremental vacuum is cancelled or interrupted
- **THEN** retained history and its new revision remain valid while bounded state records reclamation pending for a later compaction attempt

#### Scenario: Pathological prune memory

- **WHEN** a ride contains far more raw rows than the target keep-set
- **THEN** native selector memory remains bounded by one bucket and field limits, temporary SQLite storage is cleaned after success/cancellation/failure, and no raw-sequence collection is retained in memory
