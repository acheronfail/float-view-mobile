## ADDED Requirements

### Requirement: Shared Apple Rust Refloat configuration codec bridge ownership

The repository SHALL provide one temporary, stateless, source-built Apple bridge that exposes the safe Rust Refloat configuration codec to the sibling iOS and macOS hosts without moving OS or stateful application ownership.

#### Scenario: Expose exactly the narrow codec surface

- **WHEN** the bridge ABI is inspected
- **THEN** it SHALL export exactly ABI-version, schema-decode, values-decode, and values-encode symbols
- **AND** it SHALL expose no allocator, destructor, callback, scheduler, database, host singleton, generic command dispatcher, or long-lived handle

#### Scenario: Exchange bounded canonical records

- **WHEN** Swift invokes a decode or encode operation
- **THEN** all inputs and outputs SHALL use caller-owned bounded byte regions and fixed scalar outputs
- **AND** schema and value metadata SHALL use safe-crate-owned versioned canonical records
- **AND** the bridge SHALL retain no pointer or record after the call

#### Scenario: Validate every ABI boundary

- **WHEN** the bridge receives pointer/length pairs, capacities, schema records, value records, or output scalars
- **THEN** it SHALL validate maximum sizes, nullability, alignment, pairwise region disjointness, record version/kind/length/count/reserved fields, UTF-8, scalar canonicality, and schema/value consistency before use
- **AND** it SHALL contain panics and return deterministic fixed statuses and output lengths

#### Scenario: Build both Apple host variants from locked source

- **WHEN** SwiftPM or Xcode builds macOS, iOS device, or iOS simulator
- **THEN** the locked Rust source SHALL produce and link the correct bridge archive and hand-written C module
- **AND** a clean build SHALL require no committed or pre-generated native archive or language binding
- **AND** source and lockfile changes SHALL invalidate the build output

#### Scenario: Remove superseded Swift protocol ownership

- **WHEN** focused adapter and selection evidence passes
- **THEN** Apple SHALL delete its Swift Qt decompressor, XML parser, schema fingerprint and wire-signature construction, value byte reader/writer, scale conversion, string framing, and CRC implementation
- **AND** source audits SHALL reject reintroduction or fallback selection of those algorithms

#### Scenario: Preserve the host boundary

- **WHEN** the bridge is integrated
- **THEN** Swift SHALL retain the stateful configuration service and application ownership
- **AND** Apple SHALL retain BLE, location, background work, files, theme, lifecycle, permissions, WebView, persistence, and scheduling ownership
- **AND** Android production sources, packages, typed bridge set, and artifacts SHALL remain unchanged

#### Scenario: Mark the bridge as temporary migration scaffolding

- **WHEN** maintainers inspect the bridge, catalogue, and ownership documentation
- **THEN** they SHALL identify it as removable after unified Rust runtime cutover
- **AND** they SHALL NOT treat the canonical records as a general generated platform API
