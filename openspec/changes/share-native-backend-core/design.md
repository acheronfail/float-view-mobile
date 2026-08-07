## Context

Float View currently has about 15,600 lines of production Swift and 14,500 lines of production Kotlin. Component-level comparison found approximately 27,900 lines in 31 mirrored components, while the genuinely platform-specific Bluetooth and location adapters are each only hundreds of lines. The wire contract and canonical conformance suite already enforce common behavior, but every protocol, persistence, application-service, and loopback change is still implemented twice.

The existing Swift and Kotlin implementations are both extraction references. Neither is the specification: the hand-written frontend contract, OpenSpec requirements, canonical SQL, immutable fixtures, canonical conformance, and physical BLE checkpoints are authoritative when the implementations disagree. The extensive dual implementations and test suites make a behavior-preserving rewrite feasible, while the availability of parallel implementation agents reduces mechanical translation cost without reducing the need for one integration owner at concurrency, lifecycle, ABI, and cutover boundaries.

The frontend remains disposable and talks only through the authenticated loopback contract. Native application processes continue to own Bluetooth, lifecycle, background work, logging, persistence, permissions, imports/exports, and complete authoritative state. Rust owns the portable implementation of that state and behavior, while Swift and Kotlin retain only OS integration and presentation.

Rust has mature compilation targets for Apple, Android, Linux, and Windows, a single Cargo dependency/build model, bundled SQLite through `rusqlite`, and maintained async HTTP/WebSocket components. A Rust core introduces an FFI boundary on Apple and Android, so that boundary must be smaller and more auditable than a generated object model. Float View can make it small because the core owns the loopback server: complete application snapshots, frontend requests/results, telemetry history, and SQL rows never need to cross FFI.

## Goals / Non-Goals

**Goals:**

- Make protocol and application behavior have one Rust implementation used by iOS, macOS, and Android.
- Reduce each host to platform API adapters, lifecycle/presentation wiring, WebView setup, and a narrow versioned bridge.
- Use one explicit C ABI with opaque handles, fixed-width values, bounded caller-owned buffers, and a host-pulled command queue; add only a small Android JNI facade over those semantics.
- Define resource, document-import, diagnostics, threading, ordering, cancellation, buffer, panic, and process-lifetime boundaries precisely enough that hosts cannot regain application behavior.
- Preserve all contract behavior, privacy boundaries, bounds, persistence semantics, and physical BLE paths.
- Prove the core and bridge on Android and Apple, and compile/run portable smoke tests on Linux and Windows before calling the core portable.
- Use one Cargo-locked dependency graph, one bundled SQLite engine/configuration, and one Rust loopback implementation on every core target.
- Delete superseded Swift and Kotlin backend behavior and their obsolete dependencies after each host passes its complete gate.

**Non-Goals:**

- Delivering Linux or Windows application shells, Bluetooth adapters, or installers.
- Replacing the SvelteKit frontend or generating its hand-written backend contract.
- Exposing Rust domain models or application snapshots through FFI.
- Using UniFFI, JNA, a permanent C++ compatibility layer, or native-to-host callbacks.
- Introducing an ORM, database migration system, or service process outside the application.
- Preserving the internal layout of either legacy backend when a smaller ownership-first Rust layout replaces it.

## Decisions

### 1. Use a Rust workspace as the single backend owner

Create `backends/core` as one Cargo workspace. Focused crates own application coordination, domain models, VESC and Refloat protocols, connection scheduling, software fixtures, SQLite stores, archive/session-data behavior, loopback routing, the public C ABI, and the Android JNI facade. The application actor depends downward on platform-port models, storage, protocol components, and transport; protocol components remain unaware of Bluetooth implementations, storage, views, HTTP, and host interoperability.

```text
Swift/Kotlin platform host
          |
  C ABI / Android JNI facade
          |
  bounded event + command bridge
          |
     application actor
    /        |          \
port model  storage   loopback transport
               |          |
      bundled SQLite   Tokio + Axum
```

Rust was selected over portable Swift because it gives the core one established build/dependency model across all target families, bundled SQLite without a build-critical submodule, maintained cross-platform async/networking components, and a conventional C ABI for future hosts. This accepts a one-time rewrite and an FFI boundary on Apple as well as Android. Existing dual implementations, immutable vectors, canonical conformance, disposable databases, and parallel implementation capacity make that trade preferable to permanently depending on the Swift Android SDK, `swift-java`, and HTTP dependencies with experimental Android/Windows support.

