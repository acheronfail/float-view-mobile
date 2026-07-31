## ADDED Requirements

### Requirement: Mirrored native session-data adapters
Apple and Android SHALL implement the same authenticated session-data request kinds with small backend-owned adapters over their production SQLite session stores. Each adapter SHALL validate the shared hand-written contract, obtain a committed snapshot, stream requested rows through an equivalent bounded first/minimum/maximum/last reducer, serialize the same columnar response, and apply standard gzip encoding using platform facilities. Swift and Kotlin implementation code SHALL remain owned by their respective backends; shared behavior SHALL be enforced through the contract, SQL schema, immutable aggregation fixtures, mock, and canonical conformance rather than a new cross-language runtime or generated API layer.

#### Scenario: Equivalent historical query
- **WHEN** Apple and Android query equivalent stored rows with the same series request
- **THEN** they return semantically identical segments, elapsed times, extrema, snapshot boundaries, and continuation behavior

#### Scenario: Platform compression
- **WHEN** the frontend accepts gzip from either host
- **THEN** each host uses its platform compression facilities and returns the same decoded contract without adding a shared native runtime dependency

#### Scenario: Backend ownership
- **WHEN** the WebView is destroyed, recreated, or aborts a query
- **THEN** native storage, active logging, snapshot consistency, cancellation, and bounded resource cleanup remain owned by the host backend

### Requirement: Historical reads do not obstruct persistence
Each host SHALL use SQLite snapshot and WAL behavior so a bounded historical read does not hold the application-service or session-writer lock while scanning a long range. Historical work SHALL have bounded concurrency and duration, SHALL release cursors and temporary buffers after completion, cancellation, or failure, and SHALL yield to durable active-session commits.

#### Scenario: Review an active long session
- **WHEN** a historical overview scans an active 150,000-sample session while another sample becomes due for persistence
- **THEN** the durable append remains bounded and the query observes only its committed snapshot boundary

#### Scenario: Superseded zoom request
- **WHEN** repeated pan or pinch interactions cancel an older native query
- **THEN** the old cursor and compression work are released without accumulating background jobs or blocking the newest request

