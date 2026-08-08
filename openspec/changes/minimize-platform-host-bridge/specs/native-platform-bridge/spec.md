## ADDED Requirements

### Requirement: One-way minimal native bridge

The private native bridge SHALL carry core-to-host commands through one host-pulled bounded queue and host-to-core observations through focused bounded event submissions. Runtime generation, operation identifier, command kind, flags, and payload length SHALL each have one authoritative representation; a command payload MUST NOT repeat envelope metadata already present in its header. Production hosts SHALL decode commands and submit events but MUST NOT retain general-purpose reverse codecs used only to construct commands or decode their own events.

#### Scenario: Host receives a command

- **WHEN** Rust enqueues a platform command
- **THEN** the off-UI-thread host worker reads one versioned header and only the command-specific bounded payload, dispatches the selected OS operation, and returns one completion for the header operation identifier

#### Scenario: Host submits an observation

- **WHEN** a native adapter observes bounded Bluetooth, location authorization or coordinates, lifecycle or effective background eligibility, or a required native effective-appearance change
- **THEN** it copies the required primitive values or bytes through the focused event entry point without constructing a frontend contract model or a second general-purpose envelope

#### Scenario: Insufficient caller buffer

- **WHEN** a waiting host supplies a payload buffer smaller than the queued command
- **THEN** Rust reports the required bounded capacity without dequeuing the command and the worker retries without data loss or reordering

### Requirement: Narrow reachable command vocabulary

Every bridge command kind SHALL have a reachable production requester and one explicit native OS responsibility. Portable clocks, secure randomness, application coordination, connection policy, protocol behavior, and filesystem work that Rust can perform uniformly SHALL remain in Rust. Unreachable command variants, unused resource namespaces, duplicate envelope fields, compatibility decoders, and host models SHALL be deleted rather than retained speculatively.

#### Scenario: Bridge surface audit

- **WHEN** verification enumerates exported bridge functions, command kinds, event kinds, and host dispatch branches
- **THEN** every item maps to a production call path and an allowed platform responsibility, with unused or Rust-portable items absent

#### Scenario: New platform request

- **WHEN** future work needs another host capability
- **THEN** its design demonstrates why an existing startup input, command, completion, or focused event cannot represent the OS operation before the private ABI is expanded

### Requirement: Rust-orchestrated native document operations

Rust SHALL own document-operation identifiers, staging paths beneath startup-approved roots, byte and content-type bounds, archive generation and validation, session import semantics, operation state, and cleanup. Native hosts SHALL implement reachable bounded import and export presentation commands and SHALL own only platform picker, save, share, security-scoped URL, content-URI, and bounded-copy behavior. A host MUST NOT interpret archive contents or select an unapproved staging path.

#### Scenario: Successful native import

- **WHEN** the frontend initiates a native import and the user selects an allowed document within the configured bound
- **THEN** Rust allocates the destination, the host copies the selected content there in bounded chunks, Rust validates and imports it, and the monotonic state reports the resulting session identifier

#### Scenario: Cancelled native import

- **WHEN** the user cancels the native import surface or its presentation owner disappears
- **THEN** the operation completes exactly once as cancelled, Rust removes any partial staging file, and no session is imported

#### Scenario: Successful native export

- **WHEN** the frontend initiates an export and Rust produces a bounded staged file
- **THEN** the matching host presents its native save or share surface using the supplied display name and content type and reports the presentation result without interpreting the export

#### Scenario: Rejected document path

- **WHEN** a document command contains a path outside the startup-approved staging root or a path that fails canonical containment validation
- **THEN** the host rejects the command before reading, writing, sharing, or presenting the path and Rust reports a bounded failure

#### Scenario: Oversized imported document

- **WHEN** selected content exceeds the Rust-supplied maximum import bytes
- **THEN** the host stops copying, removes the partial output, completes with failure, and Rust imports nothing

#### Scenario: Presentation replacement during a document operation

