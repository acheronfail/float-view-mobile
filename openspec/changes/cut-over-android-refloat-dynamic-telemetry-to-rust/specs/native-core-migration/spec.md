## ADDED Requirements

### Requirement: Android Rust Refloat dynamic telemetry cutover

The Android backend SHALL use `refloat-dynamic-telemetry` as its sole production implementation of Refloat command-32 layout and command-31 realtime decoding. Rust SHALL decide exact envelopes, UTF-8 identifiers, negotiated ordering and bounds, mask-dependent extents, complete finite source float16 values, source ticks, semantic state, behavior flags, charging values, active-alert words, fatal latch, firmware-fault classification, known fields, and bounded ordered extensions before publishing canonical versioned records. Kotlin SHALL only validate and mechanically map those records into existing layout and telemetry models.

The selected boundary SHALL be stateless across calls: Android SHALL retain only the canonical bounded layout record in its existing connection generation, clear it on every reset, replace it only after a completely valid new layout, and supply it to Rust for each realtime decode. No Kotlin command-31/32 wire parser, float16 decoder, semantic mapping, fallback, Rust opaque handle, allocation, JVM reference, or callback SHALL remain across the boundary. Compatibility, request construction, framing, direct/CAN routing, polling, cache merging, observation time, application state, persistence, transport, and OS capabilities SHALL remain with their existing owners.

#### Scenario: Negotiate and retain one layout

- **WHEN** Android receives a valid command-32 response for the current connection generation
- **THEN** Rust returns the exact canonical layout record, Kotlin publishes the existing ordered field lists, and the same bounded record is retained only until reset or valid replacement

#### Scenario: Reject a layout replacement atomically

- **WHEN** Android receives a malformed command-32 response after accepting a layout
- **THEN** the replacement fails without partially changing the accepted field lists or canonical record

#### Scenario: Decode realtime through the retained layout

- **WHEN** Android receives a valid command-31 response after layout negotiation
- **THEN** Rust validates the canonical layout and payload and returns one complete canonical sample that Kotlin mechanically maps without interpreting raw command bytes

#### Scenario: Fail closed without a Kotlin fallback

- **WHEN** the ABI, JVM inputs, layout record, payload, status, record extent, version, code, flags, options, reserved bytes, numeric values, alert/fault relation, extension entries, or canonical absent values are invalid
- **THEN** Android returns its established bounded protocol failure without partial state, payload diagnostics, stale-layout reconstruction, or native wire decoding

#### Scenario: Preserve the complete source float16 range

- **WHEN** production Android telemetry contains zero, signed zero, a subnormal, a normal, exponent `0x1f`, or either finite source saturation endpoint
- **THEN** Rust publishes the exact finite value and Kotlin does not impose IEEE infinity or NaN semantics on the source format

#### Scenario: Preserve higher-level Android owners

- **WHEN** Rust returns a valid layout or realtime record
- **THEN** existing Android compatibility, routing, framing, scheduling, polling, connection lifetime, cache merging, application state, persistence, background execution, and frontend publication behave unchanged

#### Scenario: Preserve Apple production ownership

- **WHEN** the Android cutover builds, packages, and executes
- **THEN** iOS and macOS continue using their selected Apple bridge without linking, loading, packaging, or calling the Android JNI bridge

#### Scenario: Retire the temporary bridges with the unified runtime

- **WHEN** a later independently specified change moves complete local application behavior and protocol orchestration behind one Rust runtime boundary
- **THEN** the per-protocol Apple FFI and Android JNI dynamic-telemetry crates and host record adapters are deleted rather than retained as permanent APIs
