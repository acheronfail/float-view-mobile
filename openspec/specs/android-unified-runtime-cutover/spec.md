# android-unified-runtime-cutover Specification

## Purpose
TBD - created by archiving change cut-over-android-to-unified-rust-runtime. Update Purpose after archive.
## Requirements
### Requirement: Android selects one unified Rust runtime

The Android backend SHALL select one process-owned unified Rust application runtime through one bounded callback-free JNI boundary and SHALL retain Bluetooth, execution, lifecycle/background service, permissions, location acquisition, native file presentation, theme/UI, loopback transport/authentication, product catalogue, and import/export presentation as Kotlin-owned OS responsibilities.

#### Scenario: Execute an ordered runtime result

- **WHEN** Android supplies a valid platform fact or frontend operation to the runtime
- **THEN** Kotlin validates the complete result and executes its bounded effects in order without a native protocol/application/session-decision fallback

### Requirement: Android cutover deletes superseded ownership

The same change that selects the runtime SHALL delete the superseded Kotlin application/protocol/session decision owners, ten protocol JNI adapters and crates, JNI support crate, tests of removed private types, per-protocol task/property/source-set entries, bridge catalog/helper, and generated protocol artifacts.

#### Scenario: Inspect production and build inputs

- **WHEN** repository audits inspect Kotlin/Rust sources, Gradle and root recipes, symbols, dependencies, and generated package directories
- **THEN** no retired decision owner, protocol bridge, fallback import, JNI symbol, build task/property, or protocol library remains

### Requirement: Android packages one coarse runtime library

Android Debug and Release builds SHALL source-build and package exactly one unified runtime JNI library for each supported ABI, and JVM tests SHALL load exactly one host runtime library through the same closed build definition.

#### Scenario: Build clean Android packages

- **WHEN** clean host-test and four-ABI Debug/Release packaging runs
- **THEN** each expected output contains the runtime library with the required architecture/symbols and contains no protocol-specific Rust library

### Requirement: Final convergence is enforced

The completed cutover SHALL reduce the combined tracked production Rust, Apple, and Android source below 59,280 lines, reduce workspace members below 34, and reduce temporary protocol bridge crates and target artifacts to zero while retaining one thin host binding per selected platform.

#### Scenario: Audit the completed migration

- **WHEN** convergence audits compare the recorded pre-runtime, Apple-cutover, and final states
- **THEN** every source/member/bridge/artifact limit passes and future reintroduction fails the normal verification gate

### Requirement: Complete software and requested physical evidence

The cutover SHALL pass focused shared-boundary/ABI/adapter/persistence tests, warnings-denied linting, clean packaging, strict OpenSpec validation, and complete `just verify`. It SHALL rerun canonical physical conformance on the connected iOS device with the ESP32 fixture after shared-boundary changes and SHALL explicitly record that Android physical testing was skipped.

#### Scenario: Accept the deletion-bearing cutover

- **WHEN** all software gates and the connected iOS fixture matrix pass
- **THEN** Android runtime ownership and final bridge retirement may be committed without claiming Android hardware evidence
