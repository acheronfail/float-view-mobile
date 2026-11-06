# Spec Delta

## MODIFIED Requirements

### Requirement: Refloat 1.3.0 selectable telemetry evidence

The repository SHALL prove exact Refloat 1.3.0 compatibility, complete bounded command-33 behavior, configuration identity, retained control behavior, older-release isolation, and shared production-host composition using immutable source-derived fixtures, focused safe-component tests, canonical conformance, root audits, and affected software/physical checks.

#### Scenario: Cover complete selectable behavior

- **WHEN** selectable telemetry regression vectors run
- **THEN** evidence MUST cover every documented non-reserved command-33 mask field in float32 form, echoed request identity, exact variable response extents, direct/CAN routes, fast/slow merge, generation replacement, persistence/export, limited alert semantics, configuration read/write/readback, and malformed/adversarial inputs without requesting unstable command 35.

#### Scenario: Exercise complete command-33 field coverage

- **WHEN** focused codec and service vectors enumerate individual and combined fast, slow, and capability-gated GNSS masks for exact Refloat 1.3.0
- **THEN** every documented non-reserved field is decoded with its source type and unit, existing typed fields and canonical bounded extensions are populated exactly, and unavailable internal-only fields remain null

#### Scenario: Reject malformed selectable responses

- **WHEN** tests supply unsupported flags or mask bits, mismatched echoed flags/masks, truncation at every boundary, trailing bytes, non-finite values, invalid states, impossible coordinates, stale routes, or replacement generations
- **THEN** no partial or stale sample is published, persisted, exported, or used as mutation safety evidence and only bounded payload-free diagnostics are produced

#### Scenario: Preserve current alert boundary

- **WHEN** exact Refloat 1.3.0 reports alert reason and fatal latch through command 33 and an active firmware fault through the retained bounded command-10 probe
- **THEN** live state preserves the existing alert reason, active-fault, latch, and fatal-clear behavior without command 35, alert history, alert names, or persisted fatal-latch history

#### Scenario: Prove configuration and Tune isolation

- **WHEN** generated 1.3.0 schema/value fixtures exercise direct and CAN schema paging, current/default reads, an eligible write, mandatory readback, cache identity, save, list, and apply
- **THEN** the selected schema profile governs admission, the actual full fingerprint and wire signature are projected unchanged, and no saved configuration is migrated or offered across an exact-version or full-fingerprint boundary

#### Scenario: Preserve older releases

- **WHEN** the canonical software matrix replays every audited Refloat 1.0.0–1.2.3 fixture after 1.3.0 support is selected
- **THEN** legacy command 10 and negotiated commands 31/32 retain their exact requests, decoding, controls, configuration, session, and failure behavior

#### Scenario: Exercise selected production composition

- **WHEN** affected software or physical host checks exercise Refloat 1.3.0
- **THEN** they use the selected shared runtime without a superseded host decoder and retain the required route, interoperability and cleanup assertions

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

- **WHEN** conformance supplies an unknown version with unusable public reads or an audited dynamic layout missing a required semantic ID
- **THEN** Apple and Android reject it before ready with the same unsupported-Refloat outcome
- **AND** a valid unknown version with a proven public core exchange instead becomes limited without inheriting audited alert, configuration-write or control semantics

#### Scenario: Slow compact polling

- **WHEN** a dynamic package is configured to expose slow compact values
- **THEN** conformance verifies one route-correct detail-level-4 command 10 becomes due each second, never overlaps the one in-flight request, does not retry immediately after timeout, and merges into later realtime samples with its own `slowObservedAt`

### Requirement: Refloat 1.3.1-beta2 interoperability evidence

Canonical conformance SHALL exercise exact beta2 admission, the existing selectable telemetry and control profiles, audited configuration identity, same-version Tunes, typed IMU-timeout state, and durable recovered history on direct and CAN routes. It SHALL preserve older-release behavior and supported baseline data. A targeted physical checkpoint SHALL confirm the core capabilities on actual beta2 firmware.

#### Scenario: Source-derived beta2 fixture

- **WHEN** the conformance fixture reports exact beta2 INFO, command-33 responses, command-35 mixed alerts, and its 172-field schema
- **THEN** both hosts reach equivalent ready state, publish live telemetry, verify configuration read/write and same-version Tune apply, and retain timeout activation and clearing without firmware-fault pollution

#### Scenario: Adjacent identities and malformed alerts

- **WHEN** a fixture reports a valid unknown beta1, beta4 or stable 1.3.2 identity, malformed suffixes, or invalid beta2 command-35 records
- **THEN** valid unknown identities may pass limited public read admission without write authority, while malformed identity/alerts fail at their respective boundaries without partial history or stale-generation state

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

