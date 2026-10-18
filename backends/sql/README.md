# Durable storage source of truth

`catalog.sql` owns saved boards, profiles, settings, immutable session summaries,
active job recovery metadata, and standalone session summaries. Each session has a
separate database initialized with `session.sql`.

The shared Rust store and session writer execute these files verbatim, enable foreign keys and
WAL, use bounded transactions, update `commit_marker` in the same transaction as
sample batches, and fsync at ride-state transitions and close. Raw voltage is
canonical; corrections are never written over it.

Recovery treats catalog rows with an active state as interrupted, validates the
session database with `PRAGMA quick_check`, truncates catalog counts to the last
commit marker, and records a recovery event. No recovery path auto-deletes data.

The shared product store's canonical marker (defined in
`../core/product-store/src/lib.rs`) establishes baseline 100 for the first public
candidate. Recognised earlier development formats reset once without migration.
Baseline data is preserved; unsupported/newer/malformed markers and missing markers
on nonempty roots fail without deletion. Ordinary corruption and I/O failure never
reset data. Increment later incompatible storage markers and implement a tested,
crash-safe supported-data upgrade. SQLite schema/user_version and portable archive
format numbering remain independent. See [the inventory](../../docs/storage-baseline.md).

`refloat_configuration` stores bounded, complete application-wide Tunes. Exact
package version, schema fingerprint, and wire signature are part of the record
identity and query index, so releases never migrate or fall back by INFO version
or signature alone. Values JSON is validated against the active audited schema
before persistence and again before apply; optimistic record revisions prevent
lost updates. Raw schemas and device-current snapshots are not stored in this
table. Shared schema cache files are separate, bounded, digest-verified, and
disposable.

Refloat 1.2+ fault history is polled through command 35 on the controller's normal
scheduler route. `firmwareFaultHistory` session events retain each
fault start/end, the raw VESC code, the normalized reason, and the controller's
original system ticks. Their ride time and wall time identify retrieval, since
history may predate connection. `fatalErrorLatched` transitions are stored
separately from active `firmwareFault` transitions. Fault-bearing sessions survive
the minimum-distance filter. History is committed through the attached session
writer as soon as it arrives, independently of the next
telemetry sample, and survives pruning and archive export/import. These records
use existing event storage, so the supported schema and archive format stay the
same. Old recordings contain no newly inferred faults.

History reads overlap the last tick and deduplicate up to the firmware's 20
records, retaining simultaneous fault start/end events. Reconnect preserves this
cursor; controller clock rollback or wrap resets it. The firmware buffer can
overwrite faults during long disconnections, and power cycling clears its
history before it can be fetched. Unsupported older packages retain active-fault
sampling only. A session bounds fault history to 10,000 events; reaching the limit
reports a storage failure rather than silently discarding faults.

## SQLite scale checks

From `backends/core`, run the opt-in stress tests against isolated temporary,
on-disk WAL databases. They clean up their files and never open application data:

```sh
cargo test --locked --offline --release \
  -p float-view-product-store -p float-view-session-sqlite \
  --test scale -- --ignored --nocapture
FLOAT_VIEW_CATALOG_SIZES=1000000 FLOAT_VIEW_SAMPLE_SIZES=1000000 \
  cargo test --locked --offline --release \
  -p float-view-product-store -p float-view-session-sqlite \
  --test scale -- --ignored --nocapture
FLOAT_VIEW_BATCH_SIZE=1 FLOAT_VIEW_SAMPLE_SIZES=10000 \
  cargo test --locked --offline --release \
  -p float-view-session-sqlite --test scale -- --ignored --nocapture
```

Default catalog sizes: 1,000, 10,000 and 100,000. Default sample sizes: 1,000,
10,000 and 199,999, with an additional live write at each checkpoint. Sizes must
increase. `FLOAT_VIEW_BATCH_SIZE` selects 1–1,024 samples per transaction (default
100). Catalog fixtures include sparse board histories, tied timestamps, active
sessions, pending deletes and JSON snapshots. Every traversal checks ordering,
filters and exact counts. Telemetry fixtures use the real writer with nine numeric
fields, an extension and package-state transitions; measure batch percentiles,
GPS writes, reader validation, deep pages, events, locations, chart reduction,
diagnostics and pruning; and verify live-reader published boundaries. A separate
10,000-record Tune traversal covers mixed identities and tied timestamps.

From the repository root, inspect production location-pruning SQL with Python's
SQLite engine:

```sh
python3 backends/sql/tests/scale.py
python3 backends/sql/tests/scale.py --samples 1000000 --locations 20000
```

This secondary audit loads the checked-in schema and pruning statement, reports
`EXPLAIN QUERY PLAN`, checks unmatched locations and rolls back its measured
delete. Rust tests use the application's bundled SQLite and are the primary
behavior checks. Normal Rust tests guard session-list filter/cursor seeks,
location neighborhood boundaries and indexed event foreign-key lookups.
Timings are observations, not CI latency assertions.

