## Purpose

Define one shared frontend API server for all current native hosts, with bounded host integration and independent transport lifetime.

## ADDED Requirements

### Requirement: One complete frontend API implementation

All supported native hosts SHALL select one shared Rust implementation of HTTP/WebSocket transport, authentication, request validation, dispatch, result/error serialization, and complete state publication. It SHALL implement every current method, route, request kind, limit, and error in the canonical frontend contract. Hosts MUST NOT retain parallel frontend serializers or method dispatch implementations.

#### Scenario: Same request across hosts

- **WHEN** identical valid requests and domain facts reach Android, iOS, or macOS
- **THEN** the shared implementation returns equivalent canonical results and errors, apart from documented platform facts and opaque values

#### Scenario: Serve the frontend

- **WHEN** a client requests the root document or an unknown content path
- **THEN** the server returns the identical embedded HTML at `/` and returns 404 elsewhere without a general filesystem server

### Requirement: Shared authentication and bounded admission

The shared server SHALL bind only to loopback, preserve assigned ports, and use one cryptographically random token per backend launch. It SHALL preserve bootstrap fields and existing authentication for WebSocket, session-data, import, and export routes. Authentication SHALL precede body parsing and storage lookup. Encoded input, decoded output, clients, queues, transfers, workers, and shutdown waits MUST have explicit tested bounds. Diagnostics MUST NOT disclose tokens, addresses, private identities, raw telemetry, or archive content.

#### Scenario: Reject an unauthorized request

- **WHEN** a protected request has an absent, incorrect, or stale token or an incorrect Origin
- **THEN** the server rejects it before body parsing or storage lookup without disclosing session existence

#### Scenario: Stream an oversized request

- **WHEN** a fragmented message or streamed body exceeds its bound, including without Content-Length
- **THEN** the server stops reading and rejects it without an unbounded allocation or application operation

#### Scenario: Assigned port is occupied

- **WHEN** the server cannot bind its assigned loopback port
- **THEN** startup fails with a bounded diagnostic and does not silently select another port

### Requirement: Bounded asynchronous host operations

Native OS and catalog operations SHALL use a versioned, closed, callback-free exchange of bounded operations, typed facts, and completions. Operations SHALL carry lifetime and correlation identity. Host work MUST execute without a shared transport lock or a blocked UI executor. Each accepted request SHALL receive exactly one terminal result unless its client is cancelled. The shared server MUST NOT create a second application runtime or bypass existing mutation ordering.

#### Scenario: Complete an OS operation

- **WHEN** an authorized request needs permission presentation, Bluetooth, native file presentation, or catalog work
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

The shared publisher SHALL construct complete frontend snapshots from ordered host facts and committed runtime state. It SHALL advance the state revision once per accepted application update and advance catalog revision only for catalog-visible changes. Publication MUST remain independent of client count. Each client SHALL retain only its newest pending state snapshot, with separately bounded correlated responses.

#### Scenario: Multiple or slow clients

- **WHEN** multiple clients subscribe and one consumes state slowly
- **THEN** each receives non-regressing complete snapshots while the slow client cannot delay persistence or grow an unbounded queue

#### Scenario: Publish saved and guest telemetry

- **WHEN** a saved sample commits or a guest sample qualifies for live publication
- **THEN** saved state reflects only committed data and guest state creates no session or catalog entry

#### Scenario: Reject stale facts

- **WHEN** host facts arrive after newer facts or after their server generation ends
- **THEN** they cannot replace current state or advance revisions

### Requirement: Transport lifetime preserves application work

Listener stop, client cancellation, and disposable WebView replacement MUST NOT destroy the application runtime, disconnect an active board, or stop permitted background recording. Transport stop SHALL cancel its waiters and transfers, release sockets, and terminate its workers within a tested bound. A listener restart within the same backend launch SHALL preserve the publisher's state, revisions, and launch authentication context.

#### Scenario: Recreate frontend during a ride

- **WHEN** the frontend disconnects or is recreated during saved recording
- **THEN** durable session progress continues and the replacement receives current complete state

#### Scenario: Stop and restart the listener

- **WHEN** native lifecycle policy stops and later starts only the listener
- **THEN** recording remains process-owned, pending transport work ends, and new clients recover without stale completion effects

#### Scenario: Stop concurrently with host work

- **WHEN** stop races with a publication, read, permission result, or transfer completion
- **THEN** shutdown remains bounded and later completion cannot access a destroyed server or mutate its replacement

### Requirement: Historical and archive work remains bounded

Shared transport SHALL preserve published-boundary reads, compression, read interruption, archive validation, and native export presentation. Historical work SHALL retain at most two concurrent reads and SHALL not block the recording executor. Large archives SHALL cross host boundaries as bounded transfers rather than oversized control envelopes. Hosts SHALL own temporary storage and archive validation, and rejected or abandoned transfers SHALL release their resources.

#### Scenario: Cancel a history response

- **WHEN** a client disconnects during a historical query, serialization, or compression
- **THEN** the backend interrupts the read and releases its lease and buffers without blocking live telemetry

#### Scenario: Transfer a valid large archive

- **WHEN** an authorized archive exceeds a host control envelope but remains within the public import bound
- **THEN** a bounded transfer completes through the existing archive validator without changing identity or storage semantics

#### Scenario: Abandon an archive

- **WHEN** an archive is oversized, malformed, cancelled, or interrupted by server stop
- **THEN** partial resources are removed and no successful import or export is reported

### Requirement: Both host migrations remove old server ownership

The completed change SHALL select the shared server on Android, iOS, and macOS. Each host migration SHALL delete its superseded routes, serializers, auth/compression helpers, and unused server dependencies. There SHALL be one native binding artifact per host and no permanent native server fallback. Source and dependency audits SHALL prove the deletion inventory and a net decrease in combined production source while retaining existing convergence limits.

#### Scenario: Inspect final production selection

- **WHEN** source, dependencies, symbols, and packaged native artifacts are audited
- **THEN** both host families select the shared server through their existing binding families without native API dispatch or obsolete server libraries
