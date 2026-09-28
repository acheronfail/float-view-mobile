# Behavior audit

This records source evidence at `fae1776b` for tasks 1.2/1.3.
The shared bounds and remaining execution tests are specified in
[resource-boundaries.md](resource-boundaries.md). Product regression cases now
live in `backends/core/product-store/tests/store.rs`; later service/host checks
remain required by their implementation tasks.

## Resolved identity decision

Apple `application/ApplicationService.swift:988` hashes
`UTF8("float-view-board-v1\0") || firmwareUUID` and prefixes the hexadecimal
digest with `vesc-v1-`. Android `application/ApplicationService.kt:1279` hashes
only the 12 UUID bytes, without a version prefix or domain separator.

For the synthetic UUID `0102030405060708090a0b0c`:

- Apple: `vesc-v1-3ad3f33d03efba4de40aeb78f4513d6b5480595764b3a9e46dcc694025fa28ec`
- Android: `206402cab345415716d9a33469feba57a90dc200c064bc0190b4191af058b0eb`

Source-derived meaningful/zero/all-FF diagnostic vectors are recorded in
[identity-baseline.json](identity-baseline.json). These are audit inputs, not
yet executable production regression coverage. Apple's helper rejects all-zero
UUIDs; Android's helper accepts them. Both helpers accept all-FF. Upstream/runtime
identity normalization must be traced before deciding whether those inputs can
reach production activation.

Both `SQLiteLogStore.ensureSavedDevice` implementations first match the supplied
stable identity, then the local platform ID. An existing match updates local
association but **does not replace its `device_history_id`**. Both archive imports
merge by exact `deviceHistoryId`; neither has an alias table. Archives deliberately
omit raw firmware UUIDs, so an old Android digest cannot be transformed into the
Apple digest from archive contents alone.

Canonical `backend-contract` requires a versioned deterministic identity, stable
archive identity and exact-identity import association. Its versioned requirement
supports choosing the Apple derivation for new identities. However, the current
change also requires unchanged identity semantics, cross-host equality and no
migrations. It does not state a legacy-identity exception or a reconciliation
policy when a canonical imported board and legacy local board both exist.

`conformance/src/transport.ts:110` checks archive round-trip identity within a
host; `src/suite.ts` exercises the product flows. Neither check specifies the
cross-platform digest for this synthetic UUID. SQL only constrains the identity
to be unique text. Those sources cannot decide the legacy behavior.

**User resolution, 2026-09-28:** the application has no users or released
versions. Earlier development data is disposable. Use the versioned Apple hash
on both hosts, reject all-zero/all-FF UUIDs as non-meaningful, and reset incompatible
owned storage with marker `12`. Do not implement legacy aliases or migrations.
Current-format opaque imported identities stay opaque; no automatic conversion
is attempted. This policy is persistent in AGENTS.md and OpenSpec context.

Regression cases: equivalent platform UUID input; zero/all-FF/absent identity;
marker-11 reset; current-marker reopen/corruption without reset; canonical import
before/after discovery; repeated import and nullable platform association.

## Established behavior and discrepancies

Paths below refer to the mirrored native capability paths named in baseline.md.

