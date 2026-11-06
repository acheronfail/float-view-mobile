## ADDED Requirements

### Requirement: Stable Refloat 1.3.1 evidence

Canonical conformance SHALL exercise exact stable 1.3.1 identity, telemetry, timeout history, configuration and same-version Tunes through direct and CAN routes using source-pinned shared fixtures. Existing beta support SHALL remain covered. Identical audited beta3/stable firmware source MAY reuse existing beta3 physical interoperability evidence. Tune-family changes SHALL receive focused software safety coverage.

#### Scenario: Stable software coverage

- **WHEN** the stable profile exercises the shared production service
- **THEN** admission, schema, verified writes, durable history and saved/guest reconnect preserve exact identity

#### Scenario: Reuse unchanged firmware evidence

- **WHEN** the complete application source is identical between audited beta3 and stable
- **THEN** the source equality and prior beta3 physical result establish unchanged wire interoperability without a repeated physical checkpoint

## MODIFIED Requirements

### Requirement: Refloat 1.3.1-beta2 interoperability evidence

Canonical conformance SHALL exercise exact beta2 admission, the existing selectable telemetry and control profiles, audited configuration identity, same-version Tunes, typed IMU-timeout state, and durable recovered history on direct and CAN routes. It SHALL preserve older-release behavior and supported baseline data. A targeted physical checkpoint SHALL confirm the core capabilities on actual beta2 firmware.

#### Scenario: Source-derived beta2 fixture

- **WHEN** the conformance fixture reports exact beta2 INFO, command-33 responses, command-35 mixed alerts, and its 172-field schema
- **THEN** both hosts reach equivalent ready state, publish live telemetry, verify configuration read/write and same-version Tune apply, and retain timeout activation and clearing without firmware-fault pollution

#### Scenario: Adjacent identities and malformed alerts

- **WHEN** a fixture reports beta1, beta4, stable 1.3.2, malformed suffixes, or invalid beta2 command-35 records
- **THEN** admission or the malformed response fails at its boundary without write traffic, partial history, or stale-generation state

#### Scenario: Saved and guest histories

- **WHEN** identical source-derived timeout transitions occur in saved and guest tracking
- **THEN** saved sessions page and export committed events, guest history remains bounded and ephemeral, and existing supported baseline sessions and archives still load

#### Scenario: Targeted physical checkpoint

- **WHEN** an exact beta2 board is available after software fixture development
- **THEN** a relevant native host validates discovery, advancing live and saved telemetry, schema/current/default reads, a safe complete configuration write/readback/restore, eligible same-version Tune application, and reconnect with original settings restored

#### Scenario: Physical coverage limit

- **WHEN** real IMU failure or an unaffected host or route cannot be exercised safely
- **THEN** the checkpoint reports the limit separately from simulated timeout coverage and does not claim unrun physical validation
