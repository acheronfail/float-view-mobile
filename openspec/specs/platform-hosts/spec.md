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

The iOS and macOS hosts SHALL occupy the explicit sibling roots `/backends/apple/ios` and `/backends/apple/macos`. Those hosts SHALL contain only application entry, lifecycle, WebView, permission presentation, export presentation, platform resources, and other platform wiring not owned by the shared backend. Shared Apple behavior SHALL remain outside both host roots in the `FloatViewBackend` library.

#### Scenario: Locate an Apple platform host

- **WHEN** a developer inspects `/backends/apple`
- **THEN** the iOS and macOS application hosts are found at the sibling `ios/` and `macos/` roots while shared backend code remains separately identified

#### Scenario: WebView replacement

- **WHEN** an Apple host replaces its WebView
- **THEN** it reconnects to the same native backend state without reimplementing backend behavior

#### Scenario: Build the macOS sibling host

- **WHEN** SwiftPM builds the `FloatViewMac` executable
- **THEN** it compiles the host and copies its resources from `/backends/apple/macos` without recreating a platform host under `Sources`

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

Cross-language shared content under `/backends` SHALL be limited to immutable SQL and protocol fixtures plus bounded portable Rust components under `/backends/core` governed by `native-core-migration`. A Rust candidate MUST name one behavioral slice and MUST NOT become a generic shared-code layer, generated API surface, or production owner before an explicit host cutover. Swift and Kotlin implementation code SHALL remain owned by their backend until each host's verified cutover deletes the corresponding superseded implementation.

#### Scenario: Shared backend resource

- **WHEN** both backends consume the same current SQL schema or protocol fixture
- **THEN** the immutable resource may reside under `/backends` without introducing a cross-language shared-code layer

#### Scenario: Bounded Rust candidate

- **WHEN** an active OpenSpec change extracts one portable component under `/backends/core`
- **THEN** repository verification permits only that named candidate and preserves native production ownership until separately verified host cutovers

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

The Apple and Android backends SHALL use the same capability-oriented source vocabulary for `application`, `bluetooth`, `files`, `lifecycle`, `location`, `protocol`, `storage`, `theme`, `transport`, and `ui` ownership beneath their native build-system source roots. Protocol code SHALL retain matching `protocol/vesc` and `protocol/refloat` subtrees while a platform owns those implementations. Apple MAY divide those capabilities among its shared backend, iOS host, and macOS host targets, while Android SHALL retain one Gradle application module; a target that owns a capability MUST use the canonical capability name rather than an ambiguous root source or platform-specific synonym.

Swift and Kotlin SHALL remain the production owners until a bounded Rust migration change explicitly cuts over a named host. A focused candidate implementation MAY coexist under `backends/core` without transferring production ownership. A host-cutover change MAY introduce one narrow audited bridge only when it names the selected Rust component, platform packaging, complete verification, and superseded-code deletion checkpoint; generated general-purpose API layers and modules per capability remain prohibited. Dependencies SHALL point downward from host/UI composition and lifecycle, through transport and application coordination, to Bluetooth, location, storage, and protocol owners. Bluetooth and protocol components MUST continue to emit typed events without depending on WebViews or loopback transport. Native Bluetooth, location, file presentation, background execution, theme synchronization, permission handling, and UI integration SHALL remain owned by the platform implementation.

#### Scenario: Locate a native capability

- **WHEN** a maintainer needs to change Bluetooth, location, file import/export, background lifecycle, theme, loopback transport, application coordination, storage, VESC protocol, or Refloat protocol behavior
- **THEN** each backend exposes the production implementation under the corresponding canonical capability path beneath its native source root, or its ownership documentation points to the explicitly cut-over portable component beneath that path's platform adapter

#### Scenario: Locate controller discovery

- **WHEN** a developer needs to change direct-or-CAN route selection
- **THEN** each native backend has one clearly named controller-discovery owner under `application` rather than route decisions spread through transport, polling, and view code

#### Scenario: Locate a protocol family

- **WHEN** a developer needs to change VESC framing or a Refloat wire layout
- **THEN** the selected production owner is found from the corresponding `protocol/vesc` or `protocol/refloat` subtree and any portable implementation is found under one focused `backends/core` component rather than a version-named application-service branch

#### Scenario: Locate platform file handling

- **WHEN** a developer needs to change session archive selection, bounded native import reading, save-panel presentation, or share-sheet presentation
- **THEN** the platform integration is found under `files` and storage archive validation remains under `storage`

#### Scenario: Locate background logging ownership

- **WHEN** a developer needs to change behavior after a background-capable mobile application leaves the foreground
- **THEN** process/application lifetime and background-service code is found under `lifecycle` while BLE remains under `bluetooth` and optional location capture remains under `location`

#### Scenario: Locate theme synchronization

- **WHEN** a developer needs to change how a native host responds to authoritative frontend theme settings or a system appearance change
- **THEN** native appearance synchronization is found under `theme` and the disposable WebView remains under `ui`

#### Scenario: Add a compatible firmware version

- **WHEN** a later VESC or Refloat release uses an existing tested wire family
- **THEN** each production backend adds a compatibility-catalog entry and fixtures without copying application-service, routing, or polling code

#### Scenario: Add a new wire family

- **WHEN** a later release changes a required wire layout
- **THEN** each production backend adds or changes the focused codec family and maps exact versions to it while shared framing and routing remain unchanged

#### Scenario: Candidate extraction

- **WHEN** a bounded Rust component has parity evidence but no host bridge
- **THEN** Swift and Kotlin remain the production owners and the Rust component is documented as a candidate only

#### Scenario: Platform-specific cutover

- **WHEN** one host selects a Rust component through its audited bridge
- **THEN** only that host changes production ownership after its complete gate passes, while OS-owned capabilities remain in the platform source tree

