## Context

Apple and Android currently duplicate the same historical-session SQLite scans inside their selected `SQLiteLogStore` implementations: marker/configuration reads, stable/extension field resolution, published-boundary filtering, ordered sample/event/location pages, and streaming series rows into equivalent native reducers. Rust now owns the canonical per-session writer in `session-sqlite-writer` and owns the platform-neutral M4/gap and prune-selection algorithms in `session-history-kernel`, but nothing connects a durable database to that kernel.

The writer candidate is deliberately unselected by all hosts. That makes this the least expensive point to replace its writer-only package identity with one durable per-session SQLite package before adding more persistence behavior. The canonical database remains disposable test-stage data and `backends/sql/session.sql` remains the sole schema source.

## Goals / Non-Goals

**Goals:**

- Rename the unselected crate/package/fixtures/recipes from writer-only `session-sqlite-writer` to durable `session-sqlite`, preserving `SessionWriter` behavior.
- Add one independent read-only connection owner for canonical session metadata, commit boundaries, numeric fields, reduced series, lossless sample pages, event pages, and location pages.
- Make every query bounded, deterministic, snapshot-consistent, published-boundary-aware, interruptible from another thread, and fail-closed with payload-free errors.
- Resolve stable fields through one closed Rust field-to-column catalogue and extension fields only through validated `session_numeric_field` evidence and bound JSON paths.
- Reuse `session-history-kernel` directly for series reduction without an Apple FFI or Android JNI boundary.

**Non-Goals:**

- Catalog SQLite, session lookup/listing, opaque cursor/base64 encoding, HTTP/JSON/gzip serialization, loopback request handling, read-lease/job/executor ownership, or response caching.
- Prune mutation, retained-row deletion, vacuum/checkpoint policy, history-revision mutation, maintenance jobs, recovery, import/export, deletion, or filesystem policy.
- Selecting the candidate from Swift/Kotlin, changing `session.sql`, migrating stored databases, adding a bridge/code generator, or deleting native readers.
- Owning BLE, location acquisition, lifecycle/background execution, permissions, native file presentation, theme, WebViews, or UI integration.

## Decisions

### 1. Consolidate the unselected package before adding a sibling reader crate

Rename the crate directory and package to `backends/core/session-sqlite` / `float-view-session-sqlite`, rename its fixtures and root recipes, and retain `SessionWriter` at the crate root. Add `SessionReader` as a sibling type with its own connection.

A separate `session-sqlite-reader` crate was rejected because it would duplicate canonical schema validation, SQLite error classification, stable-field mapping, dependency/audit plumbing, and later require another consolidation step. Sharing one connection was rejected because active writes and historical reads need independent SQLite transactions and executor ownership.

### 2. Read one caller-selected file through an independent read-only connection

`SessionReader::open` accepts a caller-chosen path, opens it with SQLite read-only/no-mutex flags, applies a short busy timeout, validates integrity plus canonical tables/indexes/singletons/cadences, and exposes only owned safe values. It neither discovers files nor retains a writer, host object, callback, task, or runtime.

Each public query starts a bounded deferred read transaction, reads the commit marker inside that transaction, validates the caller's `PublishedBoundary`, performs the scan, and closes the transaction before returning. The boundary requires an equal `history_revision` and requested sample/event/location sequences no newer than the durable marker. This permits an active writer to advance after the frontend's published snapshot while preventing reads across a prune/revision boundary.

Opening mutable/query-planning abstractions or accepting arbitrary SQL was rejected because the future runtime needs a closed safe persistence API, not a database wrapper.

### 3. Expose storage primitives, not the loopback wire contract

The reader owns platform-neutral query values:

- `metadata` and `marker` snapshots;
- ordered unique field descriptors;
- `reduce_series` over 1–16 fields, a nonnegative elapsed range, 1–1,024 buckets, canonical cadence/gap policy, at most 200,000 source rows, and the kernel's 65,536-output bound;
- `sample_page` over 1–16 fields with an optional `(elapsed_ms, sequence)` continuation key and a 1–2,000 row limit;
- `event_page` with the closed ten transition kinds, optional continuation key, and a 1–500 row limit;
- `location_page` with an optional last location sequence, a 1–2,000 row limit, deterministic gaps over intervals greater than ten seconds, and the published location boundary.

Opaque cursors, request identity, session IDs, timestamps formatted as strings, JSON envelopes, gzip, and response-byte accounting stay above this API for later Rust application/server composition. Moving them now was rejected because it would combine SQLite ownership with the local-server regression surface.

### 4. Resolve selected fields without SQL injection or copied host code

