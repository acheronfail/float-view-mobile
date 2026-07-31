## ADDED Requirements

### Requirement: Canonical session-data conformance

The canonical black-box suite SHALL exercise the same session-data HTTP requests against the mock, Apple, Android, iOS, and macOS targets. It SHALL verify bootstrap discovery, token and Origin authentication, no-store responses, gzip decoding, columnar shapes, field and range minimization, committed boundaries, stable errors, request and decompressed-response byte bounds, concurrency cleanup, and the continued 65,536-byte WebSocket limit without duplicating assertions in target adapters.

#### Scenario: Cross-platform data response

- **WHEN** canonical conformance sends one valid series, samples, or events request to any supported target
- **THEN** the decoded response satisfies the same hand-written contract and immutable expected result

#### Scenario: Unauthenticated HTTP query

- **WHEN** a query has an invalid token or Origin
- **THEN** every target rejects it before session lookup and exposes no private identifier or launch token

#### Scenario: Bounds and cleanup

- **WHEN** conformance exercises oversized, cancelled, concurrent, invalid-range, invalid-field, and stale-boundary requests
- **THEN** every target fails or completes within the declared limits and remains responsive to a later valid request and durable append

### Requirement: Immutable historical aggregation vectors

Conformance SHALL own immutable platform-neutral vectors containing ordered raw samples, null and gap cases, a bounded query, and exact expected first/minimum/maximum/last segments. The frontend mock, Swift reducer, Kotlin reducer, and canonical suite SHALL consume those vectors. Vectors SHALL cover bucket boundaries, extrema order and deduplication, duplicate elapsed times, sequence tie-breaking, sparse and extension fields, finite values, exact-nearest selection, snapshot exclusion, and event cursors.

#### Scenario: Extrema order

- **WHEN** minimum and maximum observations occur in either order inside a bucket
- **THEN** every implementation emits them once in original `(elapsedMs, sequence)` order with the same expected segments

#### Scenario: Snapshot exclusion

- **WHEN** fixture rows exist beyond the request's committed sequence boundaries
- **THEN** every implementation excludes them from series, exact-sample, and event results

### Requirement: Representative long-session performance gate

Verification SHALL include a deterministic 150,000-sample complete-catalog session and a representative supported mobile checkpoint. It SHALL measure from available session metadata to the first useful overview SVG and from settled zoom or pan input to the newest rendered SVG. The overview SHALL complete within one second and settled navigation within 250 milliseconds, with bounded WebView memory, response geometry, native query concurrency, and no raw-sample-proportional SVG output. Timing diagnostics SHALL be concise and retained on failure.

#### Scenario: Mobile overview budget

- **WHEN** the representative mobile target opens the deterministic long session
- **THEN** it renders a useful overview within one second and reports bounded response and SVG point counts

#### Scenario: Mobile zoom budget

- **WHEN** the representative mobile target settles a zoom or pan over that session
- **THEN** the newest requested range renders within 250 milliseconds and superseded requests do not subsequently replace it
