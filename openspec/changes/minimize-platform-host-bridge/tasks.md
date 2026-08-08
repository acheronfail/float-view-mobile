## 1. Reconcile and baseline

- [x] 1.1 Sync/archive `share-native-backend-core`, or explicitly reconcile its pending platform-host, development-loop, and conformance deltas before modifying the same capabilities.
- [x] 1.2 Record production Swift and Kotlin inventories excluding tests, generated sources, dependencies, and build outputs; classify every remaining file by allowed host responsibility.
- [x] 1.3 Build a bridge reachability table linking every current command/event kind to its Rust requester, host dispatcher, completion shape, bounds, UI-thread requirement, and test coverage.
- [x] 1.4 Confirm from retained firmware source and supported fixtures that the meaningful VESC device UUID is the existing fixed 12-byte `protocol-vesc` firmware field, including the all-zero unavailable case.
- [x] 1.5 Compare a fixed-layout platform-event header with focused event exports and select the smaller audited C/JNI/Swift/Kotlin surface while preserving caller-owned buffers and no callbacks.

## 2. Stable private identity

- [x] 2.1 Add Rust domain types and validation that distinguish a bounded private platform transport association from the meaningful 12-byte VESC firmware UUID saved-board key.
- [x] 2.2 Update discovery and connect coordination to carry the private Apple peripheral UUID or Android BLE address without creating a process-random device-token alias.
- [x] 2.3 Update saved-mode identification to use exact meaningful firmware UUID bytes as the private stable board key and retain the existing private random fallback only when firmware identity is unavailable.
- [x] 2.4 Revalidate the firmware UUID on reconnect before accepting a stored platform association as the same saved board; reject conflicting or stale associations without merging boards.
- [x] 2.5 Keep guest-mode UUID and platform associations in bounded live state only and prove that neither becomes durable saved identity.
- [x] 2.6 Update private catalog/storage representation and disposable schema identity as required; recreate incompatible development data without migration or heuristic matching.
- [x] 2.7 Add Rust tests for repeated identity, process restart, reconnect generation, two distinct UUIDs, unavailable/all-zero UUID fallback, cloned/conflicting UUID handling, guest isolation, and forget-board cleanup.
- [x] 2.8 Add privacy tests proving firmware UUIDs, Apple peripheral UUIDs, and Android BLE addresses are absent from frontend state, results, errors, diagnostics, ordinary logs, archives, exports, and public `deviceHistoryId` values.

## 3. Minimal Rust bridge model

- [x] 3.1 Refactor `host-ports` so the outer command header kind is the actual platform command discriminator and payloads no longer repeat runtime generation, operation identifier, or an inner command tag.
- [ ] 3.2 Reconcile the focused platform-event ABI with direct location-authorization delivery, lifecycle/effective background eligibility, the selected per-host appearance source, removal of the unconsumed logging-service event, and the dedicated raw Bluetooth notification submission.
- [x] 3.3 Define command completion semantics so host completion reports OS acceptance/failure while ordered transport events drive Rust connection readiness and terminal application state.
- [x] 3.4 Move discovered-device deduplication, pending connect/disconnect intent, connection generation, state normalization, saved/guest policy, and bounded diagnostic normalization into Rust.
- [ ] 3.5 Add reachable bounded native import/export presentation commands from their frontend contract paths, then delete only command kinds, event variants, resource namespaces, and compatibility decoders without production owners.
- [ ] 3.6 Update Rust codec/ABI vectors for every retained command payload, document completion, event submission, malformed value, maximum value, stale generation, duplicate completion, and ordering case.

## 4. C ABI and Android JNI

- [ ] 4.1 Update the C ABI version and exported functions/structures for the reconciled command/event bridge, containing panics and copying every input before return.
- [ ] 4.2 Regenerate the checked-in C header with the pinned tool and extend the pure-C harness for document commands, selected event calls, sizes, alignments, buffer resizing without dequeue, stop/wake, stale handles, and panic containment.
- [ ] 4.3 Update the Rust JNI facade and explicit `JNI_OnLoad` registration for the reconciled event calls and document command header without retaining `JNIEnv`, callbacks, classes, or global references.
- [ ] 4.4 Update JNI tests for ABI/build mismatch, bounded document/array/string copying, invalid path/UUID/association input, duplicate and stale completion, wait-buffer growth, shutdown wake, and runtime restart.
- [ ] 4.5 Update build identity, header provenance, Android packaging evidence, Apple library packaging, and dependency documentation for the atomically reconciled private ABI.

## 5. Portable startup ownership

