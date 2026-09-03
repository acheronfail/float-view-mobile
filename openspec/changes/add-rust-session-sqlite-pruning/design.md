## Context

`session-history-kernel::PruneSelector` already performs bounded platform-neutral selection, and `session-sqlite` already validates canonical databases and owns safe reader/writer connections. The selected Swift and Kotlin stores still duplicate the glue between them: reading protected facts and ordered samples, materializing the keep set, deleting rows in one SQLite transaction, and updating metadata and the commit marker. Their surrounding activity/read-lease locks, catalog reconciliation, maintenance jobs, and reclamation calls remain host-runtime concerns.

This change adds an unselected Rust persistence operation only. It changes neither the canonical schema nor a production package, so exact Rust SQLite evidence is the correctness gate and the physical matrix proves absence of host regressions.

## Goals / Non-Goals

**Goals:**

- Give `session-sqlite` a bounded preview object that contains the exact keep set and immutable database boundary needed for a later atomic commit.
- Stream ordered canonical rows directly into `PruneSelector`, protecting numeric-field source rows, event source rows, session endpoints, phase transitions, and summary source sequences.
- Apply a still-current preview in one immediate SQLite transaction, deleting only eligible samples and orphaned-by-resolution locations and atomically updating prune metadata/counts/revision.
- Fail closed on invalid/coarser-resolution rules, incompatible data, interruption, stale boundaries, lock contention, or injected transaction failure with stable payload-free errors.
- Keep the Rust API safe and directly composable by a future catalog/application runtime without creating temporary FFI/JNI layers.

**Non-Goals:**

- Selecting Rust from Apple or Android, changing/removing their pruning code, or adding any bridge/generated binding.
- Owning active-session/read-lease exclusion, catalog reconciliation, maintenance scheduling/cancellation, checkpoint/vacuum policy, recovery, deletion, retention, import/export, or filesystem discovery.
- Changing the frontend contract, canonical schema, native database format, or existing physical location behavior.

## Decisions

### Preview is an opaque owned plan bound to a commit marker

`SessionPruner::preview` opens one validated read/write-capable session connection, begins a deferred read transaction, reads the marker and metadata, and streams the complete bounded source set. Its returned `PrunePlan` exposes only summary facts while privately retaining sorted selected sequences and the marker facts required for revalidation. A plan has a fixed upper source-row bound and stable redacted `Debug` output.

Commit begins `BEGIN IMMEDIATE`, re-reads the marker and retained resolution, and compares them with the plan before any mutation. Any writer progress or prior prune makes the plan stale. This is preferable to recomputing silently because callers can present an exact preview and know that the committed result is the same selection.

### Rust streams directly through the existing kernel

The pruner reads rows ordered by `(monotonic_elapsed_ms, sequence)` and feeds only `speedKph` and `packVoltageV`, exact ride phase, and the protected flag to `PruneSelector`. Protected sequences are derived inside the same snapshot from `session_numeric_field.first_sequence`, `session_event.source_sample_sequence`, and commit-marker summary source sequences. Finalized bucket selections are appended to a bounded keep vector; raw source rows are never materialized for the ride length.

This directly composes the two existing safe owners and avoids duplicating selection SQL or creating a pruning-specific bridge. The source-row ceiling bounds execution even though the selector's live algorithmic memory is bucket-bounded.

### Commit uses a temporary keep table and one durable transaction

For a non-no-op plan, commit creates a connection-local temporary `prune_keep` table, inserts sorted unique sequences with bound values, deletes samples absent from it, and deletes locations with no retained sample within the target resolution. It then updates `session_metadata` with the timestamp, resolution, and `m4-source-v1`, and updates stored sample/location counts, committed time, and `history_revision + 1` without changing recorded counts or published sequence maxima.

Every step is checked and rollback removes partial effects. The temporary table is dropped on all exits. `PRAGMA wal_checkpoint(PASSIVE)` and `incremental_vacuum` stay outside this API because reclamation timing belongs to future runtime maintenance policy.

### No-op commits validate but do not advance history

A preview whose keep count equals stored count is explicitly skipped. Committing it still validates database identity/boundary and returns the preview result, but performs no metadata or marker mutation. This preserves the selected hosts' observable no-op behavior and prevents gratuitous history-revision changes.

### One reusable interruption boundary

The pruner exposes a cloneable SQLite interrupt handle equivalent to the reader's existing mechanism. Cancellation during preview or mutation maps to the same closed cancelled category, releases statements/transactions, and leaves the database reusable. No thread, callback, async runtime, or host cancellation policy enters the crate.

## Risks / Trade-offs

- **[A preview keep set grows with retained history]** → Cap source rows, use sorted compact integer sequences, stream raw input, and fail before publishing a partial plan.
- **[State can advance between preview and commit]** → Bind the plan to complete relevant marker/resolution facts and revalidate inside `BEGIN IMMEDIATE` before mutation.
- **[Foreign-key evidence could be deleted]** → Treat every field/event/summary source as protected and test foreign keys enabled with exact evidence preservation.
- **[Interrupted or failed deletion could corrupt counts]** → Keep all logical mutations and marker updates in one rollback-tested transaction; omit reclamation pragmas.
- **[Physical tests do not execute the unselected candidate]** → Use exact/adversarial Rust SQLite tests for correctness and treat the device matrix only as production-isolation evidence.

## Migration Plan

1. Commit this proposal, design, delta specs, and task plan.
2. Add prune values/errors/lifecycle, streaming preview, atomic commit, fixtures, and focused audits without changing selected production sources or schema.
3. Pass locked focused tests, warnings-denied workspace linting, strict OpenSpec validation, `git diff --check`, and `just verify`.
4. Run the canonical ESP32 matrix on macOS, Android, and iOS, deferring only timed location acquisition.
5. Commit implementation, sync canonical specs, and archive the change.
6. Next extract Rust session catalog/recovery ownership, then compose persistence under the Rust application/local-server runtime before host cutovers and deletion of obsolete native and bridge paths.

Rollback before selection deletes the pruning module/API/fixtures and restores its verification recipe; no production package or durable production database requires migration or rollback.

## Open Questions

None. Host selection, catalog reconciliation, maintenance/reclamation policy, and deletion of native pruning remain separate changes.