#### Scenario: Backend source documentation

- **WHEN** a developer reads the backend ownership documentation
- **THEN** it identifies the canonical Swift/Kotlin capability locations, any explicitly cut-over `backends/core` owner, native source-root differences, dependency direction, privacy boundary, and version-to-family extension rule

#### Scenario: Layout drift

- **WHEN** production code is added under a superseded or ambiguous backend source location
- **THEN** normal repository verification fails with the unexpected location

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

Each host SHALL map every stable contract field to its dedicated SQL column and SHALL parse `extension_json` at most once per row only when requested extensions are present. Series scans and keyset sample/event pages SHALL constrain the published sequence and elapsed range, preserve `(elapsedMs, sequence)` order, and use representative checked query plans without an avoidable full-table sort. Metadata SHALL obtain available fields from bounded field-presence rows rather than scanning samples.

#### Scenario: Stable field query

- **WHEN** a series request asks for two stable fields from a complete-catalog session
- **THEN** each host selects those two numeric columns plus ordering fields without decoding stable per-row JSON or unrequested telemetry

#### Scenario: Extension query

- **WHEN** a request asks for several valid extension fields
- **THEN** each selected row's bounded extension object is decoded no more than once and only the requested values reach the reducer

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

### Requirement: Mirrored native location ownership

Apple and Android SHALL implement equivalent application-service location ownership, permission mapping, session-clock assignment, durable append, bounded reads, export, and lifecycle behavior. Location adapters SHALL not depend on WebSocket, Svelte, or browser geolocation code.

#### Scenario: Native ownership

- **WHEN** the frontend WebView is destroyed or replaced
- **THEN** the native application service remains the sole owner of location capture and the active session's recorder state

#### Scenario: Cross-platform result

- **WHEN** equivalent permission and provider fixtures are supplied to Apple and Android
- **THEN** both backends expose the same contract status, row shape, boundary semantics, and stable errors

### Requirement: Android background location service

Android SHALL extend the existing active-session foreground logging service with the required location foreground-service declaration, notification behavior, runtime permission flow, and settings fallback. Location capture SHALL remain tied to the authoritative active session and SHALL not create an independent logging service.

#### Scenario: Android background ride

- **WHEN** an authorized location-enabled board in `riding` is backgrounded
- **THEN** the existing foreground service remains eligible to receive and persist location updates and the next frontend connection recovers its state

#### Scenario: Android missing background authorization

- **WHEN** foreground location is allowed but background capability is not granted
- **THEN** the backend reports the bounded capability state and offers the Android settings path without stopping board telemetry logging

### Requirement: Apple Core Location lifecycle

The Apple backend SHALL use Core Location through the shared application service, declare the required iOS usage/background configuration, and map iOS/macOS authorization changes into the common contract. The iOS UI-server lifecycle SHALL not stop the native recorder when the app is backgrounded.

#### Scenario: iOS background capture

- **WHEN** an authorized location-enabled iOS ride enters the background
- **THEN** Core Location remains configured for the permitted background mode and native persistence continues independently of the WebView server

#### Scenario: iOS foreground-only grant

- **WHEN** an iOS ride has only foreground location authorization and enters the background
- **THEN** the Apple backend reports background capture unavailable while board logging continues

#### Scenario: macOS authorization

- **WHEN** macOS reports a location authorization change
- **THEN** the shared Apple service publishes the corresponding bounded status and starts/stops capture according to settings and session state

### Requirement: Non-browser mock seam

Mock and browser-development hosts SHALL expose deterministic location permission/status fixtures and session-data rows through the native contract seam. They SHALL not request browser geolocation or create platform-specific frontend permission branches.

#### Scenario: Deterministic mock

- **WHEN** conformance runs against the mock backend with location fixtures enabled
- **THEN** it receives repeatable permission transitions, location rows, gaps, and boundaries through the same contract as native targets

### Requirement: Mirrored saved and guest tracking policy

Apple and Android SHALL implement one explicit native tracking-mode policy with equivalent behavior. The policy SHALL be selected before connection work begins, retained across reconnect and WebView recreation, and checked at every durable side-effect boundary including saved-device creation, Board Profile writes, controller preference or learned-hint writes, session creation/append/deletion, location recording, catalog revision changes, and exportable ride metadata. Saved mode SHALL use the existing durable owners; guest mode SHALL use bounded in-memory live state and perform none of those durable writes. Swift and Kotlin SHALL not rely on frontend omission as the isolation mechanism.

#### Scenario: Saved native connection

- **WHEN** both native hosts receive saved connection intent
- **THEN** both create or use the saved board, run the existing durable session pipeline, and expose equivalent catalog and live state behavior

#### Scenario: Guest native connection

- **WHEN** both native hosts receive guest connection intent
- **THEN** both expose live telemetry through the same state contract while creating no saved device, profile write, controller hint, session row, location row, catalog revision, or durable ride metadata

#### Scenario: Guest reconnect and recreation

- **WHEN** a guest connection reconnects or its WebView is recreated
- **THEN** both hosts retain guest mode and bounded live state without promoting any in-memory data to persistence

#### Scenario: Native policy coverage

- **WHEN** a future durable write is added to the connection or telemetry lifecycle
- **THEN** it has one explicit saved/guest policy decision and cannot be reached from guest mode accidentally

### Requirement: Native board history projection

Apple and Android SHALL expose equivalent board-scoped session pages and bounded board summary projections from the existing saved-device/session catalog. Summary calculation SHALL use existing catalog/session data or bounded queries and SHALL not add a second aggregate persistence model. Guest activity SHALL be absent from every board projection.

#### Scenario: Equivalent board history

