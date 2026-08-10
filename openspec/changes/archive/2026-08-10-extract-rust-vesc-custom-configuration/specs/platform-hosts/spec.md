## ADDED Requirements

### Requirement: Rust custom-configuration candidate ownership

The focused Rust VESC custom-configuration candidate SHALL live at `backends/core/vesc-custom-configuration` and SHALL remain independent of framing and identity bridges, host frameworks, schema/XML and typed-value codecs, Bluetooth, routing, scheduling, application coordination, persistence, transport, and frontend code. Apple and Android SHALL continue to locate their selected production custom-configuration codecs beneath their respective `protocol/vesc` trees until separate host-specific cutovers pass focused, packaging, canonical, and physical verification and delete the corresponding superseded native codec.

#### Scenario: Locate candidate and production owners

- **WHEN** a maintainer traces commands 92–95 during this extraction step
- **THEN** the safe candidate is plainly identified under `backends/core` while the selected Swift and Kotlin production owners remain under each backend's `protocol/vesc` path

#### Scenario: Reject accidental host integration

- **WHEN** repository verification inspects bridge symbols, native package dependencies, application sources, and generated artifacts
- **THEN** the candidate adds no C or JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path

#### Scenario: Preserve higher-level configuration ownership

- **WHEN** the candidate encodes or decodes opaque configuration envelopes in tests
- **THEN** schema assembly, decompression, XML, signatures, typed values, safety policy, mutation coordination, scheduling, routing, storage, and contract serialization remain with their existing owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the custom-configuration candidate is built and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, application behavior, storage, and loopback transport remain unchanged in their existing platform owners