### Requirement: Cosmetic schema admission evidence

Canonical conformance SHALL prove the cosmetic admission boundary, actual schema identity, legacy resolution and retained write/Tune safety through independent source-derived fixtures and focused shared-service cases. Identical fixture artifacts SHALL be reused; a new release label alone SHALL NOT require duplicated payloads or an exhaustive profile-test copy.

#### Scenario: Cosmetic and protected-property matrix

- **WHEN** source-derived schema mutations change allowed label/help text and each protected property class, including properties absent from the normalized codec
- **THEN** independent expected keys and outcomes distinguish admitted cosmetics from rejected changes, even under identical wire CRCs; malformed, oversized and path-lookalike cases never bypass validation

#### Scenario: Service lifecycle and isolation

- **WHEN** direct/CAN cases exercise a cosmetic variant through cold fetch, cache hit/miss/corruption, reconnect, schema paging, values, edited write and same-identity Tune apply
- **THEN** actual metadata/full fingerprints survive projection, old revisions/identities cannot authorize writes, existing mutation/timeout/flywheel gates hold and success still requires complete readback

#### Scenario: Release and stored-data independence

- **WHEN** a validated limited release supplies a matching schema, or supported historical beta data is read independently of live firmware admission
- **THEN** schema compatibility grants reads only, and stored sessions, events, archives and Tunes remain readable under existing storage rules without enabling board writes or cross-version application

### Requirement: Independent future-version limited support evidence

Canonical conformance SHALL verify unknown-version public reads, feature isolation, exact provenance and zero device mutation traffic through the shared service and public contract on direct/CAN routes. Future identity fixtures SHALL be bounded variations of independently pinned source evidence, not production-policy-derived compatibility oracles.

#### Scenario: Future release matrix

- **WHEN** fixtures identify as 1.3.1-beta4, stable 1.3.2 and stable 1.4.0 with valid INFO v2 and the compatible public core read protocol
- **THEN** each reaches ready and records actual bounded identity without a runtime release entry, while every board-change request remains denied
- **AND** saved/guest tracking, reconnect, frontend recreation and supported session/export/archive reads retain their existing boundaries

#### Scenario: Core admission failure matrix

- **WHEN** fixtures supply wrong package name, malformed/unsupported INFO, invalid echo/tick/extent, mismatched masks, truncated/trailing/nonfinite core values, stale route/generation or no core response
- **THEN** bounded failure occurs before readiness or durable activation and no partial sample or device mutation frame is emitted
- **AND** unknown VESC identities still fail at their existing boundary before Refloat fallback

#### Scenario: Feature isolation and unknown state

- **WHEN** proven core telemetry is accompanied by an invalid optional group, incompatible/missing schema or unknown categorical values
- **THEN** independent numeric observations/recording continue, unavailable values/status remain null/unknown, and unknown ride state does not accumulate riding time
- **AND** no unstable alert/internal-layout probes, invented clear events or repeated failed optional probes occur

#### Scenario: Zero mutation authority across entry points

- **WHEN** limited fixtures have matching schemas and callers exercise controls, edited writes, Tune apply, shutdown, flywheel-disable and lighting queries through UI, direct requests and queued/stale paths
- **THEN** traffic assertions prove zero prohibited frames at admission and dispatch, stable failures and no optimistic control state, even after capability or generation changes

#### Scenario: Authoritative frontend availability

- **WHEN** mock/direct cases publish verified, limited, loading, failed, reconnecting and BMS snapshots
- **THEN** client and both hosts agree on the small support summary, configuration/Tune/alert availability and write-block reason
- **AND** the frontend uses those facts without version eligibility lists, permits limited recording without a blocking update dialog, and preserves single pointer/non-primary/keyboard activation semantics

### Requirement: Audited beta support survives stable release admission

Conformance SHALL preserve beta2/beta3 live capabilities and stored-data access when a stable release is introduced. Shared profile or fixture cleanup SHALL NOT require removing still-working audited mappings or duplicating fixtures for each marketing version.

#### Scenario: Stable release added to shared profiles

- **WHEN** an isolated compatibility fixture adds a stable 1.3.2 mapping to existing verified profiles
- **THEN** beta2 and beta3 retain exact discovery, telemetry, alert safety, configuration write/readback and same-version Tune behavior
- **AND** the audited 1.3.1 Tune family stays shared while other versions remain isolated and supported beta-created sessions, events and archives remain readable

#### Scenario: Maintenance follows protocol changes

- **WHEN** a later audited release has identical protocol/schema bytes
- **THEN** evidence can reuse verified source fixtures with a small explicit mapping and focused differences; new payload/profile code is needed only for changed bytes or behavior
- **AND** synthetic future-version evidence is not reported as physical interoperability with unreleased firmware
