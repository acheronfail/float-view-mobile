## ADDED Requirements

### Requirement: Shared Apple Rust Refloat legacy telemetry bridge ownership

The shared Apple backend SHALL expose the safe Refloat legacy-telemetry component through one temporary, focused, versioned, stateless C bridge used by both iOS and macOS. The bridge SHALL export exactly an ABI-version query and one decode operation; use bounded borrowed input, caller-owned record and scalar outputs, checked null/alignment/overflow/disjointness/capacity, fixed statuses, and panic containment; and depend only on the safe telemetry and protocol-neutral Apple FFI-support crates.

The canonical version-1 record SHALL be owned and serialized only by the safe telemetry component. It SHALL distinguish sample and active-fault results; carry closed classifications, boolean and option flags, reserved zero bytes, and exact finite big-endian numeric values; distinguish unavailable field weakening and detail-tail values from numeric zero; and have fixed bounded extents. Swift SHALL validate and mechanically map the record without interpreting command-10 bytes. Locked source builds, the hand-written C module, SwiftPM, and Xcode SHALL select the correct archive architecture for macOS, iOS device, and iOS simulator without generated bindings or committed native binaries.

#### Scenario: Exercise one bounded Apple bridge call
- **WHEN** Apple passes a valid bounded legacy telemetry payload and disjoint caller-owned outputs
- **THEN** the bridge publishes exactly the safe component's complete record and required length with the fixed success status and retains no caller memory

#### Scenario: Validate every Apple record form
- **WHEN** Swift receives a sample or active-fault record
- **THEN** it accepts only the exact version, kind, extent, closed codes, flags, option/detail consistency, finite values, canonical absent fields, reserved bytes, and complete cursor required by that form

#### Scenario: Contain bridge and record failure
- **WHEN** the bridge or adapter receives null, oversized, overlapping, misaligned, insufficient-capacity, malformed, ABI-incompatible, impossible, or internally failed input
- **THEN** it fails closed through the established status/error without partial publication, payload data, addresses, generated fallback code, or native payload parsing

#### Scenario: Package both sibling Apple hosts
- **WHEN** macOS, iOS device, and iOS simulator products build from a clean artifact state
- **THEN** each links the exact two-symbol bridge architecture and shared adapter from source with no committed archive or generated header

#### Scenario: Delete superseded Swift ownership
- **WHEN** production selection and regression evidence pass
- **THEN** the shared Apple source contains no command-10 byte reader, length table, fixed-point/float decoder, semantic-state mapping, payload enum/fault classification, field-weakening sentinel logic, or runtime fallback

#### Scenario: Keep Android and OS capabilities isolated
- **WHEN** the Apple bridge is built and selected
- **THEN** Android retains its six JNI bridge artifacts and Kotlin legacy decoder while Bluetooth, location, background execution, lifecycle, permissions, native files, theme, WebViews, SQLite, and loopback transport remain in their existing owners

#### Scenario: Mark the bridge as migration scaffolding
- **WHEN** repository ownership is inspected before the unified Rust runtime cutover
- **THEN** the Apple bridge is documented as a temporary selection boundary whose protocol-specific symbols disappear after all hosts consume Rust-owned application behavior
