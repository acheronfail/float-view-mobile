## ADDED Requirements

### Requirement: Rust firmware identity candidate ownership

The focused Rust firmware identity candidate SHALL live at `backends/core/vesc-firmware-identity` and SHALL remain independent of framing bridges, host frameworks, Bluetooth, application coordination, persistence, transport, and frontend code. Apple and Android SHALL continue to locate their selected production firmware identity decoders beneath their respective `protocol/vesc` trees until separate host-specific cutovers pass focused, packaging, canonical, and physical verification and delete the corresponding superseded native parser.

#### Scenario: Locate candidate and production owners

- **WHEN** a maintainer traces VESC firmware identity decoding during this extraction step
- **THEN** the Rust candidate is plainly identified under `backends/core` while the selected Swift and Kotlin production owners remain under each backend's `protocol/vesc` path

#### Scenario: Reject accidental host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** the candidate adds no C or JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path

#### Scenario: Preserve OS capability ownership

- **WHEN** the firmware identity candidate is built and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, storage, and loopback transport remain unchanged in their existing platform owners
