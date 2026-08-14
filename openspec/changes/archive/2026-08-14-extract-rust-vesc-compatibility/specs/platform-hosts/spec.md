## ADDED Requirements

### Requirement: Rust VESC compatibility candidate ownership

The unselected safe Rust VESC compatibility candidate SHALL live at `backends/core/vesc-compatibility` immediately above the safe firmware-identity component and below future controller discovery and mutation coordination. Apple and Android SHALL retain their selected Swift and Kotlin compatibility owners until later unified-runtime cutover deletes them.

#### Scenario: Locate candidate and selected owners
- **WHEN** a maintainer traces supported VESC versions, test-build access, read/write authorization, or shutdown availability
- **THEN** the safe candidate SHALL be plainly identified under `backends/core` while selected production policy remains in each backend's `protocol/vesc` tree

#### Scenario: Keep the candidate out of host packages
- **WHEN** Apple and Android packages build with the candidate in the workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, runtime fallback, or host callback

#### Scenario: Preserve selected application ownership
- **WHEN** production discovery evaluates identities or application code authorizes reads, writes, and shutdown
- **THEN** existing native compatibility, discovery, routing, scheduling, mutation, state publication, and failure owners SHALL remain selected during this change

#### Scenario: Preserve OS capability ownership
- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare bridge deletion rather than another bridge
- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL be documented for direct Rust controller-discovery composition and no `vesc-compatibility-apple-ffi` or `vesc-compatibility-android-jni` crate SHALL exist
