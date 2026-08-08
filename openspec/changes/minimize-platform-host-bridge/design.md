## Context

The completed `share-native-backend-core` change established one Rust application actor, storage implementation, contract dispatcher, loopback server, protocol stack, C ABI, and Android JNI facade. Its cutover left production hosts at approximately 3,700 lines of Swift across iOS/macOS and 3,400 lines of Kotlin, excluding roughly 1,600 lines of host tests. The largest avoidable groups are mirrored software VESC implementations, general-purpose binary codecs, an intermediate Bluetooth coordination layer, and bootstrap capabilities that standard Rust code can own.

The present bridge transports every platform command inside an outer command header whose `kind` is always the platform-command value, then repeats the runtime generation and operation identifier and adds another command tag inside the payload. Hosts also compile the reverse directions mainly to construct and inspect messages in tests. Swift and Kotlin Bluetooth bridge adapters generate process-random device tokens above already-private platform identifiers, coordinate pending connection operations, and normalize application-facing state before Rust receives it.

There are two distinct identity moments:

```text
BLE discovery                    Valid VESC firmware response
      |                                      |
      v                                      v
private platform association       meaningful 12-byte VESC UUID
(Apple peripheral UUID /           (stable saved-board identity)
 Android BLE address)
      |                                      |
      +---- connect and identify ------------+
```

The platform association is required to address a peer before firmware bytes can be read. It is not board identity. Once Rust parses a meaningful VESC firmware UUID, those exact UUID bytes become the stable private saved-board key selected by this change. Existing privacy-preserving public identities remain separate.

The first implementation pass minimized the bridge before all required platform capabilities were reachable. Rust currently stages session exports but no production host command presents them; native document presenters and Android content-URI support remain disconnected. Apple caches location authorization changes without submitting them to Rust. Logging-service and theme observations cross the private bridge even though the coordinator does not consume them, while the frontend already observes system appearance through `prefers-color-scheme`. The host trees also mix capability entry points, shared Apple implementation, application shells, and bridge coordination in ways that make support status difficult to discover.

This follow-up assumes `share-native-backend-core` is synced and archived, or that its canonical spec deltas are otherwise reconciled first. It deliberately changes the private ABI without compatibility negotiation because all native artifacts ship atomically.

## Goals / Non-Goals

**Goals:**

- Remove the Swift and Kotlin software VESC peripherals and use the existing Rust software host in every software transport mode.
- Make the bridge production-directional: Rust commands flow out through the pull queue; focused native observations and completions flow in.
- Represent command metadata once and delete unreachable command/event/model surface.
- Move secure randomness, clocks, application coordination, and uniformly portable bootstrap/filesystem behavior into Rust.
- Use the meaningful VESC firmware UUID bytes themselves as stable private saved-board identity while preserving a separate native-private platform transport association.
- Keep Bluetooth application policy, pending connection state, saved/guest ownership, public state normalization, and diagnostic normalization in Rust.
- Preserve OS-specific GATT, permission, lifecycle, background service, location, document, resource, WebView, and presentation responsibilities in Swift/Kotlin.
- Define an explicit, discoverable capability model and consistent entry-file convention for iOS, macOS, and Android, including documented stubs for unsupported capabilities.
- Restore native import/export presentation under Rust-owned staging, validation, application state, and cleanup.
- Make location authorization, background eligibility, and effective appearance reachability explicit and remove parallel or unconsumed observation paths.
- Place macOS beside iOS under the Apple host tree and share only genuinely common Apple implementation through an explicit common area.
- Make remaining host ownership auditable and prevent superseded paths from returning.

**Non-Goals:**

- Moving CoreBluetooth, Android Bluetooth/GATT, Core Location, Android location, foreground services, permissions, or native presentation into Rust.
- Making the VESC firmware UUID, Apple peripheral UUID, or Android BLE address a frontend, archive, export, or diagnostic identifier.
- Changing VESC framing, Refloat/BMS protocols, session storage formats, or saved/guest semantics. A narrow hand-written frontend contract delta for native document operations is in scope.
- Adding generated host bindings, UniFFI, callbacks from Rust runtime threads, long-lived JNI references, or a second runtime.
- Requiring a particular final line count. Counts expose trends; ownership and reachability determine acceptance.

## Decisions

### 1. Use a command-kind header and focused event submissions

