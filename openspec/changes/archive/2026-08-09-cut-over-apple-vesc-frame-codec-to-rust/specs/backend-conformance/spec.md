## ADDED Requirements

### Requirement: Apple Rust frame codec cutover evidence

The Apple stateless frame-codec ownership transfer SHALL NOT complete until focused Rust/C/Swift checks, clean Apple packaging checks, `just verify`, canonical macOS execution, iOS simulator build/launch, and the representative physical iOS BLE checkpoint all exercise the selected Rust production path without a Swift framing fallback. Evidence SHALL cover immutable short/long vectors, the 4,096-byte payload limit, malformed length/header/footer/CRC/trailing bytes, caller-buffer failures, fragmentation, leading noise, multiple frames, and bounded stream failure.

The physical checkpoint SHALL use the production CoreBluetooth and shared Apple protocol path with fixture-supplied fragmented notifications and SHALL include the canonical iOS ride/background-recovery behavior. After that checkpoint succeeds, repository verification SHALL require the Swift CRC, encoder, and exact decoder to be absent and SHALL rerun the complete software gate. This cutover SHALL add no Android target adapter or alter Android's required results.

#### Scenario: Exercise the compiled ABI

- **WHEN** focused verification runs the C harness and Swift codec tests
- **THEN** the hand-written header, ABI version, fixed statuses, buffer bounds, panic containment, immutable frame bytes, and established Swift error mappings agree with the safe Rust component

#### Scenario: Verify Apple software production selection

- **WHEN** `just verify` and clean macOS and iOS simulator checks run after adapter cutover
- **THEN** the shared Apple codec and retained stream tests pass through the linked Rust functions and no Swift production algorithm or fallback is present

#### Scenario: Verify physical iOS framing

- **WHEN** canonical physical iOS conformance performs a fragmented BLE ride and mobile background recovery
- **THEN** production CoreBluetooth traffic is encoded and each complete inbound frame is validated by Rust while the same session and additional samples survive foreground recovery

#### Scenario: Delete superseded Apple code

- **WHEN** focused, packaging, canonical, and physical checkpoints have passed
- **THEN** the change deletes the Swift CRC/encode/exact-decode algorithms, retains only the thin adapter and stream-boundary state, and passes the complete software gate again

#### Scenario: Preserve Android conformance

- **WHEN** the Apple codec cutover is verified
- **THEN** Android continues to pass its existing canonical and physical requirements through the Kotlin framing implementation without JNI or Rust packaging
