## 1. Portability and Bridge Proofs

- [ ] 1.1 Record and pin the Rust toolchain, required macOS/iOS/Android/Linux/Windows targets, Cargo lockfile policy, Android NDK/JDK, selected Cargo/NDK packaging tool, `cbindgen` version, Apple packaging inputs, reference images, checksums, and reproducible setup/validation commands.
- [ ] 1.2 Create the minimal `/backends/core` Cargo workspace with focused runtime, ABI, and Android-JNI products and prove a library plus target-appropriate test/smoke executable builds on native macOS, iOS simulator, Android emulator/device, and Linux/Windows reference images.
- [ ] 1.3 Define the conservative C ABI status codes, version/build identity, opaque runtime handle lifecycle, caller-buffer rules, fixed command header, pointer/length invariants, panic policy, and symbol visibility; generate and check in the first header with pinned `cbindgen`.
- [ ] 1.4 Add a pure-C ABI harness covering header compilation, layout/identity checks, create/start/stop/destroy, invalid and stale handles, copied inputs, caller-owned outputs, insufficient-capacity retry without dequeue, panic containment, and stop waking a blocked waiter.
- [ ] 1.5 Implement a minimal bounded Rust application actor and host-command queue; prove one dedicated worker can wait, dispatch a synthetic command, return a matching completion, reject duplicates/stale generations, enforce item/byte bounds, and shut down without reentrancy.
- [ ] 1.6 Build a minimal macOS/iOS Swift wrapper over the generated header, prove the command pump stays off the UI thread across repeated start/stop and WebView/presentation replacement, and record the selected deterministic static-library/XCFramework packaging strategy.
- [ ] 1.7 Build a minimal Android APK with one Rust library per required ABI and a hand-written explicitly registered JNI facade; prove the Kotlin worker blocks only in command retrieval, uses call-scoped references, copies bounded buffers, survives Activity recreation, and wakes cleanly without Rust-to-Kotlin callbacks.
- [ ] 1.8 Add a production-shaped Tokio/Axum smoke fixture covering loopback bind, bootstrap HTTP, bounded headers/bodies/connections/write buffers, WebSocket clients/queues, upgrade/fragmentation, idle/read timeouts, invalid origin/token/size rejection, clean shutdown, suspension, and restart on every runtime matrix target.
- [ ] 1.9 Add a Cargo-locked bundled-`rusqlite` smoke fixture that records SQLite version/source/options and proves canonical schema open, WAL write/read, interruption, checkpoint, close, and reopen on every target without loading a system SQLite implementation.
- [ ] 1.10 Prove the selected Cargo-locked SHA-256, secure-random, gzip/zlib-compatible compression, XML, CRC, and archive crates on the target matrix and document the responsibility and license disposition of each direct dependency.
- [ ] 1.11 Measure current Swift/Kotlin application size and cold-start baselines plus Rust static/dynamic packaging results, record reviewed acceptance budgets, and pause for design revision if ABI, JNI, SQLite options, loopback behavior, packaging, or budgets require a broad compatibility layer or weakened invariant.

## 2. Port Protocols and Domain Behavior

- [ ] 2.1 Establish focused Rust crates/modules and immutable fixture loaders for domain, VESC, Refloat, BMS, application, storage, loopback, bridge, and deterministic software adapters without introducing a generic utility crate.
- [ ] 2.2 Port bounded byte readers/writers, CRC framing, response attribution, command identifiers, CAN forwarding, and malformed-frame vectors from the authoritative requirements and existing dual implementations.
- [ ] 2.3 Port firmware identity parsing, exact compatibility catalogs, controller routes, controller discovery models, and immutable valid/invalid identity vectors.
- [ ] 2.4 Port Refloat legacy/dynamic telemetry families, normalized mappings, polling descriptors, detailed telemetry catalog behavior, and golden vectors.
- [ ] 2.5 Port Refloat semantic controls, lighting, shutdown, mutation authorization models, flywheel boundaries, and their exact route/payload/result vectors.
- [ ] 2.6 Port Refloat configuration chunk assembly, Qt/zlib/XML decoding, CRC32C signatures, typed values, read/write/readback semantics, schema fixtures, and exact-version Tune models.
- [ ] 2.7 Port VESC BMS firmware parsing, auxiliary-node identity, values decoding, route handling, polling models, privacy boundaries, and source-derived malformed/valid vectors.
- [ ] 2.8 Port deterministic software VESC/Bluetooth behavior needed by core tests and compare Rust protocol outputs against the complete existing Swift/Kotlin vector tables before treating a protocol family as migrated.