| Area            | Existing evidence and shared behavior to preserve                                                                                                                                                                                                                                                         | Remaining issue/coverage                                                                                                                                                                               |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Settings        | `Domain.SettingsRecord`: revision 0, probe 500 ms, idle persistence 1,000 ms, riding 100 ms, tail 5,000 ms, reconnect 300,000 ms, minimum distance 100 m, system theme, metric units, null gap threshold, both GPS switches false. Stores merge allowed patches, check revision and clamp numeric ranges. | Apple reads revision from the SQL column; Android reads the JSON revision. Shared authority should be the canonical SQL revision; add inconsistent-column/JSON fixture.                                |
| Profile         | Nullable series 1–36, parallel 1–20, CAN preferences 0–253, nonempty motor up to 128 UTF-8 bytes; missing fields are series/parallel/motor. Profiles start revision 0 with null preferred BMS. Preferred controller changes clear routing hints.                                                          | Inventory all unknown-field, JSON size and profile-event split-commit behavior before porting.                                                                                                         |
| Saved boards    | Name trimmed, empty becomes null, 128 UTF-8 byte limit, optimistic revision; imported board has null platform association; immutable session snapshot separate from mutable profile.                                                                                                                      | Apple updates last-seen on reconnect while Android does not. Canonical identity policy is resolved above; update last-seen on each observed connection.                                                |
| Tune            | Canonical SQL exact package/version/fingerprint/wire signature; revisions start at 1; bounded complete values validated by application configuration owner before storage/apply.                                                                                                                          | Record full compatibility/conflict fixtures; list cursor's store identity binding is narrower than the complete identity.                                                                              |
| Schema cache    | CRC must be nonzero; exact identity and SHA-256 digest; compressed data at most 32 KiB, envelope at most 64 KiB; retain eight newest files.                                                                                                                                                               | Kotlin compares serialized identity objects while Swift uses typed equality. Use typed field equality in shared code; malformed/digest/size/eviction tests required.                                   |
| Session list    | Shared server `dispatch.session_list` binds cursor to history ID, query shape, catalog revision; max 100. Native store uses sort-key/session-ID keyset plus one lookahead row.                                                                                                                            | Preserve outer cursor validation when removing native inner cursor dispatch; do not copy the weaker store-only cursor as the public cursor.                                                            |
| Commit/recovery | Session marker is authoritative; reconcile catalog before later reads/appends. Both recover interrupted active sessions and retain only persisted riding with distance at least the configured threshold.                                                                                                 | SQL README's “No recovery path auto-deletes data” contradicts the current contract/implementation retention rule. Correct documentation. Do not treat an unreadable session as proven below threshold. |
| Import          | 128 MiB max, archive metadata version 1, integrity check, marker and nonempty samples, new local session/job IDs, exact portable history identity; strip manifest before strict reader.                                                                                                                   | Both commit catalog attachment before reconciliation, then delete destination on any reconciliation error: fault tests must prevent a committed catalog row referencing deleted data.                  |
| CSV             | Stable column order in both stores; quote text and double embedded quotes; optional GPS columns accuracy/longitude/latitude, nearest location within ±10,000 ms, ties by earliest location sequence.                                                                                                      | Record exact column/escaping and semantic numeric fixtures before moving encoding.                                                                                                                     |
| Reconnect       | Native application uses 1,000 ms exponential delay, exponent at most 5, capped at 30,000 ms and remaining configured budget; saved session survives retry.                                                                                                                                                | Generation-qualified stale timer, reset/radio/replacement tests still required.                                                                                                                        |
| Discovery       | Expires after 120,000 ms; Apple coalesces publications every 250 ms.                                                                                                                                                                                                                                      | No native entry-count cap found; snapshot truncation is not bounded discovery admission.                                                                                                               |
| Location        | Saved mode, active session, setting enabled, riding and authorized/foreground-only provider required. Capture elapsed is base persisted elapsed plus monotonic time since provider start. Append must succeed before location publication.                                                                | Validate lifecycle/background loss and stale job/generation in the final service.                                                                                                                      |

## Numeric boundary inventory started

All rows need exact-bound/+1 rejection tests at admission, plus cancellation and
cleanup tests where applicable. This list is not yet the complete task 1.3 matrix.

