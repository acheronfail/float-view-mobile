## ADDED Requirements

### Requirement: Coordinated semantic and full-configuration controls

Semantic package, flywheel, lighting, and fatal-error controls SHALL retain their dedicated exact-version commands and authoritative confirmation behavior when full Refloat configuration is available. They SHALL share one native mutation-exclusivity policy with configuration write and Tune apply, SHALL invalidate or refresh current configuration values when a confirmed semantic action changes a serialized configuration field, and MUST NOT be reimplemented as frontend edits to the generated form. A configuration write SHALL not optimistically alter semantic control state; later authoritative command response or telemetry SHALL remain its source.

#### Scenario: Package state changes outside editor
- **WHEN** a confirmed semantic package action changes a value represented by the loaded configuration schema
- **THEN** the native owner marks current configuration values stale and requires a fresh command-93 read before a full write

#### Scenario: Full write affects live controls
- **WHEN** a verified full configuration changes package or lighting-related values
- **THEN** semantic control state updates only from its existing authoritative response or telemetry sources rather than from the submitted draft

#### Scenario: Independent lighting runtime state
- **WHEN** runtime lighting state is not part of the persistent schema value being edited
- **THEN** the dedicated lighting control continues independently while still respecting the shared one-mutation-at-a-time gate

