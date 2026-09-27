# Design

## Context

See `proposal.md` for motivation and scope. Swift and Kotlin currently combine transport, contract dispatch, payload serialization, and host application calls in their local servers.

The selected Rust application runtime already owns protocol decisions and session data. Native services still own Bluetooth, location, catalog metadata, settings, Tunes, file presentation, and runtime effect execution. The existing host boundary is versioned and callback-free. The frontend contract and SQL schema remain authoritative external contracts.

The measured server baseline is 1,905 Swift lines plus 1,794 Kotlin lines. This is a source inventory, not a promise to remove all 3,699 lines.

## Goals / Non-Goals

**Goals:** One server implementation and one frontend contract projection across iOS, macOS, and Android. Preserve bounded work, canonical behavior, native background recording, and independently testable host integration.

**Non-Goals:** No new platform, catalog rewrite, generated binding, remote listener, TLS endpoint, new protocol policy, or general static-file server. The application runtime remains free of server threads and network dependencies.

## Decisions

### 1. Put transport above the existing runtime

Add `backends/core/loopback-server` as a transport owner. Its internal modules own routes, authentication, contract records, dispatch, publication, and bounded host operation exchange. Reuse existing domain records where practical. Hand-write frontend wire records against `frontend/src/lib/backend/contract.ts`.

The existing Apple FFI and Android JNI packages compose the server and runtime host integration. Keep one native archive/library per host. Do not add transport-specific binding crates or make `application-runtime` depend on the server.

The dependency direction is host shell and binding, then shared transport, then application/domain boundaries. Native Bluetooth and storage do not import HTTP types.

Alternative: share only authentication or an HTTP parser. Rejected because method dispatch, serialization, and state publication would remain duplicated.

### 2. Preserve one application owner with bounded host operations

Retain the existing process-owned native application service and its selected Rust runtime. The server must not create another runtime or bypass that service's mutation ordering.

Requests become typed host operations with server-generation and operation IDs. Host workers drain a bounded queue, execute permitted application/OS/catalog work, and submit bounded typed completions. Native adapters supply domain facts, not frontend JSON envelopes. Shared Rust alone maps results and failures into the public API.

Use closed start, stop, drain/wait, complete, publish-facts, and cancel operations through the existing binding family. Extend versioned envelopes and exact-symbol audits together. A bounded wait may block a dedicated host worker, never the UI executor. Rust invokes no native callbacks and retains no platform objects or caller buffers.

Release server and registry locks before executing host operations or entering the application runtime. Completion re-enters the shared server only after host work releases its application lock. Cancellation uses operation IDs, and server generations reject stale completions after restart or destruction.

Queue saturation produces a bounded terminal failure or closes an abusive client. Never retry an accepted mutation automatically. Client loss cancels reads and response waiters, but does not undo an accepted board mutation or delete operation. Existing operation cancellation methods retain their current semantics.

Alternative: move catalog storage and all application integration simultaneously. Rejected because that expands this change beyond transport and obscures parity failures.

### 3. Move every public route and serializer together

The shared server owns `/`, `/api/bootstrap`, `/api/ws`, `/api/session-data`, `/api/session-import`, and `/api/session-export`. It owns every current control method and session-data kind in the TypeScript contract, including location and Refloat configuration/Tunes.

Hosts supply the identical embedded HTML bytes and validated launch metadata. Serve HTML only at `/`. Unknown content paths return 404 without filesystem lookup. Preserve native ports 11010, 11011, and 11012, bootstrap fields, proxy behavior, content types, status codes, and stable error vocabulary.

Bind only to `127.0.0.1`. Generate one cryptographically random token per backend launch and retain it across disposable frontend replacement. Validate exact Origin and the existing route-specific token placement before body parsing or storage lookup. Preserve no-store headers on authenticated data responses and redact private input from diagnostics.

Enforce bounds while reading, including bodies without Content-Length and fragmented WebSocket messages. Retain contract bounds: 65,536-byte control messages, 16,384-byte session-data requests, 4 MiB decoded session-data responses, and 128 MiB imports. Keep all field, page, cursor, and schema limits from the contract.

Do not send archive bytes through the 1 MiB command envelope or encode them as base64. Use bounded chunks and opaque transfer handles scoped to the server generation. Hosts own temporary storage and archive validation. The server owns HTTP transfer limits and cleanup requests. Paths never come from a frontend request. Cap concurrent transfers and remove partial data after rejection, cancellation, or stop.

### 4. Publish state once per application change