- **WHEN** both hosts contain equivalent sessions for two saved boards
- **THEN** each returns the same board-filtered ordering, summary values, cursor behavior, and catalog revision semantics

#### Scenario: Board deletion or pruning

- **WHEN** a session is deleted or pruned
- **THEN** the owning board's next summary and history query reflect the authoritative catalog without stale guest or deleted rows

#### Scenario: Empty board history

- **WHEN** a saved board has no retained sessions
- **THEN** both hosts return a bounded empty board history and zero-or-unknown summary values according to the shared contract, without creating placeholder sessions

### Requirement: Native board identity, archive, and forget parity

Apple and Android SHALL derive the same versioned stable opaque board identity from the same meaningful VESC identity, keep raw firmware and local platform identifiers out of portable archives, and allow imported boards to exist with a nullable local platform association. Archive import SHALL merge sessions into a matching `deviceHistoryId` or create a configurable board entry before attaching them. Both hosts SHALL implement `savedDevice.delete` with the same all-owned-history cleanup and SHALL reject deletion while that board is actively connected.

#### Scenario: Cross-platform board identity

- **WHEN** equivalent VESC identity data is observed by Apple and Android
- **THEN** both hosts resolve the same stable opaque board identity without exposing the raw identifier

#### Scenario: Imported board without discovery

- **WHEN** either host imports an archive for a board with no local platform association
- **THEN** the board appears in Sessions, remains configurable, and does not become connectable until Home discovers and explicitly saves a matching board

#### Scenario: Forget parity

- **WHEN** an inactive board is forgotten on either host
- **THEN** the board, profile, private routing metadata, and all sessions owned by its history identity are removed with equivalent catalog revision behavior

### Requirement: Mirrored native BMS ownership

Apple and Android SHALL implement matching owners for connection-wide VESC request scheduling, common and hardware-specific firmware identity parsing, auxiliary-node discovery, bounded BMS values decoding, and slow BMS polling. Application coordination SHALL remain above protocol routing and Bluetooth, and no BMS protocol owner SHALL depend on WebSocket, frontend, persistence, or view code.

#### Scenario: Cross-platform source layout

- **WHEN** a maintainer traces BMS discovery or decoding on either backend
- **THEN** the matching Swift and Kotlin owner is available under the established application/backend and `protocol/vesc` trees with equivalent bounds and state transitions

#### Scenario: One write owner

- **WHEN** controller and BMS work both need the VESC stream
- **THEN** each platform routes typed requests through its one connection-wide scheduler rather than writing from independent polling timers

#### Scenario: Hand-written contract mapping

- **WHEN** native BMS state is serialized for the frontend
- **THEN** Apple and Android explicitly map every bounded contract field with equivalent null, unit, array, and status semantics and no generated API layer

### Requirement: Native lifecycle and privacy boundary

Native services SHALL continue auxiliary discovery and BMS polling while the frontend is disposable and while background execution is permitted, cancel every BMS generation and queued request on connection replacement, and exclude BMS UUIDs and full BMS telemetry from default logs and durable files.

#### Scenario: WebView disposal

- **WHEN** the frontend is destroyed while a selected BMS remains attached
- **THEN** native polling continues where the OS permits and a replacement frontend recovers the latest authoritative BMS state

#### Scenario: Connection generation changes

- **WHEN** disconnect, reconnect, preference change, or device replacement invalidates auxiliary work
- **THEN** late callbacks and queued work from the older generation cannot select a route or publish a snapshot

#### Scenario: Default diagnostics

- **WHEN** BMS discovery, polling, timeout, or malformed input is diagnosed normally
- **THEN** logs retain only bounded high-level route/status codes and do not contain UUIDs, full cell arrays, or complete raw telemetry frames

### Requirement: Mirrored native Refloat configuration ownership

Apple and Android SHALL add matching owners for custom-schema chunk assembly, bounded Qt/zlib and XML decoding, normalized schema validation, CRC32C signature construction, typed value encoding/decoding, configuration operation lifecycle, and exact-version saved-configuration persistence. Protocol owners SHALL depend only on VESC framing/routing and configuration models; one application-level mutation coordinator SHALL own authorization, exclusivity, and scheduler submission for semantic controls, configuration writes, and Tune apply; no feature owner may retain a parallel write path. Storage SHALL own schema cache and Tunes; loopback serialization SHALL expose only normalized bounded contract values. Frontend code MUST NOT parse device XML, compute wire signatures, choose routes/commands, or persist authoritative Tunes.

#### Scenario: Locate schema codec

- **WHEN** a developer changes ConfigParams parsing or a VESC value encoding
- **THEN** corresponding focused Swift and Kotlin protocol owners and shared golden vectors are available under mirrored Refloat/VESC paths

#### Scenario: Disposable WebView

- **WHEN** the frontend is recreated during schema loading or after saved configurations exist
- **THEN** native connection, operation, cache, and durable Tune ownership remain intact and are recovered through authenticated contract methods

#### Scenario: Mirrored flywheel boundary

- **WHEN** either native backend observes flywheel mode
- **THEN** its shared coordinator invalidates configuration current state and rejects every device mutation except eligible flywheel disable with behavior equivalent to the other backend

#### Scenario: Storage schema change

- **WHEN** exact-version saved configurations are introduced during test-stage development
- **THEN** canonical catalog SQL and both native stores change together, the storage version is bumped, and incompatible local storage is recreated rather than migrated

#### Scenario: No new parser dependency

- **WHEN** the platform implementations decode the compressed schema
- **THEN** they use bounded platform XML and zlib facilities unless pre-apply review demonstrates a smaller maintained dependency is necessary and records that decision

### Requirement: Shared Apple Rust frame codec bridge ownership

