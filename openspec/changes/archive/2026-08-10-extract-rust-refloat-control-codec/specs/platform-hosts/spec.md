## ADDED Requirements

### Requirement: Rust Refloat control codec candidate ownership

The focused Rust Refloat control candidate SHALL live at `backends/core/refloat-control` and SHALL remain independent of VESC framing, firmware identity, custom configuration, BMS decoding, host frameworks, CAN routing, compatibility catalogues, authorization, scheduling, telemetry, application coordination, persistence, transport, and frontend code. Apple and Android SHALL continue to locate their selected production codecs beneath their respective `protocol/refloat` trees until separate host-specific cutovers pass focused, packaging, canonical, and physical verification and delete the corresponding native codec.

#### Scenario: Locate candidate and production owners

- **WHEN** a maintainer traces Refloat live-control wire encoding during this extraction step
- **THEN** the safe candidate is plainly identified under `backends/core` while the selected Swift and Kotlin production owners remain beneath each backend's `protocol/refloat` path

#### Scenario: Reject accidental host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** the candidate adds no C or JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path

#### Scenario: Preserve Refloat application ownership

- **WHEN** the candidate encodes or decodes control payloads in component tests
- **THEN** compatibility lookup, direct/CAN selection, VESC framing, mutation authorization, pending-work exclusion, scheduling, confirmation, telemetry, and state publication remain with their native owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the Refloat control candidate is built and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain unchanged in their existing platform owners