The outer command header's `kind` becomes the actual platform command discriminator. Its existing generation, operation ID, flags, and payload length remain authoritative and are not repeated inside the payload. Command-specific payloads contain only their arguments, encoded with fixed-width big-endian primitives and bounded length-delimited UTF-8/bytes where required.

Host-to-core observations use focused ABI/JNI operations for:

- Bluetooth radio and authorization state;
- discovered private platform association, optional name, and RSSI;
- connection transport state;
- location authorization and bounded coordinates;
- lifecycle/background eligibility;
- a focused effective-appearance observation only for a host whose WebView cannot reliably supply it; and
- raw Bluetooth notification bytes through the existing hot-path operation.

Logging-service activity is not a separate event while lifecycle already carries effective background eligibility. System appearance has one source per host: either the verified WebView media-query path or the focused native observation, never both.

The C ABI may implement these as a small fixed-layout event header plus payload or as focused exported functions, whichever produces the smaller audited header and wrappers. It must not reintroduce a general object model. One C layout harness and JNI registration test freeze the selected representation before host conversion.

Host command decoders remain hand-written because they map private command arguments directly to OS APIs. Production command encoders and event decoders are deleted. Tests construct commands in Rust or use immutable byte vectors at the ABI boundary and assert host effects rather than maintaining a second reverse implementation.

Alternatives considered:

- Keep the existing symmetric codec and only remove unused methods: lower migration risk, but retains duplicated envelope metadata and most mirrored serialization ownership.
- Generate Swift/Kotlin models: reduces hand-written code but violates the repository's small hand-written boundary and broadens build/tooling ownership.
- Use callbacks or callback vtables: removes the blocking waiter but creates reentrancy, lifetime, JVM attachment, and shutdown hazards prohibited by the existing architecture.

### 2. Keep only commands with reachable OS effects

Before freezing the new kind table, implementation records every Rust request site, host dispatch branch, completion shape, maximum payload, UI-thread requirement, and conformance scenario. A command survives only if a production path needs an asynchronous OS effect. Retained families are physical Bluetooth, location start/stop and settings/permission presentation, background-service control, native document import/export presentation, runtime readiness, and packaged-resource access that cannot be supplied at startup.

Native document presentation uses bounded `PresentImport` and `PresentExport` commands. Rust creates exact staging paths beneath startup-validated roots and supplies only the destination or staged path, maximum import bytes, allowed content types, suggested display name, and content type required by the OS surface. Hosts validate that a received path remains beneath the configured staging root, present the platform UI, copy through security-scoped URLs or content URIs in bounded chunks when required, and complete with success, cancelled, unavailable, or failed. They never interpret archive contents.

The hand-written frontend contract initiates a bounded document operation and receives an operation identifier. Complete monotonic state snapshots report presenting, completed, cancelled, or failed state; a successful import includes the Rust-created session identifier. Browser development may use the authenticated upload/download endpoints behind the same frontend controller. Rust owns operation correlation, archive generation/import, validation, staging expiry, cleanup of partial and completed files, and all application-facing errors.

Secure randomness and clocks move to existing Rust owners. Storage roots become validated runtime startup inputs. Available-space work moves to Rust only if it has a reachable consumer and can be implemented with the existing dependency graph; otherwise the unused command is deleted. SQL and software fixtures become Rust-owned compile-time or core resources. Android `AssetManager` and Apple bundle lookup remain host capabilities only for packaged assets not reasonably represented by a validated startup file or copied bounded bytes.

No new dependency is planned. If portable available-space reporting later requires a crate, that dependency needs a separate documented decision and must replace more code than it adds.

### 3. Use the VESC firmware UUID bytes as private stable board identity

Rust's existing VESC firmware parser owns the fixed-width controller/device UUID. For a valid, meaningful non-zero UUID, the exact 12 bytes become the canonical private saved-board logical key. Storage may serialize those bytes canonically for indexing, but public `savedDeviceId` and `deviceHistoryId` remain privacy-preserving derived identifiers; the raw UUID does not cross the frontend contract or archive boundary.

Before identification, discovery and connect commands use a separate private platform association:

- Apple supplies the canonical CoreBluetooth peripheral UUID bytes/text used by `retrievePeripherals` and current scan callbacks.
- Android supplies the canonical BLE address used by `BluetoothAdapter` and current scan callbacks.