The shared Apple backend SHALL expose its production Rust VESC frame codec beneath `protocol/vesc` through thin hand-written Swift adapters and one Apple-specific C ABI crate under `backends/core`. Rust SHALL own CRC, encode, exact decode, and every VESC stream boundary decision. Swift SHALL own only bounded caller-side byte storage, mechanical consumption using Rust-returned lengths, and established Swift error mapping. The safe framing crate SHALL remain independent of Apple and unsafe code; the bridge crate SHALL contain only bounded representation conversion and SHALL depend only on the safe framing crate and the Rust standard library.

The versioned bridge SHALL use fixed integer statuses, primitive pointer/length arguments, caller-owned bounded regions, panic containment, and an explicit application-symbol allowlist. Its stream scanner SHALL return only the fixed outcome and validated primitive frame/consumption lengths for borrowed input; complete-frame payload output SHALL continue through the exact-decoder operation. Rust allocator ownership, Rust layouts, callbacks, opaque runtime handles, generated bindings, and unrelated capability APIs MUST NOT cross this boundary. Bluetooth, location, background lifecycle, file presentation, theme synchronization, permissions, transport, application behavior, and UI composition SHALL remain with the existing Apple owners.

#### Scenario: Locate Apple framing ownership

- **WHEN** a maintainer follows the Apple `protocol/vesc` codec and stream adapters
- **THEN** the adapters identify Rust as the production CRC, encode, exact-decode, and stream-boundary owner while Swift contains only caller-owned bounded byte storage

#### Scenario: Share the cutover across Apple hosts

- **WHEN** iOS or macOS compiles the shared `FloatViewBackend`
- **THEN** both sibling hosts link the same versioned frame-codec ABI and use the same Swift adapters

#### Scenario: Scan one borrowed stream region

- **WHEN** Swift asks Rust to inspect its current bounded stream bytes
- **THEN** Rust reports incomplete input, a complete candidate frame, an oversized declaration, or aggregate overflow together with only validated primitive consumption and frame lengths and retains no pointer after the call

#### Scenario: Decode one scanned frame

- **WHEN** the scanner reports a complete candidate frame
- **THEN** Swift mechanically selects the reported region, Rust exact decode decides its validity and payload, and Swift consumes the reported prefix even when exact decoding returns the established malformed-frame error

#### Scenario: Contain a bridge failure

- **WHEN** the bridge receives a null, oversized, overlapping, misaligned, malformed, insufficient-capacity, or internally failed call
- **THEN** it writes nothing outside the supplied bounds, unwinds no panic across C, returns the documented fixed status, and logs no address, token, payload, or full frame

#### Scenario: Reset stream storage

- **WHEN** connection generation reset discards Apple stream state
- **THEN** Swift clears its caller-owned bytes without invoking a Rust destructor or retaining a cross-generation Rust handle

#### Scenario: Preserve OS capability ownership

- **WHEN** Apple selects Rust stream scanning
- **THEN** CoreBluetooth and all other OS-owned capabilities continue through their existing shared or sibling-host adapters without a Rust callback or lifecycle handle

### Requirement: Android Rust frame codec bridge ownership

The Android backend SHALL expose its production Rust VESC frame codec beneath `protocol/vesc` through a thin hand-written Kotlin adapter and one Android-specific JNI bridge crate under `backends/core`. Rust SHALL own CRC, encode, exact decode, and every stream-boundary decision while Kotlin MAY retain only bounded caller-owned stream storage and mechanical slicing and consumption. The safe framing crate SHALL remain independent of Android, JNI, and unsafe code; the bridge crate SHALL contain only bounded JNI representation conversion and SHALL depend only on the safe framing crate, the Rust standard library, and the existing locked minimal JNI ABI binding.

The versioned bridge SHALL use fixed integer statuses, JVM-owned primitive arrays, explicit lengths and capacities, panic containment, and an exact application-symbol allowlist. The stateless scanner operation SHALL return only validated frame offset, frame length, and consumed length values and SHALL retain no input or state after the call. Rust layouts, allocator ownership, callbacks, returned native pointers, opaque handles, generated bindings, and unrelated capability APIs MUST NOT cross this boundary. Bluetooth, location, background lifecycle, file presentation, theme synchronization, permissions, transport, application behavior, storage, and UI composition SHALL remain with the existing Android owners in the single Gradle application module.

#### Scenario: Locate Android framing ownership

- **WHEN** a maintainer follows the Android `protocol/vesc` codec and stream owners
- **THEN** the Kotlin adapter identifies Rust as the production CRC, encode, exact-decode, and stream-boundary owner while Kotlin storage and mechanical consumption remain plainly identified

#### Scenario: Cross one bounded JNI codec call

- **WHEN** Kotlin requests frame encoding or exact decoding
- **THEN** it supplies bounded JVM-owned input, output, and output-length arrays and receives a fixed status without transferring native allocator ownership or retaining a native reference

#### Scenario: Cross one bounded JNI scanner call

- **WHEN** Kotlin supplies its current bounded notification bytes for stream scanning
- **THEN** Rust returns a fixed status and validated caller-owned primitive frame and consumption lengths without retaining the array, allocating returned memory, or creating native stream state

#### Scenario: Contain a JNI bridge failure

- **WHEN** the bridge receives a null, oversized, aliased, malformed, wrong-shaped, insufficient-capacity, pending-exception, or internally failed call
- **THEN** it writes nothing outside the supplied arrays, unwinds no panic across JNI, returns the documented fixed failure, and logs no address, token, payload, or full frame

#### Scenario: Keep one Android application module

- **WHEN** Gradle builds the Android host with complete Rust framing selected
- **THEN** the generated native libraries are packaged by the existing `app` module without adding a native API module, generated model layer, or general shared runtime

#### Scenario: Preserve OS capability ownership

