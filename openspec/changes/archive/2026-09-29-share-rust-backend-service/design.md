## Context

See proposal.md for motivation. The selected Rust runtime already owns protocol, connection coordination and per-session SQLite; the selected loopback server owns the frontend contract. However, the server currently drains domain operations into native `ApplicationService`, which calls native stores or re-enters Rust. Native snapshots then encode Rust results back into server facts.

Exploration found 9,747 lines across the two `ApplicationService`, `SQLiteLogStore`, `RuntimeRecords`, `ServerOperations` and `ServerFacts` file sets. This is an investigation inventory, not a claim that every line is deletable. The production convergence baseline is 49,922 lines (Rust 32,760, Apple 9,490, Android 7,672), 26 workspace members and two host binding families.

Important remaining policy includes native exponential reconnect scheduling; saved/guest activation and close/retention; stable board identity hashing; settings/profile validation; Tune identity/revision checks; schema caches; catalog reconciliation; pending deletes; archive manifest validation; CSV location matching; and location capture eligibility/session-clock assignment. Native OS file protection, permission brokers and background-service execution must remain intact.

## Goals / Non-Goals

**Goals:** A host can run the complete product by supplying OS capabilities, without implementing domain CRUD, SQL, archive formats, retry policy or snapshot projections. Select this boundary on both existing hosts and prove it through production paths. Preserve current externally observable behavior and dependency direction.

**Non-Goals:** Linux UI/Bluetooth support or Linux release packaging; replacing lower Rust kernels; broad public API redesign; migrations or archive format changes; removing platform-specific permission/lifecycle semantics. A direct Rust test host proves the boundary, not Linux support.

## Decisions

### Pre-release data policy (user decision, 2026-09-28)

There are no released versions or users. No legacy data migration, identity
aliasing, or earlier-development archive compatibility is required. Select the
versioned `vesc-v1-` identity derived from SHA-256 of
`UTF8("float-view-board-v1\0") || meaningfulFirmwareUUID` on both hosts. Treat
all-zero/all-FF or absent UUIDs as unavailable and generate a random opaque
identity instead. Reset incompatible development stores through marker `12`
when the shared store is selected; preserve the canonical SQL layout and current
archive encoding. Do not add an old Android hash lookup or an identity migration.
Current-format opaque imported identities remain valid; import must not invent a
firmware identity from them. Ordinary corruption or failed I/O never authorizes
a reset. AGENTS.md records this policy until the first actual release.

### 1. Compose a shared service above existing owners

Introduce at most two cohesive safe crates: `backend-service` for portable application orchestration, and `product-store` for catalog/product persistence and archives. Reuse `application-runtime`, `session-sqlite` and existing protocol/policy crates directly. Do not create a crate or FFI module per product feature.

```text
Native host shell / existing C or JNI binding / direct Rust host
          |                         |
          | OS facts/effects        | listener setup and lifecycle
          v                         v
    backend-service <--------- loopback-server
          |
          +---- application-runtime ---- existing domain kernels
          |              |
          +---- product-store ---- session-sqlite / canonical SQL
```

Host composition creates one service and injects its shared handle into the server. The service owns exactly one application runtime and product store; it does not depend on the server. The server consumes owned snapshots and asynchronous domain results directly. Keep the lower application runtime free of HTTP, native objects and executors. A shared composition registry above both manages their independent lifetimes using the existing host-binding families.

Alternative: sharing only SQL would leave native activation, identity, reconnect, Tune and publication policy. Alternative: putting everything in loopback-server would couple recording lifetime to transport and reverse dependencies. Neither meets the objective.

### 2. Draw the boundary around OS capabilities, not product methods

| Shared Rust service                                                   | Native host                                                                          |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Settings, saved boards, profiles, Tunes and validation/revisions      | Permission prompts and platform authorization facts                                  |
| Catalog, recovery, retention, pending deletion and maintenance        | App-private root selection, protection/backup attributes and free-space observations |
| Portable archive/CSV encoding, validation, staging and cleanup        | File picker, scoped URI/security access and save/share presentation                  |
| Saved/guest activation, identity and connection/reconnect decisions   | BLE discovery/connect/disconnect/read/write/subscription execution                   |
| Timer deadlines, retry policy and stale-generation rejection          | Clock readings and cancellable timer execution                                       |
| Location capture eligibility, elapsed-time assignment and persistence | OS provider execution and permission/lifecycle capability reporting                  |
| Shared state, revisions and command completion                        | WebView, theme application and background-service execution                          |

