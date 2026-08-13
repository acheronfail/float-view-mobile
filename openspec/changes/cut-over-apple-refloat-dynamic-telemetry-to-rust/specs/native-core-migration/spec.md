## ADDED Requirements

### Requirement: Apple Rust Refloat dynamic telemetry cutover

The shared Apple backend SHALL use `refloat-dynamic-telemetry` as its sole production implementation of Refloat command-32 layout and command-31 realtime decoding on iOS and macOS. Rust SHALL decide exact envelopes, UTF-8 identifiers, negotiated ordering and bounds, mask-dependent extents, complete finite source float16 values, source ticks, semantic state, behavior flags, charging values, active-alert words, fatal latch, firmware-fault classification, known fields, and bounded ordered extensions before publishing canonical versioned records. Swift SHALL only validate and mechanically map those records into existing layout and telemetry models.

The selected boundary SHALL be stateless across calls: Apple SHALL retain only the canonical bounded layout record in its existing connection generation, clear it on every reset or replacement, and supply it to Rust for each realtime decode. No Swift command-31/32 wire parser, float16 decoder, semantic mapping, fallback, Rust opaque handle, allocation, reference, or callback SHALL remain across the boundary. Compatibility, request construction, framing, direct/CAN routing, polling, cache merging, observation time, application state, persistence, transport, and OS capabilities SHALL remain with their existing owners.

#### Scenario: Negotiate and retain one layout

- **WHEN** Apple receives a valid command-32 response for the current connection generation
- **THEN** Rust returns the exact canonical layout record, Swift publishes the existing ordered field lists, and the same bounded record is retained only until reset or replacement

#### Scenario: Decode realtime through the retained layout

- **WHEN** Apple receives a valid command-31 response after layout negotiation
- **THEN** Rust validates the canonical layout and payload and returns one complete canonical sample that Swift mechanically maps without interpreting raw command bytes

#### Scenario: Fail closed without a Swift fallback

- **WHEN** the ABI, caller regions, layout record, payload, status, record extent, version, code, flags, options, reserved bytes, numeric values, alert/fault relation, extension entries, or canonical absent values are invalid
- **THEN** Apple returns its established bounded protocol failure without partial state, payload diagnostics, stale-layout reuse, or native wire decoding

#### Scenario: Preserve the complete source float16 range

- **WHEN** production Apple telemetry contains zero, signed zero, a subnormal, a normal, exponent `0x1f`, or either finite source saturation endpoint
- **THEN** Rust publishes the exact finite value and Swift does not impose IEEE infinity or NaN semantics on the source format

#### Scenario: Preserve higher-level Apple owners

- **WHEN** Rust returns a valid layout or realtime record
- **THEN** existing Apple compatibility, routing, framing, scheduling, polling, connection lifetime, cache merging, application state, persistence, background execution, and frontend publication behave unchanged

#### Scenario: Preserve Android production ownership

- **WHEN** the Apple cutover builds, packages, and executes
- **THEN** Android continues using its Kotlin dynamic decoder without linking, loading, packaging, or calling the Apple bridge

#### Scenario: Retire the temporary bridge with the unified runtime

- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the per-protocol Apple FFI and Android JNI dynamic-telemetry crates and host record adapters are deleted rather than retained as permanent APIs