- **WHEN** Android selects Rust stream boundary scanning
- **THEN** BLE, background logging, location, file presentation, theme, lifecycle, permissions, and WebView integration continue through their existing Kotlin and Android framework owners without a Rust callback

### Requirement: Rust firmware identity candidate ownership

The safe Rust firmware identity component SHALL live at `backends/core/vesc-firmware-identity` and SHALL remain independent of host bridges, framing, host frameworks, Bluetooth, application coordination, persistence, transport, and frontend code. The shared Apple backend SHALL select it through one focused Apple bridge beneath `protocol/vesc`; Android SHALL continue to locate its selected Kotlin firmware identity decoder beneath its own `protocol/vesc` tree until a separate Android cutover passes focused, packaging, canonical, and physical verification and deletes that superseded parser.

#### Scenario: Locate component and production owners

- **WHEN** a maintainer traces VESC firmware identity decoding after the Apple cutover
- **THEN** the safe component and Apple bridge are plainly identified under `backends/core`, iOS and macOS share a thin adapter under Apple `protocol/vesc`, and Android's selected Kotlin owner remains under Android `protocol/vesc`

#### Scenario: Reject accidental cross-host integration

- **WHEN** repository verification inspects bridge symbols, package dependencies, application sources, and generated artifacts
- **THEN** Apple links only its focused identity bridge, Android links no identity Rust artifact, and neither host gains generated bindings, callbacks, opaque handles, or unrelated production calls

#### Scenario: Preserve OS capability ownership

- **WHEN** Apple selects Rust firmware identity decoding
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, storage, and loopback transport remain unchanged in their existing platform owners

### Requirement: Shared Apple Rust firmware identity bridge ownership

The shared Apple backend SHALL expose production Rust firmware identity decoding through a thin hand-written Swift adapter and one Apple-specific bridge crate under `backends/core`. The bridge SHALL depend only on the safe identity component and standard library and SHALL use a hand-written C module, fixed version/status values, bounded borrowed input, a caller-owned versioned byte record, explicit lengths, panic containment, and an exact two-symbol application allowlist. Rust allocations, layouts, references, callbacks, opaque handles, generated bindings, payload logs, names, UUID diagnostics, and unrelated capabilities MUST NOT cross the boundary.

The bridge SHALL build from locked offline source for macOS, iOS device, and active iOS simulator targets and SHALL be linked by both sibling Apple hosts without committing a native archive. The existing framing bridge SHALL retain its separate crate, header, four-symbol allowlist, and ownership unchanged.

#### Scenario: Cross one bounded identity call

- **WHEN** Swift supplies one firmware payload and caller-owned result region
- **THEN** Rust validates the regions, decodes the identity, writes only the documented bounded record, returns a fixed status, and retains no pointer or allocation

#### Scenario: Map one successful record

- **WHEN** the bridge returns success
- **THEN** Swift validates the exact record version/flags/lengths and mechanically constructs `FirmwareInfo` without interpreting VESC payload structure

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives null, oversized, overlapping, insufficient-capacity, malformed, ABI-incompatible, or internally failed input
- **THEN** it writes nothing outside supplied bounds, unwinds no panic across C, returns the documented bounded result, and logs no payload or private identity

#### Scenario: Share Apple production selection

- **WHEN** iOS or macOS compiles the shared `FloatViewBackend`
- **THEN** both link the same identity bridge and select the same Swift adapter and safe Rust decoder

#### Scenario: Keep framing independent

- **WHEN** the identity bridge is built and audited
- **THEN** the existing framing bridge and safe framing component retain their current symbols, package graph, and production behavior

#### Scenario: Preserve Android isolation

- **WHEN** Gradle builds Android during or after the Apple cutover
- **THEN** no Apple identity archive, C module, build helper, or Rust identity dependency enters the Android application graph

### Requirement: Android Rust firmware identity bridge ownership

The Android backend SHALL expose production Rust firmware identity decoding beneath `protocol/vesc` through a thin hand-written Kotlin adapter and one Android-specific JNI bridge crate under `backends/core`. The bridge SHALL depend only on the safe identity component, the Rust standard library, and the existing locked minimal JNI ABI binding. It SHALL use fixed version/status values, bounded copied input, caller-owned primitive arrays, a versioned byte record, explicit lengths, pending-exception checks, panic containment, and an exact two-symbol application allowlist.

The safe identity crate SHALL remain independent of Android, JNI, unsafe code, host frameworks, framing, Bluetooth, application coordination, persistence, transport, and frontend code. Rust allocations, layouts, references, strings, callbacks, opaque handles, generated bindings, payload logs, names, UUID diagnostics, and unrelated capabilities MUST NOT cross the boundary.

#### Scenario: Cross one bounded Android identity call

- **WHEN** Kotlin supplies one firmware payload and correctly shaped caller-owned output arrays
- **THEN** Rust validates the JNI regions, copies bounded input, decodes the identity, writes only the documented bounded record and metadata, returns a fixed status, and retains no Java reference or Rust allocation

#### Scenario: Map one successful Android record

- **WHEN** the bridge returns success
- **THEN** Kotlin validates ABI, status, flags, lengths, exact record extent and text before mechanically constructing the existing `FirmwareInfo` without interpreting VESC payload structure

#### Scenario: Contain Android bridge failure

- **WHEN** the bridge receives null, aliased, oversized, wrongly shaped, insufficient-capacity, malformed, ABI-incompatible, exception-pending, or internally failed input
- **THEN** it writes no unvalidated partial model, unwinds no panic across JNI, returns the documented bounded result, and logs no payload or private identity

#### Scenario: Package source-built Android identity

