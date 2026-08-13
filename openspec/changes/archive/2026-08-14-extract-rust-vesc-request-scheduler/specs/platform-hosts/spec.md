## ADDED Requirements

### Requirement: Rust VESC request scheduler candidate ownership

The unselected safe Rust request scheduler candidate SHALL live at `backends/core/vesc-request-scheduler` below native application coordination and above host BLE/timer execution. Apple and Android SHALL retain their selected Swift and Kotlin connection-wide schedulers until later host or unified-runtime cutovers prove production Rust execution and delete the superseded decision logic.

#### Scenario: Locate candidate and selected owners
- **WHEN** a maintainer traces request arbitration, response attribution, cancellation, or deadline behavior
- **THEN** the safe candidate SHALL be plainly identified under `backends/core` while selected production owners remain in each backend's `application` tree

#### Scenario: Keep the candidate out of host packages
- **WHEN** Apple and Android packages build with the candidate present in the workspace
- **THEN** the candidate SHALL add no native library, bridge symbol, Gradle task, SwiftPM product, Xcode phase, runtime fallback, or generated artifact to either host

#### Scenario: Preserve application ownership
- **WHEN** production discovery, telemetry, BMS, control, configuration, reconnect, or session work submits and receives VESC requests
- **THEN** the existing native schedulers and application services SHALL retain all selected execution, publication, and failure behavior during this change

#### Scenario: Preserve OS capability ownership
- **WHEN** the scheduler candidate is built and tested
- **THEN** Bluetooth, background work, location, lifecycle, permissions, native files, theme, WebViews, SQLite, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare bridge deletion rather than another bridge
- **WHEN** maintainers inspect the migration path after this extraction
- **THEN** the candidate SHALL be documented for direct composition in one unified Rust runtime and SHALL introduce no `vesc-request-scheduler-apple-ffi` or `vesc-request-scheduler-android-jni` crate
