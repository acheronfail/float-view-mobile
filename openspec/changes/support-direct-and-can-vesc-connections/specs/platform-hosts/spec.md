## ADDED Requirements

### Requirement: Mirrored native connection ownership
The Apple and Android backends SHALL use matching ownership boundaries for application coordination, controller discovery, telemetry polling, Bluetooth stream setup, VESC framing and commands, firmware parsing, exact compatibility mapping, and Refloat protocol families. Apple SHALL place application owners under `backends/apple/Sources/FloatViewBackend/application`, VESC protocol owners under `protocol/vesc`, and Refloat owners under `protocol/refloat`. Android SHALL retain its established application owners under `backends/android/app/src/main/java/fail/acheron/floatview/backend` and place corresponding VESC and Refloat owners under `protocol/vesc` and `protocol/refloat`. Swift and Kotlin implementations MUST remain backend-owned, dependencies MUST point downward from application coordination to Bluetooth and protocol components, and superseded monolithic paths MUST be deleted after extraction.

#### Scenario: Locate controller discovery
- **WHEN** a developer needs to change direct-or-CAN route selection
- **THEN** each native backend has one clearly named controller-discovery owner rather than route decisions spread through transport, polling, and view code

#### Scenario: Locate a protocol family
- **WHEN** a developer needs to change VESC framing or a Refloat wire layout
- **THEN** the matching Swift and Kotlin file is found under the corresponding `protocol/vesc` or `protocol/refloat` subtree rather than a version-named application-service branch

#### Scenario: Add a compatible firmware version
- **WHEN** a later VESC or Refloat release uses an existing tested wire family
- **THEN** each backend adds a compatibility-catalog entry and fixtures without copying application-service, routing, or polling code

#### Scenario: Add a new wire family
- **WHEN** a later release changes a required wire layout
- **THEN** each backend adds or changes the focused codec family and maps exact versions to it while shared framing and routing remain unchanged

#### Scenario: Backend source documentation
- **WHEN** a developer reads the backend ownership documentation
- **THEN** it identifies the mirrored Swift/Kotlin locations, dependency direction, privacy boundary, and version-to-family extension rule
