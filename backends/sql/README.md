# Durable storage

[catalog.sql](catalog.sql) defines saved boards, profiles, settings, session
summaries and recovery metadata. Each recording uses [session.sql](session.sql).
The shared Rust product store and session writer execute these schemas; native
hosts do not maintain parallel SQL implementations.

## Constraints

- Keep sample batches and their `commit_marker` in the same transaction. Recovery
  reconciles catalog summaries to committed session boundaries.
- Preserve foreign keys, WAL and bounded transactions. Ride-state transitions and
  closure require durable publication. Raw voltage is canonical; never overwrite it
  with corrected/estimated values.
- Recording-boundary events protect both sides of an interruption from pruning.
  Missing historical reasons or context remain unknown rather than inferred.
- Catalog schema, owned-storage marker and archive format have independent versions.
  Follow [storage-baseline.md](../../docs/storage-baseline.md) for supported upgrades,
  reset boundaries and frozen fixtures. Corruption never authorizes a reset.

Detailed payloads, indexes and resource bounds belong to the schema, Rust
readers/writers and their tests. Public query semantics belong to
[backend-contract](../../openspec/specs/backend-contract/spec.md).

## SQLite scale checks

From `backends/core`, run opt-in tests on isolated temporary WAL databases. They
clean up their own files and never open application data:

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

The [product-store](../core/product-store/tests/scale.rs) and
[session-sqlite](../core/session-sqlite/tests/scale.rs) scale suites own fixture
sizes, assertions and supported environment overrides. For isolated measurements:

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

Avoid concurrent benchmarks when comparing latency. Synthetic fixtures can exceed
production admission bounds without changing them. Historical Linux software runs
recorded at `fae154490` on 2026-10-13 passed million-row catalog/sample cases;
detailed measurements are retained locally. Warm temporary-disk results do not
establish phone flash/fsync or cold-cache latency and are not CI timing assertions.

From the repository root, inspect location-pruning plans with Python SQLite:

```sh
python3 backends/sql/tests/scale.py
python3 backends/sql/tests/scale.py --samples 1000000 --locations 20000
```

This secondary audit checks the production pruning statement and rolls back the
measured delete. Rust tests use the application's bundled SQLite and remain the
primary behavior checks.

## Recorded connection context

`connectionContext` events preserve the firmware and route observed at each saved
activation/reconnect. They are independent session events, survive pruning and
archive round trips, and contain no private device identities. Older recordings
remain valid without backfilling context; CSV remains telemetry-only.

Use the [session-connection-context spec](../../openspec/specs/session-connection-context/spec.md)
and reader/writer tests for payload validation, idempotency, ordering and query
bounds. Do not infer earlier connection facts from a later activation.
