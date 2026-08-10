## ADDED Requirements

### Requirement: Rust VESC controller-data candidate ownership

The focused Rust VESC controller-data candidate SHALL live at `backends/core/vesc-controller-data` and SHALL remain independent of framing, firmware identity, custom configuration, BMS decoding, Refloat protocols, host frameworks, CAN routing, compatibility catalogues, scheduling, application coordination, persistence, transport, and frontend code. Apple and Android SHALL continue to locate their selected production decoders in `VescProtocolService` beneath their respective protocol trees until separate host-specific cutovers pass focused, packaging, canonical, and physical verification and delete the corresponding native parsing.

#### Scenario: Locate candidate and production owners

- **WHEN** a maintainer traces command-4 realtime or command-14 current-limit decoding during this extraction step
- **THEN** the safe candidate is plainly identified under `backends/core` while the selected Swift and Kotlin production owners remain in each backend's VESC protocol service

#### Scenario: Reject accidental host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** the candidate adds no C or JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path

#### Scenario: Preserve VESC application ownership

- **WHEN** the candidate decodes controller data in component tests
- **THEN** request construction, firmware compatibility, direct/CAN selection, framing, scheduling, telemetry polling, fault response, and state publication remain with their existing owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the controller-data candidate is built and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, SQLite, and loopback transport remain unchanged in their existing platform owners
