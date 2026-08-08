## 1. Reconcile and baseline

- [ ] 1.1 Sync/archive `share-native-backend-core`, or explicitly reconcile its pending platform-host, development-loop, and conformance deltas before modifying the same capabilities.
- [ ] 1.2 Record production Swift and Kotlin inventories excluding tests, generated sources, dependencies, and build outputs; classify every remaining file by allowed host responsibility.
- [ ] 1.3 Build a bridge reachability table linking every current command/event kind to its Rust requester, host dispatcher, completion shape, bounds, UI-thread requirement, and test coverage.
- [ ] 1.4 Confirm from retained firmware source and supported fixtures that the meaningful VESC device UUID is the existing fixed 12-byte `protocol-vesc` firmware field, including the all-zero unavailable case.
- [ ] 1.5 Compare a fixed-layout platform-event header with focused event exports and select the smaller audited C/JNI/Swift/Kotlin surface while preserving caller-owned buffers and no callbacks.

## 2. Stable private identity

- [ ] 2.1 Add Rust domain types and validation that distinguish a bounded private platform transport association from the meaningful 12-byte VESC firmware UUID saved-board key.
- [ ] 2.2 Update discovery and connect coordination to carry the private Apple peripheral UUID or Android BLE address without creating a process-random device-token alias.
- [ ] 2.3 Update saved-mode identification to use exact meaningful firmware UUID bytes as the private stable board key and retain the existing private random fallback only when firmware identity is unavailable.
- [ ] 2.4 Revalidate the firmware UUID on reconnect before accepting a stored platform association as the same saved board; reject conflicting or stale associations without merging boards.
- [ ] 2.5 Keep guest-mode UUID and platform associations in bounded live state only and prove that neither becomes durable saved identity.
- [ ] 2.6 Update private catalog/storage representation and disposable schema identity as required; recreate incompatible development data without migration or heuristic matching.
- [ ] 2.7 Add Rust tests for repeated identity, process restart, reconnect generation, two distinct UUIDs, unavailable/all-zero UUID fallback, cloned/conflicting UUID handling, guest isolation, and forget-board cleanup.
- [ ] 2.8 Add privacy tests proving firmware UUIDs, Apple peripheral UUIDs, and Android BLE addresses are absent from frontend state, results, errors, diagnostics, ordinary logs, archives, exports, and public `deviceHistoryId` values.

## 3. Minimal Rust bridge model

- [ ] 3.1 Refactor `host-ports` so the outer command header kind is the actual platform command discriminator and payloads no longer repeat runtime generation, operation identifier, or an inner command tag.
- [ ] 3.2 Implement the selected focused platform-event ABI with fixed-width bounded values and retain the dedicated raw Bluetooth notification submission.
- [ ] 3.3 Define command completion semantics so host completion reports OS acceptance/failure while ordered transport events drive Rust connection readiness and terminal application state.
- [ ] 3.4 Move discovered-device deduplication, pending connect/disconnect intent, connection generation, state normalization, saved/guest policy, and bounded diagnostic normalization into Rust.
- [ ] 3.5 Delete command kinds, event variants, resource namespaces, and compatibility decoders without reachable production owners; wire required document staging/export commands from their contract paths before retaining them.
- [ ] 3.6 Add Rust codec/ABI vectors for every retained command payload, event submission, malformed value, maximum value, stale generation, duplicate completion, and ordering case.

## 4. C ABI and Android JNI

- [ ] 4.1 Update the C ABI version and exported functions/structures for the minimized command/event bridge, containing panics and copying every input before return.
- [ ] 4.2 Regenerate the checked-in C header with the pinned tool and extend the pure-C harness for sizes, alignments, command kinds, event calls, buffer resizing without dequeue, stop/wake, stale handles, and panic containment.
- [ ] 4.3 Update the Rust JNI facade and explicit `JNI_OnLoad` registration for the selected event calls and minimized command header without retaining `JNIEnv`, callbacks, classes, or global references.
- [ ] 4.4 Add JNI tests for ABI/build mismatch, bounded array/string copying, invalid UUID/association input, duplicate and stale completion, wait-buffer growth, shutdown wake, and runtime restart.
- [ ] 4.5 Update build identity, header provenance, Android packaging evidence, Apple library packaging, and dependency documentation for the atomically changed private ABI.

## 5. Portable startup ownership

- [ ] 5.1 Add a bounded runtime startup configuration for requested port, physical/software transport mode, platform-selected storage roots, and required packaged-resource inputs or access mode.
- [ ] 5.2 Move launch-token secure randomness and production monotonic/wall clocks to Rust while retaining deterministic Rust clock/random injection for conformance.
- [ ] 5.3 Move reachable available-space and uniformly portable filesystem checks into Rust using the existing dependency graph, or delete them when the reachability audit proves they are unused.
- [ ] 5.4 Move canonical SQL and software fixtures to Rust-owned resources and reduce Apple bundle/Android `AssetManager` commands to packaged assets that cannot be supplied safely at startup.
- [ ] 5.5 Remove storage-root, random, clock, resource-namespace, and other bootstrap command round trips superseded by startup configuration.
- [ ] 5.6 Test invalid roots/resources/modes/ports, cold startup, readiness ordering, suspend/resume, explicit stop, process restart, and absence of Bluetooth work before successful initialization.