- **WHEN** Gradle builds host tests or Android debug and release applications
- **THEN** it builds the exact locked source-owned identity bridge for the host or all four supported Android ABIs, loads or packages only the expected library, and commits no native binary

#### Scenario: Keep host bridges independent

- **WHEN** the Android identity bridge is built and audited
- **THEN** the existing Android framing bridge and Apple identity bridge retain their separate crates, symbols, packages, dependencies, and production behavior

#### Scenario: Preserve OS capability ownership

- **WHEN** Android selects Rust firmware identity decoding
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, storage, and loopback transport continue through their existing platform owners without a Rust callback

### Requirement: Equivalent native flywheel mutation lifecycle

Apple and Android SHALL retain hand-written but behaviorally equivalent application-service ordering for current-generation Refloat telemetry, mutation confirmation, flywheel-mode cancellation, configuration invalidation, state publication, waiter completion, and bounded timeout. Platform Bluetooth adapters and WebView hosts SHALL remain outside this coordination rule and MUST NOT implement a parallel confirmation path.

#### Scenario: Equivalent enable lifecycle

- **WHEN** either native host receives flywheel-mode telemetry for its pending enable action
- **THEN** its application service confirms through the mutation coordinator and returns one successful complete state

#### Scenario: Equivalent disable lifecycle

- **WHEN** either native host receives repeated flywheel telemetry followed by normal-mode telemetry for its pending disable action
- **THEN** it preserves then confirms the action with the same externally observable state ordering

#### Scenario: Disposable WebView

- **WHEN** the WebView is absent or recreated during either transition
- **THEN** native confirmation, timeout, and safety cleanup continue independently and a later state snapshot exposes the authoritative result

### Requirement: Rust custom-configuration candidate ownership

The focused Rust VESC custom-configuration component SHALL live at `backends/core/vesc-custom-configuration` and SHALL remain independent of framing and identity bridges, host frameworks, schema/XML and typed-value codecs, Bluetooth, routing, scheduling, application coordination, persistence, transport, and frontend code. Apple and Android SHALL select the component through separate focused bridges and thin adapters beneath each host's `protocol/vesc` path only after focused, packaging, canonical, and physical verification deletes that host's superseded native implementation.

#### Scenario: Locate component and production owners

- **WHEN** a maintainer traces commands 92–95 after both host cutovers
- **THEN** the safe component and separate focused host bridges are plainly identified under `backends/core`, while the thin shared-Apple and Android adapters remain beneath each backend's `protocol/vesc` path

#### Scenario: Reject accidental cross-host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** Apple links only its focused C bridge, Android packages only its focused JNI bridge, and neither host links, calls, generates bindings for, or packages the other host's bridge artifact

#### Scenario: Preserve higher-level configuration ownership

- **WHEN** the Rust component encodes or decodes opaque configuration envelopes for host production
- **THEN** schema assembly, decompression, XML, signatures, typed values, safety policy, mutation coordination, scheduling, routing, storage, and contract serialization remain with their existing owners

#### Scenario: Preserve OS capability ownership

- **WHEN** both custom-configuration bridges are built, selected, and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, storage, and loopback transport remain unchanged in their existing platform owners

### Requirement: Shared Apple Rust custom-configuration bridge ownership

The Apple backend SHALL expose the safe custom-configuration component through one dedicated, versioned, stateless C ABI shared by iOS and macOS. The ABI SHALL contain exactly three exported operations: version query, request encode, and response decode. It SHALL use fixed operation, status, and result-record values; caller-owned bounded input and output regions; pairwise region validation; panic containment; payload-safe diagnostics; and no Rust allocator, callback, or opaque lifetime crossing the boundary.

The hand-written Apple C module, locked source-build helper, SwiftPM linkage, and Xcode linkage SHALL build the same bridge for macOS, iOS device, and iOS simulator. No generated binding or native archive SHALL be committed. The existing Apple framing and firmware-identity bridges SHALL remain independent, and Android SHALL neither link nor package this Apple bridge.

#### Scenario: Exercise the bridge contract

- **WHEN** compiled Rust and C checks call every operation with valid and invalid buffers, capacities, regions, operations, payloads, and result records
- **THEN** the exact three-symbol ABI returns only fixed bounded statuses and records, contains panics, and transfers no allocation or lifetime ownership

#### Scenario: Build every Apple target family

- **WHEN** clean Apple packaging builds macOS, iOS device, and iOS simulator products from the repository-pinned source
- **THEN** each product links the correct bridge architecture and shared Swift adapter without a generated binding, committed archive, or host-only substitution

#### Scenario: Keep bridges independent

- **WHEN** maintainers inspect dependencies and exported symbols
- **THEN** custom-configuration depends only on its safe component and standard library and neither changes nor expands framing or firmware-identity bridge ownership

#### Scenario: Keep platform services native

- **WHEN** shared Apple production uses Rust command envelopes
- **THEN** CoreBluetooth, background execution, location, native files, theme, lifecycle, permissions, WebViews, application services, SQLite, and loopback transport remain owned by the Apple host

#### Scenario: Isolate Android

- **WHEN** Android unit, packaging, canonical, and physical checks run during this Apple change
- **THEN** they continue through the unchanged Kotlin codec without resolving an Apple C module, library, or symbol

### Requirement: Android Rust custom-configuration bridge ownership

The Android backend SHALL expose the safe custom-configuration component beneath `protocol/vesc` through one dedicated, versioned, stateless JNI bridge. The JNI surface SHALL contain exactly three exported operations: version query, request encode, and response decode. It SHALL use fixed operation, status, and primitive result values; JVM-owned bounded byte and integer arrays; explicit lengths; alias and pending-exception checks; panic containment; payload-safe diagnostics; and no Rust allocator, callback, returned pointer, or opaque lifetime crossing the boundary.