### Measurements (2026-10-13)

Release builds on this Linux development host with local temporary storage,
WAL and `synchronous=FULL`; these are software measurements, not phone storage
latencies. Before/after runs used the same datasets:

| Operation                                                                                 |        Before |         After |
| ----------------------------------------------------------------------------------------- | ------------: | ------------: |
| Traverse 100,000-row catalog, completed sessions, 100 per page                            |     14,458 ms |        505 ms |
| Traverse sparse board history in that catalog                                             |        235 ms |         26 ms |
| Telemetry throughput growing from 10,000 to 200,000 samples, 100 per batch                | 15,705 rows/s | 80,270 rows/s |
| Telemetry batch p95 at that scale                                                         |       6.54 ms |       1.25 ms |
| Deep 100-sample page in 200,000 samples                                                   |      13.79 ms |       0.15 ms |
| Location neighborhood delete: 200,000 samples, 2,000 locations                            |      9,320 ms |       2.29 ms |
| Production prune commit: 200,000 samples, after location fix, before/after event FK index |      2,089 ms |        267 ms |

Session lists now use explicit filters and tuple cursors, with indexes matching
both equality predicates and the complete ordering. Unused duration/distance
indexes were removed to avoid maintaining changing aggregates on every catalog
update. Telemetry inserts reuse prepared statements. Sample/event time ranges
start at their cursors; Tunes use a tuple seek inside the identity index.
Location pruning uses indexed `BETWEEN` instead of applying `ABS` to every sample.
The event source-sample index prevents foreign-key checks from scanning events
for every removed sample. The shared storage marker permits only the recognised pre-baseline development
reset; baseline and subsequent supported data are preserved.

The larger run seeded 1,000,000 catalog rows and recorded 1,000,000 telemetry
samples. Completed-session traversal took 5.19 seconds **for all 988,697 visible
completed rows**; telemetry throughput was 81,690 rows/s, batch p95 was 1.23 ms,
and a deep sample page took 0.15 ms. Traversing 9,000 matching Tunes from 10,000
records took 24 ms. Location deletion with 1,000,000 samples and 20,000 locations
took 19 ms. A separate 10,000-sample singleton-transaction run also passed,
including pruning and live-reader checks.

### Operation audit and limits

The SQL owners are `product-store` and `session-sqlite`; Apple and Android use
these shared stores. Reviewed all their query/write sites:

| Path                                       | Behavior at scale                                                                                                                                                                                                                                    |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Boards/profiles/settings/route hints       | Primary/unique-key access; boards capped at 50; settings singleton.                                                                                                                                                                                  |
| Session catalog/admission/history/deletion | Composite list indexes, partial active index and pending-delete keyset index. Board history aggregates read matching sessions.                                                                                                                       |
| Telemetry/fields/transitions               | Bounded batches, atomic markers, cached inserts and indexed tails. Out-of-order insertion deliberately recomputes retained-sample aggregates; ordered recording does not.                                                                            |
| Samples/events/locations/markers/series    | Indexed elapsed-time reads and bounded pages; validated JSON field paths. Markers capped at 1,000. Series streams at most 200,001 source rows and rejects requests above 200,000.                                                                    |
| Pruning                                    | Bounded selection, protected samples, indexed neighborhoods/FK checks and atomic revisions. Above 200,000 source samples remains unsupported.                                                                                                        |
| Diagnostics                                | At most 32 records per transaction, 5,000 retained rows; indexed sequence deletion and immediate yield to writer contention.                                                                                                                         |
| Tunes/editor drafts                        | Exact identity index/tuple pagination; primary-key drafts with bounded records and payloads.                                                                                                                                                         |
| Archives/CSV                               | Backup and paged reads with size limits. Import validates schema, counts, integrity and foreign keys. Existing round-trip and hostile-input tests cover correctness.                                                                                 |
| Recovery/maintenance/storage totals        | Recovery and totals visit session files; integrity checks, archive validation and pruning counts read database content. Maintenance statistics use explicit single-session or upper-bound predicates; all-session statistics aggregate catalog rows. |

Reader opening still performs `quick_check(1)`: about 71 ms at 200,000 samples
and 355 ms at 1,000,000. Startup reconciles every catalog session, opening and
validating its file. These linear costs preserve current corruption/recovery
guarantees and can outweigh page-query costs when repeatedly opening readers.
The follow-up below measures startup with thousands of session files and archive
throughput. Cold-cache performance and device flash/fsync latency remain unmeasured.
Catalog fixtures intentionally have no per-session files: one million catalog
rows stresses indexes beyond the current 100,000 directory-entry admission bound.
The million-sample run verifies a bounded 200,000-row chart window and rejection
of an oversized full-session chart request; it does not change resource limits.

