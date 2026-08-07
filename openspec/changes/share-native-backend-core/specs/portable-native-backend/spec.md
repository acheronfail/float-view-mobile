## ADDED Requirements

### Requirement: One portable backend behavior owner

The repository SHALL provide one shared Rust backend implementation for application coordination, VESC and Refloat protocols, normalized state, durable persistence, session retrieval, exports, authenticated contract dispatch, and loopback HTTP/WebSocket behavior. Every shipped native host MUST execute that implementation rather than a translated or independently maintained equivalent.

#### Scenario: Shared behavior change

- **WHEN** a developer changes protocol parsing, state transition, persistence, or contract-dispatch behavior
- **THEN** the change is implemented and unit-tested once in the Rust backend and is consumed unchanged by every shipped host

#### Scenario: Frontend recreation

- **WHEN** any host destroys and recreates its WebView while native work continues
- **THEN** the replacement frontend reconnects to the same process-owned Rust backend state without recreating or translating backend state in the host

### Requirement: Explicit platform adapter boundary

The portable backend SHALL access Bluetooth, location, lifecycle/background eligibility, permission/settings/theme/service presentation, document selection/staging, export presentation, immutable packaged resources, diagnostics, storage roots, available-space reporting, clocks, and secure randomness only through bounded typed internal ports transported by the native bridge. Resource requests MUST contain validated logical names rather than arbitrary traversal paths. Document requests MUST direct the host to a core-provided bounded staging destination without giving the host archive semantics. Platform adapters MUST NOT parse VESC or Refloat payloads, validate/import archives, own authoritative application state, execute SQL, dispatch frontend methods, or retain unbounded event or command queues.

#### Scenario: Bluetooth notification

- **WHEN** a platform Bluetooth adapter receives a notification
- **THEN** it copies bounded raw bytes plus ordering and generation context into the Rust backend without interpreting the VESC frame

#### Scenario: Stale platform event

- **WHEN** an adapter event or operation completion arrives for a cancelled or replaced runtime or connection generation
- **THEN** the Rust backend rejects it without state publication, persistence, or a frontend-visible event

#### Scenario: Packaged resource lookup

- **WHEN** the core requests a known frontend, SQL, or software-fixture resource by bounded logical name
- **THEN** the host returns the identically packaged bounded bytes without exposing an arbitrary host path or interpreting their application semantics

#### Scenario: Native document import

- **WHEN** a host document picker returns URI-backed or security-scoped content
- **THEN** the host copies it into the core-provided staging destination within the configured byte limit while the core alone validates, imports, publishes state, and emits frontend-visible results

#### Scenario: Concurrent platform events

- **WHEN** platform events arrive concurrently or from an OS-required thread
- **THEN** their copied bounded values enter one process-owned application actor in established adapter order without synchronous reentrancy into mutable core state

### Requirement: Versioned bounded native ABI

The portable backend SHALL expose one private versioned C ABI generated into a checked-in header by pinned `cbindgen`. The ABI MUST contain only build/version identity, opaque runtime lifecycle, bounded event submission, bounded operation completion/cancellation, focused raw-byte submission, and blocking host-command retrieval into caller-owned memory. It MUST use fixed-width C-compatible values and explicit pointer/length pairs; it MUST NOT expose Rust strings, slices, collections, traits, unwinding, allocator ownership, domain models, frontend contract models, snapshots, telemetry histories, or SQL rows. Normal application builds MUST consume the checked-in header without running `cbindgen`.

#### Scenario: Input ownership

- **WHEN** Swift, Kotlin/JNI, or a C harness submits a valid event or completion
- **THEN** the ABI validates and copies every required input before returning and retains no borrowed host pointer

#### Scenario: Output ownership

- **WHEN** a host retrieves a command
- **THEN** the ABI writes only within the caller-owned bounded buffer, reports required capacity without dequeuing when the buffer is insufficient, and leaves no Rust allocation owned by the host

#### Scenario: Panic boundary

- **WHEN** an exported ABI operation encounters a Rust panic
- **THEN** no unwind crosses into host code and the runtime either becomes observably failed with a bounded diagnostic or terminates according to its recorded panic policy

#### Scenario: Header drift

