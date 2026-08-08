## ADDED Requirements

### Requirement: Rust owns software native transport

Simulator, emulator, browser-host, and software-conformance modes SHALL use the Rust software host and Rust software VESC peripheral. Swift and Kotlin MUST NOT contain their own VESC frame parser, command classifier, response generator, fixture semantics, or software Bluetooth adapter. Native composition SHALL select software or physical transport through one bounded startup setting without changing frontend contract behavior.

#### Scenario: Apple simulator development

- **WHEN** the iOS simulator or a software-selected macOS host starts
- **THEN** the Rust software host supplies deterministic discovery, connection, byte responses, and fixture behavior while Swift retains only lifecycle and presentation wiring

#### Scenario: Android emulator development

- **WHEN** the Android emulator starts in software transport mode
- **THEN** the Rust software host supplies the same deterministic behavior without constructing Kotlin software Bluetooth or VESC objects

#### Scenario: Physical mode

- **WHEN** a supported host starts in physical transport mode
- **THEN** it uses its production OS Bluetooth adapter and the Rust software host remains inactive

#### Scenario: Software and physical identity

- **WHEN** equivalent contract scenarios run through software and physical transport modes
- **THEN** both use the same Rust application actor, dispatcher, persistence, loopback server, and connection semantics, differing only at the byte transport boundary
