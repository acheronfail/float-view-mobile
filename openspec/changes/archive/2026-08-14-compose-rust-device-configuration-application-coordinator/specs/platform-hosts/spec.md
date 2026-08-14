## ADDED Requirements

### Requirement: Rust device configuration composition ownership

The unselected safe Rust connection application coordinator SHALL directly own the device-mutation coordinator and Refloat configuration service below a future platform-neutral runtime and above existing safe protocol/application models. Apple and Android SHALL retain their selected native application, configuration, and mutation owners until later runtime and host-cutover changes prove production Rust execution and delete both native paths and temporary protocol bridges.

#### Scenario: Locate the composed and selected owners

- **WHEN** a maintainer traces controller activation, semantic mutations, configuration reads/writes, Tune apply, scheduler attribution, cache intent, confirmation, failure, or reset
- **THEN** the composed safe candidate SHALL be plainly identified in `backends/core/connection-application-coordinator` while production execution remains in each backend’s existing application service and child owners

#### Scenario: Keep composition out of host packages

- **WHEN** Apple and Android packages build with the composed candidate in the locked workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, runtime fallback, callback, or host object

#### Scenario: Preserve selected application ownership

- **WHEN** production controls, configuration, Tune apply, shutdown, telemetry confirmation, scheduler failure, backgrounding, reconnect, or reset occur
- **THEN** existing Swift and Kotlin owners SHALL retain all selected authorization, serialization, scheduling, cache execution, publication, and failure behavior during this change

#### Scenario: Preserve OS capability ownership

- **WHEN** the composed candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, sessions, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare runtime composition and bridge deletion

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL expose one direct safe Rust application boundary for later persistence and loopback-runtime composition, no coordinator-specific Apple FFI or Android JNI crate SHALL exist, and later host cutovers SHALL delete superseded native coordinators and all no-longer-used protocol bridges
