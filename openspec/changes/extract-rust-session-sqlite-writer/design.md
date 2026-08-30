## Context

The selected Apple and Android `SQLiteLogStore` implementations each own a catalog database, one active per-session database, product/settings/profile data, session writes and reads, pruning, exports/imports, recovery, deletion, cancellation, and filesystem policy. The per-session write subset is nevertheless the same on both hosts: execute `backends/sql/session.sql`, initialize `session_metadata` and `commit_marker`, insert normalized samples, derive first numeric-field sources and transition events, recompute marker summaries, deduplicate locations, and continue correctly after reopening.

The Rust workspace already owns normalized Refloat telemetry and the session-history reduction/prune kernel. It does not yet mutate durable state. This change establishes that first persistence owner without selecting it from a host. Stored databases remain disposable test-stage data, so the canonical schema is used directly and no migration machinery is introduced.

## Goals / Non-Goals

**Goals:**

- Create one platform-neutral Rust owner for a single canonical session SQLite file.
- Consume the existing safe `NormalizedSample` value directly and persist exact stable/extension values, transition evidence, numeric-field discovery, and marker summaries.
- Make session creation, each sample batch, and each location batch atomic, bounded, deterministic, reopenable, and fail-closed.
- Use the exact repository schema and a locked SQLite implementation that can later compile for Apple and Android without platform database wrapper code.
- Keep the candidate unselected, callback-free, bridge-free, and below future catalog/runtime composition.

**Non-Goals:**

- Catalog SQLite, settings, profiles, schema caches, routing hints, maintenance jobs, session listing/detail/cursors, history reads, pruning, imports/exports, deletion/retention, or interrupted-session recovery.
- Selecting Rust in Swift/Kotlin packages, adding FFI/JNI, changing `session.sql`, migrating existing databases, or deleting native storage code.
- Owning filesystem policy, available-space decisions, native file presentation, BLE, background/location acquisition, local serving, frontend publication, or an async/thread executor.

## Decisions

### 1. Own a real session file, not a SQL-plan abstraction

Add `backends/core/session-sqlite-writer` with a concrete `SessionWriter` that owns one `rusqlite::Connection`. `create` accepts a caller-chosen new path plus positive idle/riding persistence cadences and a commit timestamp; `open` validates the existing canonical metadata/marker shape. The public API accepts samples and locations and returns bounded marker snapshots.

A trait, generated SQL plan, or mock persistence port was rejected because it would move no durable behavior and would leave transaction/error semantics duplicated. Moving the complete native stores was rejected because catalog, files, queries, recovery, and serving are separate regression surfaces.

### 2. Compile the canonical schema and publish creation atomically

The crate uses `include_str!("../../../sql/session.sql")`, so the repository schema remains the only table/index/pragma source. Creation validates input and target absence, initializes a private sibling temporary database, executes the exact schema, inserts `session_metadata` and `commit_marker` transactionally, checkpoints/closes it, and atomically publishes the completed main file with a same-directory no-clobber hard link before unlinking the private name. Failure removes only the candidate-owned temporary main/WAL/SHM files and never overwrites the target.

`open` checks `PRAGMA quick_check`, `user_version`, required singleton rows, positive cadence bounds, and the exact required table set before accepting writes. It does not upgrade or repair an incompatible file.

### 3. Use locked `rusqlite` with bundled SQLite and deterministic JSON

Use `rusqlite 0.37` with `bundled` and `serde_json` without order-preservation features. `rusqlite` replaces hand-written statement preparation/binding/error cleanup and supplies a maintained safe API; bundled SQLite makes the eventual mobile build independent of Android framework SQLite, Apple module maps, pkg-config, or a host-installed library. `serde_json` replaces hand-written escaping and, with its default sorted map, produces canonical extension/event objects.

This is intentionally the first third-party dependency-bearing domain crate. The audit locks exact versions, permits only the transitive SQLite/JSON build graph, rejects networking/async/ORM/migration dependencies, and keeps every dependency inside the unselected Rust workspace.

### 4. Persist bounded ordered samples in one immediate transaction

`append_samples` accepts 1–1,024 samples per call. Each batch must be strictly ordered by `(elapsed_ms, sequence)`, every integer must fit the durable SQLite representation, every present numeric value must be finite, and extensions must contain at most 32 unique non-empty names of at most 64 UTF-8 bytes with a final JSON payload of at most 16,384 bytes. All input is validated before mutation.

The transaction uses `INSERT OR IGNORE` for sample sequence replay, preserving the native idempotent stored-row behavior while `recorded_sample_count` advances by the submitted batch length. Only newly inserted rows discover stable/extension fields and emit transition events. Event values follow the existing closed wire strings and compare against the last stored sample before the batch. Event sequences continue from the durable maximum.