The locked source-build helper and Gradle wiring SHALL build the same bridge for host JVM tests and all four supported Android ABIs in debug and release. No generated binding or native library SHALL be committed. The existing Android framing and firmware-identity bridges and all Apple bridges SHALL remain independent.

#### Scenario: Exercise the Android bridge contract

- **WHEN** focused Rust and JVM checks call every operation with valid and invalid arrays, shapes, aliases, capacities, operations, payloads, exceptions, and result metadata
- **THEN** the exact three-symbol JNI ABI returns only fixed bounded statuses and primitive records, contains panics, and transfers no allocation or lifetime ownership

#### Scenario: Build every Android target ABI

- **WHEN** clean Android packaging builds host tests plus debug and release applications
- **THEN** the source-owned bridge is loaded for JVM tests and packaged for arm64-v8a, armeabi-v7a, x86, and x86_64 without a generated binding, committed library, or host-only substitution

#### Scenario: Keep Android bridges independent

- **WHEN** maintainers inspect dependencies, libraries, and exported symbols
- **THEN** custom configuration depends only on its safe component, standard library, and locked minimal JNI binding and neither changes nor expands framing or firmware-identity bridge ownership

#### Scenario: Keep Android platform services native

- **WHEN** Android production uses Rust command envelopes
- **THEN** Bluetooth, background execution, location, native files, theme, lifecycle, permissions, WebViews, application services, SQLite, and loopback transport remain owned by Android

#### Scenario: Isolate Apple

- **WHEN** Apple unit, packaging, canonical, and physical checks run during the Android cutover
- **THEN** iOS and macOS continue through the unchanged Apple bridge without resolving an Android JNI library or symbol

### Requirement: Rust VESC BMS values candidate ownership

The focused Rust BMS-values component SHALL live at `backends/core/vesc-bms-values` and SHALL remain independent of framing, firmware identity and custom-configuration bridges, host frameworks, CAN routing, compatibility, discovery, scheduling, polling, application coordination, persistence, transport, and frontend code. Apple and Android SHALL select it through separate focused bridges and thin adapters beneath their respective `protocol/vesc` trees after verification deletes each superseded native structural parser.

#### Scenario: Locate component and production owners

- **WHEN** a maintainer traces command-96 BMS-values decoding after both host cutovers
- **THEN** the safe component and both host bridges are plainly identified under `backends/core`, iOS and macOS share the Apple adapter, and Android uses its own thin adapter beneath `protocol/vesc`

#### Scenario: Reject accidental cross-host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** Apple links only its focused BMS C bridge, Android packages only its focused BMS JNI bridge, and neither host links, calls, generates bindings for, or packages the other host's artifact

#### Scenario: Preserve BMS application ownership

- **WHEN** either host decodes command-96 payloads through Rust
- **THEN** compatibility, discovery, direct/CAN selection, request scheduling, polling, observed-time assignment, availability, state publication, and live-only policy remain with that host's existing native owners

#### Scenario: Preserve OS capability ownership

- **WHEN** both BMS-values bridges are built, selected, and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain unchanged in their existing platform owners

### Requirement: Shared Apple Rust BMS values bridge ownership

The Apple backend SHALL expose the safe BMS values component through one dedicated, versioned, stateless C ABI shared by iOS and macOS. The ABI SHALL contain exactly two exported operations: version query and decode. It SHALL use fixed status and record values; caller-owned bounded input and output regions; explicit expected-CAN representation; pairwise region validation; panic containment; payload-safe diagnostics; and no Rust allocator, layout, callback, reference, or opaque lifetime crossing the boundary.

The successful versioned record SHALL encode explicit flags, counts, reported CAN ID, optional-tail metadata, exact big-endian IEEE-754 scalar and collection values, one-byte balancing entries, and bounded status bytes. Swift SHALL validate the complete record and mechanically map it into `BmsSnapshot` without interpreting command-96 payload structure. The hand-written Apple C module, locked source-build helper, SwiftPM linkage, and Xcode linkage SHALL build the same bridge for macOS, iOS device, and iOS simulator without committing a generated binding or native archive.

#### Scenario: Exercise one bounded bridge call

- **WHEN** Swift supplies one payload, optional expected CAN ID, and caller-owned result regions
- **THEN** Rust validates every region, decodes the payload, writes only the documented bounded record, returns one fixed status, and retains no pointer, allocation, or lifetime

#### Scenario: Map one successful record

- **WHEN** the bridge returns a valid success record
- **THEN** Swift validates its version, flags, reserved byte, counts, extents, finite numeric bits, boolean values, reported CAN ID, and optional-tail consistency before constructing one complete snapshot

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives null, oversized, overlapping, insufficient-capacity, malformed, ABI-incompatible, invalid-record, or internally failed input
- **THEN** it writes nothing outside supplied bounds, unwinds no panic across C, exposes no partial snapshot, and logs no payload or decoded value

#### Scenario: Build every Apple target family

- **WHEN** clean Apple packaging builds macOS, iOS device, and iOS simulator products from repository-pinned source
- **THEN** each product links the correct two-symbol bridge architecture and shared adapter without a generated binding, committed archive, or host-only substitution

#### Scenario: Keep bridges and Android independent

- **WHEN** maintainers inspect dependencies, exported symbols, and Android packages
- **THEN** the BMS values bridge depends only on its safe component and standard library, existing Apple bridges retain their own symbols and behavior, and Android contains no Apple BMS bridge artifact or dependency

### Requirement: Android Rust BMS values bridge ownership

The Android backend SHALL expose the safe BMS-values component through one dedicated, versioned, stateless JNI ABI. The ABI SHALL contain exactly two exported operations: version query and decode. It SHALL use fixed status and record values; JVM-owned bounded byte and integer arrays; explicit expected-CAN representation and lengths; alias and pending-exception validation; panic containment; payload-safe diagnostics; and no Rust allocator, layout, callback, returned pointer, or opaque lifetime crossing the boundary.

