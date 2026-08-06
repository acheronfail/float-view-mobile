## ADDED Requirements

### Requirement: Version-specific disposable Refloat configuration

The interactive fixture SHALL compose each selected audited Refloat release with its exact source-derived compressed custom schema, signature, defaults, and mutable current configuration at index `0`. It SHALL implement commands 92–95 with source-correct chunks and complete signed values through the existing direct/CAN topology, validate full writes, reproduce documented firmware normalization, and make later command 93 reads reflect accepted values. Configuration state SHALL be disposable, bounded, non-actuating, reset when the selected package changes, and available to canonical scripted scenarios without moving scenario logic into ESP32 firmware.

#### Scenario: Schema chunk sequence

- **WHEN** a client requests bounded command-92 chunks for a selected exact package
- **THEN** the fixture returns the exact total length, requested offset, and source-derived bytes without serving another release's schema

#### Scenario: Current and defaults remain distinct

- **WHEN** an accepted write changes current state and the client subsequently requests commands 93 and 94
- **THEN** current reports the accepted canonical values while defaults remain the exact selected-release defaults

#### Scenario: Package selection changes

- **WHEN** the powered-off fixture is changed to another exact Refloat version
- **THEN** schema, signature, current/default values, and configuration counters reset atomically before the next advertisement

#### Scenario: No motor actuation

- **WHEN** any valid configuration write is accepted
- **THEN** only disposable simulated configuration state changes and no ESP32 pin or command can actuate a motor

### Requirement: Strict custom-configuration rejection

The fixture SHALL answer configuration mutations only for normal writable audited firmware, index `0`, complete valid signatures and values, and configured safe scenario state. It SHALL reject test-version writes, unknown indexes, malformed or partial values, prohibited modes, unsafe state, arbitrary app-data tuning, firmware/motor configuration, and motor-control commands without weakening the existing fail-fast allowlist. While simulated package mode is flywheel it SHALL reject every mutation except eligible flywheel disable, withhold command-93 current values, invalidate loaded current state, and require a fresh current read after returning to normal mode; schema and default reads may remain available.

#### Scenario: Test firmware write

- **WHEN** a test-version fixture receives command 95
- **THEN** it preserves current configuration and supplies no simulated successful mutation

#### Scenario: Invalid signature

- **WHEN** command 95 contains a signature from another Refloat release
- **THEN** the fixture preserves all values and later current read returns the unchanged complete configuration

#### Scenario: Existing semantic controls

- **WHEN** a source-correct package, flywheel, lighting, or fatal control is received
- **THEN** its dedicated live-control behavior remains available and coordinated with, but not replaced by, the full configuration model

#### Scenario: Flywheel configuration isolation

- **WHEN** simulated flywheel mode is active and a client requests command 93, command 95, Tune apply, or another mutation
- **THEN** the fixture supplies no current snapshot or mutation success except for eligible flywheel disable and preserves disposable persistent configuration values
