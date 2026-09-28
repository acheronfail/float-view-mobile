# Behavior audit in progress

This records source evidence at `fae1776b` for tasks 1.2/1.3. It is not a claim
that the full behavior/limit inventory or regression coverage is complete.

## Blocking identity decision

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

**Proposed resolution, pending user decision:** select Apple's versioned
derivation for newly identified boards; preserve existing catalog/archive IDs
as opaque legacy identities; explicitly document that legacy Android history
does not automatically merge into a newly created canonical board. Do not rewrite
history or reset a same-version catalog. An alternative is a separately specified
identity alias/migration policy, including collision, reconnection and archive
behavior. That expands the current no-migration scope.

Required regression cases after the decision: equivalent Apple/Android UUID
input; zero/all-FF normalization; same local association with legacy history;
legacy archive import before/after canonical discovery; canonical imported board
coexisting with legacy local association; repeated import and retained session
ownership. No identity-dependent implementation has been selected yet.

## Established behavior and discrepancies

Paths below refer to the mirrored native capability paths named in baseline.md.

| Area            | Existing evidence and shared behavior to preserve                                                                                                                                                                                                                                                         | Remaining issue/coverage                                                                                                                                                                               |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Settings        | `Domain.SettingsRecord`: revision 0, probe 500 ms, idle persistence 1,000 ms, riding 100 ms, tail 5,000 ms, reconnect 300,000 ms, minimum distance 100 m, system theme, metric units, null gap threshold, both GPS switches false. Stores merge allowed patches, check revision and clamp numeric ranges. | Apple reads revision from the SQL column; Android reads the JSON revision. Shared authority should be the canonical SQL revision; add inconsistent-column/JSON fixture.                                |
| Profile         | Nullable series 1–36, parallel 1–20, CAN preferences 0–253, nonempty motor up to 128 UTF-8 bytes; missing fields are series/parallel/motor. Profiles start revision 0 with null preferred BMS. Preferred controller changes clear routing hints.                                                          | Inventory all unknown-field, JSON size and profile-event split-commit behavior before porting.                                                                                                         |
| Saved boards    | Name trimmed, empty becomes null, 128 UTF-8 byte limit, optimistic revision; imported board has null platform association; immutable session snapshot separate from mutable profile.                                                                                                                      | Apple updates last-seen on reconnect while Android does not. Identity policy above remains unresolved.                                                                                                 |
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

Still to settle before task 1.3 completion: discovery/saved-board admission,
aggregate cache/staging bytes, bounded directory/startup scans, profile/settings
serialized limits, worker shutdown/retained-handle policy, cleanup-reserved
capacity, all waiter maps and completion queues. New caps must be recorded as
observable behavior changes before selecting either host. No new cap has been
silently chosen by this audit.
