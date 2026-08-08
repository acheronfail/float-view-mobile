## 1. Portability and Bridge Proofs

- [x] 1.1 Record and pin the Rust toolchain, required macOS/iOS/Android targets, Cargo lockfile policy, Android NDK/JDK, selected Cargo/NDK packaging tool, `cbindgen` version, Apple packaging inputs, checksums, and reproducible setup/validation commands.
- [x] 1.2 Create the minimal `/backends/core` Cargo workspace with focused runtime, ABI, and Android-JNI products and prove a library plus target-appropriate test/smoke executable builds on native macOS, iOS simulator, and Android emulator/device.
- [x] 1.3 Define the conservative C ABI status codes, version/build identity, opaque runtime handle lifecycle, caller-buffer rules, fixed command header, pointer/length invariants, panic policy, and symbol visibility; generate and check in the first header with pinned `cbindgen`.
- [x] 1.4 Add a pure-C ABI harness covering header compilation, layout/identity checks, create/start/stop/destroy, invalid and stale handles, copied inputs, caller-owned outputs, insufficient-capacity retry without dequeue, panic containment, and stop waking a blocked waiter.
- [x] 1.5 Implement a minimal bounded Rust application actor and host-command queue; prove one dedicated worker can wait, dispatch a synthetic command, return a matching completion, reject duplicates/stale generations, enforce item/byte bounds, and shut down without reentrancy.
- [x] 1.6 Build a minimal macOS/iOS Swift wrapper over the generated header, prove the command pump stays off the UI thread across repeated start/stop and WebView/presentation replacement, and record the selected deterministic static-library/XCFramework packaging strategy.
- [x] 1.7 Build a minimal Android APK with one Rust library per required ABI and a hand-written explicitly registered JNI facade; prove the Kotlin worker blocks only in command retrieval, uses call-scoped references, copies bounded buffers, survives Activity recreation, and wakes cleanly without Rust-to-Kotlin callbacks.
- [x] 1.8 Add a production-shaped Tokio/Axum smoke fixture covering loopback bind, bootstrap HTTP, bounded headers/bodies/connections/write buffers, WebSocket clients/queues, upgrade/fragmentation, idle/read timeouts, invalid origin/token/size rejection, clean shutdown, suspension, and restart on every runtime matrix target.
- [x] 1.9 Add a Cargo-locked bundled-`rusqlite` smoke fixture that records SQLite version/source/options and proves canonical schema open, WAL write/read, interruption, checkpoint, close, and reopen on every target without loading a system SQLite implementation.
- [x] 1.10 Prove the selected Cargo-locked SHA-256, secure-random, gzip/zlib-compatible compression, XML, CRC, and archive crates on the target matrix and document the responsibility and license disposition of each direct dependency.
- [x] 1.11 Measure current Swift/Kotlin application size and cold-start baselines plus Rust static/dynamic packaging results, record reviewed acceptance budgets, and pause for design revision if ABI, JNI, SQLite options, loopback behavior, packaging, or budgets require a broad compatibility layer or weakened invariant.

## 2. Port Protocols and Domain Behavior

- [x] 2.1 Establish focused Rust crates/modules and immutable fixture loaders for domain, VESC, Refloat, BMS, application, storage, loopback, bridge, and deterministic software adapters without introducing a generic utility crate.
- [x] 2.2 Port bounded byte readers/writers, CRC framing, response attribution, command identifiers, CAN forwarding, and malformed-frame vectors from the authoritative requirements and existing dual implementations.
- [x] 2.3 Port firmware identity parsing, exact compatibility catalogs, controller routes, controller discovery models, and immutable valid/invalid identity vectors.
- [x] 2.4 Port Refloat legacy/dynamic telemetry families, normalized mappings, polling descriptors, detailed telemetry catalog behavior, and golden vectors.
- [x] 2.5 Port Refloat semantic controls, lighting, shutdown, mutation authorization models, flywheel boundaries, and their exact route/payload/result vectors.
- [x] 2.6 Port Refloat configuration chunk assembly, Qt/zlib/XML decoding, CRC32C signatures, typed values, read/write/readback semantics, schema fixtures, and exact-version Tune models.
- [x] 2.7 Port VESC BMS firmware parsing, auxiliary-node identity, values decoding, route handling, polling models, privacy boundaries, and source-derived malformed/valid vectors.
- [x] 2.8 Port deterministic software VESC/Bluetooth behavior needed by core tests and compare Rust protocol outputs against the complete existing Swift/Kotlin vector tables before treating a protocol family as migrated.

