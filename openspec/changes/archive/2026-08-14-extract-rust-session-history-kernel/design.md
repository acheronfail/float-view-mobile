## Context

The selected Apple and Android stores each contain a platform-neutral `SessionHistoryReducer` beside roughly 3,400 lines of host SQLite/file work. Both reducers implement ordered M4 downsampling, continuity generations, and prune-row selection, while the frontend keeps an independent reference implementation and immutable expected vectors. The future Rust SQLite/session runtime needs these algorithms, but moving either native store wholesale would also entangle schema bootstrapping, recovery, WAL coordination, cancellation, exports, settings, profiles, filesystem policy, and loopback serving.

The canonical contract already fixes the important limits and selection semantics: at most 16 fields, 1,024 buckets, 65,536 returned observations, an ordered `(elapsedMs, sequence)` key, explicit or cadence-derived gaps, elapsed-zero-aligned prune buckets, protected semantic evidence, and no more than eight ordinary selected rows per prune bucket. The current hosts remain production owners throughout this extraction.

## Goals / Non-Goals

**Goals:**

- Create one standard-library-only safe Rust kernel for deterministic historical series reduction and prune-row selection.
- Make both algorithms consume ordered rows incrementally with memory bounded by the declared field/bucket limits rather than raw ride length.
- Preserve canonical ties, gaps, late/null fields, phase cadence, role deduplication, protected evidence, and prune coverage behavior with shared expected fixtures and adversarial tests.
- Establish the direct dependency that a later Rust SQLite/session runtime will use, without adding a temporary host bridge.

**Non-Goals:**

- Selecting the Rust candidate from Swift, Kotlin, Apple packages, Android packages, or the frontend.
- Opening SQLite, executing SQL, managing WAL/recovery/leases, storing retained sequences, importing/exporting files, or changing `backends/sql`.
- Encoding transport cursors or JSON/base64, serving loopback requests, changing the frontend contract/reference, or changing any durable format.
- Moving session creation, telemetry/location writes, summary projection, settings, profiles, schema caches, or native file presentation.
- Deleting the selected native reducers before a later direct Rust runtime cutover.

## Decisions

### 1. Extract an application kernel before a database adapter

Add `backends/core/session-history-kernel` above raw stored rows and below the future Rust persistence/runtime. It accepts owned or borrowed safe Rust values and returns owned reduced columns or bounded prune selections. It never accepts a database handle, host callback, foreign record, serialized transport payload, or executor.

Moving the complete stores now was rejected because their combined surface spans unrelated product and OS responsibilities and could not be proven as one regression-bounded increment. Extracting only models or traits was also rejected because it would add scaffolding without moving behavior.

### 2. Use ordered streaming state with explicit finite bounds

`SeriesAccumulator` is configured with unique bounded field names, an inclusive nonnegative elapsed range, bucket count, explicit/automatic gap policy, positive persistence cadences, and retained resolution. Each appended row must be nondecreasing by `(elapsed_ms, sequence)`, lie inside the range, provide exactly one optional value per field, and contain only finite present values. It retains only first/minimum/maximum/last candidates per field/bucket plus continuity generations. Bucket arithmetic uses checked or widened integer operations, counter overflow fails closed, and final output preserves field order and chronological candidate order.

`PruneSelector` consumes the same ordered row shape with per-row protected evidence. It derives session endpoints, first present field sources, and both sides of riding-phase transitions; caller-classified event and summary sources enter through the protected flag. It finalizes elapsed-zero-aligned buckets as the stream advances, always retains protected rows, then covers ordinary first/last and ordered per-field minimum/maximum roles through deterministic maximum coverage with an explicit eight-row cap. Finalized selections are returned incrementally so the later SQLite owner can place them in a file-backed keep-set without the kernel retaining a ride-sized collection.

An unordered convenience API was rejected: sorting an arbitrary ride would hide unbounded memory and duplicate the database's indexed ordering responsibility.

### 3. Follow the canonical contract when dormant native edge behavior differs

The immutable contract/reference vectors and canonical requirements are the candidate authority. Explicit thresholds round to the nearest millisecond, equal extrema select the earliest ordered row, and pruning enforces the normative eight ordinary-row cap. The selected native paths do not change in this extraction; any newly exposed edge discrepancy becomes explicit cutover evidence rather than being copied into the future owner.

### 4. Keep fixtures implementation-independent and dependencies at zero

Commit literal TSV fixtures under `backends/fixtures/session-history/` for series rows/configuration/expected columns and prune rows/protection/expected selections. Rust tests parse those fixtures with small test-only readers; production code includes no frontend, Swift, Kotlin, JSON, SQL, or fixture parser. The crate uses only `std`, so the offline locked graph and mobile targets gain no third-party dependency.

### 5. Remain unselected and bridge-free until the runtime exists

The workspace and root verification include the crate, but Apple and Android build catalogues, SwiftPM/Xcode inputs, Gradle tasks, packaged libraries, exported symbols, and native source remain unchanged. Repository audits reject a `session-history-apple-ffi`, `session-history-android-jni`, generated binding, native dependency, SQLite/network dependency, or selected host reference. Later Rust persistence calls the safe kernel directly; later host cutovers delete both native reducers along with the protocol bridges no longer needed by the complete Rust runtime.

## Risks / Trade-offs

- **[Canonical edge semantics can expose dormant host drift]** → Keep the candidate unselected, record canonical expected vectors, and require decoded black-box parity before a later cutover changes production ownership.
- **[M4 state reaches the 65,536-observation ceiling]** → Compose field, bucket, role, and total-output bounds before allocation and fail closed on checked arithmetic or counter exhaustion.
- **[Pruning can silently lose semantic evidence]** → Separate protected rows from the eight ordinary candidates, derive endpoints/field starts/phase boundaries in the kernel, and test dense conflicting roles, ties, and repeated coarser selection.
- **[A kernel API could force future SQLite buffering]** → Require ordered incremental append/finalize APIs and return finalized prune buckets rather than accepting or returning a complete raw ride.
- **[Another temporary bridge would recreate the code-growth problem]** → Prohibit host ABI crates and defer selection until the platform-neutral Rust persistence/runtime calls the kernel directly.

## Migration Plan

1. Commit this proposal, design, delta specs, and tasks.
2. Add the standard-library Rust crate, shared literal fixtures, focused vector/adversarial tests, and ownership/dependency audits without touching native or frontend production sources.
3. Run focused checks, warnings-denied workspace linting, `just verify`, strict OpenSpec validation, and the canonical ESP32 matrix on macOS, Android, and iOS with only bounded location acquisition deferred.
4. Commit the implementation, sync the delta specs, and archive the change.
5. In later changes, add the Rust SQLite/session owner that streams database rows through this kernel, compose it into the Rust application runtime and loopback server, cut over each host, and delete the native reducers and obsolete bridge crates.

Rollback before selection removes the unreferenced crate, fixtures, audit entries, and OpenSpec delta. No production package, database, or durable state requires rollback.

## Open Questions

None. SQLite ownership, cursor serialization, loopback serving, production selection, and native deletion remain separate changes.