- **WHEN** CI regenerates the C header from the pinned Cargo inputs
- **THEN** it byte-matches the checked-in header and a C compiler verifies the declared layouts and functions on every required target

#### Scenario: ABI mismatch

- **WHEN** a host wrapper loads a core library with a different ABI version or build identity
- **THEN** startup fails before creating application state or starting Bluetooth, storage, or loopback work

### Requirement: Pull-based host command lifecycle

Each process-owned Rust runtime SHALL provide one bounded host-command queue consumed by one dedicated host bridge worker through a blocking wait operation. The wait MUST NOT run on a UI thread and MUST return only a bounded command, timeout, clean-stop, insufficient-capacity, invalid-handle, or bounded failure result. Hosts SHALL dispatch commands to required OS threads and return typed completions or cancellations asynchronously by operation identifier and runtime generation. Rust runtime threads MUST NOT call Swift or Kotlin, synchronously re-enter host code, or retain host callback objects.

#### Scenario: Platform command

- **WHEN** the Rust application actor requires Bluetooth, location, resource, document, permission, lifecycle, or presentation work
- **THEN** it enqueues one bounded command that the dedicated host worker retrieves and dispatches without blocking the application actor or UI thread

#### Scenario: Command completion

- **WHEN** host work completes, fails, or is cancelled
- **THEN** the host submits a bounded result with the matching operation and generation and the Rust actor accepts it at most once

#### Scenario: Runtime shutdown

- **WHEN** a host stops the runtime while its bridge worker is waiting or operations are pending
- **THEN** shutdown closes the command queue, wakes the waiter, cancels pending operations, rejects later completions, and permits handle destruction only after bridge work is detached

#### Scenario: Queue overload

- **WHEN** command count or aggregate command bytes reaches its configured bound
- **THEN** the core applies the specified bounded backpressure or stable failure behavior without allocating an unbounded queue or dropping an acknowledged operation silently

### Requirement: Portable core build matrix

The Rust workspace SHALL compile from one Cargo dependency graph and checked-in lockfile for macOS, iOS, Android, Linux, and Windows using pinned supported toolchains. macOS SHALL run native core tests, the C ABI harness, and portable smoke executables; iOS SHALL run embedded smoke through a simulator application/test bundle; Android SHALL build every packaged ABI and run JNI/instrumentation smoke on an emulator or device; Linux and Windows SHALL run core tests, the C ABI harness, and portable smoke executables in documented reference images. Linux and Windows success MUST NOT by itself advertise an application host as supported.

#### Scenario: Portable dependency regression

- **WHEN** core source or a Cargo dependency no longer builds or runs on one matrix target
- **THEN** the software verification gate fails with the target and first actionable compiler, linker, loader, or runtime diagnostic

#### Scenario: Compile-only future host

- **WHEN** Linux or Windows passes the portable workspace build without a complete host adapter and canonical conformance result
- **THEN** it remains an unsupported product host

### Requirement: Portable authenticated loopback transport

The Rust backend SHALL own one bounded loopback-only HTTP/WebSocket implementation for bootstrap, embedded frontend resources, session-data requests, imports/exports, and authoritative control/event traffic. Every host MUST preserve exact origin and launch-token authentication; header count/byte, body/message, write-buffer, concurrent connection, WebSocket-client, queue, and idle/read-time bounds; conflated state delivery; clean shutdown; and restart behavior.

#### Scenario: Target loopback smoke test

- **WHEN** a target runs the production Rust server smoke suite
- **THEN** bootstrap, bounded HTTP request/response, WebSocket upgrade, fragmented allowed traffic, rejection of invalid authentication or exceeded header/body/message/buffer/connection/client/queue/time limits, shutdown, and restart all complete through loopback

#### Scenario: Server dependency failure

- **WHEN** the selected Rust transport reproducibly cannot satisfy the required smoke suite on a required target
- **THEN** the target remains unsupported until the defect is fixed or a separately reviewed replacement preserves the same internal server boundary and conformance behavior

### Requirement: One bundled SQLite implementation

The Rust backend SHALL use one Cargo-locked `rusqlite` storage implementation with bundled SQLite and one documented engine identity and compile-option set on every target. Builds and runtime verification MUST fail rather than substitute system SQLite, Android `SQLiteDatabase`, an Apple SQLite module, a separately compiled engine, or another ORM. The canonical SQL and stable database/archive semantics MUST remain shared and unchanged except through their owning specifications.