## 3. Build the Rust Application, Storage, and Server

- [x] 3.1 Define internal typed platform-port events/commands and the bounded bridge codecs for Bluetooth, location, lifecycle/background eligibility, resources, document staging, diagnostics, storage roots/space, clocks/randomness, settings/theme/permissions, service state, and export presentation.
- [x] 3.2 Implement the process-owned application actor, bounded ingress/command/storage/server channels, operation/generation lifecycle, state revision publication, and deterministic clock/random adapters.
- [x] 3.3 Port controller discovery, connection-wide request scheduling, telemetry polling, auxiliary discovery, reconnect policy, and Bluetooth stream policy into the Rust actor with one write owner.
- [x] 3.4 Port mutation coordination, shutdown, Refloat configuration service, adaptive logging, saved/guest tracking policy, BMS ownership, and authoritative application-state mapping.
- [x] 3.5 Implement the bundled-`rusqlite` catalog/session store with canonical SQL, serialized writers, storage-version recreation, settings/board/Tune/schema-cache behavior, and startup recovery.
- [x] 3.6 Port dedicated cancellable WAL readers, published history boundaries, session-data queries, field validation, configurable-gap reducer, pagination/cursors, deletion leases, pruning, incremental compaction, and interruption cleanup.
- [x] 3.7 Port archive validation/import, bounded staging consumption, export creation, session metadata/provenance, and portable file interoperability without crossing archive semantics into hosts.
- [x] 3.8 Implement Rust loopback bootstrap/static resources, exact origin/token authentication, request/result/error dispatch, complete monotonic `state.changed` publication, WebSocket lifecycle, and all connection/message/queue/time bounds.
- [x] 3.9 Implement authenticated session-data HTTP, validated columnar serialization, gzip delivery, import/export routes, cancellation, bounded response allocation, and logical packaged-resource serving.
- [x] 3.10 Consolidate protocol, application-state, storage, server, bridge-codec, privacy, and deterministic-adapter tests under the Cargo workspace; delete duplicate test cases only after equivalent Rust coverage is demonstrated.
- [x] 3.11 Run the canonical software suite against a standalone Rust backend using deterministic platform adapters and resolve every contract, ordering, persistence, bounds, privacy, and lifecycle discrepancy before platform cutover work proceeds.
- [x] 3.12 Compose one production Rust runtime that owns the application actor, bundled catalog/session stores, contract dispatcher, authenticated loopback server, and bounded platform bridge; publish generation-scoped loopback readiness to hosts, enforce ordered shutdown/restart, and make standalone canonical conformance launch this same graph with deterministic adapters instead of a separate fixture backend.

## 4. Migrate Apple Hosts

