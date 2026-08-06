## ADDED Requirements

### Requirement: Coordinated semantic and full-configuration controls

Semantic package, flywheel, lighting, and fatal-error controls SHALL retain their dedicated exact-version commands and authoritative confirmation behavior when full Refloat configuration is available. They SHALL use the same native mutation coordinator as configuration write and Tune apply, SHALL invalidate or refresh current configuration values when a confirmed semantic action changes a serialized configuration field, and MUST NOT be reimplemented as frontend edits to the generated form. While flywheel mode is active the coordinator SHALL reject every device mutation except the exact eligible command 22 transition that disables flywheel. Entering flywheel SHALL invalidate loaded current configuration values; they MUST remain unavailable until normal mode is confirmed and a fresh command-93 read succeeds. A configuration write SHALL not optimistically alter semantic control state; immediate scheduled response/telemetry refresh SHALL re-establish any affected package or lighting state.

#### Scenario: Package state changes outside editor

- **WHEN** a confirmed semantic package action changes a value represented by the loaded configuration schema
- **THEN** the native owner marks current configuration values stale and requires a fresh command-93 read before a full write

#### Scenario: Full write affects live controls

- **WHEN** a verified full configuration changes package or lighting-related values
- **THEN** the native owner schedules the applicable authoritative telemetry or lighting refresh and updates semantic state only from that source rather than from the submitted draft

#### Scenario: Independent lighting runtime state

- **WHEN** runtime lighting state is not part of the persistent schema value being edited
- **THEN** the dedicated lighting control continues independently while respecting the shared mutation coordinator, and the UI distinguishes current runtime state from the persistent startup default

#### Scenario: Flywheel protects temporary configuration

- **WHEN** telemetry confirms entry into flywheel mode
- **THEN** all loaded current configuration values become stale and every mutation except eligible flywheel disable is rejected until normal mode and a fresh current read are confirmed