## 3. Build the Rust Application, Storage, and Server

- [ ] 3.1 Define internal typed platform-port events/commands and the bounded bridge codecs for Bluetooth, location, lifecycle/background eligibility, resources, document staging, diagnostics, storage roots/space, clocks/randomness, settings/theme/permissions, service state, and export presentation.
- [ ] 3.2 Implement the process-owned application actor, bounded ingress/command/storage/server channels, operation/generation lifecycle, state revision publication, and deterministic clock/random adapters.
- [ ] 3.3 Port controller discovery, connection-wide request scheduling, telemetry polling, auxiliary discovery, reconnect policy, and Bluetooth stream policy into the Rust actor with one write owner.
- [ ] 3.4 Port mutation coordination, shutdown, Refloat configuration service, adaptive logging, saved/guest tracking policy, BMS ownership, and authoritative application-state mapping.
- [ ] 3.5 Implement the bundled-`rusqlite` catalog/session store with canonical SQL, serialized writers, storage-version recreation, settings/board/Tune/schema-cache behavior, and startup recovery.
- [ ] 3.6 Port dedicated cancellable WAL readers, published history boundaries, session-data queries, field validation, configurable-gap reducer, pagination/cursors, deletion leases, pruning, incremental compaction, and interruption cleanup.
- [ ] 3.7 Port archive validation/import, bounded staging consumption, export creation, session metadata/provenance, and portable file interoperability without crossing archive semantics into hosts.
- [ ] 3.8 Implement Rust loopback bootstrap/static resources, exact origin/token authentication, request/result/error dispatch, complete monotonic `state.changed` publication, WebSocket lifecycle, and all connection/message/queue/time bounds.
- [ ] 3.9 Implement authenticated session-data HTTP, validated columnar serialization, gzip delivery, import/export routes, cancellation, bounded response allocation, and logical packaged-resource serving.
- [ ] 3.10 Consolidate protocol, application-state, storage, server, bridge-codec, privacy, and deterministic-adapter tests under the Cargo workspace; delete duplicate test cases only after equivalent Rust coverage is demonstrated.
- [ ] 3.11 Run the canonical software suite against a standalone Rust backend using deterministic platform adapters and resolve every contract, ordering, persistence, bounds, privacy, and lifecycle discrepancy before platform cutover work proceeds.

## 4. Migrate Apple Hosts

- [ ] 4.1 Implement the hand-written Swift ABI wrapper with version/build validation, opaque-handle ownership, copied inputs, caller-owned command buffers, stable status mapping, one dedicated command-pump queue, and stop/wake teardown.
- [ ] 4.2 Adapt existing CoreBluetooth and Core Location implementations to bounded Rust bridge events/commands with explicit runtime/connection generations, ordering, cancellation, and privacy-safe diagnostics.
- [ ] 4.3 Adapt Apple lifecycle, background eligibility, bundle resources, storage roots/space, clocks/randomness, security-scoped document staging, permissions/settings/theme/service state, export, and UIKit/AppKit presentation without shared application semantics.
- [ ] 4.4 Make the macOS executable build/link the locked Rust product and matching generated header/module map as a mutually exclusive backend variant, with early ABI/native-library mismatch diagnostics.
- [ ] 4.5 Make the iOS Xcode project build/link the same Rust source/features for device and simulator architectures as a mutually exclusive backend variant, without committed native binaries or a second dependency resolution.
- [ ] 4.6 Add focused Apple tests for ABI buffer/handle ownership, command-worker threading, adapter mapping, resource/document behavior, WebView replacement, lifecycle suspension/restart, process relaunch, pending-operation cancellation, and native packaging identity.
- [ ] 4.7 Run complete macOS and iOS focused builds, loopback/storage/lifecycle checks, canonical software conformance, and representative physical BLE checkpoints against the Rust variants.
- [ ] 4.8 Select the Rust-backed Apple path, delete the Swift application-service, protocol, SQLite, server, serialization, and duplicate behavior-test owners, remove FlyingFox/system-SQLite/backend-only dependencies, and retain only Swift hosts/adapters/wrappers.

## 5. Migrate the Android Host

