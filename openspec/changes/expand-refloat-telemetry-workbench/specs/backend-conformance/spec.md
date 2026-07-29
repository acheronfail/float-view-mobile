## ADDED Requirements

### Requirement: Detailed telemetry compatibility matrix
Software conformance SHALL exercise complete detailed telemetry for Refloat 1.0.0, 1.1.0, and 1.2.3 through direct and CAN routes. It SHALL verify exact command 1 fixture compatibility, command 10 detail levels, command 31/32 bytes, route wrapping, field negotiation, version-applicable active state and tails, normalization, nullability, unknown bounded extensions, and absence of command 35, data-recorder, or unsupported write requests from either backend.

#### Scenario: Six route and package combinations
- **WHEN** the detailed telemetry matrix runs
- **THEN** each supported package through direct and CAN routing produces its exact protocol family and equivalent normalized semantic values where the source carries them

#### Scenario: Slow compact polling
- **WHEN** a dynamic package is configured to expose slow compact values
- **THEN** conformance verifies one route-correct detail-level-4 command 10 becomes due each second, never overlaps the one in-flight request, does not retry immediately after timeout, and merges into later realtime samples with its own `slowObservedAt`

### Requirement: Detailed persistence and contract coverage
Canonical conformance SHALL verify that both backends commit, recover, serialize, retrieve, diagnose, and export every typed detailed field, observed discrete transition, and representative bounded extension with stable names, units, nullability, and privacy. It SHALL cover storage failure, frontend recreation, reconnect, selected-controller change, session interruption, decreasing device counters, and unknown future enum values.

#### Scenario: End-to-end detailed sample
- **WHEN** the simulator supplies a sample containing representative fields from every semantic group
- **THEN** live state, durable session detail, transition events, diagnostics, and export agree on the committed sequence and values on Apple and Android

#### Scenario: One controller per session
- **WHEN** a product selects a different discovered controller after recording detailed telemetry
- **THEN** conformance observes the old session end, route-scoped telemetry clear, and the new controller contribute only to a new session

#### Scenario: Privacy and bounds
- **WHEN** detailed telemetry, scenario aliases, learned identities, oversized fields, and extension limits are exercised
- **THEN** conformance observes the specified 65,536-byte message/document, 64-field layout, 64-byte key, 4,096-byte aggregate-name, 32-extension, 16,384-byte JSON, 50-diagnostic, and 2,000-point/event response bounds and no private controller UUID in contract state, logs, errors, diagnostics, or exports

### Requirement: Workbench software verification
The software gate SHALL test scenario-document validation and round trips, deterministic private identity derivation, topology editing, semantic scenes, overrides, version projections, manual disconnect and identity transitions, fragmentation, coordinator serialization, diagnostics bounds, and accessible control activation without requiring an ESP32 or native device.

#### Scenario: Scenario document round trip
- **WHEN** a valid complex scenario is exported and imported
- **THEN** software verification observes equivalent public configuration and wire behavior but no exported private UUID

#### Scenario: Scene projection matrix
- **WHEN** representative semantic scenes are encoded through each supported Refloat family
- **THEN** exact fixtures and decoders agree on present fields, absent fields, state flags, and response bounds

### Requirement: Representative physical detailed telemetry checkpoint
The physical BLE checkpoint SHALL exercise detailed telemetry on Android and macOS with at least one direct and one CAN-routed dynamic scene, one legacy scene, response fragmentation, a manual reconnect event, and representative warning and field-weakening values. iOS SHALL exercise one dynamic detailed scene and background recovery through the shared Apple backend without duplicating the full macOS matrix.

#### Scenario: Physical workbench scene
- **WHEN** a developer selects a detailed riding or warning scene in `just fixture dev` and operates the native product
- **THEN** the platform reaches ready, displays and persists the expected normalized fields, survives the required lifecycle event, and retrieves them after disconnect
