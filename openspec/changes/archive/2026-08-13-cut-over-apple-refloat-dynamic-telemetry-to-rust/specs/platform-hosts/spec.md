## ADDED Requirements

### Requirement: Shared Apple Rust Refloat dynamic telemetry bridge ownership

The shared Apple backend SHALL expose the safe Refloat dynamic-telemetry component through one temporary, focused, versioned, stateless C bridge used by both iOS and macOS. The bridge SHALL export exactly an ABI-version query, layout decode, and realtime decode; use bounded borrowed inputs, caller-owned record outputs, checked null/alignment/overflow/disjointness/capacity, fixed statuses, deterministic output initialization, and panic containment; and depend only on the safe telemetry and protocol-neutral Apple FFI-support crates.

Canonical layout and sample records SHALL be owned, parsed, and serialized only by the safe telemetry component. Swift SHALL validate and mechanically map them without interpreting command-31/32 bytes. Locked source builds, the hand-written C module, SwiftPM, and Xcode SHALL select the correct archive architecture for macOS, iOS device, and iOS simulator without generated bindings or committed native binaries. The Android builder and typed Gradle catalog SHALL keep their existing closed seven-bridge set and reject the Apple-only entry.

#### Scenario: Exercise bounded layout decode

- **WHEN** Apple passes a valid command-32 payload and disjoint caller-owned output regions
- **THEN** the bridge publishes exactly the safe component’s canonical bounded layout record and required length while retaining no caller memory

#### Scenario: Exercise bounded realtime decode

- **WHEN** Apple passes a valid canonical layout record, command-31 payload, and disjoint caller-owned output regions
- **THEN** the bridge publishes exactly the safe component’s complete canonical sample record and required length while retaining no pointer, allocation, or lifetime

#### Scenario: Validate every Apple record form

- **WHEN** Swift receives a layout or sample record
- **THEN** it accepts only exact version, kind, extent, counts, UTF-8, ordering, closed codes, flags, options, finite values, canonical absent fields, alert/fault relations, extension entries, reserved bytes, and complete cursor consumption

#### Scenario: Contain bridge and record failure

- **WHEN** the bridge or adapter receives null, oversized, overlapping, misaligned, insufficient-capacity, malformed, ABI-incompatible, impossible, stale-layout, or internally failed input
- **THEN** it fails closed through the established status/error without partial publication, payload data, addresses, retained memory, generated fallback code, or native payload parsing

#### Scenario: Package both sibling Apple hosts

- **WHEN** macOS, iOS device, and iOS simulator products build from a clean artifact state
- **THEN** each links the exact three-symbol bridge architecture and shared adapter from source with no committed archive or generated header

#### Scenario: Delete superseded Swift ownership

- **WHEN** production selection and regression evidence pass
- **THEN** shared Apple source contains no command-31/32 envelope reader, identifier parser, layout bound/uniqueness algorithm, extent calculator, source float16 decoder, semantic/alert/fault mapping, extension classifier, or runtime fallback

#### Scenario: Keep Android and OS capabilities isolated

- **WHEN** the Apple bridge is built and selected
- **THEN** Android retains its existing seven JNI bridge artifacts and Kotlin dynamic decoder while Bluetooth, location, background execution, lifecycle, permissions, native files, theme, WebViews, SQLite, and loopback transport remain in their existing owners

#### Scenario: Mark the bridge as migration scaffolding

- **WHEN** repository ownership is inspected before the unified Rust runtime cutover
- **THEN** the Apple bridge is documented as a temporary selection boundary whose protocol-specific symbols disappear after all hosts consume Rust-owned application behavior
