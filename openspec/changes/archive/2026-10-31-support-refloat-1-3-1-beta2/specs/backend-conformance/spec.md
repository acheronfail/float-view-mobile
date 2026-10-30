# Spec Delta

## MODIFIED Requirements

### Requirement: Detailed telemetry compatibility matrix

Software conformance SHALL exercise complete detailed telemetry for every audited stable Refloat release in 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3 through direct and CAN routes and across audited VESC 6.02, 6.05, 6.06, and 7.00. It SHALL verify the complete VESC identity baseline, audited nonzero test-version guarded write access, source-applicable bounded hardware-CRC suffix, exact command 1 fixture compatibility, command 10 detail levels and metres/second conversion, command 31/32 bytes and native kilometres/hour values, route wrapping, required-field negotiation, version-applicable active state and tails, normalization, nullability, unknown bounded extensions, and bounded source-correct command-35 alert reads while excluding data-recorder and unsupported write requests from either backend.

#### Scenario: Audited route and package combinations

- **WHEN** the detailed telemetry matrix runs
- **THEN** each audited stable VESC/controller-package pair through direct and CAN routing produces its exact protocol profiles and equivalent normalized semantic values where the source carries them

#### Scenario: Stable VESC identity families

- **WHEN** the matrix supplies 6.02 without a CRC suffix and 6.05, 6.06, or 7.00 with the source-defined bounded suffix
- **THEN** Apple and Android select the same `vescCoreReadOnly` profile, ignore CRC value for compatibility, and proceed to route and Refloat capability validation

#### Scenario: Audited test-version write access

- **WHEN** conformance supplies a known 6.02/6.05/6.06/7.00 identity with a nonzero test-version marker
- **THEN** Apple and Android complete Refloat negotiation, allow read requests and guarded writes, and expose read/write access beside the VESC and Refloat versions

#### Scenario: Controller compatibility rejection

- **WHEN** conformance supplies VESC 7.01 with a nonzero test-version marker, another unknown release, or a malformed identity baseline
- **THEN** Apple and Android reject the identity with `unsupported_controller`, reject malformed identity within the bounded identification deadline, and never begin Refloat negotiation or durable session creation

#### Scenario: Compatibility rejection

- **WHEN** conformance supplies an unknown version, an unaudited prerelease version, or a dynamic layout missing a required semantic ID
- **THEN** Apple and Android reject it before ready with the same unsupported-Refloat outcome

#### Scenario: Slow compact polling

- **WHEN** a dynamic package is configured to expose slow compact values
- **THEN** conformance verifies one route-correct detail-level-4 command 10 becomes due each second, never overlaps the one in-flight request, does not retry immediately after timeout, and merges into later realtime samples with its own `slowObservedAt`

## ADDED Requirements

### Requirement: Refloat 1.3.1-beta2 interoperability evidence

Canonical conformance SHALL exercise exact beta2 admission, the existing selectable telemetry and control profiles, audited configuration identity, same-version Tunes, typed IMU-timeout state, and durable recovered history on direct and CAN routes. It SHALL preserve older-release behavior and supported baseline data. A targeted physical checkpoint SHALL confirm the core capabilities on actual beta2 firmware.

#### Scenario: Source-derived beta2 fixture

- **WHEN** the conformance fixture reports exact beta2 INFO, command-33 responses, command-35 mixed alerts, and its 172-field schema
- **THEN** both hosts reach equivalent ready state, publish live telemetry, verify configuration read/write and same-version Tune apply, and retain timeout activation and clearing without firmware-fault pollution

#### Scenario: Adjacent identities and malformed alerts

- **WHEN** a fixture reports beta1, beta3, stable 1.3.1, malformed suffixes, or invalid beta2 command-35 records
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
