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
- Make remaining host ownership auditable and prevent superseded paths from returning.

**Non-Goals:**

- Moving CoreBluetooth, Android Bluetooth/GATT, Core Location, Android location, foreground services, permissions, or native presentation into Rust.
- Making the VESC firmware UUID, Apple peripheral UUID, or Android BLE address a frontend, archive, export, or diagnostic identifier.
- Changing VESC framing, Refloat/BMS protocols, frontend envelopes, session storage formats, or saved/guest semantics.
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
- logging-service and theme observations; and
- raw Bluetooth notification bytes through the existing hot-path operation.

The C ABI may implement these as a small fixed-layout event header plus payload or as focused exported functions, whichever produces the smaller audited header and wrappers. It must not reintroduce a general object model. One C layout harness and JNI registration test freeze the selected representation before host conversion.

Host command decoders remain hand-written because they map private command arguments directly to OS APIs. Production command encoders and event decoders are deleted. Tests construct commands in Rust or use immutable byte vectors at the ABI boundary and assert host effects rather than maintaining a second reverse implementation.

Alternatives considered:

- Keep the existing symmetric codec and only remove unused methods: lower migration risk, but retains duplicated envelope metadata and most mirrored serialization ownership.
- Generate Swift/Kotlin models: reduces hand-written code but violates the repository's small hand-written boundary and broadens build/tooling ownership.
- Use callbacks or callback vtables: removes the blocking waiter but creates reentrancy, lifetime, JVM attachment, and shutdown hazards prohibited by the existing architecture.

### 2. Keep only commands with reachable OS effects

Before freezing the new kind table, implementation records every Rust request site, host dispatch branch, completion shape, maximum payload, UI-thread requirement, and conformance scenario. A command survives only if a production path needs an asynchronous OS effect. Expected retained families are physical Bluetooth, location start/stop and settings/permission presentation, background-service control, document staging/export presentation when reachable from the contract, runtime readiness, and packaged-resource access that cannot be supplied at startup.

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

## Migration Plan

1. Sync/archive or otherwise reconcile `share-native-backend-core`, then capture the current production host inventory, bridge reachability table, ABI layouts, and saved-device behavior as evidence.
2. Add Rust tests for the selected command header/event ABI, production-direction calls, startup configuration, UUID identity, private platform association, redaction, software transport selection, and shutdown.
3. Update the C ABI, pinned generated header, C harness, JNI registration, and build identity atomically; convert the Swift runtime and Android worker while keeping physical Bluetooth behavior unchanged.
4. Move clocks/random/startup inputs and reachable resources into Rust, prune unused command kinds, and prove cold start, suspend/resume, restart, import/export, and readiness behavior.
5. Route simulator/emulator/software modes through the Rust software host, pass canonical software scenarios, and delete both native software peripheral/adapter pairs and their fixtures/dependencies.
6. Replace randomized discovery tokens with bounded private platform associations, resolve saved identity from the VESC firmware UUID, recreate incompatible test data, and pass saved/guest/restart/privacy conformance.
7. Collapse intermediate Bluetooth coordination and models into Rust plus the two OS adapters, preserving GATT/write/lifecycle behavior and passing focused host tests.
8. Run the host ownership/reachability audit, `just verify`, and the explicit iOS/Android/macOS physical BLE checkpoint. Delete every superseded path only after its host gate passes.
9. Sync the delta specs and archive the change together after the complete gate. Rollback is a source-level revert of the atomic private-ABI change plus recreation of disposable storage; mixed old/new native artifacts are unsupported.

## Open Questions

- Confirm from the retained firmware sources and all supported identity fixtures that the meaningful VESC firmware device UUID is always the existing 12-byte field parsed by `protocol-vesc`; any additional firmware UUID form requires a separate compatibility decision rather than heuristic normalization.
- During implementation, choose between one fixed-layout platform-event header and several focused event exports based on measured header/JNI/wrapper size and the C harness. Both must satisfy the same one-way requirements and no-callback boundary.
