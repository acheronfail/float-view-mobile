## ADDED Requirements

### Requirement: Android Rust Refloat control codec cutover

The Android backend SHALL use the safe Rust Refloat control component as its sole production implementation of package, flywheel, legacy/stable lighting, zero-mask lighting-query, and fatal-clear custom-app-data payload encoding and lighting-response structural decoding. Rust SHALL decide the VESC command and Refloat package prefix, control command and parameter bytes, inverted package semantics, flywheel defaults and footpad-abort byte, lighting profile/reserved bytes/mask/flags/query behavior, fatal-clear availability, exact response extent and prefix, defined response bits, and every structural or unsupported failure. Kotlin MAY perform only bounded versioned bridge validation, primitive model conversion, caller-owned array copying, and established error mapping.

The cutover SHALL use a narrow audited Android JNI bridge, SHALL contain no runtime or source fallback to Kotlin control-wire algorithms, and SHALL delete the superseded Kotlin implementation after required verification. Compatibility lookup, route selection, VESC framing, mutation authorization, scheduling, confirmation, telemetry, application state, and Apple Refloat control ownership SHALL remain outside this change.

#### Scenario: Encode Android production controls

- **WHEN** Android production requests any supported package, flywheel, lighting mutation/query, or fatal-clear control
- **THEN** the thin Kotlin adapter obtains the exact bounded payload from Rust and executes no Kotlin command, parameter, mask, flag, reserved-byte, query, or fatal-availability algorithm

#### Scenario: Decode Android production lighting

- **WHEN** Android production receives a valid legacy or stable lighting response
- **THEN** Rust validates its complete structure and Kotlin mechanically maps the returned two-bit result into the existing lighting state

#### Scenario: Preserve established Android failures

- **WHEN** Rust reports an invalid caller contract, malformed response, unsupported command, insufficient output, ABI mismatch, invalid primitive result, pending exception, or contained internal failure
- **THEN** Kotlin returns the established bounded protocol exception without logging payload/state, publishing a partial result, or executing a fallback codec

#### Scenario: Preserve higher-level Refloat owners

- **WHEN** Rust returns a control payload or decoded lighting state
- **THEN** existing Kotlin compatibility, routing, framing, authorization, scheduling, confirmation, telemetry, and application-state owners behave unchanged after representation mapping

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android Refloat control cutover completes
- **THEN** iOS and macOS continue using their existing safe Rust codec and Apple bridge without linking, packaging, or calling the Android JNI artifact
