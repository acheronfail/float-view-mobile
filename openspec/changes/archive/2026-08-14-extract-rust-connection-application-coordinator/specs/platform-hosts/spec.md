## ADDED Requirements

### Requirement: Rust connection application coordinator candidate ownership

The unselected safe Rust connection application coordinator candidate SHALL live at `backends/core/connection-application-coordinator` above controller discovery, request scheduling, Refloat telemetry, and VESC BMS telemetry and below future unified runtime execution. Apple and Android SHALL retain their selected application-service orchestration until later runtime cutovers prove production Rust execution and delete both native paths.

#### Scenario: Locate candidate and selected owners

- **WHEN** a maintainer traces connection generations, request attribution, discovery sequencing, readiness, telemetry/BMS startup, child timers, or atomic reset
- **THEN** the safe candidate SHALL be plainly identified under `backends/core` while production execution remains in each backend's existing application service and helpers

#### Scenario: Keep the candidate out of host packages

- **WHEN** Apple and Android packages build with the candidate in the workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, runtime fallback, callback, or host object

#### Scenario: Preserve selected application ownership

- **WHEN** production connections identify, select, negotiate, activate, poll, publish, background, reconnect, fail, or reset
- **THEN** existing Swift and Kotlin owners SHALL retain all selected execution, persistence, publication, background behavior, and failure handling during this change

#### Scenario: Preserve OS capability ownership

- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, loopback transport, sessions, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare native and bridge deletion

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL be documented for direct unified-runtime composition and no connection-coordinator Apple FFI or Android JNI crate SHALL exist

### Requirement: Runtime activation preserves native persistence boundary

The coordinator candidate SHALL separate supported-controller readiness from application activation so the selected native persistence/session owners remain authoritative until their own Rust migration changes.

#### Scenario: Await current persistence confirmation

- **WHEN** controller discovery reaches a supported ready decision
- **THEN** the candidate SHALL expose the exact selected facts and SHALL NOT start telemetry, attached BMS work, durable session behavior, or ready publication until a current generation-qualified activation command arrives

#### Scenario: Reject failed or stale activation

- **WHEN** persistence/session preparation fails or activation belongs to an old, reset, failed, standalone-BMS, or already-active generation
- **THEN** the candidate SHALL start no child service and SHALL leave current state unchanged apart from a bounded error or stale diagnostic
