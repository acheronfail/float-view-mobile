## MODIFIED Requirements

### Requirement: Apple Rust frame codec cutover evidence

Apple framing ownership SHALL NOT be considered complete until focused Rust/C/Swift checks, clean Apple packaging checks, `just verify`, canonical macOS execution, iOS simulator build/launch, and the representative physical iOS BLE checkpoint all exercise Rust CRC, encode, exact decode, and stream boundary scanning without a Swift framing fallback. Evidence SHALL cover immutable short/long vectors, the 4,096-byte payload limit, the 8,192-byte aggregate stream limit, malformed length/header/footer/CRC/trailing bytes, caller-region failures, one-byte and fixture fragmentation, split long headers, leading noise, multiple frames, incomplete suffix retention, reported consumption lengths, malformed-frame recovery, and aggregate overflow recovery.

The physical checkpoint SHALL use production CoreBluetooth and the shared Apple protocol path with fixture-supplied fragmented notifications and SHALL include the canonical iOS ride/background-recovery behavior. After that checkpoint succeeds, repository verification SHALL require Swift CRC, frame encode/exact decode, and stream header/length/noise parsing algorithms to be absent and SHALL rerun the complete software gate. This cutover SHALL add no Android target adapter or alter Android's required results.

#### Scenario: Exercise the compiled ABI

- **WHEN** focused verification runs the C harness and Swift codec/stream tests
- **THEN** the hand-written header, ABI version, exact four-symbol allowlist, fixed statuses, pointer and primitive-output bounds, panic containment, scanner consumption lengths, immutable frame bytes, and established Swift error mappings agree with the safe Rust component

#### Scenario: Verify Apple software production selection

- **WHEN** `just verify` and clean macOS and iOS simulator checks run after stream adapter cutover
- **THEN** the shared Apple codec and stream tests pass through linked Rust scanning and exact decoding and no Swift production framing algorithm or fallback is present

#### Scenario: Verify physical iOS stream framing

- **WHEN** focused physical iOS conformance performs a fragmented BLE ride and mobile background recovery
- **THEN** production CoreBluetooth notification bytes are bounded in Swift, interpreted and exactly decoded by Rust, and the same session plus additional samples survive foreground recovery

#### Scenario: Delete superseded Apple stream code

- **WHEN** focused, packaging, canonical, and physical checkpoints have passed
- **THEN** the change deletes Swift stream header/length/noise parsing, retains only the thin caller-owned byte-storage adapter, and passes the complete software gate again

#### Scenario: Preserve Android conformance

- **WHEN** the Apple stream-boundary cutover is verified
- **THEN** Android continues to pass its existing canonical and physical requirements through the Kotlin framing implementation without JNI, NDK, or Rust packaging
