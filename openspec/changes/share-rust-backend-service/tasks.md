## 1. Baseline and behavior inventory

- [x] 1.1 Inventory native product/storage/application policy and map every retained native responsibility to an OS capability. Verify both hosts' call paths and record exact deletion targets, source/package/dependency baseline and ownership exceptions.
- [x] 1.2 Record validation, defaults, identity/revision, cursor, cache, recovery, retention, archive/CSV and lifecycle semantics against contract/SQL/conformance evidence. Resolve Swift/Kotlin differences with regression fixtures and an explicit behavior table.
- [x] 1.3 Inventory numeric limits for all queues, collections, bytes, staging/cache retention, SQLite waits and shutdown. Deliver a boundary-test matrix with exact ceilings, admission points, cleanup capacity and any proposed observable changes before implementation.

## 2. Shared product storage

- [x] 2.1 Establish cohesive shared storage/service components with existing locked dependencies. Verify downward dependencies, no second runtime, at most 28 members and no new binding artifact or generated API.
- [x] 2.2 Implement catalog startup, canonical schema loading, root confinement and existing schema-marker reset behavior. Test valid reopen, incompatible disposable storage, corruption, unavailable/protected roots and symlink/path escape without deleting unrelated files.
- [x] 2.3 Move settings, saved-board identity/association, profiles and routing hints. Verify canonical defaults, patch bounds, optimistic conflicts, nullable imported associations and private identifier exclusion.
- [x] 2.4 Move Tune persistence and schema caches. Test exact compatibility identity, complete values, record conflicts, pagination, digest/size rejection and bounded eviction against recorded fixtures.
- [x] 2.5 Move session creation/catalog queries and marker reconciliation. Test keyset ordering/cursor binding, immutable snapshots and fault injection before/after session commit and catalog update.
- [x] 2.6 Move close/retention, interrupted-session recovery, board forgetting and pending-delete/cancel behavior. Test active-board rejection, idempotent restart, retained committed data and reader/export lease races.
- [x] 2.7 Move persisted maintenance coordination into the shared service using existing prune kernels. Test bounded batch cursors, cancellation, process restart and committed logical prune with interrupted compaction.

## 3. Shared archive and file lifecycle

- [x] 3.1 Implement shared archive staging, manifest validation and import attachment through bounded file streams. Test exact/+1 import bounds, malformed schema/metadata, local session IDs versus portable board identity, WAL/manifest normalization and failure cleanup.
- [x] 3.2 Implement shared archive and CSV exports using existing session readers. Compare semantic archive fixtures and exact CSV columns/escaping/GPS matching with canonical expectations; verify long exports do not block recording.
- [ ] 3.3 Implement service-issued import/export leases and bounded native external-file transfer/presentation. Test stale/foreign handles, cancellation, frontend disconnect during presentation, abandonment expiry and next-launch cleanup.

## 4. Shared application policy and OS boundary

- [ ] 4.1 Compose one process-owned service with one application runtime and store. Test construction/destruction, ordered mutations, independent listener lifetime and runtime identity through the safe constructor.
- [ ] 4.2 Move discovery expiry, board identity and saved/guest activation. Test bounded discovery, failed storage preparation, stale activation and guest isolation before any ready publication.
- [ ] 4.3 Move reconnect/backoff, session attach/close/retention and terminal failure policy. Test radio changes, configured deadlines, reset/replacement, stale timers and uninterrupted saved-session identity on recovery.
- [ ] 4.4 Move location capture eligibility and session-clock assignment while retaining native acquisition. Test permission/settings/ride-state changes, background capability loss, stale observations and commit-before-publication.
- [ ] 4.5 Move configuration/control request completion and Tune validation/apply coordination. Test stale schema/revisions, concurrent requests, disconnect/cancellation and exactly-once board mutations through existing kernels.
- [ ] 4.6 Implement the closed OS fact/effect/completion exchange and bounded admission. Test malformed envelopes, generations, duplicates, cleanup under saturation, late UI outcomes, worker contention and bounded shutdown.
- [ ] 4.7 Connect shared server dispatch and publication directly to the service. Verify no native domain-operation round trip, monotonic revisions independent of client count and historical/transport limits with recording under load.
- [ ] 4.8 Adapt the existing C/JNI families to the service lifetime and OS exchange. Test caller-buffer bounds, invalid handles, panic containment, destruction and exact symbols; remove superseded entry points when unused.

