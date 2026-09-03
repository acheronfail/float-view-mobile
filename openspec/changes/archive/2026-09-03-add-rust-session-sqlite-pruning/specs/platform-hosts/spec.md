## ADDED Requirements

### Requirement: Rust session SQLite pruning candidate ownership

The unselected Rust session-pruning implementation SHALL live inside `backends/core/session-sqlite`, directly below the future Rust catalog/application/runtime and directly above `session-history-kernel`. Apple and Android SHALL retain their complete selected pruning paths until separate runtime cutovers prove production execution and delete superseded native behavior.

#### Scenario: Locate candidate and selected owners

- **WHEN** a maintainer traces prune preview, retained-row selection, SQLite deletion, metadata/count revision, catalog reconciliation, or reclamation
- **THEN** platform-neutral selection and per-session mutation candidates SHALL be plainly identified in the two Rust core crates while selected execution and runtime policy remain in each backend's existing `SQLiteLogStore`

#### Scenario: Keep pruning out of host builds

- **WHEN** Apple and Android packages build with the candidate in the locked workspace
- **THEN** it SHALL add no native library, C/JNI symbol, Gradle task, SwiftPM product, Xcode phase, generated binding, callback, host object, executor, packaged SQLite copy, or runtime fallback

#### Scenario: Preserve platform capabilities and storage behavior

- **WHEN** production sessions are created, recorded, queried, pruned, backgrounded, recovered, imported, exported, deleted, or closed
- **THEN** existing Swift and Kotlin stores plus Bluetooth, background execution, location acquisition, lifecycle, permissions, native files, theme, WebViews, loopback transport, and UI integration SHALL retain all selected behavior

#### Scenario: Prepare direct runtime composition

- **WHEN** maintainers inspect the migration path
- **THEN** the candidate SHALL expose one direct safe Rust pruning boundary for future catalog/runtime composition, no pruning-specific FFI/JNI crate SHALL exist, and later complete host cutovers SHALL delete superseded native pruning and protocol bridges unused by the Rust runtime

