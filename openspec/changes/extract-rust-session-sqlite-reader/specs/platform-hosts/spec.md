## RENAMED Requirements

- FROM: `### Requirement: Rust session SQLite writer candidate ownership`
- TO: `### Requirement: Rust session SQLite candidate ownership`

## MODIFIED Requirements

### Requirement: Rust session SQLite candidate ownership

The unselected Rust per-session SQLite writer and reader SHALL live together at `backends/core/session-sqlite` below future Rust catalog, application, and loopback-runtime ownership. Apple and Android SHALL retain their complete selected stores until later direct Rust runtime cutovers prove production execution and delete both native paths.

#### Scenario: Locate candidate and selected owners

- **WHEN** a maintainer traces session schema creation, telemetry/location transactions, derived events/fields, commit-marker summaries, published-boundary validation, historical series, sample/event/location scans, or interruption
- **THEN** the consolidated Rust candidate SHALL be plainly identified under `backends/core/session-sqlite` while production execution remains in each backend's existing `SQLiteLogStore`

#### Scenario: Keep the package out of host builds

- **WHEN** Apple and Android packages build with the candidate in the locked workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, packaged SQLite copy, callback, host object, executor, or runtime fallback

#### Scenario: Remove the writer-only package identity

- **WHEN** maintainers inspect the retained Rust workspace, fixtures, recipes, audits, and ownership documentation
- **THEN** `session-sqlite` SHALL be the only per-session SQLite candidate and the superseded `session-sqlite-writer` crate, package, fixture path, recipe names, and possible bridge paths SHALL be absent

#### Scenario: Preserve selected storage behavior

- **WHEN** production sessions are created, recorded, queried, pruned, recovered, imported, exported, deleted, backgrounded, or closed
- **THEN** existing Swift and Kotlin stores, schemas, reducers, cursor codecs, loopback handlers, cancellation executors, and application services SHALL retain all selected behavior during this change

#### Scenario: Preserve OS capability ownership

- **WHEN** the candidate is built and tested
- **THEN** Bluetooth, background execution, location acquisition, lifecycle, permissions, native files, theme, WebViews, production SQLite, loopback transport, and UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Prepare direct runtime composition and bridge deletion

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL expose direct safe Rust writer, reader, and reduction boundaries for future catalog/runtime composition, no session SQLite bridge crate SHALL exist, and later host cutovers SHALL delete both native stores and every protocol bridge unused by the complete Rust runtime