Refactor existing host envelopes into versioned closed facts, OS effects and completions. Remove native operations such as `list_tunes`, `get_session`, `update_settings`, `prune_session`, cache lookup and archive validation. Move native waiters for portable control/configuration requests into the shared service. Native records remain only for facts/effects actually needed by OS adapters. No full domain-model reflection or Rust → native → Rust snapshot round trip remains.

OS facts include radio/authorization, discovery results, transport events/bytes, location observations, lifecycle constraints, clock values and storage availability. Effects include BLE operations, timer arm/cancel, authorized provider start/stop, permission/settings UI, background execution and file presentation. Every asynchronous completion carries service lifetime and operation identity; listener generation separately scopes transport waiters. Hosts can deny unavailable OS capabilities, but cannot invent product policy. Notifications/UI may receive bounded display facts without owning authoritative domain state.

Keep callbacks out of C/JNI. Preserve caller-owned buffers and exact exported-symbol audits. Replace obsolete entry points when their last host user is cut over, retaining one library per binding family. A Rust host calls the same safe service directly without encoding its domain state through native JSON.

### 3. Move durable product semantics together with their ordering

Execute `backends/sql/catalog.sql` and `session.sql` verbatim using the locked Rust SQLite dependency. Preserve foreign keys, WAL, schema marker behavior and current product defaults. Keep storage disposable-test reset behavior; do not add migrations or silently reinterpret existing fields. Sharing ownership alone does not justify changing SQL or archive versions.

Port catalog/product operations as one owner: identity normalization and local association, saved-board/profile/settings optimistic revisions, routing hints, schema cache identity/digest/eviction, Tunes CRUD and apply validation, session summaries/keyset cursors, persisted maintenance state and deletion. Preserve distinct local session IDs versus portable board-history identity on import. If Swift/Kotlin disagree, resolve against the frontend contract, canonical SQL and conformance evidence, capture a regression fixture and document the resolution; do not select a behavior arbitrarily.

The session commit marker remains authoritative. Activation prepares catalog/session storage before generation-qualified runtime activation. A committed sample is published only after required catalog reconciliation; a catalog failure cannot fabricate rollback of an already committed sample. Reconcile before reading or appending again. Startup handles interrupted active sessions, pending deletion, maintenance and abandoned staging before exposing ready state. Do not describe separate catalog/session commits as one atomic transaction: use markers, ordered writes and idempotent reconciliation. Ordinary interruption/corruption must not trigger the incompatible-schema reset path.

### 4. Keep long storage work off the recording executor

Use one bounded service command owner for ordered portable mutations. Historical reads retain the existing two-reader pool. Archive/maintenance work uses at most two bounded blocking workers, separate from the live writer; per-session leases coordinate deletion, export, prune and reads. Workers produce owned completion records, never native callbacks, and retain cancellation/admission leases until their work actually terminates. Never hold the service mutation lock while waiting for a UI effect or long worker operation.

Carry forward transport limits: 64 admitted operations, eight per client, eight effects per drain, 1 MiB host envelopes, 64 KiB binary chunks, two transfers, 128 MiB imports, 16 sockets, eight pending responses plus one latest snapshot, two historical readers and 4 MiB decoded history responses. Use reserved bounded capacity for cancellation/cleanup so a full request queue cannot prevent resource release. Keep the two-second listener-stop bound independent of permitted recording. Full service shutdown rejects new commands, cancels interruptible workers, closes recording durably and reports bounded completion/failure; baseline numeric SQLite busy/worker shutdown deadlines before implementing it and test hostile lock contention.

Inventory every collection and on-disk retention limit before porting: settings/profile bytes, names, schema/value payloads, cache entries/bytes, discovery entries, cursor pages, transfer staging and orphan export retention. Preserve current stricter limits; introduce explicit caps where current platform code has none and record any resulting contract effect before selecting either host. Do not materialize whole catalogs, recordings, keep-sets or 128 MiB import buffers. Reuse bounded keyset scans and row streaming.

### 5. Share archive bytes; keep external file access native

Rust creates and validates `.floatview` archives and CSV in its app-private root, owns transfer IDs/staged files, and performs transactional catalog attachment and failure cleanup. Preserve metadata stripping before the strict session reader, archive board identity, GPS export settings, CSV columns/escaping/nearest-location rules and file-format validation. Use bounded streaming from staged paths, SQLite interruption and constrained schema/manifest validation. Reject traversal, symlink escape, malformed SQLite/schema/metadata and stale or foreign transfer handles without reading arbitrary host files. Only the shared store resolves session IDs to internal paths.