#### Scenario: Fresh repository setup

- **WHEN** a developer or CI job prepares a fresh checkout and builds with locked Cargo resolution
- **THEN** Cargo resolves the recorded bundled SQLite source without a git submodule or mutable developer-local repository

#### Scenario: Database engine identity

- **WHEN** storage verification runs on each target
- **THEN** it reports and validates the pinned SQLite version, source identity, required enabled/disabled compile options, WAL behavior, interruption, checkpoint, and canonical-schema compatibility before accepting the target

#### Scenario: Reviewed upstream update

- **WHEN** maintainers advance `rusqlite`, its SQLite source dependency, or a storage-related feature
- **THEN** the Cargo lockfile, provenance, engine identity/options, license disposition, and storage matrix are reviewed and updated together

#### Scenario: Cross-platform database behavior

- **WHEN** equivalent canonical fixtures are opened, appended, interrupted, recovered, queried, pruned, and exported on different targets
- **THEN** they produce equivalent durable boundaries and contract results using the same Rust store implementation

#### Scenario: Incompatible development storage

- **WHEN** the bundled engine opens an incompatible disposable pre-release storage version
- **THEN** the backend deletes and recreates it from the canonical schema without running a migration

### Requirement: Bounded Android host interoperability

The Android host SHALL call the Rust backend through one small hand-written JNI facade over the native bridge semantics. Kotlin SHALL own one dedicated bridge worker that blocks only in host-command retrieval outside the UI thread. Other JNI entry points MUST validate and copy bounded values, enqueue work, and return. Native methods MUST be explicitly registered during library load. The JNI facade MUST NOT retain a `JNIEnv`, Java/Kotlin callback object, application-class global reference, complete snapshot, telemetry history, SQL row, or frontend contract model; Rust runtime threads MUST NOT attach to the JVM or call Kotlin.

#### Scenario: Android lifecycle recreation

- **WHEN** Android recreates its Activity or WebView while the process-owned backend remains alive
- **THEN** the replacement presentation wrapper and bridge worker attach to the existing runtime, stale operations remain generation-safe, and no second backend runtime is created

#### Scenario: Android bridge teardown

- **WHEN** Android stops the backend or replaces its bridge worker
- **THEN** the blocking JNI wait wakes, pending commands are cancelled, local references have ended with their calls, and no native work later targets the detached worker

#### Scenario: Android process death

- **WHEN** Android kills the application process and later relaunches it
- **THEN** all prior handles, commands, operations, and staging tokens are invalid, exactly one new Rust runtime is created, and only durable SQLite state is recovered

#### Scenario: JNI reference stress

- **WHEN** Android repeatedly submits concurrent Bluetooth events, waits for commands, cancels operations, recreates its Activity/WebView, and starts/stops the runtime
- **THEN** verification observes bounded buffers, local-call-scoped JVM references, one bridge worker and runtime per process, no native-to-JVM callback, and no use of a released handle

### Requirement: Dependency and privacy containment

Core dependencies SHALL use a checked-in Cargo lockfile, be documented, and be limited to maintained crates or vendored source packages that replace substantial duplicated behavior. Rust toolchains, target components, Android cross-compilation tools, generated-header inputs, and any downloaded build artifacts MUST be version-pinned and checksum-verified. Normal application builds MUST use locked resolution, MUST NOT consult mutable developer-local repositories, and MUST NOT require header generation or network access after setup has fetched the locked sources. Required third-party licenses and notices MUST be retained and packaged where applicable. No dependency, C/JNI bridge, or generated header MAY log launch tokens, Bluetooth addresses, platform identifiers, coordinates, full telemetry, SQLite rows, or unbounded payloads by default.

#### Scenario: Dependency audit

- **WHEN** the shared backend migration is complete
- **THEN** every direct dependency has a documented owner, exact locked identity, target support, license/notice disposition, update procedure, and replaced responsibility, and dependencies used only by deleted implementations are absent

#### Scenario: Bridge failure diagnostic

- **WHEN** a platform bridge or portable dependency reports a failure
- **THEN** the diagnostic contains bounded stage and error information without private identifiers or full payload contents
