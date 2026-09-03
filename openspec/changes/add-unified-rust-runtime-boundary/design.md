## Context

Ten protocol families are selected through separate Apple FFI and Android JNI crates, while the portable discovery, scheduling, telemetry, mutation, configuration, and session-SQLite components remain unselected. Swift and Kotlin therefore still own almost identical connection/application state machines and storage glue. The current 43-member workspace contains about 5,400 non-test lines of temporary host-bridge machinery and about 12,000 non-test lines of unselected application/persistence candidates; another narrow extraction would increase the migration midpoint without making a host cheaper.

The next safe step is not an OS backend or loopback server in Rust. It is one safe application-runtime component whose input/output boundary is already shaped for a later single Apple bridge and single Android bridge. This change proves that composition without selecting production. Separate later changes select Apple, select Android, and then delete the obsolete bridge families.

## Goals / Non-Goals

**Goals:**

- Add one `application-runtime` crate that owns a connection coordinator, VESC stream lifetime, framing of scheduled writes, and an optional canonical session writer.
- Accept only bounded platform-neutral commands containing explicit clock, Bluetooth, configuration-cache, session-path, and location facts.
- Return ordered owned effects for BLE writes, timers/deadlines, routing-hint/cache persistence, activation, state publication, telemetry/BMS publication, session marker publication, diagnostics, and failures.
- Persist a normalized telemetry sample before exposing it as published live state whenever an active writer is attached.
- Directly expose the existing bounded session reader/pruner operations through safe Rust methods so later coarse host bridges do not need persistence-specific native implementations.
- Establish exact deletion and LOC gates for the later host cutovers.

**Non-Goals:**

- A C ABI, JNI ABI, generated binding, callback, async executor, thread, BLE adapter, timer implementation, filesystem discovery, catalog database, loopback server, JSON HTTP/WebSocket contract, location provider, lifecycle/background policy, or UI behavior.
- Selecting Apple or Android, changing the frontend contract/schema, migrating durable databases, or removing production code in this candidate change.
- Adding Windows/Linux hosts or extracting another independent candidate.

## Decisions

### One safe runtime owner above existing components

`ApplicationRuntime` contains `ConnectionApplicationCoordinator`, `vesc_frame::StreamDecoder`, and at most one `session_sqlite::SessionWriter`. It calls the existing safe component APIs directly and does not reproduce their protocol, state-machine, or SQLite algorithms.

A general trait/plugin system was rejected because there is one product composition. Merging all existing crates immediately was rejected because their focused test and dependency boundaries remain useful; crate consolidation can follow bridge deletion if measurements justify it.

### Commands are facts; effects are ordered work

The runtime uses a closed Rust `Command` enum. Hosts supply monotonic time and epoch observation time on relevant commands. `ReceiveBytes` accepts one bounded BLE notification fragment and processes every complete decoded payload in arrival order. Coordinator effects are translated into a closed runtime `Effect` enum; scheduled payloads are framed before `WriteBytes` is emitted.

The runtime retains no host reference and invokes no callback. A callback-driven design was rejected because it would couple Rust lifetimes to Swift/Kotlin concurrency and make reentrancy difficult to audit. JSON inside the safe crate was rejected because typed Rust values give stronger composition evidence; serialization belongs only in the later two coarse host adapters.

### Persistence is attached explicitly and publication follows commit

`AttachSession` creates or opens one caller-selected canonical session database using supplied cadence and epoch metadata. `DetachSession` checkpoints and closes it. A normalized telemetry effect is appended synchronously before the runtime emits `PublishTelemetry` and `PublishSessionMarker`; a failed commit emits a closed persistence failure and prevents publication of uncommitted telemetry. Location observations use a separate explicit command and marker effect.

The runtime does not discover session paths, update the catalog, decide ride-retention policy, import/export archives, or own maintenance scheduling. Those remain host facts/policies until later bounded runtime work. SQLite callbacks and a runtime-owned executor were rejected; later host adapters may serialize calls on their existing application queues.