- [x] 5.1 Add a bounded runtime startup configuration for requested port, physical/software transport mode, platform-selected storage roots, and required packaged-resource inputs or access mode.
- [x] 5.2 Move launch-token secure randomness and production monotonic/wall clocks to Rust while retaining deterministic Rust clock/random injection for conformance.
- [x] 5.3 Move reachable available-space and uniformly portable filesystem checks into Rust using the existing dependency graph, or delete them when the reachability audit proves they are unused.
- [x] 5.4 Move canonical SQL and software fixtures to Rust-owned resources and reduce Apple bundle/Android `AssetManager` commands to packaged assets that cannot be supplied safely at startup.
- [x] 5.5 Remove storage-root, random, clock, resource-namespace, and other bootstrap command round trips superseded by startup configuration.
- [x] 5.6 Test invalid roots/resources/modes/ports, cold startup, readiness ordering, suspend/resume, explicit stop, process restart, and absence of Bluetooth work before successful initialization.

## 6. One Rust software transport

- [x] 6.1 Extend production runtime composition to select the existing Rust software host internally and ensure software mode emits no native Bluetooth commands.
- [x] 6.2 Route macOS software selection, iOS simulator, Android emulator, browser-host development, and software conformance through the Rust software transport setting.
- [x] 6.3 Prove that software and physical modes share the same application actor, dispatcher, stores, loopback server, lifecycle handling, contract state, and shutdown graph.
- [x] 6.4 Add canonical software-host parity and bounded request-transcript coverage for discovery, connect, firmware identity, VESC responses, disconnect, restart, saved mode, and guest mode.
- [x] 6.5 Delete Swift `SoftwareVescPeripheral`/`SoftwareBluetoothAdapter`, Kotlin `SoftwareVescPeripheral`/`SoftwareBluetoothAdapter`, their native fixture loaders, duplicate tests, and dependencies used only by those paths.
- [x] 6.6 Ensure production physical device builds cannot select software transport except through an explicit supported debug/development configuration.

## 7. Apple host minimization

- [ ] 7.1 Reconcile `FloatViewCoreRuntime` with the document commands and selected focused event/completion calls while preserving one off-main pull worker, presentation replacement, buffer ownership, and shutdown ordering.
- [ ] 7.2 Update retained Swift command-specific decoders and focused event submissions for the reconciled vocabulary; keep production command encoders and event decoders absent.
- [x] 7.3 Update `CoreBluetoothAdapter` to expose only private peripheral association, OS state/GATT readiness, write backpressure, callback order, and raw notification behavior required by Rust.
- [x] 7.4 Remove Swift random token maps, pending application connection policy, broad bridge-domain models, unreachable saved-device configuration, and Rust-owned Bluetooth policy tests.
- [ ] 7.5 Reduce Apple platform host services to the reachable capability entry points, including native documents, background execution, location authorization, selected appearance source, lifecycle/WebView, and the runtime bridge.
- [ ] 7.6 Update focused Swift tests for CoreBluetooth mapping, direct Core Location authorization, native documents, background eligibility, appearance, presentation replacement, command dispatch, event submissions, buffer resize, stale generation, and stop/wake without reverse production codecs.
- [ ] 7.7 Pass macOS and iOS simulator build, lifecycle, document, appearance, browser-host, software-host, and canonical focused gates before deleting superseded Apple code or layout paths.

## 8. Android host minimization

- [ ] 8.1 Reconcile `ProcessCoreBridge` and `NativeCore` declarations with the document commands and selected focused event/completion JNI calls while preserving one process runtime and one blocking non-main worker.
- [ ] 8.2 Update retained Kotlin command-specific decoders and focused event submissions for the reconciled vocabulary; keep production command encoders and event decoders absent.
- [x] 8.3 Update `AndroidBluetoothAdapter` to expose only private BLE-address association, Android permission/radio/GATT readiness, negotiated writes, callback order, and raw notification behavior required by Rust.
- [x] 8.4 Remove Kotlin random token maps, pending application connection policy, broad `backend/Domain.kt` models, unreachable saved-device configuration, and Rust-owned Bluetooth policy tests.
- [ ] 8.5 Reduce Android platform host services to the reachable capability entry points, including native documents/content URIs, background execution, location authorization, selected appearance source, lifecycle/WebView, and the runtime bridge.
- [ ] 8.6 Preserve Activity recreation, permission-result routing, foreground-service type refresh, WebView replacement, reachable document staging/import/export sharing, and process shutdown using the minimized runtime owner.
- [ ] 8.7 Update focused JVM/instrumented tests for Android Bluetooth mapping, direct location authorization, native documents, background eligibility, appearance, presentation replacement, command dispatch, event submission, ABI identity, stale generation, and stop/wake without reverse production codecs.
- [ ] 8.8 Pass Android unit, instrumentation, emulator software-host, lifecycle, document, appearance, browser-host, and canonical focused gates before deleting superseded Kotlin code or layout paths.

