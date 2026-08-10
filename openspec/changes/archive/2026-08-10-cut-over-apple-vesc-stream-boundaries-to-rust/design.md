## Context

The preceding Apple cutover selected Rust for VESC CRC, frame encoding, and exact frame decoding while deliberately leaving notification-stream reassembly in Swift. Production ownership is therefore split at a framing boundary: the shared Swift `VescFrameStreamDecoder` still decides where noise ends, which header form is present, how much input is incomplete, and which complete frame should be removed, while Rust independently validates the selected frame.

The safe `backends/core/vesc-frame` crate already has a bounded, tested `StreamDecoder` that implements those decisions. The production Apple bridge is a small versioned C ABI with caller-owned buffers, fixed status values, panic containment, and an exact symbol allowlist. The shared Swift stream adapter is correspondingly small and is reset under the existing connection-generation lifecycle. This makes stream-boundary selection the smallest next vertical cut: it removes one duplicated algorithm without moving Bluetooth, protocol payload interpretation, application state, persistence, or ownership of a long-lived object across FFI.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole source of Apple VESC stream-boundary decisions while preserving the public Swift codec/stream interfaces and observable error and recovery behavior.
- Reuse one stateless Rust scanner from both the existing safe `StreamDecoder` and Apple production so their framing semantics cannot drift.
- Extend the audited Apple ABI by one bounded operation whose inputs and outputs remain borrowed or caller-owned primitives.
- Keep Swift responsible only for bounded byte storage, mechanical slicing/consumption, and stable Swift error mapping.
- Prove the cutover through focused component/ABI/Swift tests, clean packaging and canonical checks, `just verify`, and a representative physical iOS background-fragmentation checkpoint before deleting the Swift parser.

**Non-Goals:**

- Moving VESC command or payload interpretation, firmware identity, BMS, Refloat, application behavior, SQLite, the loopback server, Bluetooth, or any other OS capability into Rust.
- Changing the frontend contract, user-visible behavior, stored data, session formats, or third-party dependencies.
- Adding Android JNI/NDK integration or changing Android's Kotlin framing implementation.
- Introducing Rust-owned stream state, opaque handles, callbacks, generated bindings, returned allocations, or Rust layout across FFI.

## Decisions

### Use a safe stateless next-frame scanner

Add a safe scanner that borrows the current bounded byte region and returns one deterministic outcome: more input is needed, one candidate frame is complete, the declared payload is oversized, or the aggregate input is oversized. Its result contains validated primitive offsets/lengths describing the frame and the prefix to consume. It retains no input and allocates no cross-language state.

The scanner owns all interpretation: leading-noise disposal, short/long header parsing, declared payload bounds, incomplete suffix preservation, complete-frame extent, and forward progress after malformed input. The existing safe Rust `StreamDecoder` will be refactored to call this primitive, retaining its current owned buffer only as a convenience API. Shared immutable vectors will lock the scanner and decoder to identical decisions.

For a complete candidate, the reported consumed prefix includes leading noise and the complete candidate frame. Swift will exact-decode the reported frame region and consume that prefix even if exact decoding reports bad footer or CRC, preserving current recovery. An oversized declaration consumes leading noise plus one offending header byte so later input can make progress. Aggregate input beyond 8,192 bytes produces the established input-too-large outcome and causes the caller to clear the bounded storage. Input with no usable header consumes disposable noise but preserves any suffix that can begin a split header.

### Version the Apple C ABI and add exactly one scanner operation

Bump the hand-written ABI version and add one application symbol for stream scanning. The operation accepts a borrowed input pointer/length and distinct aligned `size_t` output pointers for frame offset, frame length, and consumed length. It returns the existing fixed status vocabulary plus an explicit `need more input` status. Complete-frame payload copying remains the job of the existing exact-decode operation.

