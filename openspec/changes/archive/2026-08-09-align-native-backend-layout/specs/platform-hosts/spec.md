## MODIFIED Requirements

### Requirement: Mirrored native connection ownership

The Apple and Android backends SHALL use the same capability-oriented source vocabulary for `application`, `bluetooth`, `files`, `lifecycle`, `location`, `protocol`, `storage`, `theme`, `transport`, and `ui` ownership beneath their native build-system source roots. Protocol code SHALL retain matching `protocol/vesc` and `protocol/refloat` subtrees. Apple MAY divide those capabilities among its shared backend, iOS host, and macOS host targets, while Android SHALL retain one Gradle application module; a target that owns a capability MUST use the canonical capability name rather than an ambiguous root source or platform-specific synonym.

Swift and Kotlin implementations MUST remain backend-owned, and matching paths MUST NOT introduce a shared implementation, generated API, FFI boundary, or module per capability. Dependencies SHALL point downward from host/UI composition and lifecycle, through transport and application coordination, to Bluetooth, location, storage, and protocol owners. Bluetooth and protocol components MUST continue to emit typed events without depending on WebViews or loopback transport. Native file presentation, background execution, theme synchronization, and permission handling SHALL remain owned by the platform implementation.

#### Scenario: Locate a native capability

- **WHEN** a maintainer needs to change Bluetooth, location, file import/export, background lifecycle, theme, loopback transport, application coordination, storage, VESC protocol, or Refloat protocol behavior
- **THEN** each backend exposes the implementation under the corresponding canonical capability path beneath its native source root

#### Scenario: Locate controller discovery

- **WHEN** a developer needs to change direct-or-CAN route selection
- **THEN** each native backend has one clearly named controller-discovery owner under `application` rather than route decisions spread through transport, polling, and view code

#### Scenario: Locate a protocol family

- **WHEN** a developer needs to change VESC framing or a Refloat wire layout
- **THEN** the matching Swift and Kotlin file is found under the corresponding `protocol/vesc` or `protocol/refloat` subtree rather than a version-named application-service branch

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
- **THEN** each backend adds a compatibility-catalog entry and fixtures without copying application-service, routing, or polling code

#### Scenario: Add a new wire family

- **WHEN** a later release changes a required wire layout
- **THEN** each backend adds or changes the focused codec family and maps exact versions to it while shared framing and routing remain unchanged

#### Scenario: Backend source documentation

- **WHEN** a developer reads the backend ownership documentation
- **THEN** it identifies the canonical Swift/Kotlin capability locations, native source-root differences, dependency direction, privacy boundary, and version-to-family extension rule

#### Scenario: Layout drift

- **WHEN** production code is added under a superseded or ambiguous backend source location
- **THEN** normal repository verification fails with the unexpected location
