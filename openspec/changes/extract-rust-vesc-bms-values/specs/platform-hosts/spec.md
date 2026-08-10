## ADDED Requirements

### Requirement: Rust VESC BMS values candidate ownership

The focused Rust BMS values candidate SHALL live at `backends/core/vesc-bms-values` and SHALL remain independent of framing, firmware identity and custom-configuration bridges, host frameworks, CAN routing, compatibility, discovery, scheduling, polling, application coordination, persistence, transport, and frontend code. Apple and Android SHALL continue to locate their selected production command-96 decoders beneath their respective `protocol/vesc` trees until separate host-specific cutovers pass focused, packaging, canonical, and physical verification and delete the corresponding native structural parser.

#### Scenario: Locate candidate and production owners

- **WHEN** a maintainer traces command-96 BMS values decoding during this extraction step
- **THEN** the safe candidate is plainly identified under `backends/core` while the selected Swift and Kotlin production owners remain beneath each backend's `protocol/vesc` path

#### Scenario: Reject accidental host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** the candidate adds no C or JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path

#### Scenario: Preserve BMS application ownership

- **WHEN** the candidate decodes command-96 payloads in component tests
- **THEN** compatibility, discovery, direct/CAN selection, request scheduling, polling, observed-time assignment, availability, state publication, and live-only policy remain with their native owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the BMS values candidate is built and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain unchanged in their existing platform owners