- [ ] 5.1 Add one Gradle-owned locked Cargo build per required Android ABI using the spike-selected NDK integration, package each Rust library exactly once, and fail toolchain, lockfile, native-library-duplication, symbol-visibility, or ABI-identity mismatches before APK packaging.
- [ ] 5.2 Implement the small Rust JNI facade with explicit `JNI_OnLoad` registration, fixed Kotlin native declarations, primitive/direct-or-array buffer validation, call-scoped references, bounded error mapping, and delegation to the shared runtime bridge without native-to-JVM callbacks.
- [ ] 5.3 Implement one process-owned Kotlin bridge worker that waits outside the UI thread, dispatches bounded commands to platform adapters, returns completions/cancellations, replaces presentation wiring across Activity/WebView recreation, and wakes/releases cleanly on shutdown.
- [ ] 5.4 Adapt the existing Android Bluetooth scan/GATT/permission implementation to bounded bridge events/commands with copied byte ownership, explicit generations/order, cancellation, and privacy-safe diagnostics.
- [ ] 5.5 Adapt Android location, foreground logging service, lifecycle/background eligibility, `AssetManager` resources, content-URI document staging, diagnostics, storage roots/space, clocks/randomness, settings/theme/permission/export, and WebView presentation without host archive/state/frontend semantics.
- [ ] 5.6 Replace Android backend initialization and loopback startup with one process-owned Rust runtime that survives Activity/WebView recreation, creates one fresh durable-state-recovering runtime after process death, and shuts down without a blocked waiter, pending command, leaked handle, or late completion.
- [ ] 5.7 Add focused Android tests for Cargo/Gradle/runtime packaging, explicit JNI registration, ABI identity, concurrent event ordering, command-buffer bounds, local reference scope, resource/document adapters, Activity/process recreation, reconnect/cancellation races, loopback lifecycle/bounds, and bundled SQLite lifecycle.
- [ ] 5.8 Run the complete canonical software suite and representative physical BLE checkpoints against the Rust-backed Android build and resolve every contract, ordering, lifecycle, persistence, bounds, privacy, and packaging mismatch.
- [ ] 5.9 Select the Rust-backed Android path, delete the Kotlin application-service, protocol, SQLite, server, serialization, and duplicate behavior-test owners, remove Ktor/backend-only dependencies, and retain only Kotlin hosts/adapters/JNI declarations.

## 6. Portable Matrix and Repository Workflows

- [ ] 6.1 Add native macOS, iOS-simulator, every-Android-ABI/emulator, Linux-reference-image, and Windows-reference-image Cargo/core/C-ABI/dependency/server/storage smoke jobs without adding incomplete application hosts or supported-platform claims.
- [ ] 6.2 Update root setup, generation, build, test, development, and audit recipes to validate exact tool/dependency/header/native-library identities, fetch locked Cargo sources explicitly, keep normal builds offline-capable and generator-free, and make `just dev macos|ios|android`, focused tests, and `just verify` use the Rust backend.
- [ ] 6.3 Add `just generate core-header` and CI drift verification using pinned `cbindgen`; ensure a changed Rust ABI cannot merge without the matching checked-in header, wrappers, version/build identity, C harness, and bridge tests.
- [ ] 6.4 Update backend ownership documentation and `AGENTS.md` retained paths for the Rust workspace, crate dependency direction, C/JNI boundary, command-pump lifecycle, thin Apple/Android hosts, dependency provenance, privacy rules, and unsupported compile-only Linux/Windows targets.
- [ ] 6.5 Audit Cargo features and lockfile, tool/artifact checksums, licenses/notices, generated-header provenance, packaged native libraries/symbols per architecture, source counts, retained paths, and the absence of UniFFI, JNA, `swift-java`, mutable local repositories, system SQLite fallbacks, or compatibility resources used only by deleted backends.

## 7. Final Verification

- [ ] 7.1 Run focused Rust protocol, application-actor, bridge-codec, resource/import staging, SQLite recovery/reader/prune/archive, loopback security/lifecycle/resource-bound, crypto/compression/XML/archive, dependency-identity, and panic-boundary tests on every required runtime matrix target.
- [ ] 7.2 Run the pure-C harness and generated-header drift checks on every target, plus focused macOS/iOS ABI, command-worker, adapter, lifecycle, packaging, and canonical conformance checks.
- [ ] 7.3 Run every packaged Android ABI build plus Android unit/instrumentation, JNI worker/recreation/process-death/reference/buffer stress, native packaging, and canonical conformance checks.
- [ ] 7.4 Run and record the explicit physical BLE checkpoint for macOS, iOS, and Rust-backed Android using the production platform adapters and programmable fixture.
- [ ] 7.5 Run `just verify` and resolve every software build, test, conformance, formatting, audit, generated-header, and generated-resource failure.