After insertion, Rust streams stored rows ordered by `(monotonic_elapsed_ms, sequence)` to recompute committed elapsed, riding elapsed, trapezoidal absolute-speed distance, absolute maximum speed/duty and earliest source sequences, and last speed/state. This avoids SQLite window-version differences and makes one future cross-platform owner. The singleton marker, events, fields, and samples commit together under `BEGIN IMMEDIATE`; any SQLite failure rolls back the whole batch.

### 5. Persist bounded locations in an independent transaction

`append_locations` accepts at most 1,024 observations, validates nonnegative durable elapsed/time values, finite latitude/longitude within canonical ranges, and absent or finite nonnegative accuracy, then orders by `(elapsed_ms, observed_at_epoch_ms, original_index)`. Existing `(elapsed, latitude, longitude)` observations are ignored, accepted rows receive consecutive durable sequences, and the location marker fields update in the same immediate transaction. An all-duplicate batch commits no marker change.

Location capture, authorization, background execution, and catalog projection remain host/runtime responsibilities; this API owns only already-observed platform-neutral facts.

### 6. Close and reopen without a runtime or bridge

`marker`, `append_samples`, and `append_locations` expose owned safe values. `checkpoint` performs a full WAL checkpoint, and dropping/closing releases the connection. A later `open` resumes from durable rows and marker state; it holds no callback, foreign handle, task, thread, async runtime, or host reference. The future Rust catalog/application runtime will call it directly, so no session-writer Apple FFI or Android JNI crate is permitted.

Errors are a closed payload-free enum separating invalid input, target existence, incompatible/corrupt database, busy/locked, full/I/O, and other database failure. Debug output never includes paths, SQL, telemetry, coordinates, extension names, or SQLite messages.

### 7. Prove behavior from immutable state fixtures and real host isolation

Commit literal sample/location TSV inputs plus expected metadata, rows, fields, events, and marker projections under `backends/fixtures/session-sqlite-writer`. Tests query the resulting database independently through `rusqlite`, cover create/reopen/replay/multi-batch and injected-trigger rollback, exercise all declared bounds, and prove the compiled schema text is byte-identical to `backends/sql/session.sql`.

The root audit requires the crate and fixtures, exact dependency allowlist, no bridge/build-catalogue/host-package selection, no copied native source, and no production Apple/Android/frontend/SQL changes. `just verify` and the unchanged physical matrix prove adding the unselected candidate does not regress selected behavior.

## Risks / Trade-offs

- **[Bundled SQLite increases Rust build time and artifact size once selected]** → Keep it unselected now, measure later mobile artifacts during runtime composition, and prefer one eventual bundled owner over two platform wrappers plus duplicated logic.
- **[Marker parity can drift at ties or null-speed boundaries]** → Commit exact cross-host fixtures for earliest extrema ties, missing prior speed, phase changes, duplicate replay, and multi-batch reopen behavior.
- **[Schema creation can leave partial files]** → Initialize a candidate-owned sibling temporary database, checkpoint and close it, then atomically publish only the completed main file.
- **[SQLite failure after some statements could expose partial facts]** → Use one immediate transaction per batch and inject mid-batch SQLite trigger failures to prove samples, events, fields, and marker all roll back.
- **[A persistence crate could become a premature runtime]** → Reject catalog, callbacks, executors, local serving, host selection, and bridge crates through source/dependency/package audits.
- **[Physical tests do not execute an unselected candidate]** → Treat them only as production-isolation evidence; candidate correctness comes from exact database state and adversarial transaction tests, with later host cutovers required to prove production execution.

## Migration Plan

1. Commit the proposal, design, delta specs, and tasks.
2. Add the locked writer crate, immutable fixtures, independent database-state tests, failure injection, and ownership/dependency audits without changing production sources or schema.
3. Run focused offline tests/lint/audits, strict OpenSpec validation, `just verify`, and the canonical ESP32 matrix on macOS, Android, and iOS, deferring only bounded location acquisition.
4. Commit implementation, sync canonical specs, and archive the change.
5. Later add the Rust session reader/pruner around this writer and `session-history-kernel`, then add catalog/recovery ownership, compose persistence into the Rust application/server runtime, cut over each host, and delete native stores plus obsolete protocol bridges.

Rollback before selection removes the unreferenced crate, fixtures, lockfile entries, audit recipes, and OpenSpec delta. No production package or durable database requires rollback.

## Open Questions

None. Catalog ownership, read/prune APIs, recovery, runtime selection, and native/bridge deletion remain separate changes.