### Follow-up: GPS pages, live recording, aggregates, recovery and exports

The opt-in suite now also includes `gps_pagination_scale`, `live_catalog_scale`,
`recovery_file_scale` and `archive_csv_scale`. For isolated measurements, select
one exact test to avoid running benchmarks concurrently:

```sh
# From backends/core:
FLOAT_VIEW_GPS_ROWS=1000000 cargo test --locked --offline --release \
  -p float-view-session-sqlite --test scale gps_pagination_scale \
  -- --ignored --nocapture --exact
cargo test --locked --offline --release -p float-view-product-store \
  --test scale recovery_file_scale -- --ignored --nocapture --exact
FLOAT_VIEW_GPS_EVERY=1 cargo test --locked --offline --release \
  -p float-view-product-store --test scale archive_csv_scale \
  -- --ignored --nocapture --exact
```

`FLOAT_VIEW_GPS_ROWS` defaults to 100,000. The GPS seed includes tied times and
location gaps; every paged sequence is checked for omissions and duplicates.
Continuation pages now seek the location primary key rather than starting again
at the elapsed-time range's beginning. Valid location sequences have nondecreasing
elapsed times; the reader checks both sequence and elapsed ordering. Regression
coverage includes ties, missing cursors, restricted/empty ranges and corrupted
backwards times. No cursor or public result format changed.

CSV's old nearest-location lookup sorted every GPS point in a 20-second window
for every sample. It now uses two elapsed-index seeks to get the closest predecessor
and successor, with index seeks for tied timestamps, then compares their distance
and sequence. This preserves earliest-sequence tie breaking and the inclusive
10-second tolerance. Tests compare results against the original neighborhood sort
for dense/tied locations, gaps and integer limits, and guard against temporary
sorts in the new plans. Existing CSV privacy/encoding tests also pass.

Additional isolated measurements on the same development host:

| Operation                                                              |            Result |
| ---------------------------------------------------------------------- | ----------------: |
| Deep 100-location page in 100,000 locations, before → after            | 5.16 ms → 0.06 ms |
| Traverse all 100,000 locations, 1,000 per page, before → after         |    269 ms → 25 ms |
| Deep 100-location page in 1,000,000 locations                          |           0.05 ms |
| Traverse all 1,000,000 locations, 1,000 per page                       |            255 ms |
| GPS CSV export: 100,000 samples + 100,000 GPS points, before → after   | 3,568 ms → 625 ms |
| Archive export of that dense-GPS fixture (19.2 MB)                     |             28 ms |
| Upload, validate and attach that archive                               |            123 ms |
| Sparse/common board history, 100,000-row catalog                       |     2.6 / 41.8 ms |
| Single/all-session maintenance counts, 100,000-row catalog             |    0.02 / 22.6 ms |
| Telemetry + catalog commit + 100-row list read, concurrent reader      |  1.4 ms batch p95 |
| Reopen/reconcile 3,002 mixed session files, ~3.4 million total samples |          2,819 ms |

Aggregate timings exclude the independent correctness queries. Maintenance scope
queries were simplified; the previous single-session query was already fast, so
there is no claimed improvement there. All-session counts and large board totals
remain linear in matching history. A million-row catalog also passed both aggregate
scopes and sparse/common/absent board totals; this remains a beyond-admission-bound
stress case. Board totals are refreshed at product transitions, not every telemetry
batch; no extra index on changing aggregates was added.

The live fixture seeds a 100,000-row catalog and records 100,000 samples while a
separate production session reader continuously reads published boundaries. Each
batch applies its marker and checks the public session list's latest committed
sequence. It completed more than 10,000 concurrent reads without write failures
or boundary leakage. It uses one product-store owner, not a second runtime.

Recovery defaults to 100, 1,000 and 3,000 cloned session files, plus two templates.
Most have 100 samples; every hundredth has 100,000 samples. Each file is a separate
valid session database. Catalog summaries start stale, and one session is active;
reopening must reconcile every count, preserve the total, clear dirty integrity
and mark the interrupted session. `FLOAT_VIEW_RECOVERY_SIZES` and
`FLOAT_VIEW_RECOVERY_LARGE_SAMPLES` configure this fixture. Full integrity validation
remains enabled. The measurement establishes warm/local startup cost at these
sizes, not device or cold-cache startup latency.

Exports default to 100,000 samples and GPS every 10 samples. Configure
`FLOAT_VIEW_EXPORT_SAMPLES` and `FLOAT_VIEW_GPS_EVERY` to vary size/density. The
fixture checks CSV line counts and archive round-trip sample/GPS counts and distance.
Running the entire opt-in suite concurrently also passed, with expected timing
variation from competing benchmark I/O.