The association is copied and bounded by the bridge, retained only in private native/core metadata, and never logged. It replaces the current process-random bridge token, so the same peer can be addressed after process recreation. Rust attaches the association to a saved board only after saved-mode firmware identification. On every reconnect it revalidates the firmware UUID before accepting the association as the same board.

An all-zero, absent, or invalid firmware UUID follows the existing no-meaningful-identity fallback: saved mode creates a private random logical identity from the Rust random owner and makes no automatic cross-device match. Guest mode persists neither identity nor association.

This is deliberately not a portable archive identity. Imported boards continue to use the public privacy-preserving `deviceHistoryId`; matching an imported entry to a physical VESC occurs inside Rust after firmware identification according to the existing saved-identity rules.

Alternatives considered:

- Hash the firmware UUID before private storage: adds indirection without protecting any additional public surface because raw firmware identity is already required transiently in Rust; public and diagnostic boundaries remain redacted.
- Use Apple/Android transport identifiers as board identity: they differ across platforms and can change independently of the VESC controller.
- Retain process-random discovery tokens: prevents stable saved-device lookup and incorrectly makes runtime transport state determine durable identity.

### 4. Move Bluetooth coordination down to Rust and OS transport up to the adapter

Native Bluetooth adapters expose the smallest transport vocabulary possible. They own OS callback serialization, service/characteristic validation, notification enablement, negotiated write chunking, platform write backpressure, permissions, and resolution of the private platform association to an OS object. They report transport observations and raw bytes.

Rust owns discovered-device deduplication, pending application connect/disconnect intent, connection generations, saved/guest mode, firmware UUID validation, state normalization, operation correlation, bounded diagnostic codes, reconnect policy, and durable association. Command completion means the host accepted or rejected the requested OS operation; readiness and failure are driven by subsequent ordered transport events, not host-side application state.

The current broad Swift/Kotlin `BluetoothEvent`, `ConnectionState`, device-token maps, and duplicate policy helpers are reduced to types required to express OS callbacks. Transport-specific choices such as Android CCCD compatibility, CoreBluetooth restoration, write-with/without-response behavior, MTU/chunk size, and queue draining remain native.

### 5. Select the Rust software host at startup

Runtime startup includes a bounded transport mode: `physical` or `software`. Physical mode emits Bluetooth commands to the native queue. Software mode routes those same commands to the Rust software host internally and emits no native Bluetooth command. Both modes share the same application actor, contract dispatcher, stores, loopback server, lifecycle, and shutdown graph.

The Rust software adapter/peripheral and source-derived fixtures are the only software VESC implementation. Swift simulator, software-selected macOS, and Android emulator composition select software mode and do not construct Bluetooth ports. Production device builds either omit the software-host feature or make it unreachable outside explicit debug/development configuration; physical conformance always selects physical mode.

### 6. Pass startup-owned values once

The runtime creation/start configuration carries copied bounded values for requested port, physical/software mode, durable/cache/temporary roots, and the minimal packaged resource inputs or access mode needed by the core. Rust validates absolute owned roots before creating stores and validates resource size/name before server startup.

The launch token comes from Rust secure randomness. Application monotonic and wall clocks use Rust production clock implementations; deterministic conformance injects deterministic Rust clocks without expanding the native ABI. This removes command/complete round trips during bootstrap and keeps runtime readiness as the first host presentation command after the complete graph is available.

### 7. Audit ownership, not a line-count quota

Verification adds an ownership audit that excludes `Tests`, `src/test`, `src/androidTest`, `.build`, generated sources, and dependencies when reporting production Swift/Kotlin counts. It also checks for banned superseded files/symbols and cross-references command kinds with Rust requesters and host dispatchers.

Expected production reductions are approximately 900–1,200 Swift/Kotlin lines combined from software transports and codec directionality, with further reductions from bootstrap and Bluetooth-layer collapse. These estimates are informational. Tests remain when they prove OS behavior or ABI safety; duplicate tests of Rust-owned policy are deleted or moved to Rust.

### 8. Give every native host the same discoverable capability shape

The native capability families are Bluetooth transport, background execution, location, native documents, system appearance, lifecycle/WebView ownership, and the runtime bridge. Each host records each family as `implemented`, `unsupported`, or `incomplete`. `Implemented` requires a production entry point and focused verification. `Unsupported` requires a small compiling stub that explains the platform limitation and returns a stable unavailable result. `Incomplete` is visible during development but fails the final ownership/conformance gate.

