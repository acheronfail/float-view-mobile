## MODIFIED Requirements

### Requirement: One shared Apple backend

iOS and macOS under `/backends/apple` SHALL use the same Apple adapters for Bluetooth, storage, lifecycle, and OS integration. Apple and Android SHALL select the same complete Rust backend service composing one application runtime, product/session storage and shared loopback server. The shared server SHALL own transport, authentication, contract dispatch, and frontend serialization while platform adapters retain only OS execution and integration.

#### Scenario: Apple behavior change

- **WHEN** shared backend behavior changes
- **THEN** both iOS and macOS receive the same implementation and contract semantics

### Requirement: Native backend ownership

Each native process SHALL own Bluetooth state, application state, background work, durable logging, and the loopback server independently of the WebView. The WebView SHALL own no durable or Bluetooth state. The process SHALL delegate portable application policy, product persistence, archive formats and state publication to the shared Rust service; native host code SHALL execute OS capabilities only.

#### Scenario: Frontend termination

- **WHEN** the WebView process or view is terminated
- **THEN** native backend state remains authoritative and recoverable by a replacement frontend

### Requirement: Runtime activation preserves native persistence boundary

The shared backend service SHALL separate supported-controller readiness from application activation and SHALL prepare shared catalog/session storage before authorizing activation of the existing runtime. Native hosts SHALL execute OS effects without implementing persistence or activation policy.

#### Scenario: Await current persistence confirmation

- **WHEN** controller discovery reaches a supported ready decision
- **THEN** the shared service SHALL expose the exact selected facts and SHALL NOT start telemetry, attached BMS work, durable session behavior, or ready publication until a current generation-qualified activation command arrives

#### Scenario: Reject failed or stale activation

- **WHEN** persistence/session preparation fails or activation belongs to an old, reset, failed, standalone-BMS, or already-active generation
- **THEN** the shared service SHALL start no child service and SHALL leave current state unchanged apart from a bounded error or stale diagnostic

### Requirement: Native standalone session catalog and grouping removal

Apple and Android SHALL expose the same bounded `session.list` control operation over the same shared Rust session catalog. The query SHALL use the catalog's `(started_at_ms, session_id)` keyset order, return at most the requested page size, encode the query shape/catalog revision/last-key context in its opaque cursor, and avoid offset scans. Complete state snapshot construction SHALL not query or serialize session summaries during telemetry-driven publication; it SHALL expose only the bounded session-catalog revision marker. The canonical schema, settings store, application service, loopback handlers, and mock SHALL contain no session-group tables, group-member tables, automatic-linking fields, group mutation ports, or linked-ride payloads. Since storage remains disposable test-stage data, incompatible group-bearing schemas SHALL be recreated without migration.

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

### Requirement: Native board identity, archive, and forget parity

The shared Rust service selected by Apple and Android SHALL derive the same versioned stable opaque board identity from the same meaningful VESC identity, keep raw firmware and local platform identifiers out of portable archives, and allow imported boards to exist with a nullable local platform association. Archive import SHALL merge sessions into a matching `deviceHistoryId` or create a configurable board entry before attaching them. The shared service SHALL implement `savedDevice.delete` with the same all-owned-history cleanup and SHALL reject deletion while that board is actively connected.

#### Scenario: Cross-platform board identity

- **WHEN** equivalent VESC identity data is observed by Apple and Android
- **THEN** both hosts resolve the same stable opaque board identity without exposing the raw identifier

#### Scenario: Imported board without discovery

- **WHEN** either host imports an archive for a board with no local platform association
- **THEN** the board appears in Sessions, remains configurable, and does not become connectable until Home discovers and explicitly saves a matching board

#### Scenario: Forget parity

- **WHEN** an inactive board is forgotten on either host
- **THEN** the board, profile, private routing metadata, and all sessions owned by its history identity are removed with equivalent catalog revision behavior

### Requirement: Mirrored native location ownership

Apple and Android SHALL execute OS location acquisition, permission prompts and lifecycle capabilities. The shared service SHALL own capture eligibility, normalized permission-state interpretation, session-clock assignment, durable append, bounded reads, export and product lifecycle policy. Location adapters SHALL not depend on WebSocket, Svelte, or browser geolocation code.

