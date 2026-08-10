## ADDED Requirements

### Requirement: Apple Rust Refloat control codec cutover

The shared Apple backend SHALL use the safe Rust Refloat control component as its sole production implementation of package, flywheel, legacy/stable lighting, zero-mask lighting-query, and fatal-clear custom-app-data payload encoding and lighting-response structural decoding on iOS and macOS. Rust SHALL decide the VESC command and Refloat package prefix, control command and parameter bytes, inverted package semantics, flywheel defaults and footpad-abort byte, lighting profile/reserved bytes/mask/flags/query behavior, fatal-clear availability, exact response extent and prefix, defined response bits, and every structural or unsupported failure. Swift MAY perform only bounded versioned bridge validation, primitive model conversion, caller-owned byte copying, and established error mapping.

The cutover SHALL use a narrow audited Apple bridge, SHALL contain no runtime or source fallback to Swift control-wire algorithms, and SHALL delete the superseded Swift implementation after required verification. Compatibility lookup, route selection, VESC framing, mutation authorization, scheduling, confirmation, telemetry, application state, and Android Refloat control ownership SHALL remain outside this change.

#### Scenario: Encode Apple production controls

- **WHEN** shared Apple production requests any supported package, flywheel, lighting mutation/query, or fatal-clear control
- **THEN** the thin Swift adapter obtains the exact bounded payload from Rust and executes no Swift command, parameter, mask, flag, reserved-byte, query, or fatal-availability algorithm

#### Scenario: Decode Apple production lighting

- **WHEN** shared Apple production receives a valid legacy or stable lighting response
- **THEN** Rust validates its complete structure and Swift mechanically maps the returned two-bit result into the existing lighting state

#### Scenario: Preserve established Apple failures

- **WHEN** Rust reports an invalid caller contract, malformed response, unsupported command, insufficient output, ABI mismatch, invalid primitive result, or contained internal failure
- **THEN** Swift returns the established bounded protocol error without logging payload/state, publishing a partial result, or executing a fallback codec

#### Scenario: Preserve higher-level Refloat owners

- **WHEN** Rust returns a control payload or decoded lighting state
- **THEN** existing Swift compatibility, routing, framing, authorization, scheduling, confirmation, telemetry, and application-state owners behave unchanged after representation mapping

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple Refloat control cutover completes
- **THEN** Android continues using its Kotlin codec with unchanged behavior and without linking, packaging, or calling the Apple bridge