- **WHEN** an Activity, window, or WebView is replaced while a native document operation is pending
- **THEN** the process-owned Rust operation remains authoritative and the host either rebinds the presentation result or cancels it exactly once without losing runtime or durable state

#### Scenario: Document staging cleanup

- **WHEN** a document operation completes, fails, is cancelled, or exceeds its bounded expiry
- **THEN** Rust removes every staging file no longer required by an active OS presentation and publishes the terminal operation state

### Requirement: One effective-appearance source per host

Each host SHALL use exactly one verified source for effective system light or dark appearance. A host whose WebView reliably supplies initial and changed `prefers-color-scheme` state MUST use that frontend source and MUST NOT submit duplicate native appearance events. A host whose WebView cannot do so SHALL submit one focused effective-appearance observation and SHALL disable the duplicate frontend subscription for that host. Persisted theme preference MUST remain separate from effective system appearance.

#### Scenario: WebView supplies effective appearance

- **WHEN** physical verification proves the host WebView reports initial and changed system appearance across foreground return and WebView recreation
- **THEN** the frontend media-query path is the sole effective-appearance source and the native bridge exposes no duplicate observation for that host

#### Scenario: Native appearance fallback

- **WHEN** a supported host WebView fails the required appearance verification
- **THEN** the host submits only bounded `light` or `dark` observations and the frontend resolves persisted preference against that single native source

### Requirement: Background state is represented once

Rust SHALL request platform background execution through a reachable host command and SHALL learn effective background eligibility through lifecycle observations. Logging-service activity MUST NOT remain a separate event when it has no production Rust consumer and command completion plus lifecycle eligibility fully represent the state.

#### Scenario: Background execution becomes eligible

- **WHEN** a host accepts a background-execution request and its OS mechanism becomes active
- **THEN** the host reports effective eligibility through the next ordered lifecycle observation without submitting a redundant logging-service event

### Requirement: Bounded startup configuration

Native hosts SHALL select only platform-owned startup inputs such as storage roots, packaged-resource access, requested development port, and physical-versus-software transport mode. Rust SHALL validate and copy those inputs before starting one process-owned runtime and SHALL generate its launch secret using the Rust dependency owner. Startup MUST complete or fail with bounded diagnostics before runtime readiness is presented.

#### Scenario: Physical runtime startup

- **WHEN** a native process starts in physical transport mode
- **THEN** one Rust runtime validates its platform-selected roots and resources, initializes portable storage and server state, and then emits one bounded readiness command

#### Scenario: Invalid startup input

- **WHEN** a root, resource, port, transport mode, or size violates its bound
- **THEN** runtime creation fails before binding the loopback server or starting Bluetooth work

### Requirement: Pull-worker and ownership safety

Each process SHALL retain one Rust runtime and one dedicated blocking command worker outside the UI thread. Rust MUST NOT call Swift or Kotlin, retain a JVM callback or environment, expose allocator ownership, or unwind across C/JNI. Stop SHALL reject new work, close the command queue, wake the waiter, cancel pending operations, join the worker, and destroy the handle only after no host call can use it.

#### Scenario: Activity or WebView replacement

- **WHEN** native presentation is replaced while the process remains alive
- **THEN** the host replaces only presentation routing while the same runtime, command worker, connection state, and durable state continue

#### Scenario: Runtime shutdown

- **WHEN** the process owner stops the runtime while the worker is blocked
- **THEN** the wait returns the stopped result, pending completions become invalid or cancelled, the worker exits, and handle destruction completes without callback reentrancy

### Requirement: Atomically versioned bridge

The C ABI, checked-in generated header, Swift wrapper, JNI facade, Kotlin declarations, packaged Rust libraries, and runtime build identity SHALL change atomically. The bridge is private to matching application builds and SHALL fail early on an ABI or build-identity mismatch rather than retain compatibility shims.

#### Scenario: Mismatched native library

- **WHEN** a host loads a Rust library with another ABI version or build identity
- **THEN** startup fails before runtime creation and reports only a bounded non-sensitive mismatch diagnostic
