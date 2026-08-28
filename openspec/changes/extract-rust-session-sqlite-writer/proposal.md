## Why

Apple and Android still duplicate the same per-session SQLite creation, telemetry/location append transactions, transition-event derivation, numeric-field discovery, and commit-marker summary behavior inside their roughly 3,400-line native stores. The safe Rust application and history components now produce and consume the relevant platform-neutral facts, so the next meaningful increment is to give Rust durable ownership of one isolated session database file before attempting catalog, query, server, or host cutover work.

## What Changes

- Add an unselected `backends/core/session-sqlite-writer` crate that creates a new disposable session database from the exact canonical `backends/sql/session.sql` schema and initializes bounded persistence metadata plus the singleton commit marker.
- Persist `refloat-telemetry-service::NormalizedSample` batches through one immediate SQLite transaction, including all stable columns, deterministic bounded extension JSON, first-source numeric-field discovery, transition events, duplicate-sequence behavior, and exact marker aggregates/counts/source sequences.
- Persist ordered validated location observations through one immediate transaction with canonical coordinate/accuracy bounds, elapsed/coordinate deduplication, monotonic local sequences, and exact location marker updates.
- Support closing/reopening the same session file and continuing samples, events, fields, locations, and summaries without exposing a host callback or bridge ABI.
- Use a small maintained Rust SQLite dependency with its locked bundled SQLite implementation so host tests and later mobile composition do not depend on a machine package manager or platform SQLite wrapper.
- Add immutable writer input/expected-state fixtures, schema-identity checks, transaction/failure/reopen/adversarial tests, and audits proving Apple, Android, frontend, catalog, loopback, exports, and package inputs remain unchanged.
- Keep the selected Swift/Kotlin stores fully responsible for production session creation, catalog convergence, reads/history/pruning, recovery, files, and runtime publication. This change adds no `*-apple-ffi` or `*-android-jni` crate and does not select Rust from either host.
- Require `just verify` plus the complete macOS, Android, and iOS ESP32 matrix; bounded location acquisition alone remains deferred to later manual verification.

Explicit non-goals are catalog SQLite ownership, session queries/cursors/history/pruning, imports/exports, retention/deletion policy, interrupted-session recovery, loopback serving, host runtime selection, native source deletion, bridge deletion, and any durable schema or frontend-contract change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Require a bounded Rust per-session SQLite writer below future catalog and runtime ownership.
- `platform-hosts`: Record the unselected writer candidate and retained selected native storage/application ownership until later direct runtime cutovers.
- `backend-conformance`: Require exact schema, transaction, aggregate, reopen, adversarial, dependency-isolation, complete software, and all-host physical evidence.

## Impact

The change affects the Rust workspace and lockfile, a new persistence crate, writer fixtures/tests, root verification recipes, migration documentation, and OpenSpec requirements. It adds locked `rusqlite`/bundled-SQLite and deterministic JSON dependencies to the unselected Rust workspace, reads but does not modify `backends/sql/session.sql`, and changes no production package, ABI, native source, frontend contract, database format, or selected behavior.