The bridge validates input bounds, nullability, alignment, distinct output regions, and prohibited overlap before invoking safe Rust; initializes or writes outputs only according to the documented result; contains panics; and exposes no Rust allocator or type layout. The symbol audit changes from three to exactly four application symbols. A flat primitive interface is preferred to a C struct because it avoids making Rust or compiler layout part of the contract.

### Keep the Swift adapter stateful but algorithm-free

`VescFrameStreamDecoder` continues to own its bounded Swift byte array so its API, reset behavior, and connection-generation integration remain unchanged. After appending notification bytes, it repeatedly asks Rust to scan the current region, mechanically selects any reported frame slice, calls the existing Rust exact decoder, and removes exactly the reported prefix. It stops on the Rust incomplete result, maps fixed failures to established Swift errors, and clears storage on aggregate overflow.

Swift must not inspect header bytes, calculate declared lengths or frame extents, search for start markers, decide incomplete suffixes, or implement alternative recovery. Reset remains a Swift buffer clear and requires no Rust destructor. This deliberately leaves storage ownership in Swift until a later application-owner migration can absorb longer-lived Rust state without creating a temporary FFI lifetime model.

### Gate deletion after representative production evidence

Implementation proceeds additively: introduce and verify the Rust primitive, extend and test the ABI, cut the shared Swift adapter over, and run focused plus clean package/canonical gates. The required physical checkpoint is the focused iOS production-CoreBluetooth background scenario with fragmented fixture notifications, rather than an unrelated full physical matrix. Only after it passes will the Swift header/length/noise parser and any temporary test reference be deleted and source audits tightened to reject its return.

This ordering preserves a reviewable rollback point before code deletion while still ending the change with one production algorithm. Android remains untouched and must continue to pass its Kotlin-owned conformance gates.

### Correct ownership documentation with the cutover

Crate and backend documentation will describe the safe framing crate as the selected Apple framing owner and distinguish the still-deferred Android and higher-layer migrations. Documentation updates are part of the final cutover so maintainers do not follow the stale description of the crate as wholly unlinked candidate code.

## Risks / Trade-offs

- **Scanner and caller could disagree about consumption.** Immutable vectors will assert offsets and consumed lengths for noise, split headers, multiple frames, malformed complete frames, oversized declarations, and overflow; Swift tests will assert the resulting recovery sequence.
- **The fourth ABI call expands unsafe validation surface.** The operation uses only primitive pointers and lengths, has explicit distinct/aligned/overlap rules, and is covered through Rust bridge tests plus the compiled C harness.
- **Scanning and exact decoding require two FFI calls per complete frame.** Notification buffers are strictly bounded and the split keeps one exact-decoder implementation; canonical and physical checks will catch unacceptable behavior before deletion. A combined or stateful interface can be reconsidered only with measured need.
- **Swift continues to own stream bytes temporarily.** This is intentional: it removes duplicated interpretation now without introducing an opaque Rust lifetime that would be replaced again when application ownership moves.
- **Final deletion depends on a physical device checkpoint.** The parser remains available only during implementation until the focused checkpoint succeeds; the change is not complete or archivable before deletion and the final full software rerun.

## Migration Plan

1. Add the safe scanner and exhaustive result/consumption tests, then refactor the existing Rust `StreamDecoder` to use it without behavior change.
2. Version the Apple ABI, add the scanner symbol and bridge/C-harness coverage, and update build and symbol audits.
3. Convert the shared Swift stream decoder into a storage-only adapter and run focused Rust, C, and Swift tests plus clean Apple packaging/canonical checks.
4. Run the representative physical iOS fragmented-background checkpoint.
5. Delete the superseded Swift stream parser/reference, tighten ownership audits and documentation, then rerun the complete software and unchanged Android gates.

Rollback before final deletion is a source revert to the preceding exact-codec cutover. There is no database, session, frontend-contract, or user-data migration.

## Open Questions

None. The change intentionally fixes the ABI and ownership boundary narrowly; broader state ownership is deferred to a separate OpenSpec change.
