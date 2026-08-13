## ADDED Requirements

### Requirement: Apple Rust Refloat compatibility ownership

The shared Apple protocol layer SHALL select the safe Rust compatibility component through one temporary stateless Apple bridge for both iOS and macOS. The Swift adapter SHALL contain only bounded canonical-record validation and mechanical model mapping. Apple SHALL retain controller-discovery transitions, request scheduling, direct/CAN route ownership, connection generations, state/session/control coordination, and all OS capabilities. Android SHALL retain its Kotlin compatibility implementation and unchanged JNI build surface until its own cutover.

#### Scenario: Locate the selected Apple owner

- **WHEN** a maintainer traces Apple INFO identity, supported-release/profile selection, or dynamic-layout admission
- **THEN** each protocol decision terminates in `backends/core/refloat-compatibility` through the narrow Apple bridge while higher-level discovery effects remain under the Apple application owner

#### Scenario: Delete superseded Swift ownership

- **WHEN** Apple production selection and regression evidence pass
- **THEN** shared Apple source contains no INFO version-offset decoder, release catalogue, profile derivation, required identifier catalogue, set-based layout admission, or runtime fallback

#### Scenario: Preserve Apple packaging from locked source

- **WHEN** macOS, iOS device, and iOS simulator products build from a clean artifact state
- **THEN** each links the exact compatibility bridge symbols and hand-written adapter from locked repository source with no committed archive or generated binding

#### Scenario: Keep Android and OS capabilities isolated

- **WHEN** the Apple bridge is built and selected
- **THEN** Android retains its existing eight JNI bridge artifacts and Kotlin compatibility owner while Bluetooth, background work, location, lifecycle, permissions, native files, theme, WebViews, SQLite, and loopback transport remain in their existing platform owners

#### Scenario: Mark the bridge as migration scaffolding

- **WHEN** repository ownership is inspected before the unified Rust runtime cutover
- **THEN** the Apple bridge is documented as a temporary selection boundary whose compatibility symbols disappear after all hosts consume Rust-owned application behavior
