## ADDED Requirements

### Requirement: Android Rust frame codec cutover evidence

Android stateless frame-codec ownership SHALL NOT be considered complete until focused Rust/JNI/Kotlin checks, clean host-JVM and Android Debug/Release packaging checks, `just verify`, canonical Android execution, and a representative physical Android BLE checkpoint all exercise Rust CRC, encode, and exact decode without a Kotlin codec fallback. Evidence SHALL cover immutable short/long vectors, empty and 4,096-byte payloads, malformed length/header/footer/CRC/trailing bytes, null/alias/capacity/status failures, fragmentation, leading noise, multiple frames, retained incomplete suffixes, malformed-frame recovery, and aggregate stream failure through the retained Kotlin stream owner.

The physical checkpoint SHALL use production Android Bluetooth and the normal Android protocol path with fixture-supplied fragmented notifications and SHALL include the canonical Android ride/background-recovery behavior. After that checkpoint succeeds, repository verification SHALL require Kotlin CRC, frame encoding, and exact-decoding algorithms to be absent and SHALL rerun the complete software gate. This cutover SHALL not alter the frontend contract, Apple target adapters, conformance scenario logic, or Android's Kotlin stream-boundary results.

#### Scenario: Exercise the compiled JNI boundary

- **WHEN** focused verification runs bridge tests and the linked Android host-JVM codec and stream tests
- **THEN** ABI version, exact three-symbol allowlist, fixed statuses, primitive-array bounds, panic containment, immutable frame bytes, and established Kotlin error mappings agree with the safe Rust component

#### Scenario: Verify Android software production selection

- **WHEN** `just verify`, clean Gradle unit tests, Debug/Release assembly, and canonical Android conformance run after adapter cutover
- **THEN** Android codec and retained stream tests pass through linked Rust encode and exact decode and no Kotlin production codec algorithm or fallback is present

#### Scenario: Verify physical Android framing

- **WHEN** focused physical Android conformance performs a fragmented BLE ride and mobile background recovery
- **THEN** production Android Bluetooth requests are Rust-encoded, every complete inbound frame is Rust-decoded, and the same session plus additional samples survive foreground recovery

#### Scenario: Delete superseded Android codec code

- **WHEN** focused, packaging, canonical, and physical checkpoints have passed
- **THEN** the change deletes Kotlin CRC, frame-construction, and exact-decode algorithms, retains the thin codec adapter and Kotlin stream-boundary state, and passes the complete software gate again

#### Scenario: Preserve Apple conformance

- **WHEN** the Android codec cutover is verified
- **THEN** iOS and macOS continue to pass their existing software, packaging, canonical, and physical requirements through the unchanged Apple Rust framing path
