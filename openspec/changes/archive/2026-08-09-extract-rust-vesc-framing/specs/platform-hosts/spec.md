## MODIFIED Requirements

### Requirement: No generic shared-code directory

Cross-language shared content under `/backends` SHALL be limited to immutable SQL and protocol fixtures plus bounded portable Rust components under `/backends/core` governed by `native-core-migration`. A Rust candidate MUST name one behavioral slice and MUST NOT become a generic shared-code layer, generated API surface, or production owner before an explicit host cutover. Swift and Kotlin implementation code SHALL remain owned by their backend until each host's verified cutover deletes the corresponding superseded implementation.

#### Scenario: Shared backend resource

- **WHEN** both backends consume the same current SQL schema or protocol fixture
- **THEN** the immutable resource may reside under `/backends` without introducing a cross-language shared-code layer

#### Scenario: Bounded Rust candidate

- **WHEN** an active OpenSpec change extracts one portable component under `/backends/core`
- **THEN** repository verification permits only that named candidate and preserves native production ownership until separately verified host cutovers

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
