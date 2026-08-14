## ADDED Requirements

### Requirement: Rust VESC BMS telemetry service candidate ownership

The unselected safe Rust VESC BMS telemetry-service candidate SHALL live at `backends/core/vesc-bms-telemetry-service` above auxiliary discovery, BMS values decoding, and request-scheduler facts and below future unified runtime execution. Apple and Android SHALL retain their selected application-service discovery and polling owners until later runtime cutover proves production Rust execution and deletes both native lifecycles.

#### Scenario: Locate candidate and selected owners

- **WHEN** a maintainer traces BMS discovery request execution, polling cadence, failure threshold, observation ownership, unavailability, recovery, or reset
- **THEN** the safe candidate SHALL be plainly identified under `backends/core` while production execution remains in each backend's existing application service and auxiliary discovery sources

#### Scenario: Keep the candidate out of host packages

- **WHEN** Apple and Android packages build with the candidate in the workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, runtime fallback, or host callback

#### Scenario: Preserve selected application ownership

- **WHEN** production connections discover, select, request, decode, attach, lose, recover, publish, reconnect, or reset a BMS
- **THEN** existing Swift and Kotlin owners SHALL retain all selected execution, timing, decoding adapters, publication, background behavior, and failure handling during this change

#### Scenario: Preserve OS capability ownership

- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, loopback transport, sessions, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare bridge deletion rather than another bridge

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL be documented for direct unified-runtime composition and no BMS telemetry-service Apple FFI or Android JNI crate SHALL exist