- [x] 4.1 Implement the hand-written Swift ABI wrapper with version/build validation, opaque-handle ownership, copied inputs, caller-owned command buffers, stable status mapping, one dedicated command-pump queue, and stop/wake teardown.
- [x] 4.2 Adapt existing CoreBluetooth and Core Location implementations to bounded Rust bridge events/commands with explicit runtime/connection generations, ordering, cancellation, and privacy-safe diagnostics.
- [x] 4.3 Adapt Apple lifecycle, background eligibility, bundle resources, storage roots/space, clocks/randomness, security-scoped document staging, permissions/settings/theme/service state, export, and UIKit/AppKit presentation without shared application semantics.
- [x] 4.4 Make the macOS executable build/link the locked Rust product and matching generated header/module map as a mutually exclusive backend variant, with early ABI/native-library mismatch diagnostics.
- [x] 4.5 Make the iOS Xcode project build/link the same Rust source/features for device and simulator architectures as a mutually exclusive backend variant, without committed native binaries or a second dependency resolution.
- [x] 4.6 Add focused Apple tests for ABI buffer/handle ownership, command-worker threading, adapter mapping, resource/document behavior, WebView replacement, lifecycle suspension/restart, process relaunch, pending-operation cancellation, and native packaging identity.
- [x] 4.7 Run complete macOS and iOS focused builds, loopback/storage/lifecycle checks, canonical software conformance, and representative physical BLE checkpoints against the Rust variants.
- [x] 4.8 Select the Rust-backed Apple path, delete the Swift application-service, protocol, SQLite, server, serialization, and duplicate behavior-test owners, remove FlyingFox/system-SQLite/backend-only dependencies, and retain only Swift hosts/adapters/wrappers.

## 5. Migrate the Android Host

- [x] 5.1 Add one Gradle-owned locked Cargo build per required Android ABI using the spike-selected NDK integration, package each Rust library exactly once, and fail toolchain, lockfile, native-library-duplication, symbol-visibility, or ABI-identity mismatches before APK packaging.
- [x] 5.2 Implement the small Rust JNI facade with explicit `JNI_OnLoad` registration, fixed Kotlin native declarations, primitive/direct-or-array buffer validation, call-scoped references, bounded error mapping, and delegation to the shared runtime bridge without native-to-JVM callbacks.
- [x] 5.3 Implement one process-owned Kotlin bridge worker that waits outside the UI thread, dispatches bounded commands to platform adapters, returns completions/cancellations, replaces presentation wiring across Activity/WebView recreation, and wakes/releases cleanly on shutdown.
- [x] 5.4 Adapt the existing Android Bluetooth scan/GATT/permission implementation to bounded bridge events/commands with copied byte ownership, explicit generations/order, cancellation, and privacy-safe diagnostics.
- [x] 5.5 Adapt Android location, foreground logging service, lifecycle/background eligibility, `AssetManager` resources, content-URI document staging, diagnostics, storage roots/space, clocks/randomness, settings/theme/permission/export, and WebView presentation without host archive/state/frontend semantics.
- [x] 5.6 Replace Android backend initialization and loopback startup with one process-owned Rust runtime that survives Activity/WebView recreation, creates one fresh durable-state-recovering runtime after process death, and shuts down without a blocked waiter, pending command, leaked handle, or late completion.
- [x] 5.7 Add focused Android tests for Cargo/Gradle/runtime packaging, explicit JNI registration, ABI identity, concurrent event ordering, command-buffer bounds, local reference scope, resource/document adapters, Activity/process recreation, reconnect/cancellation races, loopback lifecycle/bounds, and bundled SQLite lifecycle.
- [x] 5.8 Run the complete canonical software suite and representative physical BLE checkpoints against the Rust-backed Android build and resolve every contract, ordering, lifecycle, persistence, bounds, privacy, and packaging mismatch.
- [x] 5.9 Select the Rust-backed Android path, delete the Kotlin application-service, protocol, SQLite, server, serialization, and duplicate behavior-test owners, remove Ktor/backend-only dependencies, and retain only Kotlin hosts/adapters/JNI declarations.

## 6. Portable Matrix and Repository Workflows