## 9. Ownership and conformance gates

- [ ] 9.1 Reconcile the portable host-surface audit so it reports production/test counts, validates capability inventories and entry files, and rejects deleted software peripherals, reverse codecs, random token maps, unused APIs, incomplete capabilities, and unreachable commands without banning required native document support.
- [ ] 9.2 Reconcile reachability checks so every retained Bluetooth, location, background, document, appearance-fallback, and readiness command/event maps to a production requester or observer, host dispatcher, bounded completion path, and conformance case.
- [ ] 9.3 Run canonical saved/guest, process-restart, WebView/Activity recreation, UUID identity, privacy, software-host parity, native/browser import/export, location authorization, appearance, lifecycle/background, overload, and shutdown scenarios across applicable targets.
- [ ] 9.4 Verify the reconciled generated header diff, pure-C harness, Rust workspace tests/lints, Swift tests/builds, Android JVM/instrumented tests/builds, frontend checks, and locked offline dependency checks.
- [ ] 9.5 Record the final production Swift/Kotlin inventory and explain every remaining capability entry, support stub, OS API adapter, lifecycle/presentation owner, and bridge wrapper.
- [ ] 9.6 Run explicit physical checkpoints on macOS, iOS, and Android for default-MTU BLE writes, notification setup, saved-device reconnect, background BLE/GPS logging, frontend replacement, stale callback rejection, native documents, and live appearance changes.
- [ ] 9.7 Run `just verify` and resolve every failure without restoring a superseded compatibility path or weakening the capability audit.

## 10. Native capability model and layout

- [ ] 10.1 Define the seven native capability families, `implemented`/`unsupported`/`incomplete` status semantics, required entry names, stable unavailable behavior, and focused-evidence fields in `backends/HOST_CAPABILITIES.md`.
- [ ] 10.2 Add an iOS host README and the `FloatViewBluetooth`, `FloatViewBackgroundExecution`, `FloatViewLocation`, `FloatViewDocuments`, `FloatViewAppearance`, `FloatViewLifecycle`, `FloatViewWebView`, and `FloatViewRuntimeBridge` Swift entry files with status and concrete-owner references.
- [ ] 10.3 Create `backends/apple/macos/` as the iOS sibling, add the same README and Swift capability entry shape, and move the existing macOS app shell and resources without changing observable behavior.
- [ ] 10.4 Create `backends/apple/common/`, move only behavior genuinely shared by iOS and macOS into it, and keep each platform entry file as the direct discoverable factory, implementation, or forwarding seam.
- [ ] 10.5 Update Swift Package, Xcode, bundle-resource, build-script, and test paths atomically for the sibling Apple layout and pass both Apple builds before deleting `Sources/FloatViewMac` or other superseded paths.
- [ ] 10.6 Create the Android `fail.acheron.floatview.host` package README and matching Kotlin capability entry files with status and concrete-owner references.
- [ ] 10.7 Add small compiling capability stubs with explanatory comments and stable unavailable results wherever a host does not support a capability; ensure no UI advertises an unsupported capability as available.
- [ ] 10.8 Remove or split generic `Domain`, `Services`, `Utils`, and `PlatformAdapter` ownership buckets whose retained responsibilities can be assigned to a named capability entry or focused private helper.
- [ ] 10.9 Extend the host audit to reject missing matrix rows, missing entry files, undocumented forwarding chains, unexplained unsupported entries, incomplete final entries, duplicate implementations, and obsolete layout paths.

## 11. Rust-owned native document operations

- [ ] 11.1 Define the hand-written frontend document-operation contract, including operation identifier, kind, presenting/completed/cancelled/failed status, bounded error, export display name, and successful-import session identifier in complete monotonic state.
- [ ] 11.2 Implement Rust document operation ownership with bounded pending/history collections, startup-approved staging roots, canonical containment validation, exact-once terminal transitions, expiry, partial-file deletion, and shutdown cleanup.
- [ ] 11.3 Add reachable bounded `PresentImport` and `PresentExport` host commands with allowed content types, maximum import bytes, display name, content type, staging path, and stable completion semantics.
- [ ] 11.4 Update the C ABI, generated header, C harness, Rust JNI registration, Swift decoder, and Kotlin decoder atomically for document commands and completions.
- [ ] 11.5 Replace ad hoc native-import DOM events and split frontend import/export paths with one document-operation controller that selects authenticated browser endpoints or native contract presentation by runtime platform.
- [ ] 11.6 Implement macOS import and export through `FloatViewDocuments.swift` using native open/save surfaces and bounded copying to or from the exact Rust staging path.
- [ ] 11.7 Implement iOS import and export through `FloatViewDocuments.swift` using native picker/share or document-export surfaces, security-scoped access, bounded copying, presentation replacement, and exact-once completion.
- [ ] 11.8 Implement Android import and export through `FloatViewDocuments.kt` using Storage Access Framework/share surfaces, bounded `ContentResolver` copying, the retained `FileProvider`, presentation replacement, and exact-once completion.
- [ ] 11.9 Add Rust/frontend/Swift/Kotlin/conformance coverage for success, cancellation, unavailable presentation, invalid containment, symlink/path escape, oversized input, partial-copy failure, duplicate/stale completion, replacement, expiry, cleanup, and browser/native parity.
- [ ] 11.10 Delete superseded document presenters, file chooser branches, DOM events, providers, resources, or tests only when reachability proves the selected platform path replaces them; retain each required native save/share/import mechanism.

