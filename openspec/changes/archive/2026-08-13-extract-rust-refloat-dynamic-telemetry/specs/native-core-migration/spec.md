## ADDED Requirements

### Requirement: Rust Refloat dynamic telemetry candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to decoding complete unframed Refloat custom-app-data command-32 layouts and command-31 dynamic telemetry. It SHALL own bounded negotiated layout representation, exact mask-dependent response structure, the source float16 format, semantic state and alert classifications, typed known fields, and bounded unknown numeric extensions without owning VESC framing, compatibility selection, direct/CAN routing, polling, layout lifetime across reconnect, compact-cache merging, application state, persistence, or transport.

The candidate SHALL accept at most 64 globally unique nonempty UTF-8 identifiers, at most 64 encoded bytes per identifier, at most 4,096 aggregate identifier bytes, and at most 32 unknown extensions. It SHALL decode the source format's complete finite `+-131008` float16 range, preserve negotiated order, source ticks, running-only values, charging data, the 1.2.x two-word active-alert tail, fatal latch, raw firmware-fault code, and stable classifications, return no partial layout or sample, expose only bounded payload-safe failures, forbid unsafe code, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Decode a complete bounded layout

- **WHEN** the candidate receives a source-derived or future-bounded command-32 layout
- **THEN** it returns the exact ordered always-present and running-only identifiers only after validating the complete envelope, UTF-8, extents, aggregate bound, count bound, and global uniqueness

#### Scenario: Decode every dynamic response form

- **WHEN** the candidate receives a valid 1.1.x or 1.2.x command-31 response against its accepted layout
- **THEN** it returns exact source ticks, semantic state, all negotiated known values, bounded extensions, running-only presence, charging values, active alerts, fatal latch, and firmware-fault information required by that form

#### Scenario: Preserve the complete source float16 range

- **WHEN** a negotiated value uses zero, a subnormal, a normal, exponent `0x1f`, or either finite saturation endpoint
- **THEN** the candidate returns the source-defined finite value with the correct sign and does not apply IEEE infinity or NaN semantics

#### Scenario: Reject malformed dynamic telemetry

- **WHEN** a layout or realtime payload has the wrong envelope, invalid UTF-8, empty or duplicate name, excessive bound, unsupported mask, inexact extent, impossible alert/fault relation, or excessive extension set
- **THEN** the candidate fails closed with a stable bounded error and exposes no partial identifiers, values, or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for dynamic layout and telemetry decoding and no supported-backend or native-code deletion claim is made
