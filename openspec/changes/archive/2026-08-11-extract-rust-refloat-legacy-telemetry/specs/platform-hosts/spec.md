## ADDED Requirements

### Requirement: Rust Refloat legacy telemetry candidate ownership

The focused Rust Refloat legacy telemetry candidate SHALL live at `backends/core/refloat-legacy-telemetry` and SHALL remain independent of VESC framing, firmware identity, custom configuration, BMS decoding, Refloat dynamic telemetry and controls, host frameworks, CAN routing, compatibility catalogues, scheduling, compact-cache merging, application coordination, persistence, transport, and frontend code. Apple and Android SHALL continue to locate their selected production command-10 decoders beneath their respective `protocol/refloat` trees until separate host-specific cutovers pass focused, packaging, canonical, and physical verification and delete the corresponding native decoder.

#### Scenario: Locate candidate and production owners

- **WHEN** a maintainer traces Refloat command-10 legacy compact decoding during this extraction step
- **THEN** the safe candidate is plainly identified under `backends/core` while the selected Swift and Kotlin production owners remain in each backend's Refloat protocol tree

#### Scenario: Reject accidental host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, generated artifacts, and bridge build entries
- **THEN** the candidate adds no C or JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path

#### Scenario: Preserve telemetry application ownership

- **WHEN** the candidate decodes compact telemetry in component tests
- **THEN** compatibility selection, request construction, route selection, framing, scheduling, cache merging, observation time, live state, sessions, persistence, and exports remain with their existing owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the legacy telemetry candidate is built and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, SQLite, and loopback transport remain unchanged in their existing platform owners
