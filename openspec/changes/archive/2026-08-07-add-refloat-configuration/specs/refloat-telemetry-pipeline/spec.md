## ADDED Requirements

### Requirement: Configuration safety evidence

The native application SHALL use authoritative normalized Refloat telemetry as safety evidence for every device mutation and for current-configuration authority. Persistent configuration eligibility SHALL require exactly idle ride state, normal package mode, no engaged footpad, and a fresh sample belonging to the current connection generation. While flywheel mode is active, every mutation except eligible flywheel disable SHALL be unavailable, loaded current configuration SHALL be invalidated, and command 93 SHALL be withheld so temporary in-memory calibration cannot become an authoritative base. Missing, unknown, stale, or contradictory evidence MUST make the write unavailable. Configuration reads SHALL remain independent of ride-session persistence and SHALL NOT create samples, events, aggregates, extensions, or exports.

#### Scenario: Fresh idle evidence

- **WHEN** current-generation telemetry reports idle, normal mode, and no footpad engagement
- **THEN** the safety evidence may permit a write if every other access and mutation condition also passes

#### Scenario: Missing legacy evidence

- **WHEN** a package family cannot supply one required safety field authoritatively
- **THEN** writes remain unavailable rather than treating the missing field as safe

#### Scenario: Configuration read during riding

- **WHEN** current or default values are requested while riding in non-flywheel package mode
- **THEN** the read may proceed at scheduler priority without changing or persisting telemetry state

#### Scenario: Configuration access during flywheel

- **WHEN** authoritative telemetry reports flywheel mode
- **THEN** schema and default reads may continue, current reads are rejected or withheld, and no current revision or device mutation is authorized except eligible flywheel disable

### Requirement: Audited custom configuration exception

The existing Refloat read and product-control allowlists SHALL additionally permit only VESC custom-configuration commands 92–95 for audited stable Refloat configuration index `0` under `refloat-configuration`. Refloat app-data tuning, Handtest, remote, RC-move, motor-control, recording, history, unknown custom indexes, malformed custom payloads, and every other mutation SHALL remain unsupported.

#### Scenario: Allowed schema and values read

- **WHEN** an audited supported controller receives a valid route-correct command 92, 93, or 94 for index `0`
- **THEN** the simulator returns only the exact source-version schema chunk or complete signed value payload

#### Scenario: Allowed verified write

- **WHEN** an audited writable idle controller in authoritatively normal package mode receives a complete valid command 95 payload through the specified configuration lifecycle
- **THEN** the simulator updates disposable configuration state for later command-93 verification

#### Scenario: Flywheel mutation remains rejected

- **WHEN** flywheel mode is active and a peer attempts command 95 or another mutation other than eligible flywheel disable
- **THEN** no success is simulated, no values change, and one bounded diagnostic is recorded

#### Scenario: Arbitrary mutation remains rejected

- **WHEN** a peer sends an unknown custom index, partial value payload, runtime tune app-data command, or other non-allowlisted mutation
- **THEN** no success is simulated, semantic board state is unchanged, and one bounded diagnostic is recorded
