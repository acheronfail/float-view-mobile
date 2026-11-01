# Spec Delta

## MODIFIED Requirements

### Requirement: Refloat 1.3.1-beta2 interoperability evidence

Canonical conformance SHALL exercise exact beta2 admission, the existing selectable telemetry and control profiles, audited configuration identity, same-version Tunes, typed IMU-timeout state, and durable recovered history on direct and CAN routes. It SHALL preserve older-release behavior and supported baseline data. A targeted physical checkpoint SHALL confirm the core capabilities on actual beta2 firmware.

#### Scenario: Source-derived beta2 fixture

- **WHEN** the conformance fixture reports exact beta2 INFO, command-33 responses, command-35 mixed alerts, and its 172-field schema
- **THEN** both hosts reach equivalent ready state, publish live telemetry, verify configuration read/write and same-version Tune apply, and retain timeout activation and clearing without firmware-fault pollution

#### Scenario: Adjacent identities and malformed alerts

- **WHEN** a fixture reports beta1, beta4, stable 1.3.1, malformed suffixes, or invalid beta2 command-35 records
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

## ADDED Requirements

### Requirement: Refloat beta3 interoperability evidence

Canonical conformance SHALL verify exact beta3 admission, direct/CAN telemetry, timeout history and write gates, configuration read/write/readback, exact-version Tunes and reconnect using source-pinned fixtures. A stationary production-host checkpoint SHALL validate beta3 interoperability with backed-up configuration and verified restoration. Damaged controller readings SHALL NOT establish riding or motor behavior.

#### Scenario: Beta3 software coverage

- **WHEN** source-derived beta3 fixtures exercise the shared service and affected frontend policy
- **THEN** exact identity, telemetry, alert safety, complete configuration and same-version Tune application pass while beta2 and older releases retain their behavior

#### Scenario: ExpressT checkpoint

- **WHEN** the available damaged controller reports beta3 through the production macOS BLE host
- **THEN** the checkpoint verifies discovery, advancing fast/slow telemetry, schema/current/default reads, a non-actuating write/readback/restore, unchanged same-version Tune apply, saved stationary samples and reconnect
- **AND** raw evidence remains local and public results identify the exercised route and exclude motor output, riding and unreliable duty/eRPM assertions
