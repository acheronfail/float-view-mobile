## ADDED Requirements

### Requirement: Rust Refloat compatibility candidate ownership

The focused Rust Refloat compatibility candidate SHALL live at `backends/core/refloat-compatibility` and SHALL remain independent of VESC framing, host bridges and frameworks, telemetry value codecs, Refloat configuration, CAN routing, scheduling, connection generations, controller-discovery effects, application coordination, persistence, transport, frontend code, and OS capabilities. Apple and Android SHALL continue to locate their selected INFO decoders and compatibility owners beneath their respective `protocol/refloat` trees until separate host-specific cutovers pass focused, packaging, canonical, and physical verification and delete the corresponding native implementation.

#### Scenario: Locate candidate and production owners

- **WHEN** a maintainer traces Refloat INFO identity or package/profile admission during this extraction step
- **THEN** the safe candidate is plainly identified under `backends/core` while selected Swift and Kotlin owners remain beneath each backend's Refloat protocol path

#### Scenario: Reject accidental host integration

- **WHEN** repository verification inspects bridge symbols, build catalogues, native package dependencies, application sources, and generated artifacts
- **THEN** the candidate adds no C or JNI symbol, native linkage, generated binding, packaged binary, callback, or production call path

#### Scenario: Preserve controller-discovery ownership

- **WHEN** candidate tests construct an INFO request, decode an identity, select a profile, or admit a layout
- **THEN** route probing, request scheduling, generations, timeouts, readiness effects, saved-device/session behavior, state publication, and issue presentation remain with their native application owners

#### Scenario: Preserve OS capability ownership

- **WHEN** the compatibility candidate is built and tested
- **THEN** BLE, background work, location, lifecycle, files, theme, permissions, WebViews, SQLite, and loopback transport remain unchanged in their existing platform owners
