## ADDED Requirements

### Requirement: Rust Refloat telemetry service candidate ownership

The unselected safe Rust Refloat telemetry-service candidate SHALL live at `backends/core/refloat-telemetry-service` above the safe telemetry decoders and request scheduler and below future runtime execution. Apple and Android SHALL retain their selected Swift and Kotlin telemetry services until a later unified-runtime cutover proves production Rust execution and deletes the superseded application policy.

#### Scenario: Locate candidate and selected owners
- **WHEN** a maintainer traces layout retries, telemetry request cadence, compact-cache merging, or normalized sequencing
- **THEN** the safe candidate SHALL be plainly identified under `backends/core` while selected production owners remain in each backend's `application` tree

#### Scenario: Keep the candidate out of host packages
- **WHEN** Apple and Android packages build with the candidate present in the workspace
- **THEN** it SHALL add no native library, FFI/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, runtime fallback, or host callback

#### Scenario: Preserve selected application behavior
- **WHEN** production discovery establishes telemetry, receives decoded records, changes settings, reconnects, backgrounds, stops, or resets a session
- **THEN** the existing native services SHALL retain all timer execution, scheduler submission, state publication, persistence, and failure behavior during this change

#### Scenario: Preserve OS capability ownership
- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare bridge deletion instead of another bridge
- **WHEN** maintainers inspect the migration path after this extraction
- **THEN** the candidate SHALL be documented for direct composition in one unified Rust runtime and no telemetry-service-specific Apple FFI or Android JNI crate SHALL exist
