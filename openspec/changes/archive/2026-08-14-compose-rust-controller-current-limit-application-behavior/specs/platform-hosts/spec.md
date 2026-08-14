## ADDED Requirements

### Requirement: Rust controller current-limit composition ownership

The unselected safe Rust connection application coordinator SHALL directly own controller current-limit request, attribution, decode, live-state, and reset behavior below a future platform-neutral runtime. Apple and Android SHALL retain their selected native application owners and existing controller-data bridges until later runtime and host-cutover changes prove production Rust execution and delete both native paths and temporary protocol bridges.

#### Scenario: Locate composed and selected owners

- **WHEN** a maintainer traces current-limit request ordering, routing, response attribution, decode, publication, failure, or reset
- **THEN** the safe composed candidate SHALL be plainly identified in `backends/core/connection-application-coordinator` using `backends/core/vesc-controller-data`, while production execution remains in each backend's application and bridge adapters

#### Scenario: Keep composition out of host packages

- **WHEN** Apple and Android packages build with the candidate in the locked workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, callback, host object, or runtime fallback

#### Scenario: Preserve selected application behavior

- **WHEN** production controllers activate, publish current limits, encounter malformed/timeout work, background, reconnect, or reset
- **THEN** existing Swift and Kotlin owners SHALL retain all selected scheduling, bridge invocation, model mapping, state publication, and failure behavior during this change

#### Scenario: Preserve OS capability ownership

- **WHEN** the composed candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, sessions, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare runtime selection and bridge deletion

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL expose one direct safe Rust current-limit application value for later runtime composition, no new bridge crate SHALL exist, and later host cutovers SHALL delete superseded native application behavior and the no-longer-used controller-data bridges