A process-owned shared publisher accepts complete validated domain facts from the host's serialized application executor. It builds frontend snapshots and owns the public state and catalog revisions. Each accepted application update advances state once, independently of subscriber count. Catalog revision advances only for catalog-visible changes.

Maintain one latest pending snapshot per client and bounded response queues. Responses retain request correlation and are never conflated. Reject stale host publication sequence numbers. A client that cannot make progress cannot block recording or other clients.

Runtime commit markers remain authoritative. Saved-mode telemetry appears only after commit. Guest telemetry must not create catalog entries. State snapshots omit history collections and private identities exactly as before.

Keep the publisher and its revision counters alive when only the listener stops. New clients receive current complete state without resetting the recording runtime or replaying obsolete events.

### 5. Separate transport work from recording and UI lifetime

A dedicated shared server executor owns sockets and response work. Blocking history, compression, archive, and catalog work use bounded workers. Retain at most two concurrent historical reads and wire client cancellation to the existing interruption path.

Document numeric connection, queue, transfer, worker, and shutdown limits before implementation selection. Tests must cover each exact boundary. Choose values compatible with current conformance and multi-client lifecycle needs.

Stop closes admission, cancels transport waiters, requests transfer cleanup, wakes host workers, and joins server work within a tested deadline. It must not destroy the application runtime, disconnect Bluetooth, or detach an active recording session. Native lifecycle owners decide when the listener is available under OS policy.

A failed bind reports startup failure without silent port substitution. A later start uses a fresh server generation. Destroy is idempotent, and late completions cannot access freed handles. Permission and file UI requests execute only on the appropriate native executor.

### 6. Replace dependencies and source, not just their entry points

Select one maintained Rust HTTP/WebSocket stack and only its required features after a cross-target build check. Reuse existing serialization dependencies. Add compression and secure randomness support only where the standard library or selected stack does not supply them.

Record exact locked versions, licenses, transitive dependency count, target support, package-size delta, and each removed dependency. A library must replace actual protocol/server complexity. Do not implement HTTP/WebSocket parsing by hand to avoid a dependency.

Remove FlyingFox from SwiftPM, Xcode package references, and resolved dependency files when no selected code needs it. Remove Android Ktor server CIO/core/WebSocket declarations and catalog entries when unused. Retain native coroutine or file libraries required elsewhere.

Delete the old server implementations, native frontend payload builders, native compression/auth helpers, and tests that only cover deleted internals. Small `transport` adapters may retain the existing server type name for host wiring. They must contain no HTTP routes or frontend method switches.

Preserve useful native integration tests and canonical assertions. Extend existing source/binding audits to forbid retired dependencies and duplicate dispatch. Record total production source before and after. Final combined source must decrease, and existing convergence limits must still pass.

## Risks / Trade-offs

- Host operations can deadlock through re-entry. Release shared locks before host execution and test concurrent request, publication, cancellation, and shutdown paths.
- A listener stop can accidentally end a ride. Keep runtime ownership independent and test persisted sequence growth across listener and WebView replacement.
- Large archives can exceed host envelope bounds. Use bounded transfer handles with explicit cleanup and test interrupted transfers at size boundaries.
- Existing servers can disagree. Use the TypeScript contract and canonical specs as authority, add a regression scenario, and avoid preserving an accidental platform difference.
- Shared networking adds Rust dependencies and target build risk. Complete the dependency and packaging check before either production migration.
- A broad migration can retain hidden duplicate serializers. Require source inventory, forbidden-reference audits, and deletion in each host migration commit.

## Migration Plan

1. Commit this proposal, design, delta specs, and task list before implementation.
2. Record the baseline and dependency decision. Build an unselected shared server and bounded host boundary, with focused tests and canonical software coverage.
3. Switch shared Apple code so iOS and macOS select the server. Delete superseded Apple code and dependencies in the same verified slice.
4. Switch Android, delete its superseded server and dependencies, and verify host tests plus all four ABIs in Debug and Release.
5. Run focused adversarial/lifecycle checks, strict spec validation, and `just verify`. Run the explicit physical checkpoint separately on Android, iOS, and macOS with the ESP32 fixture.
6. Record actual physical results and any unavailable coverage without claiming a pass. Sync and archive only after all required tasks and gates are complete.

Before both host migrations complete, only the unmigrated host retains its old server. There is no runtime feature flag or permanent fallback. If a migration fails, revert its coherent source/dependency commit. No database rollback or migration is needed.

## Open Questions

The exact Rust networking packages and internal numeric worker limits remain implementation selections. The compatibility, ownership, bounds, deletion, and verification requirements above constrain them without changing the public contract.