Define the canonical 36 stable numeric field names and SQLite columns once in Rust. A requested name must be unique, non-empty, at most 64 UTF-8 bytes, and either in that closed catalogue or present as an `extension` row in `session_numeric_field`. Stable column expressions come only from the closed catalogue; extension values use parameter-bound SQLite JSON paths rather than interpolated names. Stored numeric and JSON values must be finite and shape-valid or the query fails closed.

The catalogue is derived from the hand-written frontend contract/native parity surface but is not generated. Copying Swift/Kotlin source, accepting arbitrary identifiers, or introducing bindgen/UniFFI was rejected.

### 5. Stream series rows directly into `session-history-kernel`

`reduce_series` scans rows ordered by `(monotonic_elapsed_ms, sequence)` and feeds each row directly into `SeriesAccumulator`; it does not retain the 200,000-row source window. It maps `idle` and `riding` to exact kernel phases, applies metadata cadences and retained resolution, and returns the kernel's columnar selected observations plus bounded source-row count.

Reimplementing M4/gap behavior in the SQLite crate or pushing reduction into SQLite window functions was rejected because the tested kernel already owns deterministic ties, gap generations, and mobile-independent bounds.

### 6. Make cancellation explicit without callbacks or an async runtime

`SessionReader::interrupter` returns a safe cloneable wrapper around the connection's SQLite interrupt handle. Another runtime-owned thread may call `interrupt`; SQLite cancellation maps to a closed `Cancelled` error, every statement/transaction is released, and later queries remain usable. The reader creates no thread and exposes no `rusqlite` type or foreign handle.

A polling callback/token and a crate-owned executor were rejected because they add lifetime/execution policy below the future application runtime. An uncancellable API was rejected because selected native historical queries already require prompt cancellation.

### 7. Prove exact behavior from literal fixtures and active-WAL concurrency

Rename existing writer fixtures under `backends/fixtures/session-sqlite` and extend them with literal query requests/expected projections covering stable and extension fields, nulls, ties, gaps, published boundaries, continuation keys, event filtering, location ordering/gaps, and reopen. Tests create canonical databases through `SessionWriter`, independently query expected rows, and exercise malformed/corrupt databases, invalid stored values, every public bound, SQLite interruption, a 200,000-row streaming ceiling, and concurrent writer progress while a reader snapshot is held.

Root audits require the consolidated package and exact dependency graph, reject the superseded writer-only path/recipes, both possible reader bridge crates, generated bindings, arbitrary SQL/public SQLite types, host selection, and production native/frontend/schema changes. Physical conformance remains production-isolation evidence because the candidate is still unselected.

## Risks / Trade-offs

- **[Renaming immediately after extraction creates documentation churn]** → Do it while the crate is unselected and has one consumer-free API; audit removal of every writer-only path so the rename is complete.
- **[A read transaction can pin WAL pages]** → Bound source rows and query lifetimes, expose prompt interruption, release every transaction on all exits, and leave executor/lease policy to the future runtime.
- **[Dynamic extension fields can create injection or malformed-JSON risk]** → Admit names only from bounded durable field evidence, bind JSON paths as parameters, validate finite values, and reject malformed JSON/value shapes.
- **[A writer can advance during an older published read]** → Validate the requested boundary inside the same read transaction and cap every table by its published sequence; reject history-revision mismatch.
- **[Read behavior could drift from the frontend contract before selection]** → Commit exact query projections and reuse the existing kernel; later host cutovers still require contract/conformance proof before deleting native readers.
- **[Physical tests do not execute the candidate]** → Treat them only as proof that the rename/addition did not alter selected packages; candidate correctness comes from exact SQLite and adversarial/concurrency tests.

## Migration Plan

1. Commit this proposal, design, delta specs, and task plan.
2. Rename the unselected crate/fixtures/recipes, retain writer parity, implement the reader, and add exact/adversarial/concurrency evidence without changing production sources or schema.
3. Run focused locked offline tests/audits, warnings-denied workspace linting, strict OpenSpec validation, `just verify`, and the canonical ESP32 matrix on macOS, Android, and iOS while deferring only timed location acquisition.
4. Commit implementation, sync canonical specs, and archive the change.
5. Next add atomic prune mutation inside `session-sqlite`, then add Rust catalog/recovery ownership and compose persistence into the Rust application/server runtime before host cutovers and bridge deletion.

Rollback before selection restores the writer-only crate/fixtures/recipe names and removes `SessionReader`; no production package or durable database requires rollback.

## Open Questions

None. Cursor/server composition, prune mutation, catalog/recovery, host selection, and native/bridge deletion remain separate changes.
