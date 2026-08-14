## ADDED Requirements

### Requirement: Rust auxiliary-node discovery candidate ownership

The unselected safe Rust auxiliary-node discovery candidate SHALL live at `backends/core/auxiliary-node-discovery` above safe controller discovery and below future unified runtime execution. Apple and Android SHALL retain their selected `application/AuxiliaryNodeDiscovery` owners until later runtime cutover proves production Rust execution and deletes both native state machines.

#### Scenario: Locate candidate and selected owners

- **WHEN** a maintainer traces exact VBMS admission, inventory reuse, preferred routing, bounded enumeration, ambiguity, or availability
- **THEN** the safe candidate SHALL be plainly identified under `backends/core` while production behavior remains in each backend's existing `application/AuxiliaryNodeDiscovery` source

#### Scenario: Keep the candidate out of host packages

- **WHEN** Apple and Android packages build with the candidate in the workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, runtime fallback, or host callback

#### Scenario: Preserve selected application ownership

- **WHEN** production connections identify, discover, select, poll, attach, lose, recover, publish, or reset a BMS route
- **THEN** existing Swift and Kotlin auxiliary/application services SHALL retain all selected execution, timing, observation, publication, and failure behavior during this change

#### Scenario: Preserve OS capability ownership

- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare bridge deletion rather than another bridge

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL be documented for direct unified-runtime composition and no `auxiliary-node-discovery-apple-ffi` or `auxiliary-node-discovery-android-jni` crate SHALL exist