- [x] 6.1 Add native macOS, iOS-simulator, and every-Android-ABI/emulator Cargo/core/C-ABI/dependency/server/storage smoke jobs.
- [x] 6.2 Update root setup, generation, build, test, development, and audit recipes to validate exact tool/dependency/header/native-library identities, fetch locked Cargo sources explicitly, keep normal builds offline-capable and generator-free, and make `just dev macos|ios|android`, focused tests, and `just verify` use the Rust backend.
- [x] 6.3 Add `just generate core-header` and CI drift verification using pinned `cbindgen`; ensure a changed Rust ABI cannot merge without the matching checked-in header, wrappers, version/build identity, C harness, and bridge tests.
- [x] 6.4 Update backend ownership documentation and `AGENTS.md` retained paths for the Rust workspace, crate dependency direction, C/JNI boundary, command-pump lifecycle, thin Apple/Android hosts, dependency provenance, and privacy rules.
- [x] 6.5 Audit Cargo features and lockfile, tool/artifact checksums, licenses/notices, generated-header provenance, packaged native libraries/symbols per architecture, source counts, retained paths, and the absence of UniFFI, JNA, `swift-java`, mutable local repositories, system SQLite fallbacks, or compatibility resources used only by deleted backends.

## 7. Final Verification

- [x] 7.1 Run focused Rust protocol, application-actor, bridge-codec, resource/import staging, SQLite recovery/reader/prune/archive, loopback security/lifecycle/resource-bound, crypto/compression/XML/archive, dependency-identity, and panic-boundary tests on every required runtime matrix target.
- [x] 7.2 Run the pure-C harness and generated-header drift checks on every target, plus focused macOS/iOS ABI, command-worker, adapter, lifecycle, packaging, and canonical conformance checks.
- [x] 7.3 Run every packaged Android ABI build plus Android unit/instrumentation, JNI worker/recreation/process-death/reference/buffer stress, native packaging, and canonical conformance checks.
- [x] 7.4 Run and record the explicit physical BLE checkpoint for macOS, iOS, and Rust-backed Android using the production platform adapters and programmable fixture.
- [x] 7.5 Run `just verify` and resolve every software build, test, conformance, formatting, audit, generated-header, and generated-resource failure.

## 8. Post-review Runtime Hardening and Simplification

- [x] 8.1 Move production contract/state composition out of `test-standalone`, make native runtime and the deterministic launcher depend on the same lower service owner, and delete the parallel mutable JSON/connection state machine after equivalent production-path coverage exists.
- [x] 8.2 Implement dynamic saved-board/history identity and multi-board scoping, remove startup creation of the fixed software board, make saved-device deletion remove all owned durable history recoverably, and prove restart does not recreate deleted boards.
- [x] 8.3 Accept saved and guest tracking through the production connection path and gate board/profile/hint/session/sample/location/catalog/export writes at one typed policy boundary; add two-board and guest-isolation production-composition tests.
- [x] 8.4 Drive polling, request timeouts, session/sample timestamps, adaptive persistence, and reconnect from the injected actor clock; honor the persistence decision, remove sequence-derived elapsed time, and replace the fixed 10 ms production polling loop with deadline/publication wakeups.
- [x] 8.5 Add bounded typed settings validation and apply accepted settings to polling, persistence, reconnect, location, history, and export policies without mutating or persisting on invalid input.
- [x] 8.6 Connect Refloat configuration and Tune contract methods to the existing coordinator, scheduler, schema cache, and Tune store; remove production fixture errors and prove eligible, stale, unsafe, and restart cases.
- [x] 8.7 Replace offset/global session listing and fixture maintenance results with board-scoped snapshot-bound cursors and authoritative delete/prune/import/export operations; make Bluetooth scan acknowledge queued work without blocking the loopback task.
- [x] 8.8 Split the production composition, contract DTO/mapping, device runtime, and deterministic host into focused modules; restrict fixture loaders to tests/software adapters, remove unused application dependencies/channels and compatibility aliases, and refresh source/dependency documentation.
- [x] 8.9 Bound completed-operation replay detection at every bridge/actor layer and remove the synthetic secondary runtime actor/event path.
- [x] 8.10 Add production-composition regression coverage for invalid settings, guest mode, two saved boards, deletion/restart, telemetry cadence/persistence, nonblocking scan, configuration/Tunes, and absence of fixed fixture state.
- [x] 8.11 Run focused persistence, runtime, ABI, host-adapter, canonical conformance, audit, and `just verify` gates and resolve every software failure.