The successful versioned record SHALL encode explicit flags, counts, reported CAN ID, optional-tail metadata, exact big-endian IEEE-754 scalar and collection values, one-byte balancing entries, and bounded status bytes. Kotlin SHALL validate the complete record and mechanically map it into `BmsSnapshot` without interpreting command-96 payload structure. The locked source-build helper and Gradle wiring SHALL build the same bridge for host JVM tests and all four supported Android ABIs in debug and release without committing a generated binding or native library.

#### Scenario: Exercise one bounded JNI call

- **WHEN** Kotlin supplies one payload, optional expected CAN ID, and JVM-owned result arrays
- **THEN** Rust validates every reference, shape, alias, primitive and bound, decodes the payload, writes only the documented bounded record and metadata, returns one fixed status, and retains no reference, allocation, or lifetime

#### Scenario: Map one successful record

- **WHEN** the bridge returns a valid success record
- **THEN** Kotlin validates its version, flags, reserved byte, counts, extents, finite numeric bits, boolean values, reported CAN ID, and optional-tail consistency before constructing one complete snapshot

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives null, oversized, aliased, wrongly shaped, insufficient-capacity, malformed, ABI-incompatible, pending-exception, invalid-record, or internally failed input
- **THEN** it writes no partial snapshot, unwinds no panic across JNI, retains no Java reference, and logs no payload or decoded value

#### Scenario: Build every Android target ABI

- **WHEN** clean Android packaging builds host tests plus debug and release applications from repository-pinned source
- **THEN** the source-owned bridge is loaded for JVM tests and packaged for `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64` without a generated binding, committed library, or host-only substitution

#### Scenario: Keep bridges and Apple independent

- **WHEN** maintainers inspect dependencies, libraries, exported symbols, and Apple packages
- **THEN** the Android BMS bridge depends only on its safe component, standard library, and locked minimal JNI binding; existing Android bridges retain their own symbols; and Apple contains no Android BMS JNI artifact or dependency

### Requirement: Rust Refloat control codec candidate ownership

The focused Rust Refloat control candidate SHALL live at `backends/core/refloat-control` and SHALL remain independent of VESC framing, firmware identity, custom configuration, BMS decoding, host frameworks, CAN routing, compatibility catalogues, authorization, scheduling, telemetry, application coordination, persistence, transport, and frontend code. Apple and Android SHALL continue to locate their selected production codecs beneath their respective `protocol/refloat` trees until separate host-specific cutovers pass focused, packaging, canonical, and physical verification and delete the corresponding native codec.

#### Scenario: Locate candidate and production owners

- **WHEN** a maintainer traces Refloat live-control wire encoding during this extraction step
- **THEN** the safe candidate is plainly identified under `backends/core` while the selected Swift and Kotlin production owners remain beneath each backend's `protocol/refloat` path

#### Scenario: Reject accidental host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** the candidate adds no C or JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path

#### Scenario: Preserve Refloat application ownership

- **WHEN** the candidate encodes or decodes control payloads in component tests
- **THEN** compatibility lookup, direct/CAN selection, VESC framing, mutation authorization, pending-work exclusion, scheduling, confirmation, telemetry, and state publication remain with their native owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the Refloat control candidate is built and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain unchanged in their existing platform owners

### Requirement: Shared Apple Rust Refloat control bridge ownership

The Apple backend SHALL expose the safe Refloat control component through one dedicated, versioned, stateless C ABI shared by iOS and macOS. The ABI SHALL contain exactly three exported operations: version query, control encode, and lighting-response decode. It SHALL use fixed operation, profile, target, status, and primitive-result values; caller-owned bounded output and scalar regions; borrowed bounded response input; pairwise region validation; panic containment; payload-safe diagnostics; and no Rust allocator, layout, callback, reference, or opaque lifetime crossing the boundary.

The hand-written `CVescRefloatControl` module, shared bridge catalogue, generic locked source-build helper, SwiftPM linkage, and Xcode linkage SHALL build the same bridge for macOS, iOS device, and iOS simulator. No generated binding or native archive SHALL be committed. Existing Apple bridges SHALL remain independent, and Android SHALL neither link nor package this Apple bridge.

#### Scenario: Exercise one bounded encode call

- **WHEN** Swift supplies one canonical control operation and caller-owned output regions
- **THEN** Rust validates every primitive/region, encodes at most nine bytes, returns fixed status/length/unsupported-command values, and retains no pointer or allocation

#### Scenario: Exercise one bounded lighting decode call

- **WHEN** Swift supplies one exact lighting response and selected profile
- **THEN** Rust returns one validated two-bit primitive result or fixed failure without exposing a Rust model or retaining input

#### Scenario: Contain bridge failure

- **WHEN** the bridge receives an invalid operation, profile, target, boolean, current state, unused parameter, null, oversized, overlapping, misaligned, insufficient-capacity, malformed, ABI-incompatible, or internally failed call
- **THEN** it writes nothing outside validated regions, unwinds no panic across C, returns the documented bounded result, and logs no payload, state, address, or token

#### Scenario: Build every Apple target family

- **WHEN** clean Apple packaging builds macOS, iOS device, and iOS simulator products from repository-pinned source
- **THEN** each product links the correct three-symbol bridge architecture and shared adapter without a generated binding, committed archive, or host-only substitution

#### Scenario: Preserve platform and Android isolation

- **WHEN** shared Apple production uses Rust Refloat control encoding and decoding
- **THEN** CoreBluetooth, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain native and Android contains no Apple bridge artifact or dependency
