## ADDED Requirements

### Requirement: Complete persisted settings shape

The mandatory `Settings` value SHALL contain `revision`, `idleProbeIntervalMs`, `idlePersistIntervalMs`, `ridingPersistIntervalMs`, `postRideTailMs`, `reconnectDurationMs`, `automaticSessionLinkingEnabled`, `sessionLinkWindowMs`, and `themePreference`. `themePreference` SHALL accept exactly `system`, `light`, or `dark`; `rawPacketCaptureEnabled` MUST NOT remain in the contract or persisted settings payload.

#### Scenario: Default settings

- **WHEN** a backend creates current default settings
- **THEN** it returns the existing timing and linking defaults with `themePreference` equal to `system` and without a raw-packet field

#### Scenario: Settings round trip

- **WHEN** the frontend updates a valid appearance, timing, or linking field at the current revision
- **THEN** every supported backend persists and returns the same complete settings shape at a newer revision

#### Scenario: Unknown settings field

- **WHEN** a settings patch contains `rawPacketCaptureEnabled` or another field outside the complete shape
- **THEN** every supported backend rejects the patch and leaves persisted settings unchanged

#### Scenario: Cross-platform conformance

- **WHEN** canonical conformance exercises settings on Android, iOS, or macOS
- **THEN** each target exposes the same defaults, accepted preference values, revision behavior, and invalid-field rejection
