## 1. Define one safe Rust stream scanner

- [x] 1.1 Add public stateless next-frame result types and a borrowed-input scanner that owns noise disposal, short/long header interpretation, declared-length bounds, incomplete suffix retention, complete-frame extent, malformed progress, and the 8,192-byte aggregate bound without unsafe code or new dependencies.
- [x] 1.2 Refactor the existing safe Rust `StreamDecoder` to use the scanner for every boundary decision while retaining its existing owned-buffer API and error/recovery behavior.
- [x] 1.3 Add immutable scanner and `StreamDecoder` parity tests for empty and one-byte input, split short/long headers, leading noise, one-byte and fixture fragmentation, multiple frames, incomplete suffixes, 4,096-byte payloads, oversized declarations, bad footer/CRC recovery, exact consumption lengths, and aggregate overflow recovery.
- [x] 1.4 Update the safe crate documentation to describe its selected Apple framing role and the still-deferred Android and higher-layer ownership boundaries.

## 2. Extend the audited Apple ABI

- [x] 2.1 Bump the fixed ABI version and extend the hand-written C header with one stream-scan operation, an explicit need-more-input status, primitive frame-offset/frame-length/consumed-length outputs, and documented caller ownership and bounds.
- [x] 2.2 Implement the scanner export with pre-dereference null, alignment, distinct-region, overlap, and input-bound validation plus fixed status mapping and panic containment; retain no pointer or allocation after the call.
- [x] 2.3 Extend Rust bridge tests and the compiled C harness across scanner success/incomplete/error outcomes, immutable inputs, output initialization and bounds, invalid caller regions, exact consumption values, malformed recovery, overflow, and contained internal failure.
- [x] 2.4 Change the ABI/version audit to require exactly version, encode, exact-decode, and stream-scan application symbols and to reject a missing, stale, or additional entry point.
- [x] 2.5 Prove the versioned four-symbol archive builds from source for clean macOS Debug/Release, iOS simulator, and unsigned iOS device configurations without changing the existing target matrix or committing native artifacts.

## 3. Cut over the shared Swift stream adapter

- [ ] 3.1 Add the thin Swift scanner bridge call, validate Rust-returned offsets/lengths before slicing, and map the fixed incomplete and failure statuses onto the established Swift stream behavior.
- [ ] 3.2 Replace Swift header, length, noise, incomplete-suffix, and overflow decisions in `VescFrameStreamDecoder` with a bounded storage/scan/exact-decode/consume loop that consumes Rust's reported prefix even when exact decoding rejects a complete candidate.
- [ ] 3.3 Preserve the public stream API, connection-generation reset, locking, error propagation, and iOS/macOS shared call sites while ensuring reset only clears Swift-owned bytes and no Rust lifetime crosses FFI.
- [ ] 3.4 Extend focused Swift tests to prove fragmented/noisy/multiple input, split long headers, incomplete suffix retention, maximum payload, malformed complete-frame recovery, oversized declarations, aggregate overflow, ABI mismatch, and stable error mappings through linked Rust scanning.
- [ ] 3.5 Update Apple/core ownership documentation to identify Rust as the Apple CRC, encode, exact-decode, and stream-boundary owner; Swift as bounded storage only; and Android as the unchanged Kotlin owner.

## 4. Verify the additive production cutover

- [ ] 4.1 Run Rust formatting, warnings-denied linting and tests, the compiled C ABI harness, focused Swift codec/stream tests, and the exact symbol, dependency, artifact, and source-ownership audits.
- [ ] 4.2 Run the clean macOS and iOS Debug/Release packaging matrix, canonical macOS execution, and an iOS simulator build/launch with both sibling hosts selecting the shared Rust scanner and no production fallback.
- [ ] 4.3 Run `just verify` with the additive stream cutover selected and resolve every software, canonical, ownership, and packaging regression before the deletion checkpoint.

## 5. Complete the physical deletion checkpoint

- [ ] 5.1 Run the focused physical iOS CoreBluetooth ride with fixture-supplied notification fragmentation and mobile background/foreground recovery, proving Rust scanning/exact decode preserves the same session and continued samples.
- [ ] 5.2 Delete the superseded Swift stream header/length/noise parser and any temporary test-only reference, then tighten repository audits so only bounded Swift storage/consumption remains and the parsing algorithm cannot return.
- [ ] 5.3 Rerun the focused Rust/C/Swift checks, exact symbol and source audits, clean Apple packaging matrix, canonical macOS execution, and iOS simulator build/launch after deletion while confirming Android still uses Kotlin with no JNI/NDK or Rust packaging.
- [ ] 5.4 Run the final `just verify` software gate and resolve all regressions without changing the frontend contract, database, stored data, OS-capability ownership, or user-visible behavior.
