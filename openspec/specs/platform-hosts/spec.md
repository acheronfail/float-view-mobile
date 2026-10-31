# Platform Hosts Specification

## Purpose

Define current native OS responsibilities, shared-service ownership and host lifecycle seams for Apple and Android.

Completed extraction, intermediate native ownership and per-protocol bridge
acceptance are historical. Original requirements and evidence remain in
[archived changes](../../changes/archive/); ongoing checks follow
[AGENTS.md](../../../AGENTS.md#verification).

## Requirements

### Requirement: Lowercase ownership layout

The repository SHALL place the frontend under `/frontend`, the shared Apple implementation and its iOS/macOS hosts under `/backends/apple`, the Android implementation and host under `/backends/android`, shared backend SQL and immutable protocol fixtures under `/backends`, canonical acceptance under top-level `/conformance`, and supporting firmware under `/tools/vesc-test-device`.

#### Scenario: Repository navigation

- **WHEN** a developer inspects the repository root
- **THEN** lowercase top-level directories identify frontend, backend, conformance, tool, and planning ownership without generic package or shared-code layers

### Requirement: One shared Apple backend

iOS and macOS under `/backends/apple` SHALL use the same Apple adapters for Bluetooth, lifecycle and OS integration. Apple and Android SHALL select the same complete Rust backend service composing one application runtime, product/session storage and shared loopback server. The shared server SHALL own transport, authentication, contract dispatch, and frontend serialization while platform adapters retain only OS execution and integration.

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

Each native process SHALL own Bluetooth state, application state, background work, durable logging, and the loopback server independently of the WebView. The WebView SHALL own no durable or Bluetooth state. The process SHALL delegate portable application policy, product persistence, archive formats and state publication to the shared Rust service; native host code SHALL execute OS capabilities only.

#### Scenario: Frontend termination

- **WHEN** the WebView process or view is terminated
- **THEN** native backend state remains authoritative and recoverable by a replacement frontend

### Requirement: Minimal dependencies

Platform builds SHALL remove dependencies used only by deleted generation, module boundaries, native UI scaffolding, or duplicate tests, while retaining focused maintained dependencies that replace substantial protocol or server code.

#### Scenario: Dependency audit

- **WHEN** implementation is complete
- **THEN** every remaining direct dependency is used by current build, runtime, or verification code

### Requirement: Focused shared-code ownership

Shared backend code SHALL live in focused Rust owners under `backends/core`, with canonical SQL and immutable protocol fixtures under `backends`. Native adapters SHALL retain OS integration without a generic generated API layer, copied portable policy or protocol-specific host bridge.

#### Scenario: Locate shared behavior

- **WHEN** both hosts need the same protocol, catalog or application behavior
- **THEN** they compose its existing shared Rust owner while host-specific OS effects remain in the platform adapter

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

### Requirement: Shared production ownership and native capability layout

Apple and Android SHALL use focused native capability paths and the same Rust owners for portable behavior. Native adapters SHALL execute OS facts/effects without duplicate protocol, storage, routing, polling, public dispatch or product policy. Bluetooth and protocol owners SHALL not depend on views or loopback transport.

#### Scenario: Locate a native capability

- **WHEN** a maintainer needs to change Bluetooth, location, file import/export, background lifecycle, theme, loopback transport, application coordination, storage, VESC protocol, or Refloat protocol behavior
- **THEN** each backend exposes the production implementation under the corresponding canonical capability path beneath its native source root, or its ownership documentation points to the explicitly cut-over portable component beneath that path's platform adapter

#### Scenario: Locate controller discovery

- **WHEN** a developer needs to change direct-or-CAN route selection
- **THEN** each native backend points to the same shared controller-discovery owner, without native route decisions in transport, polling or view code

#### Scenario: Locate a protocol family

- **WHEN** a developer needs to change VESC framing or a Refloat wire layout
- **THEN** the native adapter points to the focused production component under `backends/core` rather than a version-named application-service branch

#### Scenario: Locate platform file handling

- **WHEN** a developer needs to change session archive selection, bounded native import reading, save-panel presentation, or share-sheet presentation
- **THEN** the platform integration is found under `files` and archive validation is owned by shared Rust storage

#### Scenario: Locate background logging ownership

- **WHEN** a developer needs to change behavior after a background-capable mobile application leaves the foreground
- **THEN** process/application lifetime and background-service code is found under `lifecycle` while BLE remains under `bluetooth` and optional location capture remains under `location`

#### Scenario: Locate theme synchronization

- **WHEN** a developer needs to change how a native host responds to authoritative frontend theme settings or a system appearance change
- **THEN** native appearance synchronization is found under `theme` and the disposable WebView remains under `ui`

#### Scenario: Add a compatible firmware version

- **WHEN** a later VESC or Refloat release uses an existing tested wire family
- **THEN** the shared owner adds an exact compatibility-catalog entry and fixtures without copying application-service, routing, or polling code

#### Scenario: Add a new wire family

- **WHEN** a later release changes a required wire layout
- **THEN** the shared owner adds or changes the focused codec family and maps exact versions to it while shared framing and routing remain unchanged

#### Scenario: Backend source documentation

- **WHEN** a developer reads the backend ownership documentation
- **THEN** it identifies the canonical Swift/Kotlin capability locations, the selected `backends/core` owners, native source-root differences, dependency direction, privacy boundary, and version-to-family extension rule

#### Scenario: Layout drift

- **WHEN** production code is added under a superseded or ambiguous backend source location
- **THEN** normal repository verification fails with the unexpected location

### Requirement: Shared detailed telemetry ownership

The shared Rust service SHALL compose focused owners for Refloat family decoding, telemetry scheduling, semantic normalization, application state, durable persistence, session retrieval, and serialization. VESC command routing SHALL remain below Refloat polling, and neither protocol owner SHALL depend on Bluetooth, storage, loopback transport, or frontend code.

#### Scenario: Add a known telemetry field

- **WHEN** support for an existing negotiated Refloat field is added
- **THEN** the shared decoder, normalized model, persistence mapping, serialization, fixtures and tests are updated without adding platform-specific frontend logic

#### Scenario: Route ownership

- **WHEN** a detailed Refloat request targets a CAN controller
- **THEN** the Refloat owner supplies the command and the VESC routing owner applies CAN forwarding exactly once

### Requirement: Equivalent native detailed persistence

The canonical shared schemas SHALL durably store every stable numeric telemetry value in the matching dedicated nullable sample column, unknown negotiated numeric values in bounded `extension_json`, observed field presence in a bounded per-session table, and discrete transitions in `session_event` with an independent monotonic event sequence, source sample sequence, observation time, elapsed time, and single-session ownership.

#### Scenario: Preserve shared durable evidence

- **WHEN** the shared service commits telemetry or updates history
- **THEN** multiple transitions from one sample SHALL be inserted in stable catalog order. Session metadata SHALL capture its positive bounded riding and idle persistence intervals. The per-session commit marker SHALL atomically record committed sample/event boundaries, recorded/stored counts, history revision, elapsed coverage, commit time, and the bounded riding/distance/maximum/source-sequence/last-value projection required to recover catalog summaries with those rows. The schema SHALL initialize incremental auto-vacuum and bounded prune metadata/job state. Denormalized catalog summaries SHALL converge before a boundary is published and SHALL be reconciled from the marker after interruption or an in-process catalog failure before that session is read or appended again. Owned-storage compatibility SHALL follow baseline preservation and explicit supported upgrades; mismatch alone never authorizes deletion.

#### Scenario: Equivalent stored sample

- **WHEN** both native backends accept equivalent normalized complete-catalog telemetry
- **THEN** dedicated columns, field presence, session-data retrieval, and export produce semantically equivalent numeric records and transitions despite native implementation differences

#### Scenario: Atomic session marker

- **WHEN** a batch introduces samples, multiple transition events, and a late extension field
- **THEN** those rows and the corresponding sample/event sequences, source evidence, recorded/stored counts, history revision, elapsed boundary, field presence, and recovery summary projection become durable in one session-database transaction

#### Scenario: Recognised development schema

- **WHEN** a recognised pre-baseline marker and validated owned development layout require reset
- **THEN** only the bounded owned development files are reset once; baseline and later supported stores, unknown layouts and exported user files remain preserved

### Requirement: Process-owned detailed polling survives frontend disposal

Detailed realtime polling, the one-second compact due time, normalization, and durable commits SHALL remain in the shared service when the WebView is absent or recreated. The shared connection owner SHALL retain one in-flight request and no separate telemetry work queue. Background and reconnect behavior SHALL follow the existing platform lifecycle rules and prevent stale responses or a replacement controller from contributing detailed samples or events to an active session.

#### Scenario: WebView replacement

- **WHEN** the frontend disappears while native detailed telemetry continues
- **THEN** accepted samples remain durably ordered and a replacement frontend recovers the latest committed sample

#### Scenario: Controller changes

- **WHEN** reconnect validation reports a replacement controller
- **THEN** detailed polling stops, the old session ends, route-scoped compact state is cleared, and no replacement-controller fields or events are appended to the old session

### Requirement: Native standalone session catalog and grouping removal

Apple and Android SHALL expose the same bounded `session.list` control operation over the same shared Rust session catalog.

#### Scenario: Keep catalog work out of telemetry snapshots

- **WHEN** the service lists sessions or publishes live state
- **THEN** the query SHALL use the catalog's `(started_at_ms, session_id)` keyset order, return at most the requested page size, encode the query shape/catalog revision/last-key context in its opaque cursor, and avoid offset scans. Complete state snapshot construction SHALL not query or serialize session summaries during telemetry-driven publication; it SHALL expose only the bounded session-catalog revision marker. The canonical schema, settings store, application service, loopback handlers, and mock SHALL contain no session-group tables, group-member tables, automatic-linking fields, group mutation ports, or linked-ride payloads. Only recognised pre-baseline owned development layouts may be reset; supported data is preserved and unsupported storage fails without deletion.

#### Scenario: Indexed session page

- **WHEN** both hosts serve successive valid `session.list` requests over a catalog larger than one page
- **THEN** both use the checked started-time/session-ID keyset order, return identical standalone summaries and continuation behavior, and do not use offset pagination

#### Scenario: Catalog query absent from telemetry snapshot

- **WHEN** a persisted telemetry sample invalidates application state
- **THEN** neither host queries or serializes a session-summary or group collection for that state event, and the unchanged session-catalog revision remains available

#### Scenario: Grouping removed from native surfaces

- **WHEN** a backend is built from the canonical supported schema and exposes settings or control methods
- **THEN** group tables, linked-ride records, automatic-linking options, and group/unlink operations are absent on Apple and Android

#### Scenario: Cursor context mismatch

- **WHEN** a session-list cursor is malformed, rebound to another query shape, or issued before a newer catalog revision
- **THEN** both hosts reject it with the same bounded error and do not return a mixed page

### Requirement: Shared historical request implementation

The shared Rust server SHALL implement authenticated session-data requests over the production Rust SQLite reader.

#### Scenario: Execute the shared history pipeline

- **WHEN** an authenticated historical request is accepted
- **THEN** the shared owner SHALL validate the shared hand-written contract, acquire the requested published history revision and boundary, select only validated columns/ranges plus internal ride phase, stream ordered rows through an equivalent bounded configurable-gap first/minimum/maximum/last reducer, serialize the same explicit columnar response, and apply standard gzip encoding. Hosts SHALL contain no parallel SQL reader, reducer, compressor or frontend serializer; the contract, SQL, immutable vectors, mock and conformance enforce shared results.

#### Scenario: Equivalent historical query

- **WHEN** Apple and Android query equivalent stored rows with the same series, samples, or events request
- **THEN** they return semantically identical snapshots, columnar values, extrema, continuity segments, ordering, and continuation behavior

#### Scenario: Shared compression

- **WHEN** the frontend accepts gzip from either host
- **THEN** the shared server returns the same decoded contract using its bounded compressor through either host

#### Scenario: Backend ownership

- **WHEN** the WebView is destroyed, recreated, or aborts a query
- **THEN** native storage, active logging, snapshot publication, cancellation, recovery, and resource cleanup remain owned by the host backend

### Requirement: Historical reads do not obstruct persistence

The shared service SHALL resolve session metadata under only a short store synchronization boundary, then use a dedicated read-only/query-only connection, one per-session read lease, and an explicit bounded WAL read transaction for the scan.

#### Scenario: Bound reader concurrency and cleanup

- **WHEN** historical reads run alongside recording
- **THEN** historical work SHALL execute away from the loopback and session-writer executors, SHALL allow no more than two concurrent reads per shared service, SHALL remain cancellable throughout query/reduction/serialization, and SHALL release cursors, transactions, connections, reducers, and compression buffers after success, cancellation, disconnect, or serialization failure. The shared Rust reader SHALL provide connection-local SQLite interruption tied to request lifetime on both host families. Releasing cancelled work within 250 milliseconds and ordinary completion within two seconds on the reference devices SHALL be recorded performance targets, not correctness deadlines.

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

Opening a historical request SHALL acquire a lease only while the catalog row is readable and not pending maintenance or deletion.

#### Scenario: Coordinate leases and maintenance

- **WHEN** a historical read overlaps storage maintenance
- **THEN** deletion and pruning SHALL prevent new leases before requesting cancellation from existing leases and SHALL mutate or remove SQLite files only after every reader releases them. If release is slow, the operation SHALL remain visibly pending and cancellable instead of deleting beneath an open connection or relying on a correctness timeout. Pending deletion cancellation SHALL restore readability only when it wins before irreversible file removal; otherwise deletion SHALL report its stable completed result. Export, compaction, and recovery SHALL use the same file-lifecycle boundary wherever their snapshot or file operations could race a reader. Startup SHALL perform bounded integrity checks and reconcile catalog counts/boundaries/revisions from the authoritative marker before any recovered session can be queried.

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

The shared service SHALL map every stable contract field to its dedicated SQL column and SHALL parse `extension_json` at most once per row only when requested extensions are present.

#### Scenario: Select only requested history fields

- **WHEN** the service scans a published historical range
- **THEN** series scans and keyset sample/event pages SHALL constrain the published sequence and elapsed range, preserve `(elapsedMs, sequence)` order, and use representative checked query plans without an avoidable full-table sort. Metadata SHALL obtain available fields from bounded field-presence rows rather than scanning samples.

#### Scenario: Stable field query

- **WHEN** a series request asks for two stable fields from a complete-catalog session
- **THEN** the shared reader selects those two numeric columns plus ordering fields without decoding stable per-row JSON or unrequested telemetry

#### Scenario: Extension query

- **WHEN** a request asks for several valid extension fields
- **THEN** each selected row's bounded extension object is decoded no more than once and only the requested values reach the reducer

### Requirement: Shared ride pruning

The shared service SHALL implement the same cancellable one-ride and all-completed-rides prune operations over an exclusive per-session maintenance lease.

#### Scenario: Preserve atomic bounded pruning

- **WHEN** a session or batch is pruned
- **THEN** the selector SHALL retain field-presence and semantic source samples plus no more than eight ordinary real source rows per elapsed-zero-aligned bucket using the shared deterministic coverage rule. It SHALL stream one bucket with bounded native memory and place retained sequences in an indexed file-backed SQLite keep-set with bounded cleanup rather than an in-memory ride-sized collection. A per-ride logical prune SHALL be transactional, increment history revision, preserve original available fields/summaries/events/recorded count, update stored count and policy, and invalidate older readers and exports. Batch progress SHALL use a single bounded persisted job, a captured upper session key, and stable keyset traversal over rides completed at job start; it SHALL resume safely after process interruption and SHALL be per-ride atomic. A cancellable WAL checkpoint and incremental compaction SHALL be resumable and SHALL NOT make a committed logical prune appear failed solely because all free pages have not yet been reclaimed.

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

Apple and Android SHALL execute OS location acquisition, permission prompts and lifecycle capabilities. The shared service SHALL own capture eligibility, normalized permission-state interpretation, session-clock assignment, durable append, bounded reads, export and product lifecycle policy. Location adapters SHALL not depend on WebSocket, Svelte, or browser geolocation code.

#### Scenario: Native ownership

- **WHEN** the frontend WebView is destroyed or replaced
- **THEN** the process-owned shared service retains capture policy and recorder state while native adapters continue permitted OS acquisition

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

### Requirement: Shared saved and guest tracking policy

The shared service SHALL enforce one explicit tracking-mode policy on both hosts.

#### Scenario: Guard durable side effects

- **WHEN** a connection enters saved or guest tracking mode
- **THEN** the policy SHALL be selected before connection work begins, retained across reconnect and WebView recreation, and checked at every durable side-effect boundary including saved-device creation, Board Profile writes, controller preference or learned-hint writes, session creation/append/deletion, location recording, catalog revision changes, and exportable ride metadata. Saved mode SHALL use the existing durable owners; guest mode SHALL use bounded in-memory live state and perform none of those durable writes. The shared service SHALL not rely on frontend omission as the isolation mechanism.

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

The shared Rust service selected by Apple and Android SHALL derive the same versioned stable opaque board identity from the same meaningful VESC identity, keep raw firmware and local platform identifiers out of portable archives, and allow imported boards to exist with a nullable local platform association.

#### Scenario: Preserve portable board associations

- **WHEN** a board archive is imported or a board is forgotten
- **THEN** archive import SHALL merge sessions into a matching `deviceHistoryId` or create a configurable board entry before attaching them. The shared service SHALL implement `savedDevice.delete` with the same all-owned-history cleanup and SHALL reject deletion while that board is actively connected.

#### Scenario: Cross-platform board identity

- **WHEN** equivalent VESC identity data is observed by Apple and Android
- **THEN** both hosts resolve the same stable opaque board identity without exposing the raw identifier

#### Scenario: Imported board without discovery

- **WHEN** either host imports an archive for a board with no local platform association
- **THEN** the board appears in Sessions, remains configurable, and does not become connectable until Home discovers and explicitly saves a matching board

#### Scenario: Forget parity

- **WHEN** an inactive board is forgotten on either host
- **THEN** the board, profile, private routing metadata, and all sessions owned by its history identity are removed with equivalent catalog revision behavior

### Requirement: Shared BMS ownership

The shared Rust service SHALL compose focused owners for connection-wide VESC request scheduling, common and hardware-specific firmware identity parsing, auxiliary-node discovery, bounded BMS values decoding, and slow BMS polling. Application coordination SHALL remain above protocol routing and Bluetooth, and no BMS protocol owner SHALL depend on WebSocket, frontend, persistence, or view code.

#### Scenario: Cross-platform source layout

- **WHEN** a maintainer traces BMS discovery or decoding on either backend
- **THEN** the native adapter points to its shared Rust owner with equivalent bounds and state transitions

#### Scenario: One write owner

- **WHEN** controller and BMS work both need the VESC stream
- **THEN** both hosts use the shared connection-wide scheduler rather than writing from independent polling timers

#### Scenario: Hand-written contract mapping

- **WHEN** native BMS state is serialized for the frontend
- **THEN** the shared server explicitly maps every bounded contract field with equivalent null, unit, array, and status semantics and no generated API layer

### Requirement: Native lifecycle and privacy boundary

The process-owned shared service SHALL continue auxiliary discovery and BMS polling while the frontend is disposable and while background execution is permitted, cancel every BMS generation and queued request on connection replacement, and exclude BMS UUIDs and full BMS telemetry from default logs and durable files.

#### Scenario: WebView disposal

- **WHEN** the frontend is destroyed while a selected BMS remains attached
- **THEN** native polling continues where the OS permits and a replacement frontend recovers the latest authoritative BMS state

#### Scenario: Connection generation changes

- **WHEN** disconnect, reconnect, preference change, or device replacement invalidates auxiliary work
- **THEN** late callbacks and queued work from the older generation cannot select a route or publish a snapshot

#### Scenario: Default diagnostics

- **WHEN** BMS discovery, polling, timeout, or malformed input is diagnosed normally
- **THEN** logs retain only bounded high-level route/status codes and do not contain UUIDs, full cell arrays, or complete raw telemetry frames

### Requirement: Shared Refloat configuration ownership

The shared Rust service SHALL compose focused owners for custom-schema chunk assembly, bounded Qt/zlib and XML decoding, normalized schema validation, CRC32C signature construction, typed value encoding/decoding, configuration operation lifecycle, and exact-version saved-configuration persistence.

#### Scenario: Keep mutation and schema ownership shared

- **WHEN** a configuration operation is executed
- **THEN** protocol owners SHALL depend only on VESC framing/routing and configuration models; one application-level mutation coordinator SHALL own authorization, exclusivity, and scheduler submission for semantic controls, configuration writes, and Tune apply; no feature owner may retain a parallel write path. Storage SHALL own schema cache and Tunes; loopback serialization SHALL expose only normalized bounded contract values. Frontend code MUST NOT parse device XML, compute wire signatures, choose routes/commands, or persist authoritative Tunes.

#### Scenario: Locate schema codec

- **WHEN** a developer changes ConfigParams parsing or a VESC value encoding
- **THEN** the focused Rust codec and shared golden vectors are available under their owning Refloat/VESC components

#### Scenario: Disposable WebView

- **WHEN** the frontend is recreated during schema loading or after saved configurations exist
- **THEN** native connection, operation, cache, and durable Tune ownership remain intact and are recovered through authenticated contract methods

#### Scenario: Mirrored flywheel boundary

- **WHEN** either native backend observes flywheel mode
- **THEN** its shared coordinator invalidates configuration current state and rejects every device mutation except eligible flywheel disable with behavior equivalent to the other backend

#### Scenario: Storage compatibility

- **WHEN** configuration persistence changes
- **THEN** canonical SQL and supported upgrades preserve baseline data; only recognised pre-baseline owned development layouts are eligible for reset

#### Scenario: Bounded schema parsing

- **WHEN** shared code decompresses and parses a schema
- **THEN** the maintained locked Rust codec enforces its XML/zlib bounds without a separate host parser

### Requirement: Runtime activation follows shared persistence preparation

The shared backend service SHALL separate supported-controller readiness from application activation and SHALL prepare shared catalog/session storage before authorizing activation of the existing runtime. Native hosts SHALL execute OS effects without implementing persistence or activation policy.

#### Scenario: Await current persistence confirmation

- **WHEN** controller discovery reaches a supported ready decision
- **THEN** the shared service SHALL expose the exact selected facts and SHALL NOT start telemetry, attached BMS work, durable session behavior, or ready publication until a current generation-qualified activation command arrives

#### Scenario: Reject failed or stale activation

- **WHEN** persistence/session preparation fails or activation belongs to an old, reset, failed, standalone-BMS, or already-active generation
- **THEN** the shared service SHALL start no child service and SHALL leave current state unchanged apart from a bounded error or stale diagnostic

### Requirement: Native startup presentation

Each supported host SHALL use its platform's normal launch and window presentation with an application-themed canvas. It SHALL NOT introduce a second splash screen, minimum splash duration, or startup visibility gate dependent on Bluetooth, backend connection, or telemetry readiness. Native theme changes SHALL be reflected in current and subsequently recreated host surfaces.

#### Scenario: Cold launch without a connected device

- **WHEN** the application launches with Bluetooth unavailable or no device connected
- **THEN** frontend content can become visible without waiting for device discovery or connection
- **AND** the native canvas remains consistent with the selected appearance during document loading

#### Scenario: Desktop window reopen

- **WHEN** the desktop window is closed and reopened while the application remains running
- **THEN** the new window and web canvas use the current application appearance
- **AND** the existing application service remains the sole runtime owner

### Requirement: Startup presentation through frontend lifecycle

Normal frontend absence during backgrounding or reconstruction SHALL preserve an application-themed surface without displaying a startup error. Existing recovery and teardown behavior SHALL retain native service/session ownership. Presentation callbacks and observers SHALL remain bounded by their owner lifetimes and SHALL NOT accumulate across repeated recreation.

#### Scenario: Return after frontend disposal

- **WHEN** the application returns to the foreground after its frontend was disposed
- **THEN** the recreated view presents the current appearance without a transient loading/error screen
- **AND** ongoing native session and connection state is preserved according to existing lifecycle policy

#### Scenario: Retained frontend resumes

- **WHEN** the application resumes with its frontend retained and system appearance changed while backgrounded
- **THEN** system-selected appearance updates while an explicit app preference remains unchanged

#### Scenario: Existing renderer recovery

- **WHEN** an existing host recovery path replaces a terminated web renderer
- **THEN** the replacement uses the current appearance and existing service owner
- **AND** no startup mask hides the recovered content or existing failure presentation

#### Scenario: Repeated frontend recreation

- **WHEN** the frontend is recreated repeatedly
- **THEN** theme observers, presentation callbacks and native view resources do not grow with recreation count

### Requirement: Startup failures remain distinguishable and visible

Startup presentation SHALL distinguish normal frontend absence from an actual startup failure. It SHALL NOT hide existing native or frontend error presentation behind a loading surface or wait indefinitely for a new startup-ready signal.

#### Scenario: Native frontend startup fails

- **WHEN** the iOS host cannot start its loopback frontend while the app is active
- **THEN** it shows a startup error rather than normal loading presentation
- **AND** successful activation through the existing lifecycle path clears the stale error

#### Scenario: Failure while backgrounded

- **WHEN** the frontend is absent during backgrounding or foreground reconstruction without a recorded startup failure
- **THEN** the host does not display a startup error

#### Scenario: Web load or connection fails

- **WHEN** document navigation or the frontend's backend connection fails
- **THEN** startup styling does not obscure the existing platform or frontend error/recovery behavior
- **AND** presentation adds no unbounded retry or wait-for-connectivity loop
