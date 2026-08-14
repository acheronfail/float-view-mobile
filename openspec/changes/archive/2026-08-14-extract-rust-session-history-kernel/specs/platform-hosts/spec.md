## ADDED Requirements

### Requirement: Rust session-history kernel candidate ownership

The unselected safe Rust session-history candidate SHALL live at `backends/core/session-history-kernel` below future Rust SQLite/session and loopback-runtime ownership. Apple and Android SHALL retain their selected session reducers and complete storage/application owners until later direct Rust runtime cutovers prove production execution and delete both native paths.

#### Scenario: Locate candidate and selected owners

- **WHEN** a maintainer traces historical M4 reduction, continuity generations, or prune-row selection
- **THEN** the safe candidate SHALL be plainly identified under `backends/core` while production execution remains in each backend's existing application reducer and SQLite store

#### Scenario: Keep the candidate out of host packages

- **WHEN** Apple and Android packages build with the candidate in the locked workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged artifact, callback, host object, or runtime fallback

#### Scenario: Preserve selected session behavior

- **WHEN** production sessions are created, recorded, queried, paged, pruned, recovered, imported, exported, deleted, backgrounded, or closed
- **THEN** existing Swift and Kotlin reducers, SQLite stores, cursor codecs, loopback handlers, and application services SHALL retain all selected behavior during this change

#### Scenario: Preserve OS capability ownership

- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, sessions, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare persistence composition and bridge deletion

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL expose one direct safe Rust history boundary for the future SQLite/session runtime, no session-history bridge crate SHALL exist, and later host cutovers SHALL delete both native reducers and all protocol bridges unused by the complete Rust runtime