| Resource                                     | Existing bound/source                                                                                  | Required admission/cleanup proof                                                                                                |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| WebSocket input / history request            | 65,536 / 16,384 bytes, `loopback-server/limits.rs`                                                     | Reject oversized input before parse/allocation expansion.                                                                       |
| Decoded history response                     | 4,194,304 bytes                                                                                        | Bound before publication/compression.                                                                                           |
| Sockets / operations / per-client operations | 16 / 64 / 8                                                                                            | Reject before side effects; client cancellation releases waiter without replaying mutation.                                     |
| Host drain / envelope                        | 8 effects / 1,048,576 bytes                                                                            | Validate complete bounded envelope; reserve cleanup capacity.                                                                   |
| Response queue / latest snapshot             | 8 / 1                                                                                                  | Slow clients cannot retain a snapshot backlog.                                                                                  |
| History readers / blocking workers           | 2 / 2                                                                                                  | Hold leases until work actually ends.                                                                                           |
| Transfers / binary chunk / import            | 2 / 65,536 / 134,217,728 bytes                                                                         | No archive-sized buffer; stale handle and cancellation cleanup.                                                                 |
| Host poll / listener stop / I/O              | 250 ms / 2 s / 30 s                                                                                    | Stop must remain independent of the recording owner.                                                                            |
| Runtime command / result / token table       | 1 MiB / 4 MiB / 2,048 tokens                                                                           | Existing host bounds remain; runtime effect ceiling 1,024.                                                                      |
| Sample/location append batch                 | 1,024 each, `session-sqlite/lib.rs`                                                                    | Validate before transaction/allocation.                                                                                         |
| Numeric extensions                           | 32 fields, 64 bytes/name, 16,384 bytes JSON                                                            | Preserve existing strict extension validation.                                                                                  |
| History                                      | 16 fields, 2,000 samples/locations, 500 events, 1,024 series buckets, 200,000 series/prune source rows | Shared reader/pruner rejects at bounds; cancellation retains reader lease.                                                      |
| Session / Tune list                          | 100 / 50 rows, one lookahead row                                                                       | Keyset scan; no whole catalog materialization.                                                                                  |
| Session / Tune cursor                        | 1,024 / 512 bytes                                                                                      | Validate query/identity/revision before lookup.                                                                                 |
| Tune fields                                  | Name 128, description 1,024, version 128, values 49,152 UTF-8 bytes                                    | Native checks precede SQL; SQL character counts alone are insufficient for byte ceilings.                                       |
| Schema cache                                 | 8 entries, 32 KiB payload, 64 KiB envelope                                                             | Typed identity/digest check and eviction without reading arbitrary files.                                                       |
| Staged exports                               | At most 20 retained by cleanup; age cutoff 86,400,000 ms                                               | Current cleanup enumerates whole directory and has no total-byte cap; active presentation lease must survive client disconnect. |
| Session SQLite writer / readers/pruner       | 1,000 ms busy timeout / 250 ms                                                                         | Hostile lock plus cancellation and shutdown tests.                                                                              |
| Native catalog waits                         | Swift sets no busy timeout in its Database wrapper; Android delegates to framework SQLite              | Choose/document explicit shared timeout; do not claim a numeric Android default without platform evidence.                      |
| Free storage thresholds                      | Warning below 1,073,741,824; critical below 524,288,000; stop below 262,144,000 bytes                  | Exact boundary does not enter lower state; native supplies availability, shared service decides.                                |

The completed proposed matrix is in resource-boundaries.md. Decisions that
resolve native differences: canonical versioned identity with development reset;
SQL settings revision authority; reject corrupt current settings instead of
silently replacing them; typed cache identity equality; update last-seen on saved
connection; 16-KiB merged profile cap; bind Tune cursors to all identity fields;
validate archive profile contents before attachment. Profile-change session events
must be ordered by the shared service after catalog persistence, with failure
reported independently of the already committed profile. No legacy data fallback
is part of these choices.

Product tests exercise conflicting SQL/JSON revisions, cross-platform identity
vectors, import/explicit association, profile route invalidation, exact/+1 UTF-8
name bounds, complete/exact-identity Tunes, cursor rebinding and cache corruption.
Session split-commit, retention, CSV and lifecycle regression tests remain part of
tasks 2.5–4.8; the audit does not mark that implementation complete.

## First shared-storage checkpoint

