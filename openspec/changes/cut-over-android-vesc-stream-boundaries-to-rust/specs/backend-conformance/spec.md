## MODIFIED Requirements

### Requirement: Android Rust frame codec cutover evidence

Complete Android frame-codec ownership SHALL NOT be considered transferred until focused Rust/JNI/Kotlin checks, clean host-JVM and Android Debug/Release packaging checks, `just verify`, canonical Android execution, and a representative physical Android BLE checkpoint all exercise Rust CRC, encode, exact decode, and stream boundary scanning without a Kotlin framing fallback. Evidence SHALL cover immutable short/long vectors, empty and 4,096-byte payloads, malformed length/header/footer/CRC/trailing bytes, null/alias/shape/capacity/status failures, one-byte and fixture fragmentation, split short and long headers, leading noise, multiple frames, retained incomplete suffixes, malformed-frame recovery, oversized declarations, exact consumption lengths, and aggregate stream failure through Kotlin-owned bounded storage.

The physical checkpoint SHALL use production Android Bluetooth and the normal Android protocol path with fixture-supplied fragmented notifications and SHALL include the canonical Android ride/background-recovery behavior. After that checkpoint succeeds, repository verification SHALL require Kotlin CRC, frame encoding, exact decoding, and stream boundary algorithms to be absent and SHALL rerun the complete software gate. This cutover SHALL not alter the frontend contract, Apple target adapters, conformance scenario logic, or established Android stream results.

#### Scenario: Exercise the compiled JNI boundary

- **WHEN** focused verification runs bridge tests and the linked Android host-JVM codec and stream tests
- **THEN** ABI version, exact four-symbol allowlist, fixed statuses, primitive-array bounds, panic containment, immutable frame bytes, exact scanner consumption results, and established Kotlin error mappings agree with the safe Rust component

#### Scenario: Verify Android software production selection

- **WHEN** `just verify`, clean Gradle unit tests, Debug/Release assembly, and canonical Android conformance run after adapter cutover
- **THEN** Android codec and stream tests pass through linked Rust encode, exact decode, and stream scanning and no Kotlin production framing algorithm or fallback is present

#### Scenario: Verify physical Android framing

- **WHEN** focused physical Android conformance performs a fragmented BLE ride and mobile background recovery
- **THEN** production Android Bluetooth requests are Rust-encoded, inbound boundaries are Rust-selected, complete frames are Rust-decoded, and the same session plus additional samples survive foreground recovery

#### Scenario: Delete superseded Android stream code

- **WHEN** focused, packaging, canonical, and physical checkpoints have passed
- **THEN** the change deletes Kotlin header, length, noise, incomplete-suffix, frame-extent, recovery-decision, and aggregate-overflow parsing while retaining only the thin codec and bounded-storage adapter, then passes the complete software gate again

#### Scenario: Preserve Apple conformance

- **WHEN** the Android stream-boundary cutover is verified
- **THEN** iOS and macOS continue to pass their existing software, packaging, canonical, and physical requirements through the unchanged Apple Rust framing path
