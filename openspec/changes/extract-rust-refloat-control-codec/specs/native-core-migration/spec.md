## ADDED Requirements

### Requirement: Rust Refloat control codec candidate parity

The `backends/core` workspace SHALL contain one safe Rust candidate with an empty production dependency graph dedicated to encoding bounded Refloat package, flywheel, legacy/stable lighting, and fatal-clear custom-app-data payloads and decoding bounded lighting responses. It SHALL accept typed caller-provided control profile and lighting state values without owning compatibility lookup, VESC framing, CAN routing, mutation authorization, scheduling, confirmation, telemetry, application state, or persistence.

The candidate SHALL preserve VESC command `36`, Refloat package ID `101`, exact Refloat command and parameter bytes, inverted package enable semantics, flywheel mode/default/footpad-abort bytes, lighting profile and mask/flag behavior, zero-mask query behavior, fatal-clear availability, exact four-byte response extent, defined lighting bits, and unsupported-command versus malformed failure behavior. It SHALL return statically bounded payloads, forbid unsafe code, log no payload or state, and remain unselected by Swift and Kotlin until separate host cutovers.

#### Scenario: Encode exact source-derived controls

- **WHEN** the candidate receives each committed package, flywheel, legacy/stable lighting, or fatal-clear input
- **THEN** it returns the exact source-derived custom-app-data payload selected native codecs produce

#### Scenario: Preserve lighting state and query semantics

- **WHEN** the candidate encodes either lighting profile for every current-state, target, and enabled combination or for no target
- **THEN** it changes only the selected bit, preserves the other bit, uses the exact mask, and emits a zero-mask query without changing the supplied flags

#### Scenario: Decode exact lighting responses

- **WHEN** the candidate receives an exact four-byte response for the supplied legacy or stable profile with any defined two-bit lighting state
- **THEN** it returns the matching LED and headlight booleans

#### Scenario: Reject unavailable or malformed controls

- **WHEN** fatal clear is unavailable or a lighting response has an empty, short, long, wrong-prefix, wrong-package, wrong-command, wrong-profile, or undefined-flag form
- **THEN** the candidate returns the matching bounded unsupported-command or malformed failure and exposes no partial state or payload diagnostic

#### Scenario: Preserve native production ownership

- **WHEN** the candidate passes component and repository verification without a new Apple or Android bridge operation
- **THEN** Swift and Kotlin remain selected for Refloat live-control encoding and response decoding and no supported-backend or native-code deletion claim is made
