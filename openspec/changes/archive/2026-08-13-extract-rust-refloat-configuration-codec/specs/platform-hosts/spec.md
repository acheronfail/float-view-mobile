## ADDED Requirements

### Requirement: Rust Refloat configuration codec candidate ownership

The unselected Rust Refloat configuration codec candidate SHALL live under `backends/core` below the native application and VESC command-envelope layers. During this change the shared Apple backend and Android backend SHALL retain production ownership of schema decoding, fingerprint and signature reconstruction, typed value serialization, configuration lifecycle, cache, scheduling, write verification, Tune persistence, application publication, and every OS capability. The candidate SHALL own no host ABI, host callback, route, connection state, persistent record, loopback contract, Bluetooth behavior, or UI behavior.

#### Scenario: Keep both hosts selected

- **WHEN** the candidate proves parity against committed fixtures
- **THEN** iOS and macOS continue through the selected Swift codec and Android continues through the selected Kotlin codec with unchanged host records and application behavior

#### Scenario: Add a later host bridge

- **WHEN** a later Apple or Android cutover selects the candidate
- **THEN** that independently specified change adds only its temporary mechanical host boundary, proves selected-path parity, and deletes that host's superseded codec algorithm without transferring application or OS ownership

#### Scenario: Preserve stateful configuration ownership

- **WHEN** schemas or values are requested, cached, invalidated, edited, saved, written, verified, or published during this candidate change
- **THEN** the existing native configuration service retains connection-generation, flywheel, revision, conflict, scheduling, readback, and durable Tune decisions

#### Scenario: Preserve OS capability ownership

- **WHEN** production regression exercises Bluetooth, background logging, location, lifecycle, permissions, native import/export, theme, or WebView behavior
- **THEN** each platform backend continues to own those capabilities and the candidate has no dependency or callback path to them
