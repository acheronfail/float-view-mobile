## ADDED Requirements

### Requirement: Rust Refloat legacy telemetry candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to decoding complete unframed Refloat custom-app-data command-10 compact telemetry. It SHALL accept the exact active-firmware-fault form and detail levels 1 through 4 without owning VESC framing, direct/CAN routing, compatibility, polling, dynamic field negotiation, application state, persistence, or transport.

The candidate SHALL preserve the VESC custom-app-data command and Refloat package/command identifiers; exact form and detail-level lengths; big-endian signed scales; one metres/second-to-kilometres/hour conversion; finite VESC auto-float distance; legacy package state, mode, footpad, stop, adjustment, alert, and firmware-fault mappings; all fixed and optional detail fields; and raw unknown fault classification. It SHALL expose the source `222` field-weakening sentinel as no value, forbid unsafe code, return allocation-free fixed-size output, expose only bounded payload-safe failures, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Decode every compact detail level

- **WHEN** the candidate receives each committed source-derived command-10 detail-level 1 through 4 payload
- **THEN** it returns every field present at that level with exact units and semantics while fields belonging only to later levels remain absent

#### Scenario: Decode an active firmware fault

- **WHEN** the candidate receives the exact five-byte command-10 fault form with a nonzero known or unknown fault code
- **THEN** it returns the raw code and stable firmware-fault classification without fabricating a normal telemetry sample

#### Scenario: Preserve unavailable field weakening

- **WHEN** a valid compact response carries upstream sentinel byte `222` in the field-weakening position
- **THEN** the candidate returns no field-weakening current rather than reporting the sentinel as a measured value

#### Scenario: Reject malformed legacy telemetry

- **WHEN** a payload has the wrong envelope, package, or command; an unsupported detail level; a zero fault code; an inexact form length; a truncated field; or a non-finite distance value
- **THEN** the candidate fails closed with a stable bounded error and exposes no partial values or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for legacy telemetry decoding and no supported-backend or native-code deletion claim is made
