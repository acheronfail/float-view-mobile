## ADDED Requirements

### Requirement: Apple Rust Refloat legacy telemetry cutover

The shared Apple backend SHALL use `refloat-legacy-telemetry` as its sole production implementation of fixed Refloat custom-app-data command-10 detail levels 1 through 4 and the active-firmware-fault form on iOS and macOS. Rust SHALL decide the exact envelope/form/length, every signed scale and unit conversion, finite distance, package state and mode, footpad, stop, setpoint, alert and firmware-fault classifications, boolean state flags, every optional detail tail, and field-weakening sentinel behavior. Swift MAY perform only strict versioned-record validation, representation conversion into the existing normalized telemetry model, established error mapping, and higher-level behavior.

The safe component SHALL own one dependency-free, unsafe-free, allocation-free canonical record and closed outcome for later platform reuse. The temporary Apple bridge SHALL publish that record through caller-owned memory with no Rust allocation, reference, callback, object, or lifetime crossing C. The cutover MUST delete superseded Swift command-10 wire parsing and MUST NOT add a runtime or source fallback. Android, dynamic telemetry, compatibility, routing, polling, cache merging, application behavior, persistence, transport, and OS capabilities SHALL remain outside this change.

#### Scenario: Decode every Apple compact detail level
- **WHEN** Apple production receives a valid direct or CAN-routed command-10 detail-level 1 through 4 payload
- **THEN** Rust supplies every exact normalized field and optional absence while Swift executes no command-10 length, byte-reading, scaling, semantic-state, enum, or sentinel algorithm

#### Scenario: Decode an Apple active firmware fault
- **WHEN** Apple production receives the exact command-10 active-firmware-fault form with a nonzero known or unknown code
- **THEN** Rust supplies the raw and classified fault through the canonical record and Swift constructs the established unknown-ride-state firmware-fault telemetry without classifying payload bytes

#### Scenario: Preserve established Apple failures
- **WHEN** Rust reports an unsupported envelope, truncated or invalid payload, invalid caller contract, insufficient output, ABI mismatch, malformed record, or contained internal failure
- **THEN** Swift returns the established bounded protocol error without partial telemetry, payload-bearing diagnostics, or a native fallback decoder

#### Scenario: Preserve higher-level Apple owners
- **WHEN** Rust returns a valid legacy telemetry record
- **THEN** existing compatibility, route attribution, framing, scheduling, polling, cache merging, state publication, session logging, and application behavior continue unchanged

#### Scenario: Preserve Android production ownership
- **WHEN** the Apple legacy-telemetry cutover completes
- **THEN** Android continues through its Kotlin command-10 decoder without linking, packaging, loading, or calling the Apple bridge

#### Scenario: Retire temporary protocol bridges later
- **WHEN** a future unified Rust application runtime owns protocol and application behavior on every host
- **THEN** the per-protocol Apple and Android legacy-telemetry bridges are removed rather than retained as a permanent API layer
