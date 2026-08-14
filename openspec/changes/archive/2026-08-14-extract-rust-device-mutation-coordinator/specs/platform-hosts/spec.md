## ADDED Requirements

### Requirement: Rust device mutation coordinator candidate ownership

The unselected safe Rust device mutation coordinator candidate SHALL live at `backends/core/device-mutation-coordinator` above safe VESC/Refloat compatibility, control, telemetry, firmware-identity, and scheduler-fact models and below future connection/runtime composition. Apple and Android SHALL retain their selected application-level mutation coordinators until later direct Rust composition and host cutovers prove production execution and delete both native paths.

#### Scenario: Locate candidate and selected owners

- **WHEN** a maintainer traces semantic-control availability, authorization, idempotence, confirmation, competing work, configuration exclusivity, controller shutdown, failure, or reset
- **THEN** the safe candidate SHALL be plainly identified under `backends/core` while production execution remains in each backend's existing mutation coordinator and application service

#### Scenario: Keep the candidate out of host packages

- **WHEN** Apple and Android packages build with the candidate in the locked workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, runtime fallback, callback, or host object

#### Scenario: Preserve selected mutation ownership

- **WHEN** production controls, configuration writes, Tune apply, controller shutdown, telemetry confirmation, failure, backgrounding, reconnect, or reset occur
- **THEN** existing Swift and Kotlin owners SHALL retain all selected authorization, serialization, scheduling, publication, and failure behavior during this change

#### Scenario: Preserve OS capability ownership

- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, loopback transport, sessions, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare direct composition and bridge deletion

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL be documented for direct composition with the Rust connection coordinator and configuration service, no mutation-specific Apple FFI or Android JNI crate SHALL exist, and later host cutovers SHALL delete superseded native coordinators and protocol bridges
