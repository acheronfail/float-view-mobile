## ADDED Requirements

### Requirement: Rust Refloat configuration service candidate ownership

The unselected Rust Refloat configuration service candidate SHALL live at `backends/core/refloat-configuration-service` above the safe configuration codec and below native schedulers, application publication, persistence, loopback transport, and OS capabilities. Apple and Android SHALL retain their selected native configuration services until separate host-cutover changes prove production Rust execution and delete the superseded decision logic.

#### Scenario: Keep the candidate platform neutral

- **WHEN** maintainers inspect its public API and dependency graph
- **THEN** the candidate SHALL expose only bounded Rust facts, events, effects, snapshots, values, routes, generations, operations, and stable errors
- **AND** it SHALL contain no Swift, Kotlin, JNI, C ABI, callback, opaque foreign handle, generated binding, Bluetooth, scheduler, storage, SQLite, loopback, frontend, clock, thread, filesystem, networking, or OS-framework dependency

#### Scenario: Keep host execution outside the candidate

- **WHEN** the candidate emits request or cache effects
- **THEN** existing native owners SHALL remain responsible for VESC envelope construction, framing, route submission, deadlines, cache I/O, persistence, state publication, and frontend responses
- **AND** the candidate SHALL retain no host reference or execute a host effect

#### Scenario: Preserve production source selection

- **WHEN** Apple and Android packages build with the candidate present
- **THEN** Swift and Kotlin SHALL continue selecting their existing configuration services and existing Rust codec bridges
- **AND** the candidate SHALL add no package input, native library, bridge symbol, Gradle task, SwiftPM product, Xcode phase, runtime fallback, or committed generated artifact

#### Scenario: Preserve every OS capability boundary

- **WHEN** production and physical regression run
- **THEN** Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebViews, SQLite, loopback transport, and host UI integration SHALL remain unchanged in their existing platform owners

#### Scenario: Define the deletion path

- **WHEN** later host cutovers select the Rust service and the unified runtime absorbs scheduling and application behavior
- **THEN** they SHALL delete the Swift/Kotlin state machines, both configuration codec bridge crates, both canonical-record adapters, and their build catalogue entries rather than preserve parallel or generated APIs