### Historical read and prune remain safe direct operations on the same coarse owner

The runtime crate re-exports or wraps the closed `SessionReader` and `SessionPruner` requests/results rather than copying query or prune behavior. These operations remain synchronous and interruptible. The later host bridge may serialize a bounded request record and result through the single runtime library, but no persistence-specific library is introduced.

### The future host ABI is one library per platform

This change intentionally adds no unsafe boundary. Its tests fix the command/effect semantics that later `application-runtime-apple-ffi` and `application-runtime-android-jni` crates must expose. Each later platform adapter will build one library and use one opaque runtime handle with explicit create/destroy and serialized command/result operations. Handles, allocation, panic containment, and thread confinement are audited in those host-cutover changes.

Per-protocol exports, direct native calls into component crates, host callbacks, generated bindings, and a common Apple/Android binary are rejected. The coarse adapters are temporary only in the sense that a future platform may bind Rust more directly; unlike the current twenty protocol bridges, they represent the supported runtime seam.

### Convergence is measured, not inferred

Commit a machine-readable baseline containing tracked production/non-test LOC by owner, 43 workspace members, 20 protocol bridge crates, 10 selected protocol families, and the current build artifact matrix. Later Apple and Android proposals must name exact source/build deletion sets and must not increase total production LOC after their complete implementation. After both select the runtime, a final change removes all protocol bridge crates, C modules/headers, Kotlin bridge loaders, Swift protocol adapters made obsolete, and their per-family build/audit recipes.

## Risks / Trade-offs

- **[The first runtime crate temporarily adds more code]** → Freeze other extraction and require every following change to be a deletion-bearing host cutover against the committed baseline.
- **[Synchronous SQLite can delay the host application queue]** → Preserve existing bounded batch sizes and busy timeouts; later host adapters keep execution off UI threads and retain interrupt handles for reads/prunes.
- **[One coarse boundary can become a generic dumping ground]** → Keep the command/effect surface closed, prohibit OS and loopback ownership, and admit new operations only when they replace native cross-host behavior.
- **[Effect translation can reorder scheduler behavior]** → Preserve vectors for every coordinator effect sequence, multiple frames per notification, timer/deadline cancellation, activation, reset, persistence-before-publication, and failures.
- **[Session creation can succeed while activation fails]** → Explicitly close/remove only caller-authorized disposable session targets on rollback; this candidate leaves catalog/recovery ownership outside and tests reusable failure state.
- **[The later FFI/JNI object boundary introduces unsafe lifetime risk]** → Defer unsafe code to named adapter crates with opaque-handle registries, panic containment, thread confinement, ABI/version checks, and host-specific physical gates.

## Migration Plan

1. Commit this proposal, design, delta specs, tasks, and the measured baseline before implementation.
2. Add the safe runtime crate, framing/coordinator/persistence composition, historical wrappers, fixtures, focused tests, and audits without changing production hosts.
3. Pass focused offline tests, warnings-denied workspace lint, strict OpenSpec validation, `git diff --check`, and `just verify`; commit the implementation.
4. Sync and archive this candidate change.
5. Propose and implement the Apple runtime adapter/cutover, deleting the selected Swift connection/protocol/persistence behavior and validating software plus connected iOS/ESP32 physical conformance before sync/archive.
6. Propose and implement the Android runtime adapter/cutover, deleting the selected Kotlin connection/protocol/persistence behavior; run all software gates but explicitly defer the Android physical checkpoint as requested.
7. Propose and implement final protocol-bridge retirement, removing all twenty old bridge crates and their Apple/Android build machinery, then verify, sync, and archive.

Rollback before host selection removes the unselected runtime crate, fixtures, baseline, and audits. Later host-cutover changes must each preserve a commit at which that host can be reverted independently without restoring a mixed fallback inside production.

## Open Questions

None for this candidate boundary. Exact opaque-handle encoding and host serialization formats are intentionally decided in the Apple and Android cutover proposals after this safe API is proven.