## 5. Direct-host proof before native selection

- [ ] 5.1 Add a test-only direct Rust launcher with synthetic OS capabilities and conformance-owned protocol simulation. Audit it for absence of product CRUD/SQL/archive/identity/reconnect policy and run canonical product scenarios through the production service constructor.
- [ ] 5.2 Add deterministic restart, storage exhaustion, split-commit, lock-contention and queue-saturation scenarios. Verify recovery, cancellation, cleanup, terminal results and continued permitted recording with bounded assertions.
- [ ] 5.3 Run focused locked Rust tests, warnings-denied lint, mock/direct conformance and target compilation. Verify production native selection remains unchanged at this checkpoint and commit the verified shared slice with task updates.

## 6. Apple selection and deletion

- [ ] 6.1 Wire Apple startup/root protection, Bluetooth, timers, permission/location adapters, background execution and file presentation to the service. Test OS-effect ordering, runtime identity, scoped file lifetime and background file access.
- [ ] 6.2 Delete Swift portable stores, policy/waiters, domain dispatch and snapshot reconstruction, retaining only OS adapters/composition. Verify deletion inventory, imports, exact symbols and absence of native catalog/archive policy.
- [ ] 6.3 Run focused Swift/FFI tests, clean macOS/iOS simulator/device packaging and canonical macOS/iOS software conformance. Verify recording survives frontend/listener recreation and commit the buildable Apple slice with task updates.

## 7. Android selection and deletion

- [ ] 7.1 Wire Android startup/storage roots, BLE, timers, permission/location brokers, foreground service and scoped file presentation to the service. Test effect ordering and supported minimum-API behavior through linked and device checks.
- [ ] 7.2 Delete Kotlin portable stores, policy/waiters, domain dispatch and snapshot reconstruction, retaining only OS adapters/composition. Verify deletion inventory, dependencies, exact JNI symbols and absence of native catalog/archive policy.
- [ ] 7.3 Run linked JVM tests, clean host/four-ABI Debug/Release packaging and canonical Android conformance. Verify Activity recreation, reconnect and background execution retain shared state and commit the buildable Android slice with task updates.

## 8. Ownership and physical acceptance

- [ ] 8.1 Update AGENTS.md, ownership/SQL docs, OpenSpec context and build/source audits. Verify only OS capability implementations remain native, both hosts select one service and obsolete compatibility paths/dependencies are removed.
- [ ] 8.2 Record final source, workspace, bindings, dependencies and package sizes. Verify net production source below 49,922, at most 28 workspace members, exactly two binding families, zero temporary bridges and unchanged public contract/SQL/archive formats.
- [ ] 8.3 Run Android, iOS and macOS ESP32 physical checkpoints, including saved/guest telemetry, recording, reconnect, permitted background location, frontend/lifecycle recovery and native file presentation. Record exact results and keep any unavailable coverage incomplete unless explicitly deferred by the user.

## 9. Final verification and completion

- [ ] 9.1 Run focused boundary, crash/recovery, archive, contention, shutdown and host integration checks after all deletions, then formatting, strict OpenSpec validation and `git diff --check`. Verify every requirement has passing evidence.
- [ ] 9.2 Run `just verify` with both hosts selecting the complete shared service and direct-host proof included in the software gate. Commit verified implementation and matching task updates; after every required gate passes, sync delta specs and archive together in the final commit.
