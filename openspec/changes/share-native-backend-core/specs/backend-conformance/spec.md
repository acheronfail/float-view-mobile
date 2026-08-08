## ADDED Requirements

### Requirement: Shared-core and host-adapter verification

Verification SHALL separate Rust backend behavior tests, native-ABI tests, and host adapter/packaging tests while retaining the canonical black-box suite as the supported-host gate. A behavior fixture covered by the Rust core MUST NOT be copied into platform host test suites.

#### Scenario: Protocol or storage behavior test

- **WHEN** a portable protocol, state, server, or storage behavior changes
- **THEN** its focused test runs once against the Rust core and canonical conformance verifies the resulting observable behavior on every shipped host

#### Scenario: Native ABI test

- **WHEN** runtime lifecycle, buffers, command transport, operation completion, panic containment, header generation, or JNI mapping changes
- **THEN** focused C-ABI and Android-JNI tests verify ownership, ordering, bounds, version identity, shutdown, and resource release without duplicating application behavior assertions

#### Scenario: Platform adapter test

- **WHEN** Bluetooth, location, lifecycle, permission, resource, document, or presentation mapping changes
- **THEN** the affected host test verifies only platform mapping, ownership, cancellation, thread dispatch, and resource release while shared contract assertions remain in `/conformance`

### Requirement: Portable dependency smoke matrix

The software gate SHALL exercise the Rust backend's HTTP/WebSocket lifecycle, bundled SQLite engine identity and WAL lifecycle, cryptography, compression, XML/archive behavior, Cargo package loading, generated C header, and native ABI on each required Apple and Android target environment. macOS SHALL run native core tests, the C harness, and smoke executables; iOS SHALL run embedded smoke in a simulator application/test bundle; Android SHALL build every packaged ABI and run JNI/instrumentation smoke on an emulator or device. A matrix failure SHALL block shared-backend acceptance and MUST NOT be converted into a skipped canonical scenario.

#### Scenario: Rust loopback server on a target

- **WHEN** a target runs the loopback smoke fixture
- **THEN** it proves authenticated bootstrap, bounded HTTP bodies, WebSocket upgrade and fragmented traffic, header/body/message/buffer/connection/client/queue/time bounds, shutdown, and restart using the production Rust server dependencies

#### Scenario: Shared production composition

- **WHEN** focused runtime smoke, standalone canonical conformance, or a shipped host starts the Rust backend
- **THEN** each path uses the same application actor, bundled stores, contract dispatcher, loopback server, and ordered shutdown composition, with deterministic adapters substituted only for standalone testing

#### Scenario: Bundled SQLite on a target

- **WHEN** a target runs the storage smoke fixture
- **THEN** it verifies the Cargo-locked engine identity/options, WAL write/read, interruption, recovery, and canonical-schema compatibility without loading a system SQLite implementation

#### Scenario: Native ABI on a target

- **WHEN** a target runs the C-ABI smoke fixture
- **THEN** it compiles the checked-in header and verifies layout, version/build identity, caller-buffer ownership, insufficient-capacity behavior, panic containment, stop/wake, and stale-handle rejection

### Requirement: Legacy backend cutover gates

The Swift and Kotlin backend implementations MUST remain available only as mutually exclusive bounded migration variants and MUST NOT be deleted until their Rust-backed hosts pass focused core/bridge/adapter tests, the canonical software suite, and the representative physical Bluetooth checkpoints required of those hosts. Legacy and Rust runtimes MUST NOT execute in the same process.

#### Scenario: Apple parity incomplete

- **WHEN** a Rust-backed macOS or iOS variant fails a required contract, C-ABI, lifecycle, persistence, or physical Bluetooth scenario
- **THEN** the corresponding Swift production path remains selectable and the failed Rust slice is not treated as migrated

#### Scenario: Apple parity complete

- **WHEN** Rust-backed macOS and iOS pass every required gate
- **THEN** the Swift application-service, protocol, SQLite, server, serialization, and duplicate behavior-test owners are deleted before Apple migration is complete

#### Scenario: Android parity incomplete

- **WHEN** the Rust-backed Android variant fails a required contract, JNI, lifecycle, persistence, or physical Bluetooth scenario
- **THEN** the Kotlin production path remains selectable and the failed Rust slice is not treated as migrated

#### Scenario: Android parity complete

- **WHEN** the Rust-backed Android variant passes every required gate
- **THEN** the Kotlin application-service, protocol, SQLite, server, serialization, and duplicate behavior-test owners are deleted before the change completes

#### Scenario: JNI lifecycle stress

- **WHEN** Android concurrently submits adapter events, repeatedly recreates the Activity/WebView, reconnects Bluetooth, waits for host commands, cancels operations, kills/relaunches the process, and stops the runtime
- **THEN** verification observes one serialized Rust runtime and one dedicated bridge worker per process, ordered generation-safe events, caller-owned bounded buffers, local-call-scoped JVM references, durable-only recovery after process death, no native-to-JVM callback, and no use of a released handle

#### Scenario: Generated header drift

- **WHEN** CI regenerates the C header from its recorded clean Cargo inputs
- **THEN** the generated header and recorded hash match the checked-in output and every host wrapper reports the matching ABI/build identity

### Requirement: Production-composition regression coverage

Canonical and focused Rust verification SHALL exercise behaviors through the same production composition used by native hosts. Coverage MUST include saved and guest connections, at least two saved boards, deletion with owned history, invalid settings values and ranges, clock-driven telemetry/persistence cadence, nonblocking scan dispatch, Refloat configuration and Tune operations, restart recovery, and the absence of fixed fixture identities or fabricated session results.

#### Scenario: Isolated model test is insufficient

- **WHEN** a typed policy, coordinator, protocol, or storage owner passes its unit tests but the production dispatcher bypasses or rejects it
- **THEN** the migration gate remains incomplete until a production-composition or canonical scenario proves the observable behavior through that owner

#### Scenario: Multi-board and guest matrix

- **WHEN** conformance records equivalent activity for two saved boards and one guest connection
- **THEN** it verifies distinct board-scoped histories, guest durable isolation, stable restart behavior, and deletion of one board without affecting the other

#### Scenario: Timing and dispatch matrix

- **WHEN** deterministic time advances across scan, telemetry, persistence, timeout, reconnect, suspend, and resume
- **THEN** conformance observes no blocking loopback request, immediate telemetry spin, fabricated elapsed time, or persistence outside the configured policy

## MODIFIED Requirements

### Requirement: One canonical suite

The repository SHALL contain one top-level `/conformance` TypeScript black-box suite that imports the hand-written frontend contract and is not copied into the Rust backend, Android host, or Apple hosts.

#### Scenario: Common execution

- **WHEN** conformance runs against mock, Rust-backed Android, Rust-backed iOS, or Rust-backed macOS
- **THEN** the same scenario and assertion implementations are used

### Requirement: Supported-backend gate

Android, iOS, and macOS SHALL NOT be considered supported unless their current host build, Rust backend package, required ABI/adapter/packaging tests, and canonical suite pass.

#### Scenario: Release verification

- **WHEN** a supported platform build is prepared
- **THEN** its Rust-core, native-ABI, host-adapter, packaging, and canonical conformance results are successful