The safe `product-store` crate now implements marker-12 startup, canonical catalog
SQL, settings, boards/profiles/private routing hints, complete Tunes and bounded
schema caches. `backend-service` establishes one runtime/store composition but
has not yet replaced native production. The workspace contains 28 members;
external package versions and both binding families are unchanged.

Focused tests cover tasks 2.1–2.6. Settings/profile updates use immediate SQLite
transactions and SQL revision authority. New profiles retain bounded unknown
fields as before, validate known fields strictly, and clear routing hints in the
same transaction. Native selection, profile-change session events and active
configuration source-revision validation remain in the later service integration
tasks; no native implementation has been removed prematurely.

Checkpoint validation (2026-09-28): all 28 new focused tests, locked/offline
workspace tests and warnings-denied lint passed. The new crates compile for
`aarch64-apple-ios` and `aarch64-linux-android`. Public contract and canonical
SQL hashes are unchanged, as are all 105 external locked package versions and
checksums. Clean Apple packaging and Android four-ABI Debug/Release packaging
passed. Frontend (188), conformance/fixture (185), Swift (16), Android unit tests
and mock/macOS conformance passed in the full gate run.

The full `just verify` invocation stopped at iOS's first `state.get` with
`unavailable`. A clean standalone iOS rerun passed (revision 60); standalone
Android conformance passed (revision 50). This is not recorded as a passing full
gate. The existing listener starts before its initial publication, so bootstrap
readiness must be tested during shared-service integration. No native selection
has changed. Direct-host proof, complete service behavior and final acceptance
remain unchecked.

## Persisted maintenance checkpoint

Maintenance now admits one session per worker turn using an internal lexical
keyset and a fixed upper session ID. It does not reuse public cursors invalidated
by pruning. Exclusive per-session leases coordinate readers, deletion, forgetting
and pruning; workers retain leases through owner-side reconciliation. Long worker
execution occurs outside the service owner lock. A cancellation request persists
before interrupting the worker; restart turns `cancel_requested` into `cancelled`
rather than reviving cancelled work. This intentionally corrects the old native
restart behavior. Running jobs resume as pending.

A committed session prune remains authoritative if catalog reconciliation fails.
Replay detects the durable retained resolution and advances the job without
pruning again. A recovered replay reports zero additional reclaimable bytes;
that field is an estimate, not a durable physical-space accounting ledger.
Incremental compaction is bounded to 256 pages per turn and best effort after the
logical commit. Failure or interruption cannot roll back the logical prune.
History revision changes invalidate public catalog cursors; ordinary sample
commits do not. Worker-pool dispatch and public request wiring remain part of the
later complete-service integration.

The startup race found in the first full-gate attempt is now guarded in the
shared listener: `/api/bootstrap` returns HTTP 503 before the first complete
snapshot. The transport regression verifies readiness before/after publication
and preservation across listener restart. The public bootstrap document and wire
contract are unchanged.

The subsequent full `just verify` run passed on 2026-09-28 after the readiness
fix, including clean Apple/Android packaging, all software tests, native
conformance (macOS revision 50, iOS revision 60, Android revision 50) and the
repository ownership/source audit. The focused three-crate run passed 53 tests;
workspace warnings-denied lint and iOS/Android target checks passed. Tasks
2.1–2.7 are complete as the shared storage/maintenance foundation. This is a
verified implementation checkpoint, not final acceptance: the complete service,
direct-host proof, native selection/deletion and physical checks remain pending.

## Shared archive and CSV checkpoint

The shared store now streams bounded imports, validates the exact canonical
schema and manifest, normalizes WAL state, and attaches a new local session
with portable board identity in one catalog transaction. Failed attachment
cleans owned files; uncertain commits leave files for reference-aware startup
recovery. Export workers retain source leases and use independent SQLite
readers, allowing another session to keep recording. CSV uses the canonical
52-column fixture, native numeric text, bounded escaping and nearest GPS
matching with deterministic sequence ties.