`backends/HOST_CAPABILITIES.md` is the cross-host matrix. Each host directory also contains a README that maps every capability to its status, discoverable entry file, implementation owner, and focused test or physical checkpoint. Missing rows or files fail the audit.

Capability entry names use `Location`, not `Gps`, because the boundary includes authorization, provider availability, accuracy, and background behavior. The required entry names are:

- `FloatViewBluetooth`;
- `FloatViewBackgroundExecution`;
- `FloatViewLocation`;
- `FloatViewDocuments`;
- `FloatViewAppearance`;
- `FloatViewLifecycle`;
- `FloatViewWebView`; and
- `FloatViewRuntimeBridge`.

Apple uses sibling `backends/apple/ios/` and `backends/apple/macos/` trees with those Swift entry files and an explicit `backends/apple/common/` area for genuinely shared implementation. A platform entry file may be a small factory or forwarding seam into `common/`, but it names the concrete owner. Android places the matching Kotlin entry files under `app/src/main/java/fail/acheron/floatview/host/`. Supporting private types may use focused subdirectories; generic `Domain`, `Services`, `Utils`, or `PlatformAdapter` files do not become miscellaneous ownership buckets.

Alternatives considered:

- Organize only by technical layer: keeps existing package conventions but makes a capability span unrelated directories and hides unsupported work.
- Omit unsupported files: reduces a few lines but makes absence indistinguishable from unfinished work.
- Duplicate shared Apple code under both hosts: improves local reading at the cost of behavioral drift; explicit forwarding entry points retain discoverability without duplication.

### 9. Treat background execution as a first-class native capability

`FloatViewBackgroundExecution` owns the platform mechanism and documents how it keeps the process-owned Rust runtime eligible to receive BLE and location work. On iOS this includes declared background modes, CoreBluetooth restoration, and background-location configuration. On Android it includes foreground-service lifecycle, service types, notification requirements, and the relationship to the application-owned runtime. On macOS it documents normal process/lifecycle behavior and provides stable unsupported results for mobile-only requests.

Rust decides when durable logging starts and stops. Hosts report lifecycle and effective background eligibility. A separate logging-service event is absent unless a future Rust consumer demonstrates state that cannot be represented by command completion plus lifecycle eligibility.

### 10. Deliver location authorization changes directly

Location capability entry points submit authorization and background-availability changes whenever the OS delegate or permission-result callback changes them; they do not merely cache state for a later poll. Coordinate observations contain only bounded latitude, longitude, and optional accuracy. Observation timestamps, display reasons, recording policy, persistence, export inclusion, and frontend normalization remain in Rust unless an OS API requires a timestamp for ordering, in which case the bridge carries a bounded monotonic ordering value rather than a host application model.

### 11. Use one effective-appearance source per host

The persisted `ThemePreference` remains Rust/frontend application state. Effective system appearance is transient presentation state. Each host first verifies the frontend's existing `prefers-color-scheme` initial read and change subscription across foreground return and WebView recreation. When that path is reliable, `FloatViewAppearance` documents the WebView as the owner and contains only native-chrome handling required by the host. When it is not reliable, `FloatViewAppearance` submits one focused effective `light` or `dark` observation and the duplicate media-query subscription is disabled for that host.

Native-only chrome such as Android system bars may observe and apply platform appearance locally without routing persisted preference through Rust. A host must not maintain simultaneous WebView and native effective-appearance sources.

## Risks / Trade-offs