## 6. One Rust software transport

- [ ] 6.1 Extend production runtime composition to select the existing Rust software host internally and ensure software mode emits no native Bluetooth commands.
- [ ] 6.2 Route macOS software selection, iOS simulator, Android emulator, browser-host development, and software conformance through the Rust software transport setting.
- [ ] 6.3 Prove that software and physical modes share the same application actor, dispatcher, stores, loopback server, lifecycle handling, contract state, and shutdown graph.
- [ ] 6.4 Add canonical software-host parity and bounded request-transcript coverage for discovery, connect, firmware identity, VESC responses, disconnect, restart, saved mode, and guest mode.
- [ ] 6.5 Delete Swift `SoftwareVescPeripheral`/`SoftwareBluetoothAdapter`, Kotlin `SoftwareVescPeripheral`/`SoftwareBluetoothAdapter`, their native fixture loaders, duplicate tests, and dependencies used only by those paths.
- [ ] 6.6 Ensure production physical device builds cannot select software transport except through an explicit supported debug/development configuration.

## 7. Apple host minimization

- [ ] 7.1 Convert `FloatViewCoreRuntime` to the new command header and focused event/completion calls while preserving one off-main pull worker, presentation replacement, buffer ownership, and shutdown ordering.
- [ ] 7.2 Replace the general Swift bridge codec with retained command-specific decoders and focused event submissions; delete production command encoders and event decoders.
- [ ] 7.3 Update `CoreBluetoothAdapter` to expose only private peripheral association, OS state/GATT readiness, write backpressure, callback order, and raw notification behavior required by Rust.
- [ ] 7.4 Remove Swift random token maps, pending application connection policy, broad bridge-domain models, unreachable saved-device configuration, and Rust-owned Bluetooth policy tests.
- [ ] 7.5 Reduce Apple platform host services to reachable bundle/document/presentation/background responsibilities after portable startup work moves to Rust.
- [ ] 7.6 Update focused Swift tests for CoreBluetooth mapping, Core Location, presentation replacement, command dispatch, event submissions, buffer resize, stale generation, and stop/wake without reverse production codecs.
- [ ] 7.7 Pass macOS and iOS simulator build, lifecycle, browser-host, software-host, and canonical focused gates before deleting the superseded Apple paths.

## 8. Android host minimization

- [ ] 8.1 Convert `ProcessCoreBridge` and `NativeCore` declarations to the new command header and focused event/completion JNI calls while preserving one process runtime and one blocking non-main worker.
- [ ] 8.2 Replace the general Kotlin bridge codec with retained command-specific decoders and focused event submissions; delete production command encoders and event decoders.
- [ ] 8.3 Update `AndroidBluetoothAdapter` to expose only private BLE-address association, Android permission/radio/GATT readiness, negotiated writes, callback order, and raw notification behavior required by Rust.
- [ ] 8.4 Remove Kotlin random token maps, pending application connection policy, broad `backend/Domain.kt` models, unreachable saved-device configuration, and Rust-owned Bluetooth policy tests.
- [ ] 8.5 Reduce Android platform host services to reachable asset/content-URI/presentation/background responsibilities after portable startup work moves to Rust.
- [ ] 8.6 Preserve Activity recreation, permission-result routing, foreground-service type refresh, WebView replacement, document staging, export sharing, and process shutdown using the minimized runtime owner.
- [ ] 8.7 Update focused JVM/instrumented tests for Android Bluetooth mapping, location, presentation replacement, command dispatch, event submission, ABI identity, stale generation, and stop/wake without reverse production codecs.
- [ ] 8.8 Pass Android unit, instrumentation, emulator software-host, lifecycle, browser-host, and canonical focused gates before deleting superseded Kotlin paths.

## 9. Ownership and conformance gates

- [ ] 9.1 Add a host-surface audit that reports production/test source counts separately and rejects the deleted native software peripherals, reverse bridge codecs, random token maps, unused saved-device APIs, and unreachable command kinds.
- [ ] 9.2 Add a reachability check that cross-references every retained command kind with a Rust production requester, host dispatcher, bounded completion path, and conformance case.
- [ ] 9.3 Run canonical saved/guest, process-restart, WebView/Activity recreation, UUID identity, privacy, software-host parity, import/export, lifecycle, overload, and shutdown scenarios across applicable targets.
- [ ] 9.4 Verify the generated header diff, pure-C harness, Rust workspace tests/lints, Swift tests/builds, Android JVM/instrumented tests/builds, and locked offline dependency checks.
- [ ] 9.5 Record the final production Swift/Kotlin inventory and explain every remaining file whose ownership is not self-evidently an OS API adapter, lifecycle/presentation owner, or bridge wrapper.
- [ ] 9.6 Run the explicit physical BLE checkpoint on macOS, iOS, and Android, including default-MTU writes, notification setup, saved-device reconnect, background/frontend replacement, and stale callback rejection.
- [ ] 9.7 Run `just verify` and resolve every failure without restoring a superseded compatibility path.
