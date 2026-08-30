## ADDED Requirements

### Requirement: Rust session SQLite writer candidate ownership

The unselected Rust session SQLite writer SHALL live at `backends/core/session-sqlite-writer` below future Rust catalog, application, and loopback-runtime ownership. Apple and Android SHALL retain their complete selected stores until later direct Rust runtime cutovers prove production execution and delete both native paths.

#### Scenario: Locate candidate and selected owners

- **WHEN** a maintainer traces session schema creation, telemetry/location transactions, derived events/fields, or commit-marker summaries
- **THEN** the Rust candidate SHALL be plainly identified under `backends/core` while production execution remains in each backend's existing `SQLiteLogStore`

#### Scenario: Keep the writer out of host packages

- **WHEN** Apple and Android packages build with the candidate in the locked workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged SQLite copy, callback, host object, or runtime fallback

#### Scenario: Preserve selected storage behavior

- **WHEN** production sessions are created, recorded, queried, pruned, recovered, imported, exported, deleted, backgrounded, or closed
- **THEN** existing Swift and Kotlin stores, schemas, reducers, cursor codecs, loopback handlers, and application services SHALL retain all selected behavior during this change

#### Scenario: Preserve OS capability ownership

- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location acquisition, lifecycle, permissions, native files, theme, WebViews, production SQLite, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare direct runtime composition and bridge deletion

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL expose one direct safe Rust writer boundary for future catalog/runtime composition, no writer bridge crate SHALL exist, and later host cutovers SHALL delete both native stores and every protocol bridge unused by the complete Rust runtime
