## ADDED Requirements

### Requirement: Configuration safety evidence

The native application SHALL use authoritative normalized Refloat telemetry as safety evidence for a persistent configuration write. Eligibility SHALL require exactly idle ride state, normal package mode, no engaged footpad, and a fresh sample belonging to the current connection generation. Missing, unknown, stale, or contradictory evidence MUST make the write unavailable. Configuration reads SHALL remain independent of ride-session persistence and SHALL NOT create samples, events, aggregates, extensions, or exports.

#### Scenario: Fresh idle evidence
- **WHEN** current-generation telemetry reports idle, normal mode, and no footpad engagement
- **THEN** the safety evidence may permit a write if every other access and mutation condition also passes

#### Scenario: Missing legacy evidence
- **WHEN** a package family cannot supply one required safety field authoritatively
- **THEN** writes remain unavailable rather than treating the missing field as safe

#### Scenario: Configuration read during riding
- **WHEN** current or default values are requested while riding
- **THEN** the read may proceed at scheduler priority without changing or persisting telemetry state

### Requirement: Audited custom configuration exception

The existing Refloat read and product-control allowlists SHALL additionally permit only VESC custom-configuration commands 92–95 for audited stable Refloat configuration index `0` under `refloat-configuration`. Refloat app-data tuning, Handtest, remote, RC-move, motor-control, recording, history, unknown custom indexes, malformed custom payloads, and every other mutation SHALL remain unsupported.

#### Scenario: Allowed schema and values read
- **WHEN** an audited supported controller receives a valid route-correct command 92, 93, or 94 for index `0`
- **THEN** the simulator returns only the exact source-version schema chunk or complete signed value payload

#### Scenario: Allowed verified write
- **WHEN** an audited writable idle controller receives a complete valid command 95 payload through the specified configuration lifecycle
- **THEN** the simulator updates disposable configuration state for later command-93 verification

#### Scenario: Arbitrary mutation remains rejected
- **WHEN** a peer sends an unknown custom index, partial value payload, runtime tune app-data command, or other non-allowlisted mutation
- **THEN** no success is simulated, semantic board state is unchanged, and one bounded diagnostic is recorded

