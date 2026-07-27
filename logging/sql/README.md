# Durable storage source of truth

`catalog-v1.sql` owns saved boards, versioned profiles, settings, immutable
session summaries, active job recovery metadata, and reversible session groups.
Each session has a separate database initialized with `session-v1.sql`.

Both native adapters must execute these files verbatim, enable foreign keys and
WAL, use bounded transactions, update `commit_marker` in the same transaction as
sample batches, and fsync at ride-state transitions and close. Raw voltage is
canonical; corrections are never written over it.

Recovery treats catalog rows with an active state as interrupted, validates the
session database with `PRAGMA quick_check`, truncates catalog counts to the last
commit marker, and records a recovery event. No recovery path auto-deletes data.