The existing locked rusqlite package enables its backup feature for bounded
page-wise snapshot copying, replacing both native archive-copy implementations
at cutover; external package versions and checksums remain unchanged.

The service file API admits two transfers with two shared archive/maintenance
worker permits. Cancellation retains worker admission until actual completion.
Idle transfers expire after 30 seconds; presentation retains files across client
disconnect until acknowledgement or 24-hour expiry. Native integration of this
API remains task 3.3 and the host cutovers. Typed service facts and operations
now belong to backend-service, with temporary server reexports until cutover.

Validation: 45 focused store/service tests and warnings-denied workspace lint
passed. Eight archive tests cover exact 128-MiB import, malformed input, SQL
rollback, startup cleanup, foreign leases, CSV fixtures, pruned location
boundaries and recording during export. Native selection is unchanged.

## Shared service and direct-host checkpoint

The process constructor composes one runtime, one catalog and an independent
maintenance executor. An exclusive OS file lock prevents a second process owner;
worker, reader and presentation leases retain that lock until resources close.
Listener creation borrows the existing owner. Direct dispatch and complete
publication use shared product facts without a native operation exchange.

Portable policy now owns discovery expiry/coalescing, saved/guest activation,
reconnect and attempt deadlines, durable session continuity, location eligibility,
configuration/control/Tune completion and correlated UI requests. Profile-change
events allocate session event sequence numbers transactionally instead of reusing
profile revisions, preserving telemetry events and the canonical schema.

The closed OS exchange has 2,048 effect slots with 1,024 slots reserved for a
runtime burst, eight effects per drain, a 1-MiB serialized bound and 8-KiB input
notification bound. Fixed cleanup slots remain available under saturation.
Request admission reserves revision capacity before mutation. Root locking,
worker cancellation and five-second contended shutdown are covered by focused
regressions. Existing store/kernel tests cover storage exhaustion, split commits,
restart recovery, SQLite contention, stale/cancelled workers and ongoing recording.

The test-only Rust host uses the production process constructor. Its TypeScript
relay provides clocks, synthetic OS callbacks and conformance-owned protocol
simulation; it contains no CRUD, SQL, archive, identity or reconnect policy.
Canonical direct conformance passed at revision 532; mock passed at revision 31.
The full locked Rust workspace tests (including C bounds and destruction),
warnings-denied lint, TypeScript check, iOS device compilation and Android arm64
compilation passed. Exact binding symbol audits include the new service calls in
the existing families. Superseded calls remain temporarily used by the native
hosts and will be removed at selection. Native production selection is unchanged
at this checkpoint. Physical acceptance and final software gates remain pending.

## Apple selection checkpoint

macOS and iOS now create the shared process service in their protected private
root. The Swift executor only maps OS callbacks and executes Bluetooth, native
timers, location, permission, theme and file-presentation effects. It keeps at
most 64 queued callbacks and 2,048 timers in a bounded table, with fixed/coalesced
cleanup slots. Listener restart borrows the existing process owner. iOS reports
foreground changes independently of disposable WebViews and protects new files
until first device unlock. Presentation dismissal releases shared file leases.

Deleted Swift catalog/session/archive stores, product models, application policy,
runtime reconstruction, operation dispatch, waiters and snapshot publication.
Removed CSQLite and bundled SQL resources; SQL remains embedded by Rust. Removed
unused C runtime and native transfer entry points from the existing Apple family,
with updated caller-buffer regression tests and exact symbol audit.

Validation: nine Swift OS/host tests passed, including canonical conformance and
listener recreation; production macOS conformance passed at revision 648 and iOS
simulator at revision 653. Clean macOS Release, iOS simulator Release and iOS
device Debug packaging passed. Shared service/C tests and warnings-denied lint
passed. A deterministic shutdown regression additionally verifies immediate
acceptance and terminal link-loss classification without reconnect. Real-device
background access and file presentation remain part of physical task 8.3.
