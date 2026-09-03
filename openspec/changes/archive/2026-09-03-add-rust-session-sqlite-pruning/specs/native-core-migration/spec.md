## ADDED Requirements

### Requirement: Safe Rust session SQLite pruning

The Rust session SQLite candidate SHALL preview and atomically apply deterministic pruning to one validated canonical session database by streaming durable source rows directly through the session-history prune kernel. It SHALL remain bounded, race-safe, interruptible, and independent of catalog or host-runtime policy.

#### Scenario: Preview exact retained evidence

- **WHEN** a valid inactive canonical session is previewed at an initial or strictly coarser aligned resolution
- **THEN** the candidate SHALL retain exact kernel-selected endpoints, field/event/summary sources, phase boundaries, and deterministic extrema rows while returning recorded, stored, retained, reclaimable, target-resolution, skipped, and immutable-boundary facts without mutating the database

#### Scenario: Apply one current plan atomically

- **WHEN** a non-skipped preview still matches its database marker and retained-resolution facts at commit time
- **THEN** one immediate transaction SHALL delete only unretained samples and locations outside the retained-resolution neighborhood, preserve recorded counts and published sequence maxima, update stored counts and `m4-source-v1` metadata, increment history revision exactly once, and expose no partial state

#### Scenario: Preserve a no-op database

- **WHEN** every stored sample is selected by a still-current preview
- **THEN** commit SHALL report `already_at_or_below_target` without changing rows, prune metadata, counts, commit time, or history revision

#### Scenario: Reject stale or invalid pruning

- **WHEN** target bounds/alignment, schema/data/order/count bounds, database identity, marker facts, or retained resolution are invalid or advance between preview and commit
- **THEN** the candidate SHALL fail closed with a stable payload-free error before durable mutation and SHALL never silently recompute a different plan

#### Scenario: Roll back failure and interruption

- **WHEN** cancellation, lock contention, SQLite failure, or an injected fault occurs during preview or mutation
- **THEN** the candidate SHALL promptly release every statement and transaction, preserve the prior durable database exactly, remain reusable where compatible, and reveal no path, SQL, telemetry, coordinate, event, JSON, or SQLite-message payload

### Requirement: Session SQLite pruning remains below runtime policy

The Rust session SQLite pruning API SHALL expose only safe owned plan, result, timestamp, boundary, error, and interruption values. It MUST NOT own catalog SQLite, active-session or read-lease exclusion, maintenance jobs, checkpoint/vacuum scheduling, imports/exports, deletion/retention, recovery, local serving, cursors/envelopes, BLE, location acquisition, callbacks, foreign handles, threads, async runtimes, OS frameworks, or frontend code.

#### Scenario: Compose future persistence directly

- **WHEN** a later Rust catalog/application runtime previews or applies session pruning
- **THEN** it SHALL call the session SQLite and history-kernel APIs directly without a pruning Apple FFI or Android JNI boundary

#### Scenario: Preserve current production ownership

- **WHEN** the pruning candidate is built and tested before runtime cutover
- **THEN** selected native stores, activity/read-lease guards, catalog reconciliation, maintenance/reclamation policy, loopback handlers, files, and application publication SHALL remain unchanged

