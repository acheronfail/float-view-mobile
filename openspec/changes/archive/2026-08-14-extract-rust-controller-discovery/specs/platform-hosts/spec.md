## ADDED Requirements

### Requirement: Rust controller-discovery candidate ownership

The unselected safe Rust controller-discovery candidate SHALL live at `backends/core/controller-discovery` above safe firmware and compatibility components and below future unified runtime execution. Apple and Android SHALL retain their selected `application/ControllerDiscovery` owners until later runtime cutover proves production Rust execution and deletes both native state machines.

#### Scenario: Locate candidate and selected owners
- **WHEN** a maintainer traces direct/CAN selection, preferred or learned route recovery, enumeration, continuity, candidates, or readiness
- **THEN** the safe candidate SHALL be plainly identified under `backends/core` while production behavior remains in each backend's existing `application/ControllerDiscovery` source

#### Scenario: Keep the candidate out of host packages
- **WHEN** Apple and Android packages build with the candidate in the workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, runtime fallback, or host callback

#### Scenario: Preserve selected application ownership
- **WHEN** production connections identify a transport, probe CAN, recover continuity, publish candidates, select a route, negotiate Refloat, or fail
- **THEN** existing Swift and Kotlin discovery/application services SHALL retain all selected execution, persistence, publication, and failure behavior during this change

#### Scenario: Preserve OS capability ownership
- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare bridge deletion rather than another bridge
- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL be documented for direct unified-runtime composition and no `controller-discovery-apple-ffi` or `controller-discovery-android-jni` crate SHALL exist