#### Scenario: Native ownership

- **WHEN** the frontend WebView is destroyed or replaced
- **THEN** the process-owned shared service retains capture policy and recorder state while native adapters continue permitted OS acquisition

#### Scenario: Cross-platform result

- **WHEN** equivalent permission and provider fixtures are supplied to Apple and Android
- **THEN** both backends expose the same contract status, row shape, boundary semantics, and stable errors

### Requirement: Mirrored native connection ownership

The Apple and Android backends SHALL use the same capability-oriented source vocabulary for `application`, `bluetooth`, `files`, `lifecycle`, `location`, `protocol`, `storage`, `theme`, `transport`, and `ui` ownership beneath their native build-system source roots. Protocol code SHALL retain matching `protocol/vesc` and `protocol/refloat` subtrees while a platform owns those implementations. Apple MAY divide those capabilities among its shared backend, iOS host, and macOS host targets, while Android SHALL retain one Gradle application module; a target that owns a capability MUST use the canonical capability name rather than an ambiguous root source or platform-specific synonym.

Apple and Android SHALL select the complete shared Rust service for portable application, storage, protocol and transport behavior; Swift and Kotlin SHALL retain only OS capability execution and host composition. A focused candidate implementation MAY coexist under `backends/core` without transferring production ownership. A host-cutover change MAY introduce one narrow audited bridge only when it names the selected Rust component, platform packaging, complete verification, and superseded-code deletion checkpoint; generated general-purpose API layers and modules per capability remain prohibited. Dependencies SHALL point downward from host/UI composition and lifecycle, through transport and application coordination, to Bluetooth, location, storage, and protocol owners. Bluetooth and protocol components MUST continue to emit typed events without depending on WebViews or loopback transport. Native Bluetooth, location, file presentation, background execution, theme synchronization, permission handling, and UI integration SHALL remain owned by the platform implementation.

#### Scenario: Locate a native capability

- **WHEN** a maintainer needs to change Bluetooth, location, file import/export, background lifecycle, theme, loopback transport, application coordination, storage, VESC protocol, or Refloat protocol behavior
- **THEN** each backend exposes the production implementation under the corresponding canonical capability path beneath its native source root, or its ownership documentation points to the explicitly cut-over portable component beneath that path's platform adapter

#### Scenario: Locate controller discovery

- **WHEN** a developer needs to change direct-or-CAN route selection
- **THEN** each native backend points to the same shared controller-discovery owner, without native route decisions in transport, polling or view code

#### Scenario: Locate a protocol family

- **WHEN** a developer needs to change VESC framing or a Refloat wire layout
- **THEN** the selected production owner is found from the corresponding `protocol/vesc` or `protocol/refloat` subtree and any portable implementation is found under one focused `backends/core` component rather than a version-named application-service branch

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
- **THEN** each production backend adds a compatibility-catalog entry and fixtures without copying application-service, routing, or polling code

#### Scenario: Add a new wire family

- **WHEN** a later release changes a required wire layout
- **THEN** each production backend adds or changes the focused codec family and maps exact versions to it while shared framing and routing remain unchanged

#### Scenario: Candidate extraction

- **WHEN** a bounded Rust component has parity evidence but no host bridge
- **THEN** the candidate remains unselected and cannot replace the already selected shared service or introduce native portable fallbacks

#### Scenario: Platform-specific cutover

- **WHEN** one host selects a Rust component through its audited bridge
- **THEN** only that host changes production ownership after its complete gate passes, while OS-owned capabilities remain in the platform source tree

#### Scenario: Backend source documentation

- **WHEN** a developer reads the backend ownership documentation
- **THEN** it identifies the canonical Swift/Kotlin capability locations, any explicitly cut-over `backends/core` owner, native source-root differences, dependency direction, privacy boundary, and version-to-family extension rule

#### Scenario: Layout drift

- **WHEN** production code is added under a superseded or ambiguous backend source location
- **THEN** normal repository verification fails with the unexpected location
