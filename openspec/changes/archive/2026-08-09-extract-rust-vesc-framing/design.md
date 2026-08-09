## Context

Float View currently ships small, hand-written VESC framing implementations in Swift and Kotlin, with a third framing implementation in the conformance fixture runner. They agree on the product's bounded wire behavior: CRC-16, short and long packet forms, exact frame validation, a 4,096-byte payload ceiling, and an 8,192-byte stream-buffer ceiling.

The reverted `share-native-backend-core` experiment attempted to introduce the Rust toolchain, cross-language bindings, platform packaging, protocols, application behavior, persistence, and the loopback server as one migration. That made it difficult to distinguish a protocol defect from a build, bridge, lifecycle, or ownership defect. This change instead establishes one independently testable Rust component without putting it on a production call path.

The repository has an installed Rust 1.93.0 host toolchain. No iOS, Android, or other cross-compilation target is required to prove this component. Authoritative protocol details are available offline in `vendor/bldc/comm/packet.c` and `vendor/bldc/util/crc.c`; current product bounds and compatibility behavior remain defined by the shipped implementations, fixtures, and tests.

## Goals / Non-Goals

**Goals:**

- Establish a minimal, locked Rust workspace at `backends/core` with exactly one VESC framing crate.
- Reproduce the shipped framing and stream-decoding behavior from authoritative sources without changing the frontend contract or production behavior.
- Exercise the Rust implementation against immutable source-derived vectors and the same edge-case classes covered by Swift, Kotlin, and conformance.
- Make Rust formatting, linting, and tests part of the ordinary `just verify` software gate.
- Define how later OpenSpec changes can move one owner at a time from native code into Rust and delete superseded code only after a verified host cutover.

**Non-Goals:**

- FFI, JNI, generated headers, Android NDK configuration, Apple static-library or XCFramework packaging, or any host callback API.
- Replacing the Swift, Kotlin, or conformance framing implementation in this change.
- BLE, location, lifecycle, files, theme, application-service, SQLite, loopback-server, or frontend-contract changes.
- A general runtime, actor model, shared service container, empty placeholder crates, or speculative common utilities.
- Installing or verifying Rust cross-compilation targets.

## Decisions

### 1. Extract a shadow component before integrating a host

The first Rust artifact is a candidate implementation exercised only by component tests and repository verification. Swift and Kotlin remain the production owners. A later Apple cutover and a later Android cutover will each be separate OpenSpec changes so packaging, bridge safety, lifecycle behavior, and deletion can be diagnosed per host.

An FFI-first extraction was rejected because it would combine wire-behavior proof with two platform build systems and language-boundary design. Repeating the broad shared-core workspace was rejected because unused future crates would recreate coupling before any boundary was proven.

### 2. Use one dependency-free production crate

`backends/core` will be a locked Cargo workspace containing one crate dedicated to VESC framing. The crate will have no normal third-party dependencies. If parsing an existing immutable JSON fixture is necessary, a minimal directly used parser may be locked as a development-only dependency; it must not enter the production dependency graph.

The workspace will pin Rust 1.93.0 and the formatting and lint components already needed by the software gate. It will not list mobile targets because no mobile artifact is built in this step. `Cargo.lock` is committed so the verification graph is deterministic.

### 3. Preserve exact product framing semantics

The crate will implement the VESC CRC-16 algorithm with polynomial `0x1021` and zero initial state. Encoding uses header `2` with a one-byte payload length through 255 bytes, header `3` with a two-byte big-endian payload length above 255 bytes, two big-endian CRC bytes, and footer `3`. Payloads above 4,096 bytes are rejected.

Exact decoding rejects unsupported headers, incomplete or trailing bytes, declared payloads above the product bound, invalid footers, and CRC mismatches. Stream decoding discards leading noise, accepts fragmented and multiple frames, retains only bounded incomplete data, and reports and clears invalid or over-limit state consistently with the shipped implementations. Current product fixtures and tests decide compatibility behavior where a product-specific bound or edge case is more restrictive or explicit than upstream firmware.

The implementation will use explicit error variants and checked length arithmetic. It will not log payloads, addresses, tokens, or telemetry.

### 4. Reuse evidence instead of inventing a Rust-only oracle

Rust tests will consume the repository's immutable source-derived frame vectors where practical and will cover valid short and long frames, empty and maximum payload boundaries, one-byte fragmentation, split headers, multiple frames, leading noise, CRC corruption, invalid footer/header, oversized declarations, buffer overflow, and recovery after malformed input. Any fixture additions must remain language-neutral and be exercised by the existing implementations or conformance where applicable.

Rust success is component parity, not backend conformance. The canonical black-box suite continues to regard Android, iOS, and macOS as the supported backends and continues to exercise their selected production paths.

### 5. Put deterministic Rust checks in the existing software gate

The root verification recipes will run locked Cargo formatting checks, Clippy with warnings denied, and workspace tests. Focused recipes may expose these checks separately, but `just verify` remains the single software gate. This change will also retain the existing Swift, Kotlin, frontend, and conformance checks so the new toolchain cannot mask a regression.

### 6. Make staged ownership explicit

`backends/core` is the only retained location for Rust backend components. A Rust candidate may coexist with native production code only when an active OpenSpec change names the bounded slice, its parity evidence, and its deletion checkpoint. Coexistence does not transfer production ownership.

A later cutover change may add one narrow, audited host bridge and select Rust as that host's production owner. It must verify the complete host gate before deleting the corresponding native implementation. Platform hosts always retain OS-owned Bluetooth, location, background lifecycle, permissions, file presentation, theme, and UI integration even as portable protocol, application, server, and persistence slices move below those adapters.

## Risks / Trade-offs

- **Temporary duplication can drift.** The Rust crate is deliberately small, shares immutable evidence with existing implementations, and is followed by separate cutover changes. Any framing behavior change must update every still-live owner until deletion.
- **A green component test does not prove host integration.** The change explicitly makes no shipping claim; future Apple and Android bridge changes own packaging, ABI, lifecycle, and physical conformance risk.
- **A new toolchain increases the default gate.** One crate, a pinned installed host toolchain, a committed lockfile, and no production dependencies keep the added work bounded and reproducible.
- **Upstream and product limits can differ.** Upstream sources define the wire algorithm while current repository bounds and compatibility tests define product acceptance.
- **Development-only dependencies can expand unnoticed.** Any fixture parser must be directly justified, development-only, locked, and visible to dependency auditing.

## Migration Plan

1. Add the pinned one-member Rust workspace and document its ownership boundary.
2. Implement CRC, exact frame encoding/decoding, and bounded stream reassembly from the offline sources and current product behavior.
3. Add source-derived parity and adversarial component tests.
4. Add Rust formatting, linting, and tests to `just verify`, then run the complete software gate.
5. Leave every production call path and packaged application unchanged.
6. Follow with separate Apple and Android host-cutover changes. Each change introduces only its required bridge, proves the host's complete gate, and deletes that host's superseded framing code after success.

Rollback is removal of the isolated workspace and its verification recipes; because this change has no production integration or data migration, rollback does not affect users or stored data.

## Open Questions

None. Whether fixture parsing needs a development-only dependency can be resolved during implementation without changing the architecture or production dependency graph.
