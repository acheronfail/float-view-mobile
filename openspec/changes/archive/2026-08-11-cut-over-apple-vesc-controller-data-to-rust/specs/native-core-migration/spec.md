## ADDED Requirements

### Requirement: Apple Rust VESC controller-data cutover

The shared Apple backend SHALL use the safe Rust controller-data component as its sole production implementation of the consumed standard `COMM_GET_VALUES` realtime subset and `COMM_GET_MCCONF` current-limit subset on iOS and macOS. Rust SHALL decide command validity, every selected big-endian signed fixed-point scale, signed ERPM, raw fault and classification, consumed-prefix extent, permitted tail behavior, VESC auto-floats, finite nonzero limits, serialization signatures, series-cell offsets, optional cell-count behavior, and every structural or numeric failure. Swift MAY perform only bounded versioned bridge-record validation, primitive representation conversion into `RealtimeValues`, `VescFault`, and `ControllerCurrentLimits`, and established error mapping.

The cutover SHALL use a narrow audited Apple bridge, SHALL contain no runtime or source fallback to Swift controller-data parsing, and SHALL delete the superseded Swift algorithms after required verification. Request construction, firmware compatibility, route attribution, framing, scheduling, telemetry polling, fault response, application behavior, Android controller-data ownership, and every OS capability SHALL remain outside this change.

#### Scenario: Decode Apple production realtime values

- **WHEN** shared Apple production receives a structurally valid direct or CAN-routed command-4 payload
- **THEN** the thin Swift adapter obtains every consumed scalar, ERPM, raw fault, and classification from Rust and executes no Swift byte-reading, scaling, or fault-classification algorithm

#### Scenario: Decode Apple production current limits

- **WHEN** shared Apple production receives a structurally valid command-14 motor configuration
- **THEN** Rust returns all four finite nonzero limits and the optional positive series-cell value while Swift executes no auto-float, signature, offset, or optional-cell algorithm

#### Scenario: Preserve established Apple failures

- **WHEN** Rust reports an unsupported command, truncated or invalid data, invalid caller contract, insufficient output, ABI mismatch, invalid result record, or contained internal failure
- **THEN** Swift returns the established bounded `VescProtocolError` class without publishing partial values, logging payload data, or executing a fallback parser

#### Scenario: Preserve higher-level controller owners

- **WHEN** Rust returns valid realtime values or current limits
- **THEN** existing Swift compatibility, direct/CAN routing, framing, scheduling, telemetry, fault-response, state-publication, and session owners behave unchanged after representation mapping

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple controller-data cutover completes
- **THEN** Android continues using its Kotlin decoder with unchanged behavior and without linking, packaging, or calling the Apple bridge
