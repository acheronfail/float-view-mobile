## ADDED Requirements

### Requirement: Rust Refloat compatibility candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to constructing the unframed Refloat INFO request, decoding bounded INFO response identity, selecting exact audited compatibility and control profiles, and admitting required dynamic telemetry identifiers. It SHALL own no VESC framing, direct/CAN routing, request scheduling, connection generation, discovery state, telemetry value decoding, application state, persistence, transport, or OS capability.

The candidate SHALL preserve command `36`, package ID `101`, INFO command `0`, requested protocol version `2`, zero request flags, legacy compact wire identity without an invented patch, version-2 major/minor/patch offsets, the exact source-release catalogue from 1.0.0 through 1.2.3, closed legacy/dynamic/dynamic-alert telemetry profiles, legacy/stable lighting profiles, fatal-clear availability, and the exact required always-present and running-only dynamic identifiers. It SHALL enforce the existing 4,096-byte payload bound and bounded layout names, return no partial result, expose only payload-safe failures, forbid unsafe code, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Construct and decode source INFO forms

- **WHEN** the candidate constructs an INFO request or decodes a valid minimal/source legacy compact or complete version-2 response
- **THEN** it returns the exact request bytes and structural wire identity, preserving legacy `1.0` without fabricating patch `.0`

#### Scenario: Select every audited profile exactly

- **WHEN** the candidate looks up any audited wire identity for source releases 1.0.0–1.0.1, 1.1.0–1.1.2, or 1.2.0–1.2.3
- **THEN** it returns the exact source entries, telemetry family, lighting profile, and fatal-clear availability without range inference

#### Scenario: Admit a complete bounded dynamic layout

- **WHEN** both identifier groups contain every exact required name plus any bounded unique future names in any order
- **THEN** the candidate accepts the layout without moving value decoding, layout lifetime, or readiness state into the component

#### Scenario: Reject unsupported or malformed compatibility input

- **WHEN** an INFO payload has a wrong envelope, unsupported protocol byte, truncated version-2 prefix, excessive extent, or a layout has invalid bounds, duplicates, a missing required identifier, or an identifier in the wrong group
- **THEN** the candidate fails closed with a stable bounded error and exposes no partial identity, profile, layout decision, or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for INFO decoding and compatibility admission and no supported-backend or native-code deletion claim is made