Alternatives considered:

- **Portable Swift:** reuses more Apple source initially, but makes Android packaging, generated JNI, runtime linkage, SQLite amalgamation ownership, and experimental server support permanent parts of the normal build.
- **UniFFI:** supplies richer Swift/Kotlin bindings, but generates a much broader object/lifetime/async surface than Float View needs. Its value is limited when the core owns the JSON server and the host boundary can be a few lifecycle and byte-oriented operations.
- **C++ based on VESC Tool:** provides upstream familiarity but still requires reconstructing Float View's application, storage, contract, and concurrency behavior without Rust's Cargo ecosystem.
- **Kotlin Multiplatform or continued mirrored implementations:** either shifts the cross-language problem to Apple or fails the maintenance and additional-host objective.

### 2. Preserve explicit platform ports behind a bounded message bridge

The core defines internal typed capabilities for:

- Bluetooth scan/connect/disconnect/GATT stream and authorization state
- location authorization, updates, and settings presentation
- lifecycle/background eligibility and logging-service state
- storage roots and available-space reporting
- immutable packaged-resource lookup for frontend, SQL, and software-fixture bytes
- permission, document-import, native export/settings, theme, and service-state presentation
- bounded privacy-safe diagnostics
- monotonic/wall clocks and secure random bytes where deterministic tests need injection

Hosts do not implement Rust traits directly across FFI. Instead, the ABI transports a small versioned vocabulary of bounded platform events, commands, and operation completions. Hot byte paths such as BLE notifications use focused entry points so bytes remain raw rather than being base64-encoded or translated into a generated object graph. Lower-frequency core-to-host work uses a fixed-width command header containing ABI version, kind, flags, runtime generation, operation identifier, and payload length followed by a bounded length-delimited payload. Host bridge code maps that private vocabulary to local typed adapters; it never interprets VESC frames, validates archives, executes SQL, constructs frontend contract results, or owns authoritative state.

Resource adapters accept validated logical names rather than arbitrary traversal paths. For native document selection, the core creates a bounded staging destination under its selected storage root, the host copies URI-backed or security-scoped content without interpreting the archive, and the core verifies size and content before import, state publication, and frontend events.

Every platform callback copies bounded values into the bridge before returning. An adapter assigns order before concurrently observed events enter the core. Runtime and connection generations reject stale work. Diagnostics contain only bounded redacted stages and error codes.

### 3. Use a host-pulled command queue and prohibit native-to-host callbacks

Each process owns exactly one Rust runtime and one bounded host-command queue. One dedicated host bridge worker calls a blocking `wait_command` operation with a caller-owned bounded buffer. The call returns a command, timeout, clean-stop result, required-capacity result without dequeueing, invalid-handle result, or bounded internal failure. It never runs on the UI thread. The host dispatches OS work to the required queue and later returns a typed completion or cancellation using the operation identifier and runtime generation.

This pull model prevents Rust runtime threads from calling Swift or Kotlin synchronously, removes callback vtables and retained JVM callback objects, and makes mutable-state reentrancy impossible at the ABI boundary. Shutdown closes the command queue, wakes every blocked waiter, cancels pending operations, and only then permits the opaque runtime handle to be destroyed. A host may replace its Activity, WebView, or presentation adapter without creating another Rust runtime while the process lives.

All mutable application, connection, storage-coordination, and publication state is serialized through one process-owned Rust application actor. Tokio server and storage tasks communicate with it through bounded channels and request identifiers. Blocking SQLite work runs on dedicated bounded storage workers; it does not block the application actor or loopback runtime. The bridge, server, and storage queues each have explicit item and byte bounds plus tested overload behavior.

### 4. Generate one conservative C header with pinned `cbindgen`

The public native ABI exposes only:

- an ABI version and build identity query;
- opaque runtime creation/start/stop/destruction;
- bounded event submission and operation completion;
- focused raw-byte submission for high-frequency Bluetooth traffic; and
- blocking retrieval of the next bounded host command into caller-owned memory.

