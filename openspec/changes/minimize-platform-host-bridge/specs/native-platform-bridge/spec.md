## ADDED Requirements

### Requirement: One-way minimal native bridge
The private native bridge SHALL carry core-to-host commands through one host-pulled bounded queue and host-to-core observations through focused bounded event submissions. Runtime generation, operation identifier, command kind, flags, and payload length SHALL each have one authoritative representation; a command payload MUST NOT repeat envelope metadata already present in its header. Production hosts SHALL decode commands and submit events but MUST NOT retain general-purpose reverse codecs used only to construct commands or decode their own events.

#### Scenario: Host receives a command
- **WHEN** Rust enqueues a platform command
- **THEN** the off-UI-thread host worker reads one versioned header and only the command-specific bounded payload, dispatches the selected OS operation, and returns one completion for the header operation identifier

#### Scenario: Host submits an observation
- **WHEN** a native adapter observes bounded Bluetooth, location, lifecycle, logging-service, or theme state
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

