# Durable storage source of truth

`catalog.sql` owns saved boards, profiles, settings, immutable session summaries,
active job recovery metadata, and reversible session groups. Each session has a
separate database initialized with `session.sql`.

The shared Rust store and session writer execute these files verbatim, enable foreign keys and
WAL, use bounded transactions, update `commit_marker` in the same transaction as
sample batches, and fsync at ride-state transitions and close. Raw voltage is
canonical; corrections are never written over it.

Recovery treats catalog rows with an active state as interrupted, validates the
session database with `PRAGMA quick_check`, truncates catalog counts to the last
commit marker, and records a recovery event. No recovery path auto-deletes data.

There are deliberately no migrations while the application is in test
development. The shared product store keeps storage marker `12` beside the
databases. Missing or incompatible markers reset only validated, app-owned files.
Corruption, protected storage and ordinary operational failures never reset data.
Update the shared store marker whenever either SQL schema changes.

`refloat_configuration` stores bounded, complete application-wide Tunes. Exact
package version, schema fingerprint, and wire signature are part of the record
identity and query index, so releases never migrate or fall back by INFO version
or signature alone. Values JSON is validated against the active audited schema
before persistence and again before apply; optimistic record revisions prevent
lost updates. Raw schemas and device-current snapshots are not stored in this
table. Shared schema cache files are separate, bounded, digest-verified, and
disposable.