Exported functions use `extern "C"`, fixed-width integers, `repr(C)` plain data, explicit pointer/length pairs, and integer status codes. They expose no Rust strings, slices, collections, trait objects, unwinding, allocator ownership, or layout without a stable C representation. Inputs are validated and copied before return. Output is written only to caller-owned buffers. Every exported function prevents panic unwinding across FFI: a caught panic poisons the runtime and maps to a stable fatal result, while any configured abort remains process-fatal. Rust allocations are destroyed only by Rust; host allocations are destroyed only by the host.

Pinned `cbindgen` generates a checked-in C header from this intentionally small subset. `just generate core-header` reproduces it from the Cargo-locked tool version; CI fails on a diff. Normal Xcode, Gradle, and application builds consume the checked-in header and do not run the generator. A pure-C compile/runtime harness validates header syntax, structure sizes/alignments, version identity, ownership, insufficient-buffer behavior, stop/wake, panic containment, and stale-handle rejection on the target matrix.

The ABI is private to atomically shipped Float View hosts, so it does not carry long-term compatibility shims or negotiate feature sets. Build and runtime version/hash checks fail early if a host wrapper and native library are accidentally mismatched.

### 5. Keep Apple and Android wrappers deliberately hand-written

Apple packages the Rust static libraries and generated header/module map as one local native product for macOS, iOS device, and simulator architectures. A small Swift wrapper owns the opaque handle, runs the blocking command pump on one dedicated dispatch queue, maps bounded commands to existing CoreBluetooth/Core Location/UIKit/AppKit adapters, and returns completions. Swift `Data` and strings are copied at the boundary and never retain Rust memory.

Android's one Gradle application builds the Rust `cdylib` once per packaged ABI using the pinned Android NDK and Cargo cross-compilation tooling. Kotlin exposes a small fixed set of `external` methods. A hand-written Rust JNI facade registers them explicitly from `JNI_OnLoad`, validates the ABI identity, and delegates to the same runtime bridge semantics. The dedicated Kotlin bridge worker is the only long-running JNI call and blocks in `waitCommand`; other calls copy bounded primitive/string/byte inputs, enqueue work, and return. The facade retains no `JNIEnv`, Java/Kotlin callback object, class-loader-dependent application class, or global reference after a call. Rust never attaches an arbitrary runtime thread to the JVM and never calls back into Kotlin.

Activity/WebView recreation replaces only host presentation state and reattaches the bridge worker to the process-owned runtime. Process death invalidates every opaque handle, command, operation, and staging token; relaunch creates exactly one new runtime and recovers only durable SQLite state.

### 6. Use Tokio and Axum for the bounded loopback transport

The Rust core owns HTTP/1.1 bootstrap, embedded frontend resources, session-data and import/export routes, and one WebSocket endpoint using Tokio and Axum. Existing authentication, exact origin and launch-token validation, body/message limits, state conflation, cancellation, and lifecycle restart semantics remain unchanged.

Acceptance is based on Float View's actual surface. Every runtime target smoke test binds only to loopback, serves bootstrap and bounded HTTP bodies, upgrades a WebSocket, exchanges fragmented and maximum-sized allowed messages, rejects invalid origin/token/oversized input, enforces bounded header count/bytes, concurrent connections, WebSocket clients, write buffers, pending response/event queues, and idle/read timeouts, closes cleanly, and restarts after lifecycle suspension or explicit stop.

Framework defaults are never treated as product bounds. The core configures or wraps each required limit explicitly and adds an outer connection/request accounting layer where the selected stack does not expose one directly. A replacement transport requires a separate design only if a reproducible target failure cannot be fixed within this boundary.

### 7. Use `rusqlite` with one bundled SQLite engine

The core uses `rusqlite` and its bundled SQLite path so every host executes one Rust storage implementation and one engine selected by the Cargo lockfile. The build records SQLite version, source ID, enabled/disabled compile options, and required thread-safety, extension-loading, double-quoted-string, target-library, WAL, interrupt, and checkpoint behavior. Verification fails rather than falling back to system SQLite, Android `SQLiteDatabase`, or an Apple SQLite module when identity or options differ.

