## MODIFIED Requirements

### Requirement: Bounded asynchronous host operations

Only native OS operations SHALL use a versioned, closed, callback-free exchange of bounded operations, typed facts, and completions. Operations SHALL carry lifetime and correlation identity. Host work MUST execute without a shared transport lock or a blocked UI executor. Each accepted request SHALL receive exactly one terminal result unless its client is cancelled. Portable catalog, product, archive and application operations SHALL execute directly in the shared backend service, without domain-operation round trips through native code. The shared server MUST NOT create a second application runtime or bypass existing mutation ordering.

#### Scenario: Complete an OS operation

- **WHEN** an authorized request needs permission presentation, Bluetooth, or native file presentation
- **THEN** the owning host executes that operation and shared Rust converts its completion into the canonical response

#### Scenario: Late or duplicate completion

- **WHEN** a completion is duplicated, unknown, or belongs to a stopped server generation
- **THEN** it produces no second response, repeated mutation, or state change in the current generation

#### Scenario: Operation admission is full

- **WHEN** a request would exceed the documented queue or operation limit
- **THEN** it fails boundedly before admission without blocking recording or growing the queue

#### Scenario: Client leaves during a mutation

- **WHEN** a client disconnects after an application mutation was accepted
- **THEN** its response waiter is released without retrying or undoing the accepted mutation

### Requirement: One monotonic state publisher

The shared publisher SHALL construct complete frontend snapshots directly from shared service state, ordered OS facts and committed runtime/storage state without native domain snapshot reconstruction. It SHALL advance the state revision once per accepted application update and advance catalog revision only for catalog-visible changes. Publication MUST remain independent of client count. Each client SHALL retain only its newest pending state snapshot, with separately bounded correlated responses.

#### Scenario: Multiple or slow clients

- **WHEN** multiple clients subscribe and one consumes state slowly
- **THEN** each receives non-regressing complete snapshots while the slow client cannot delay persistence or grow an unbounded queue

#### Scenario: Publish saved and guest telemetry

- **WHEN** a saved sample commits or a guest sample qualifies for live publication
- **THEN** saved state reflects only committed data and guest state creates no session or catalog entry

#### Scenario: Reject stale facts

- **WHEN** host facts arrive after newer facts or after their server generation ends
- **THEN** they cannot replace current state or advance revisions

### Requirement: Historical and archive work remains bounded

Shared transport SHALL preserve published-boundary reads, compression, read interruption, archive validation, and native export presentation. Historical work SHALL retain at most two concurrent reads and SHALL not block the recording executor. Large archives SHALL use bounded streams; any required OS file-access boundary SHALL use bounded transfers rather than oversized control envelopes. The shared backend service SHALL own app-private staging, archive validation, import/export encoding and cleanup. Native hosts SHALL retain only authorized external file access and file presentation. Rejected or abandoned transfers SHALL release their resources while active OS presentation leases remain valid.

#### Scenario: Cancel a history response

- **WHEN** a client disconnects during a historical query, serialization, or compression
- **THEN** the backend interrupts the read and releases its lease and buffers without blocking live telemetry

#### Scenario: Transfer a valid large archive

- **WHEN** an authorized archive exceeds a host control envelope but remains within the public import bound
- **THEN** a bounded transfer completes through the existing archive validator without changing identity or storage semantics

#### Scenario: Abandon an archive

- **WHEN** an archive is oversized, malformed, cancelled, or interrupted by server stop
- **THEN** partial resources are removed and no successful import or export is reported
