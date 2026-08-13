## ADDED Requirements

### Requirement: Rust Refloat dynamic telemetry candidate ownership

The focused Rust Refloat dynamic telemetry candidate SHALL live at `backends/core/refloat-dynamic-telemetry` and SHALL remain independent of VESC framing, firmware identity, legacy compact telemetry, Refloat controls and configuration, host frameworks, compatibility catalogues, CAN routing, scheduling, polling, reconnect generations, compact-cache merging, application coordination, persistence, transport, and frontend code. Apple and Android SHALL continue to locate their selected production command-31/32 decoders beneath their respective `protocol/refloat` trees until separate host-specific cutovers pass focused, packaging, canonical, and physical verification and delete the corresponding native decoder.

#### Scenario: Locate candidate and production owners

- **WHEN** a maintainer traces Refloat command-31/32 decoding during this extraction step
- **THEN** the safe candidate is plainly identified under `backends/core` while the selected Swift and Kotlin production owners remain in each backend's Refloat protocol tree

#### Scenario: Reject accidental host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, generated artifacts, and bridge build entries
- **THEN** the candidate adds no C or JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path

#### Scenario: Preserve telemetry application ownership

- **WHEN** the candidate decodes negotiated dynamic telemetry in component tests
- **THEN** compatibility selection, layout replacement, route selection, framing, scheduling, polling, cache merging, observation time, live state, sessions, persistence, and exports remain with their existing owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the dynamic telemetry candidate is built and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, SQLite, and loopback transport remain unchanged in their existing platform owners
