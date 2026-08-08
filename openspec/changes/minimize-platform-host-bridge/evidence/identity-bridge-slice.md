# Stable identity and focused bridge slice

## Implemented

- Added distinct bounded Rust types for private platform associations and meaningful nonzero 12-byte VESC firmware UUIDs.
- Persisted meaningful firmware UUIDs only for saved boards, retained the existing private fallback for unavailable identity, and revalidated UUID identity on reconnect.
- Replaced host-generated random aliases with direct private Apple/Android transport associations; Rust publishes only per-runtime redacted discovery identifiers.
- Changed the command header kind to the actual command discriminator and removed the duplicated version, generation, operation, and inner tag from command payloads.
- Added focused C and explicitly registered JNI event entry points while retaining the bounded raw BLE notification entry point.
- Updated the private ABI to version 3 with build identity `float-view-abi03` and regenerated the pinned C header/checksum.
- Removed production Swift/Kotlin platform-event envelope encoders and decoders.

## Focused verification

- `cargo check --workspace --all-targets --offline`
- `just core-header-check`
- `just test apple`
- `just test android`
- Focused Rust tests for domain identity bounds, storage, backend identity behavior, bridge codecs, C ABI, and runtime queues.

The loopback-binding host-runtime test requires permission to bind localhost in the execution sandbox; the same startup path is exercised by the native suites and the complete gate will be run with its required permissions before completion.
