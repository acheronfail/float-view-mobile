## MODIFIED Requirements

### Requirement: Disposable current storage schema

The native backends SHALL execute the shared canonical first-production SQLite schemas, retain WAL recovery and integrity handling, and reset only recognised pre-baseline owned development storage at baseline 100, preserve supported baseline data, and reject unknown/newer/malformed/missing-on-nonempty markers without deletion. The session schema SHALL use dedicated nullable columns for every stable numeric telemetry field, bounded JSON only for negotiated numeric extensions, bounded write-time numeric-field presence metadata, chronological transition rows linked to source samples, elapsed indexes, incremental auto-vacuum, session-captured riding/idle persistence intervals, history/prune metadata, and one authoritative commit marker containing committed sample/event boundaries, recorded/stored counts, history revision, elapsed coverage, and the bounded riding/distance/maximum/source-sequence/last-value projection needed to recover catalog summaries. The catalog SHALL contain bounded persisted prune-job progress. Denormalized catalog metadata SHALL be reconciled from the authoritative session database after interruption or an in-process catalog failure before that session is published or appended again.

#### Scenario: Schema mismatch

- **WHEN** the store encounters a recognised pre-baseline marker and validated owned layout
- **THEN** the backend resets only validated app-owned development files to baseline 100 once, preserving unrelated exports; supported baseline data and unsupported storage are never erased by this path

#### Scenario: Interrupted session recovery

- **WHEN** the backend starts with an interrupted active session whose session marker is newer than its catalog row
- **THEN** it performs bounded integrity recovery, reconciles published boundaries and counts from the session database, and only then exposes the recovered result

#### Scenario: Catalog update fails after session commit

- **WHEN** a session transaction commits but its denormalized catalog update fails in the running process
- **THEN** the backend marks that session dirty, publishes no newer boundary, and reconciles from the marker before accepting another append or metadata read

#### Scenario: Late extension field

- **WHEN** a valid extension field first appears after the first 2,000 committed samples
- **THEN** snapshot metadata includes it through write-time field presence without scanning or sampling the ride
