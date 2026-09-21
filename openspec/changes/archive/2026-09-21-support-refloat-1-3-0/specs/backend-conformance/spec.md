## ADDED Requirements

### Requirement: Refloat 1.3.0 selectable telemetry evidence

The repository SHALL prove exact Refloat 1.3.0 compatibility, complete bounded command-33 behavior, configuration identity, retained control behavior, older-release isolation, and shared production-host composition using immutable source-derived fixtures, focused safe-component tests, canonical conformance, root audits, and the complete software and physical gates. Evidence MUST cover every documented non-reserved command-33 mask field in float32 form, echoed request identity, exact variable response extents, direct/CAN routes, fast/slow merge, generation replacement, persistence/export, limited alert semantics, configuration read/write/readback, and malformed/adversarial inputs without requesting unstable command 35.

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
- **THEN** only the exact audited 1.3.0 fingerprint and signature are accepted and no saved configuration is migrated or offered across the 1.2.3/1.3.0 boundary

#### Scenario: Preserve older releases

- **WHEN** the canonical software matrix replays every audited Refloat 1.0.0–1.2.3 fixture after 1.3.0 support is selected
- **THEN** legacy command 10 and negotiated commands 31/32 retain their exact requests, decoding, controls, configuration, session, and failure behavior

#### Scenario: Pass all-host gates

- **WHEN** the change is ready for completion
- **THEN** focused tests, warnings-denied linting, dependency/source-layout audits, `just verify`, strict OpenSpec validation, and the Android/iOS/macOS ESP32 physical checkpoint pass with production using the shared Rust runtime and no superseded host decoder added
