## ADDED Requirements

### Requirement: Canonical firmware identity record ownership

Apple and Android SHALL obtain firmware-identity bridge records from one canonical platform-neutral owner in `vesc-firmware-identity` while preserving separate hand-written Apple FFI and Android JNI boundary crates. The shared record owner SHALL contain only bounded safe representation logic; Apple SHALL retain C pointer/region publication, Android SHALL retain call-local JNI array handling, and each host SHALL retain its existing adapter, model validation, error mapping, package, and OS capability ownership.

#### Scenario: Trace one record owner

- **WHEN** a maintainer traces how a decoded firmware identity becomes host-consumable bytes
- **THEN** the record layout and serialization are found once in the safe identity component and each platform bridge contains only its explicit host-boundary validation and publication

#### Scenario: Update the portable record once

- **WHEN** a later independently specified change revises the firmware-identity bridge record
- **THEN** its Rust layout and serialization change in the safe identity component rather than in parallel Apple and Android encoders, while each affected public host contract is still reviewed and versioned explicitly

#### Scenario: Keep bridge artifacts independent

- **WHEN** Apple and Android build or package firmware identity
- **THEN** Apple links only its existing two-symbol archive, Android loads or packages only its existing two-symbol JNI library, and neither platform depends on the other platform's bridge or artifact

#### Scenario: Preserve native owners

- **WHEN** either host consumes the shared identity record
- **THEN** compatibility, routing, scheduling, application behavior, Bluetooth, background work, location, lifecycle, permissions, native files, theme, WebView integration, loopback transport, and persistence remain in their existing native owners