The canonical SQL and stable SQLite file format remain unchanged. The Rust store preserves serialized writes, dedicated cancellable WAL readers, `sqlite3_interrupt` behavior through the safe wrapper's interrupt owner, recovery, pruning, archives, and exports. Because data remains disposable test data, an incompatible storage version is deleted and recreated rather than migrated.

If the selected bundled feature cannot express a required compile option reproducibly on every target, implementation pauses for a focused dependency/build decision; it does not add a second SQLite engine or silently weaken the option set.

### 8. Keep the Rust dependency graph small and auditable

The intended direct responsibility owners are:

- Tokio and Axum for runtime, HTTP, and WebSocket behavior;
- `rusqlite` plus its bundled SQLite dependency for storage;
- `serde`/`serde_json` for the existing hand-written JSON contract mapping;
- focused maintained crates for SHA-256, secure randomness, gzip/zlib-compatible compression, XML, CRC, and archive handling where the standard library has no equivalent;
- pinned `cbindgen` as generation-only tooling; and
- one Android-only JNI crate for raw JNI safety and registration.

Every direct dependency must replace substantial custom or duplicated code, use a reviewed exact Cargo resolution, and have documented ownership, license/notice disposition, target support, and update procedure. `Cargo.lock` is committed; CI and normal builds use locked resolution, and clean/offline verification proves that no mutable developer-local repository is consulted. `cbindgen` output and any packaging metadata are checked in and reproducible, but Rust libraries and platform binaries are built from source rather than committed.

### 9. Migrate in verified vertical slices

1. Pin the Rust/NDK/Cargo packaging toolchains and prove a minimal runtime, C header, C harness, Swift import, Android JNI worker, SQLite, HTTP/WebSocket, crypto, compression, XML, and archive surface on the target matrix.
2. Freeze the ABI invariants and host-command vocabulary before parallel behavioral porting begins. One integration owner controls runtime, bridge, and cross-crate contracts; parallel work owns non-overlapping protocol, storage, server, and fixture modules.
3. Port protocol/domain code and immutable vectors into focused Rust crates without host behavior changes.
4. Port storage, loopback, contract dispatch, and application coordination behind the Rust actor and typed internal ports; first run the complete software backend through deterministic adapters without platform FFI.
5. Integrate macOS and iOS through the C ABI while retaining the existing Swift backend as a mutually exclusive build path. Delete the Swift backend behavior only after both Apple hosts pass focused, canonical, lifecycle, and physical gates.
6. Integrate Android through the JNI facade and Kotlin adapters while retaining the existing Kotlin backend as a mutually exclusive build path. Delete the Kotlin backend behavior only after the Rust-backed Android host passes focused, canonical, lifecycle, and physical gates.
7. Add Linux and Windows core compile/smoke jobs; no host is advertised as supported by compile-only success.

Every slice remains buildable. Legacy and Rust runtimes are never active in the same process, and the frontend does not gain a compatibility branch. Once a legacy backend is deleted, rollback is by reverting the coherent cutover commit rather than maintaining a permanent second implementation.

### 10. Verify at four levels

- **Core tests:** protocol vectors, state machines, storage lifecycle, server security/bounds, and deterministic software adapters run once against Rust.
- **ABI tests:** the pure-C harness, header drift, panic/ownership/buffer tests, Swift wrapper tests, and JNI worker stress verify the narrow bridge independently of application behavior.
- **Host adapter tests:** platform permissions, BLE event/command mapping, lifecycle, presentation, resource/document staging, native packaging, and queue teardown are tested per platform.
- **Canonical conformance:** mock, macOS, iOS, and Rust-backed Android run the existing black-box contract suite; physical checkpoints continue through real OS Bluetooth stacks.

| Target | Required evidence |
| --- | --- |
| macOS | Cargo build/core tests, C ABI harness, Rust dependency/server/storage smoke, Swift host tests, canonical conformance |
| iOS simulator | static-library/package build, C ABI and embedded dependency/server/storage smoke, lifecycle tests, canonical conformance |
| Android emulator/device | every packaged ABI build, APK load, JNI worker and dependency/server/storage smoke, lifecycle tests, canonical conformance |
| Linux reference image | Cargo build/core tests plus C ABI and portable dependency/server/storage smoke; no host support claim |
| Windows reference image | Cargo build/core tests plus C ABI and portable dependency/server/storage smoke; no host support claim |

