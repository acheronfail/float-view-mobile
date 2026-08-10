## 1. Extend the audited Android JNI bridge

- [ ] 1.1 Bump the Android framing ABI to version 2, add the fixed `NEED_MORE_INPUT` status and internal scanner result model, and route every semantic boundary outcome through the existing safe Rust `scan_next` without changing the safe crate or adding dependencies.
- [ ] 1.2 Add exactly one `scanNext` JNI export accepting a borrowed JVM byte array and exact three-element integer result array, with pre-access environment, pending-exception, null, shape, identity, bound, initialization, and returned-range validation plus panic containment and no retained reference or allocation.
- [ ] 1.3 Handle aggregate inputs above 8,192 bytes without copying their contents, report the complete consumed length for clearing, and preserve fixed output rules for success, incomplete input, oversized declarations, invalid callers, and internal failure.
- [ ] 1.4 Extend Rust bridge and host-JVM native-call tests across empty and maximum input, split short/long headers, leading noise, multiple frames, malformed complete frames, oversized declarations, aggregate overflow, exact offsets/consumption, wrong-shaped or aliased arrays, pending exceptions, immutable input, and contained panic.
- [ ] 1.5 Update ABI, dependency, exact four-symbol, architecture, and tracked-artifact audits, then prove the versioned bridge builds from locked source for the host JVM and all four existing Android ABIs in Debug and Release without changing Apple artifacts.

## 2. Cut the Kotlin stream adapter over

- [ ] 2.1 Add the private `scanNext` JNI declaration and thin Kotlin status/result adapter, verify ABI version 2, defensively validate returned primitive ranges, and map incomplete and failure statuses onto the established `VescProtocolException` behavior.
- [ ] 2.2 Replace `VescFrameStreamDecoder` header, length, noise, incomplete-suffix, frame-extent, recovery, and aggregate-overflow decisions with a storage/scan/slice/consume/exact-decode loop while preserving its public API, caller-owned bounded storage, connection-generation reset, ordering, and call sites.
- [ ] 2.3 Ensure Kotlin consumes Rust's complete-candidate prefix before propagating exact-decode footer or CRC failure, advances exactly as Rust reports for oversized declarations, clears exactly as Rust reports for aggregate overflow, and contains no production parser or fallback path.
- [ ] 2.4 Extend linked host-JVM Kotlin tests for one-byte and fixture fragmentation, split short and long headers, disposable noise, incomplete suffix retention, ordered multiple frames, empty and maximum payloads, bad footer/CRC recovery, oversized declarations, aggregate overflow recovery, ABI mismatch, invalid scanner results, and stable errors.
- [ ] 2.5 Quarantine any pre-cutover Kotlin parser only as a private test-source parity checkpoint, absent from production selection and packaging, and compare its immutable results with Rust until the physical deletion checkpoint.
- [ ] 2.6 Update Android/core ownership documentation to identify Rust as the complete Android framing-algorithm owner, Kotlin as bounded storage and mechanical consumption only, Apple as unchanged, and all higher protocol/runtime ownership as deferred.

## 3. Verify the additive production cutover

- [ ] 3.1 Run Rust formatting, warnings-denied linting and tests, bridge dependency/status/symbol audits, host-JVM native-call coverage, focused linked Kotlin codec/stream tests, and source/artifact ownership audits through the selected Rust scanner.
- [ ] 3.2 Run clean Android Debug/Release four-ABI packaging and APK inspection, the complete Android unit suite, canonical Android build/launch/conformance, and unchanged Apple packaging/canonical checks while proving every loaded or packaged library reports ABI version 2.
- [ ] 3.3 Run `just verify` with Rust stream scanning selected and no production Kotlin fallback, resolving every software, canonical, packaging, dependency, ownership, and behavior regression before the physical checkpoint.

## 4. Complete the physical deletion checkpoint

- [ ] 4.1 Run the focused physical Android production-Bluetooth ride with fixture-supplied fragmented notifications and mobile background/foreground recovery, proving Rust boundary selection and exact decoding preserve the same session and continued samples.
- [ ] 4.2 Delete the superseded Kotlin stream parser and temporary test parity reference, then tighten repository audits so only Kotlin bounded storage, defensive range validation, mechanical slicing/consumption, and error mapping remain and the parsing algorithm cannot return.
- [ ] 4.3 Rerun focused Rust/JNI/Kotlin checks, exact dependency/symbol/source audits, clean host/four-ABI packaging, canonical Android execution, and unchanged Apple packaging/canonical verification after deletion.
- [ ] 4.4 Run the final `just verify` software gate and resolve all regressions without changing the frontend contract, database, stored data, OS-capability ownership, public framing behavior, or user-visible behavior.