- **[Firmware UUID is available only after connection]** → Keep platform transport association explicitly separate, attach it only after valid saved-mode identification, and revalidate UUID on reconnect.
- **[A VESC reports an all-zero or cloned UUID]** → Treat all-zero as unavailable, preserve the existing random private fallback, test duplicate UUID behavior, and never silently merge two concurrently observed transport associations without an explicit conflict result.
- **[Android addresses can rotate for privacy-enabled peripherals]** → Treat the address as a replaceable private association rather than logical identity; the firmware UUID remains authoritative after connection and a later verified association can replace the old one.
- **[Raw private identifiers leak during diagnostics]** → Centralize redaction in Rust, keep adapter diagnostics to bounded stage/code values, and add byte/text privacy sweeps for firmware UUIDs and platform associations.
- **[A larger set of focused ABI calls broadens the symbol table]** → Compare fixed event-header and focused-function variants using the C/JNI harness and select the smallest auditable representation; every symbol must have one production caller.
- **[Removing reverse codecs makes host tests less convenient]** → Drive bridge messages from Rust fixtures/harnesses and assert OS adapter effects; keep only immutable boundary vectors needed to verify host decoding.
- **[Atomic ABI conversion temporarily breaks one host]** → Land the Rust header/JNI and both host conversions as one coherent verified implementation phase; no compatibility path ships or remains afterward.
- **[Software behavior differs from prior native simulators]** → Make Rust software-host parity part of canonical acceptance before deleting native implementations.
- **[Existing development databases use random-token board IDs]** → Bump/recreate disposable test-stage storage rather than attempt a heuristic identity migration.
- **[Capability entry files become ceremonial wrappers]** → Require each entry to name its concrete owner, focused verification, and support status; reject forwarding chains that do not improve discoverability.
- **[Native document paths escape the staging root]** → Rust allocates paths beneath validated roots, hosts revalidate containment before access, payloads remain bounded, and Rust removes partial or expired files.
- **[A picker or share surface outlives its WebView or Activity]** → Track document operations in the process-owned Rust runtime, replace only presentation routing, and complete or cancel exactly once when presentation disappears.
- **[Two appearance sources race]** → Select one verified source per host and make duplicate source registration an audit failure.
- **[Background capability appears enabled but the OS cannot sustain it]** → Report effective eligibility separately from user intent and cover real suspend/resume, restoration, notification, BLE, and location behavior in physical checkpoints.
- **[Moving macOS breaks package or resource discovery]** → Change package paths and resources atomically, build both Apple hosts before deleting old paths, and retain forwarding files only for the duration of the cutover.

## Migration Plan

1. Sync/archive or otherwise reconcile `share-native-backend-core`, then capture the current production host inventory, bridge reachability table, ABI layouts, and saved-device behavior as evidence.
2. Add Rust tests for the selected command header/event ABI, production-direction calls, startup configuration, UUID identity, private platform association, redaction, software transport selection, and shutdown.
3. Update the C ABI, pinned generated header, C harness, JNI registration, and build identity atomically; convert the Swift runtime and Android worker while keeping physical Bluetooth behavior unchanged.
4. Move clocks/random/startup inputs and reachable resources into Rust, prune unused command kinds, and prove cold start, suspend/resume, restart, and readiness behavior.
5. Route simulator/emulator/software modes through the Rust software host, pass canonical software scenarios, and delete both native software peripheral/adapter pairs and their fixtures/dependencies.
6. Replace randomized discovery tokens with bounded private platform associations, resolve saved identity from the VESC firmware UUID, recreate incompatible test data, and pass saved/guest/restart/privacy conformance.
7. Define the capability matrix and stable entry files; move macOS beside iOS, introduce the Apple common area, update build/resource paths atomically, and delete old layout paths after both Apple hosts build.
8. Add Rust document-operation state and reachable import/export commands, connect Apple and Android presentation/copying, update the targeted frontend contract, and pass native and browser import/export conformance before removing superseded document paths.
9. Deliver location authorization changes directly, select and verify one appearance source per host, and remove unconsumed logging/theme event paths.
10. Collapse intermediate Bluetooth coordination and models into Rust plus the OS adapters, preserving GATT/write/lifecycle/background behavior and passing focused host tests.
11. Reopen previously completed tasks whose document/layout/appearance acceptance is not met. Run the portable host ownership/reachability audit, `just verify`, and explicit iOS/Android/macOS physical BLE, background, location, document, and appearance checkpoints. Delete every superseded path only after its host gate passes.
12. Sync the delta specs and archive the change together after the complete gate. Rollback is a source-level revert of the atomic private-ABI and layout changes plus recreation of disposable storage; mixed old/new native artifacts are unsupported.

## Open Questions

- Confirm from the retained firmware sources and all supported identity fixtures that the meaningful VESC firmware device UUID is always the existing 12-byte field parsed by `protocol-vesc`; any additional firmware UUID form requires a separate compatibility decision rather than heuristic normalization.
- During implementation, choose between one fixed-layout platform-event header and several focused event exports based on measured header/JNI/wrapper size and the C harness. Both must satisfy the same one-way requirements and no-callback boundary.
- Confirm whether each physical WebView reports live `prefers-color-scheme` changes reliably; only failing hosts retain the focused native effective-appearance observation.