`just verify` remains the full software gate and orchestrates or reports all required jobs. A local environment may delegate unavailable reference-image jobs to CI, but the change cannot complete until every required result is successful and attributable to the pinned source/toolchain revision.

## Risks / Trade-offs

- **Rewriting working behavior can introduce subtle parity faults** -> port in bounded vertical slices, use both legacy implementations as diagnostic references, and require immutable vectors plus canonical conformance before each cutover.
- **A C ABI can turn ownership mistakes into memory corruption** -> keep the surface opaque and caller-buffer-based, prohibit borrowed memory after return and unwinding, generate one header, and stress it through C/Swift/JNI harnesses with sanitizers where supported.
- **A private command vocabulary can become a second application protocol** -> limit it to platform ports, keep application/frontend models in Rust, version it atomically, and reject host-side semantic processing in review and tests.
- **A blocking command pump can hang teardown** -> make stop close the bounded queue and wake waiters, use one dedicated host worker, and stress suspension, recreation, cancellation, process death, insufficient buffers, and repeated start/stop.
- **JNI remains cumbersome even without callbacks** -> expose only a fixed method set, explicitly register it, keep all references local to calls, never attach Rust threads to the JVM, and verify with Android lifecycle/reference stress.
- **`cbindgen` development and feature support are ad hoc** -> use only conservative C constructs, pin the tool, check in the header, compile it as C on every target, and keep normal application builds independent of generation.
- **Rust async networking can still have mobile lifecycle defects** -> own the runtime explicitly, test the exact loopback surface on device/simulator, and require clean stop/restart rather than relying on desktop success.
- **Bundled SQLite defaults may not match required hardening options** -> assert engine/source identity and compile options first, retain canonical lifecycle fixtures, and stop for design revision instead of accepting fallback behavior.
- **Rust libraries may increase application size or cold-start time** -> record current baselines and acceptance budgets during the initial spike, strip symbols appropriately, and package each required native library once per architecture.
- **Parallel implementation can create integration churn** -> freeze ABI and crate seams first, assign non-overlapping owners, keep one runtime/bridge integration owner, and merge only independently verified slices.
- **One implementation can hide platform-specific lifecycle defects** -> retain host-level lifecycle and physical BLE conformance; shared code does not replace platform acceptance.
- **Linux/Windows compile success may be mistaken for product support** -> specs distinguish portable-core build targets from supported application hosts.

## Migration Plan

The migration is internal and staged; there is no wire rollout or database migration. The contract, requirements, SQL, fixtures, and conformance are authoritative. The current Swift and Kotlin implementations remain mutually exclusive fallback variants until their hosts reach deletion checkpoints. A Rust-backed failure can select the corresponding legacy variant until that checkpoint; after deletion, rollback reverts the coherent cutover commit.

Activity or WebView recreation reattaches a replacement host wrapper to the existing process-owned runtime. Process death invalidates every native/JNI handle, pending command, operation, staging token, and presentation, then creates exactly one new runtime on relaunch; only durable SQLite state is recovered.

Storage cutover first proves that bundled SQLite opens existing canonical fixtures and exported archives, records the engine identity/options, and passes interruption/recovery behavior. Any incompatible pre-release database is recreated from canonical SQL. Portable archives remain the supported cross-platform data path.

## Open Questions

- What exact maximum command payload and total command-queue byte budget should the ABI enforce? The bridge spike records representative Bluetooth, resource, document, permission, and presentation commands before fixing these values.
- What exact APK/IPA size and cold-start budgets should gate the runtime spike? The first task records current Swift/Kotlin baselines and proposed thresholds before accepting Rust packaging.
- Should Android use a pinned `cargo-ndk` invocation or an equally reproducible direct Cargo/NDK configuration? The spike selects the smaller Gradle-owned path and records every packaged artifact per ABI.
- Should Apple packaging produce one generated XCFramework consumed by a local Swift package or link target-specific static libraries directly? The spike selects the path with deterministic Xcode dependency tracking, simulator/device coverage, and no committed binaries.
- Which Windows and Linux CI images become compile/smoke references? They are selected from supported Rust environments during toolchain setup and do not imply shipped-host support.
