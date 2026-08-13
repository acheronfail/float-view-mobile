## Context

Both production backends implement the same connection-scoped Refloat configuration lifecycle above the now-shared Rust codec. The services own release-catalogue admission, optional schema-cache use, bounded command-92 paging, generation and operation matching, flywheel gating, current/default revisions, command-95 acknowledgement followed by command-93 verification, normalized readback comparison, and recovery. They differ mainly in language syntax and host scheduler/cache adapters. Moving this state machine directly through C/JNI would require long-lived foreign objects or large record adapters; extracting and proving a safe Rust owner first keeps this change independent of those temporary boundaries.

## Goals / Non-Goals

**Goals:**

- Define one safe, stateful Rust service whose public API is platform-neutral typed facts, events, snapshots, cache decisions, and request effects.
- Reuse the existing Rust configuration codec directly, without reserializing schema/value records internally.
- Match every selected Swift/Kotlin transition, bound, catalogue rule, revision, normalization, failure, and reset behavior with deterministic tests.
- Make later host cutovers delete native decision logic rather than translate it into another permanent bridge API.
- Prove that adding the isolated candidate changes no production package, symbol, runtime path, or physical behavior.

**Non-Goals:**

- Selecting the service in Apple or Android production.
- Adding C/JNI exports, callbacks, opaque handles, generated bindings, or host-owned pointer lifetimes.
- Moving scheduler execution, Bluetooth, command framing, cache storage, SQLite, loopback dispatch, frontend models, files, location, lifecycle, or background services.
- Changing any persistent format or deleting native services in this extraction step.

## Decisions

### Use a dedicated service crate above the codec

`refloat-configuration-service` will depend only on `refloat-configuration-codec`. It will own the state machine and release catalogue while the codec remains independently usable and unaware of lifecycle. This preserves downward dependencies and makes the intended future runtime composition explicit.

Alternative: expand the codec crate. Rejected because catalogue/cache/generation/write verification are application behavior rather than wire-codec concerns.

### Represent host integration as facts, events, and effects

The service accepts immutable begin facts (route identity, generation, firmware identity/capabilities, reported package version, safety flags, optional cached schema), response events, failure events, and flywheel/current invalidation commands. It emits bounded effects for schema pages, current/default reads, writes, verification reads, cache store/remove, or no work, plus snapshots for publication. Routes and generations are opaque canonical integers; the service does not construct VESC envelopes or call a host.

Alternative: callbacks or async traits for scheduler/cache operations. Rejected because they couple the candidate to host execution, obscure deterministic transition tests, and complicate future FFI deletion.

### Keep schema and values as native Rust objects inside the service

The service invokes `decode_qt_compressed`, `decode_values`, and `encode_values` directly and retains bounded normalized Rust objects. Canonical bridge records remain temporary host-cutover representations and are not used between Rust crates. Public snapshots clone only bounded published metadata/value sets needed for parity tests.

Alternative: feed canonical byte records through the service. Rejected because that would make migration scaffolding a permanent internal API and duplicate validation/serialization work.

### Make cache I/O an explicit decision, not an owned port

Begin receives optional cached bytes with the exact cache identity facts. The returned transition includes `StoreSchema` or `RemoveSchema` alongside any request effect. Cache read/write remains host-owned until persistence migration, while the Rust service solely decides whether cached content is admissible.

Alternative: a cache trait owned by the service. Rejected for this candidate because host callbacks are unnecessary and would weaken isolation.

### Test against immutable fixtures and native transition vectors

Rust tests will use the committed release fixtures and explicit vectors derived from both native test suites. They will cover every catalogue entry, legacy `1.0` resolution, exact paging, corrupt cache, direct/CAN opaque routes, stale generation/operation/context, flywheel entry/exit, current/default revisions, write/readback success and conflict, failures, reset/reconnect, bounds, and deterministic payload-safe errors. Repository audits will prove no host selection or bridge catalogue change.

## Risks / Trade-offs

- **Candidate and native services coexist temporarily** → Audit production isolation and follow immediately with independent Apple and Android cutovers after parity is established.
- **A public Rust API could accidentally mirror host-specific types** → Use closed platform-neutral facts/events/effects and reject imports from bridge, scheduler, transport, persistence, or frontend crates.
- **Stateful behavior increases test-state combinations** → Use exhaustive transition vectors, deterministic snapshots, operation/generation boundary tests, and adversarial sequences with bounded collections.
- **Cache effects can be ordered incorrectly by a future host adapter** → Specify atomic transition output ordering and require host-cutover tests to apply cache decisions before publishing follow-up requests.
- **Physical runs do not execute the candidate yet** → Treat them only as regression/isolation evidence; later host cutovers must prove production execution before native deletion.

## Migration Plan

1. Add and verify the isolated safe service candidate and audits.
2. In a later Apple change, select the Rust state owner through the existing migration boundary or a unified runtime boundary, delete the Swift decision logic, and rerun complete gates.
3. In a later Android change, do the same and delete the Kotlin decision logic.
4. Move scheduler, application publication, loopback behavior, and persistence behind the unified Rust runtime in separately bounded changes.
5. Once hosts call the unified runtime rather than protocol slices, delete all `*-apple-ffi`, `*-android-jni`, canonical-record adapters, and bridge build catalogue entries.

Rollback for this extraction is deletion of the unselected crate and its audits; production binaries and stored data are unchanged.

## Open Questions

None. Host selection and the eventual unified runtime ABI are intentionally deferred to later changes with their own evidence.