## 12. Background, location, and appearance capabilities

- [ ] 12.1 Implement and document each host's `FloatViewBackgroundExecution` entry so it owns only OS mechanism and effective eligibility while Rust owns durable logging intent and policy.
- [ ] 12.2 Verify Android foreground-service lifecycle, connected-device/location types, notification permission and channel, Application-owned runtime continuity, stop/restart behavior, and lifecycle eligibility reporting.
- [ ] 12.3 Verify iOS background-mode declarations, CoreBluetooth restoration, background-location configuration, WebView-independent observation delivery, and runtime continuity; document macOS process behavior and mobile-only stubs.
- [ ] 12.4 Add a focused location-state sink so Apple delegate authorization/background-availability changes reach Rust immediately without waiting for a coordinate or external poll.
- [ ] 12.5 Route Android permission-result and provider-state changes through the same focused location-state semantics and prove denied, foreground-only, background, disabled, and unavailable transitions.
- [ ] 12.6 Remove unused host-only location timestamps, display reasons, serialization conformances, and duplicate normalization after focused tests prove Rust receives every required primitive.
- [ ] 12.7 Physically verify initial `prefers-color-scheme`, live light/dark change, foreground return, and WebView recreation on iOS, macOS, and Android and record the selected source in each `FloatViewAppearance` entry.
- [ ] 12.8 Keep the frontend media-query source and remove native appearance submission on hosts that pass; on any failing host add only the focused effective light/dark fallback and disable the duplicate frontend source for that host.
- [ ] 12.9 Keep native-only chrome consistent with the selected effective appearance without persisting system appearance as user preference or routing duplicate policy through Rust.
- [ ] 12.10 Remove the unconsumed logging-service event and any duplicate theme synchronization after command completion, lifecycle eligibility, and selected appearance-source tests pass.

## 13. Flatten remaining native coordination

- [ ] 13.1 Replace broad Swift Bluetooth/domain events with focused bridge-facing transport sinks where this removes duplicate mapping without moving CoreBluetooth callback generation, restoration, GATT validation, write pacing, or stale-callback protection into Rust.
- [ ] 13.2 Replace broad Kotlin Bluetooth/domain events with focused bridge-facing transport sinks where this removes duplicate mapping without moving Android permission/radio/GATT callbacks, CCCD compatibility, write pacing, or stale-callback protection into Rust.
- [ ] 13.3 Remove intermediate connection states, duplicate authorization/state normalization, unused model fields, and reverse/test helpers from production hosts when Rust or focused boundary tests already own them.
- [ ] 13.4 Move test-only command injection wrappers into test/debug source sets or explicitly test-gated builds while retaining pure-C/JNI/Swift/Kotlin boundary coverage.
- [ ] 13.5 Extract one shared Apple runtime composition owner for startup inputs, transport selection, bridge worker, location, background capability, presentation replacement, and shutdown; keep iOS and macOS entry files as thin platform shells.
- [ ] 13.6 Prove every host still owns exactly one process runtime and one off-UI-thread pull worker and that WebView/window/Activity replacement does not replace Bluetooth, location, background, document, or durable state owners.

## 14. Reconciled completion gates

- [ ] 14.1 Run the capability-matrix, predictable-entry, support-stub, reachability, ownership, privacy, and portable-shell audits with no `incomplete` capability remaining.
- [ ] 14.2 Run focused document, location-authorization, appearance, lifecycle/background, bridge-ordering, and presentation-replacement conformance on browser, simulator, emulator, iOS, macOS, and Android as applicable.
- [ ] 14.3 Run focused Rust, pure-C, Swift, Kotlin, frontend, simulator, emulator, and browser gates followed by the explicit physical checkpoints in 9.6.
- [ ] 14.4 Record the final production host inventory and capability evidence, update retained dependency/build provenance, and remove every superseded code and layout path whose replacement has passed its host gate.
- [ ] 14.5 Run `just verify`, resolve every failure without weakening the requirements, then sync and archive the change only after all reopened and added tasks are complete.
