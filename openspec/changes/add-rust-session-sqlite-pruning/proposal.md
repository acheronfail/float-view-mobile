## Why

Rust already owns canonical per-session SQLite reads/writes and bounded prune selection, but no Rust component atomically applies that selection to a durable session. Adding this final per-session mutation now removes another duplicated Swift/Kotlin algorithm before catalog and application-runtime composition, while the candidate remains unselected and production behavior stays unchanged.

## What Changes

- Add bounded prune preview and atomic prune commit APIs to the unselected Rust `session-sqlite` package, streaming canonical sample rows directly through `session-history-kernel` and retaining protected field, event, endpoint, and ride-transition evidence.
- Bind a preview to its immutable commit marker and require commit-time revalidation so writer progress, concurrent pruning, or stale callers fail closed without partial mutation.
- Delete only unretained samples and locations no longer near a retained sample, then atomically update prune metadata, stored counts, commit time, and history revision while preserving recorded counts and published sequence boundaries.
- Add exact SQLite fixtures and adversarial tests for initial and aligned coarser pruning, no-op previews, stale plans, cancellation, rollback, active-WAL behavior, malformed databases, bounds, and payload-free diagnostics.
- Extend root verification to require direct composition with the existing history kernel and reject new FFI/JNI crates, generated bindings, host selection, copied native pruning code, schema drift, and production Apple/Android/frontend changes.
- Keep native active-session/read-lease exclusion, catalog reconciliation, maintenance jobs, checkpoint/incremental-vacuum policy, filesystem selection, and all OS capabilities unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Extend the consolidated Rust per-session SQLite candidate with bounded, race-safe prune preview and atomic mutation behavior.
- `platform-hosts`: Preserve selected Swift/Kotlin stores and all platform ownership while the new Rust mutation remains unselected.
- `backend-conformance`: Require exact/adversarial SQLite pruning evidence, production-isolation audits, the complete software gate, unchanged available-host physical behavior, and an explicit migration-final Android hardware gate when no handset is available for this unselected slice.

## Impact

- Affects `backends/core/session-sqlite`, its fixtures, root verification recipes/audits, and OpenSpec ownership documentation.
- Reuses the existing locked `rusqlite` and `session-history-kernel` graph; no new dependency, runtime, bridge, generated binding, or cross-platform API is introduced.
- Changes no frontend contract, canonical SQL schema, production native source/package input, catalog, durable production database, or selected host behavior in this step.
- Runs the macOS and iOS ESP32 matrices now; records Android hardware as unavailable and defers that physical matrix to the migration-final gate before any Rust runtime selection.
- Non-goals are host cutover, native pruning-code deletion, catalog/recovery ownership, maintenance scheduling, imports/exports, local serving, application behavior, and bridge removal; those remain later independently gated changes.