For native-picked files, the OS adapter holds scoped access and copies bounded chunks to a service-issued import sink; it does not parse archives. For save/share, Rust returns a service-issued export lease and the adapter presents that file. Keep the lease while the platform still needs it; bounded expiry/acknowledgment and next-launch cleanup reclaim abandoned staging. Frontend disconnect does not delete a file still owned by an active OS presentation. No raw user-provided path becomes a storage command, and no file payload enters JSON or diagnostics.

### 6. Delete each host's superseded ownership on selection

Migrate Apple first, then Android, with canonical parity and packaging at each checkpoint. Temporary coexistence is limited to this change and to hosts not yet selected. Remove both native `SQLiteLogStore` implementations, obsolete `LogSchema`/storage-policy helpers, portable `ApplicationService` policies and waiters, duplicated product/runtime records, domain `ServerOperations`, and `ServerFacts` projection paths once replaced. Retain or rename only actual OS adapters/composition. Do not retain facade classes containing hidden SQL or business rules.

Remove direct native SQLite/hash/archive dependencies only where the selected native source no longer needs them. Do not remove system frameworks needed by real OS capabilities. Reuse locked rusqlite, serde/JSON, hashing, randomness and compression packages; no new external package is required by this design. Any implementation-time addition needs a recorded license/features/size rationale and deletion benefit.

Update AGENTS.md, ownership READMEs, SQL docs, OpenSpec context, host symbol/build checks and audits to name the new selected owner. Audit production paths for catalog SQL, portable archive parsing, product validation, reconnect backoff, session retention decisions and domain snapshot projection outside Rust. Do not ban legitimate platform permission or lifecycle branching.

### 7. Prove the future-host cost directly

Add a test-only direct Rust host that supplies synthetic BLE, timers, permissions, location, file presentation and lifecycle facts using the same service constructor selected by native production. Keep protocol simulation and contract assertions in `conformance`; the launcher only adapts process/OS effects. The test host must not implement product CRUD, SQL, archive encoding, identity, reconnect or session rules and is not a supported fourth backend. Audit its allowed responsibilities.

Use it for deterministic fault injection (catalog/session split-commit failure, lock contention, storage exhaustion, stale effects, process restart) and run the canonical suite unchanged where applicable. Existing native fixtures continue to exercise their real C/JNI paths. Physical Android/iOS/macOS ESP32 checks remain required, especially recording, reconnect, background location, frontend recovery and native file presentation.

## Risks / Trade-offs

- [Scope is larger than catalog extraction] → Stage cohesive storage and service slices, then host cutovers; completion requires deleting both copies, not stopping at another candidate.
- [Cross-database crashes or cleanup lose committed work] → Marker-authoritative reconciliation and fault injection at each durable boundary; preserve existing retention/recovery semantics.
- [Shared worker contention interrupts BLE/recording] → Separate bounded workers, explicit leases, short mutation sections and saturation/cancellation tests.
- [OS file protection/scoped access is lost] → Native root preparation before Rust open and native external-file leases; verify iOS background capture and Android scoped presentation on devices.
- [Internal ABI changes create two runtimes] → One service-owned runtime, same safe constructor across bindings, lifetime tests and exact symbol/package audits.
- [Host reduction hides a new duplicate policy layer] → Move ownership, reuse existing lower kernels, delete old Rust/native glue and audit the direct test host.
- [Package size increases despite less source] → Measure all packages and dependency deltas; require net production source below 49,922, at most 28 workspace members, exactly two host binding families and no temporary bridges.

## Migration Plan

Commit the complete planning artifacts before implementation. Capture behavior/bounds/deletion inventory and source/package baseline. Implement shared storage and service against direct-host parity without changing production selection. Select Apple and delete its superseded owners; verify and commit. Select Android and delete its superseded owners; verify and commit. Complete ownership audits, canonical conformance and the physical checkpoint, then run the final software gate and sync/archive together.

Rollback is a coherent revert of the affected host-cutover commit and its matching binding selection, not a runtime switch or permanent fallback. Preserve the unchanged SQL/archive formats; if a later discovered format change becomes necessary, revise this proposal before implementation rather than silently adding migrations.
