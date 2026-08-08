# Baseline evidence

Recorded before implementation on 2026-08-08. Counts exclude Apple `.build`, Swift `Tests`, iOS lifecycle tests, Android `src/test`, Android `src/androidTest`, generated sources, and dependencies.

## Predecessor reconciliation

`share-native-backend-core` is complete at 69/69 tasks but remains an active, unarchived change. Its deltas establish the Rust shared-core ownership on which this change depends. The overlapping capabilities reconcile as follows:

| Capability                | Predecessor result                                                                    | This follow-up                                                                                                                          |
| ------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `platform-hosts`          | Replaces mirrored Swift/Kotlin backend behavior with one Rust core and thin OS hosts. | Adds auditable deletion of remaining software transports, bridge-domain coordination, portable bootstrap work, and unreachable surface. |
| `native-development-loop` | Builds one locked Rust backend for Apple and Android development.                     | Selects the Rust software host for simulator/emulator/browser development and deletes native software peripherals.                      |
| `backend-conformance`     | Exercises the production Rust composition and private C/JNI bridge.                   | Adds one-way bridge, reachability, firmware-UUID identity, privacy, and host-surface audit coverage.                                    |

There is no requirement reversal: implementation uses the predecessor's completed Rust composition and treats this change as a strict narrowing of its host boundary. Canonical spec synchronization must apply/archive `share-native-backend-core` before this change is finally synced and archived. The predecessor is intentionally not moved during apply because archiving delta specs requires a separate sync choice.

## Production source inventory

### Apple: 3,703 lines

| Ownership class                          | Files                                                                               | Lines | Disposition                                                                                  |
| ---------------------------------------- | ----------------------------------------------------------------------------------- | ----: | -------------------------------------------------------------------------------------------- |
| General bridge codec                     | `FloatViewPlatformBridgeCodec.swift`                                                |   574 | Replace with command-only decoding and focused event calls.                                  |
| Intermediate core/Bluetooth coordination | `RustCorePlatformAdapter.swift`, `PlatformPorts.swift`, `VescBluetoothPolicy.swift` |   711 | Move application policy/identity/generations to Rust; retain only OS transport types/policy. |
| C ABI runtime wrapper                    | `FloatViewCoreRuntime.swift`                                                        |   321 | Retain and minimize around the pull worker and focused ABI.                                  |
| Physical Bluetooth                       | `CoreBluetoothAdapter.swift`                                                        |   479 | Retain CoreBluetooth/GATT/write behavior; remove saved/application policy.                   |
| Native software transport                | `SoftwareVescPeripheral.swift`, `SoftwareBluetoothAdapter.swift`                    |   243 | Delete after selecting the Rust software host.                                               |
| Location                                 | `CoreLocationAdapter.swift`                                                         |   166 | Retain OS adapter.                                                                           |
| Resource/document host service           | `ApplePlatformHostServices.swift`                                                   |   323 | Retain reachable OS work; remove portable bootstrap capabilities.                            |
| Native presentation                      | `AppleSystemPresentation.swift`                                                     |   267 | Retain.                                                                                      |
| macOS entry/lifecycle/WebView            | `FloatViewMac.swift`                                                                |   323 | Retain and simplify software-mode composition.                                               |
| iOS entry/lifecycle/WebView              | `RustContentView.swift`, `WebView.swift`, `FloatViewApp.swift`                      |   247 | Retain and simplify software-mode composition.                                               |
| Package declaration                      | `Package.swift`                                                                     |    49 | Retain; remove dependencies only when their owners disappear.                                |

### Android: 3,368 lines

| Ownership class                          | Files                                                                                                             | Lines | Disposition                                                                                  |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ----: | -------------------------------------------------------------------------------------------- |
| General bridge codec                     | `PlatformBridgeCodec.kt`                                                                                          |   377 | Replace with command-only decoding and focused event calls.                                  |
| Intermediate core/Bluetooth coordination | `RustBluetoothPlatformAdapter.kt`, `RustAndroidPlatformAdapter.kt`, `backend/Domain.kt`, `VescBluetoothPolicy.kt` |   698 | Move application policy/identity/generations to Rust; retain only OS transport types/policy. |
| JNI runtime/worker/composition           | `CoreBridgeWorker.kt`, `NativeCore.kt`, `RustBackendRuntime.kt`                                                   |   335 | Retain and minimize around the process runtime and pull worker.                              |
| Physical Bluetooth                       | `AndroidBluetoothAdapter.kt`, `AndroidBluetoothPermissionPolicy.kt`, `VescGattProfile.kt`                         |   696 | Retain Android BLE/GATT/permission/write behavior; remove saved/application policy.          |
| Native software transport                | `SoftwareVescPeripheral.kt`, `SoftwareBluetoothAdapter.kt`                                                        |   244 | Delete after selecting the Rust software host.                                               |
| Location                                 | `AndroidLocationProvider.kt`                                                                                      |   171 | Retain OS adapter.                                                                           |
| Resource/document host service           | `AndroidPlatformHostServices.kt`                                                                                  |   201 | Retain reachable Android work; remove portable bootstrap capabilities.                       |
| Native presentation                      | `AndroidPresentationBroker.kt`                                                                                    |   156 | Retain.                                                                                      |
| Activity/WebView/lifecycle               | `MainActivity.kt`, `FloatViewApplication.kt`                                                                      |   234 | Retain.                                                                                      |
| Background execution                     | `LoggingForegroundService.kt`                                                                                     |   151 | Retain Android foreground-service behavior.                                                  |
| Permission presentation                  | `PermissionBrokers.kt`                                                                                            |   105 | Retain.                                                                                      |

## Current command reachability

The current payload tag is listed because the outer command header currently uses only generic platform kind `1`. `UI` means the host may have to dispatch/present through the platform UI owner; `OS` means adapter/service work; `portable` means the command is a bootstrap round trip selected for removal.

| Tag | Command               | Production requester             | Current completion                    | Host affinity                   | Decision                                                                                   |
| --: | --------------------- | -------------------------------- | ------------------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------ |
|   1 | Bluetooth start scan  | `app-backend`                    | status, immediate acceptance          | OS Bluetooth queue              | Retain as outer command kind.                                                              |
|   2 | Bluetooth stop scan   | `app-backend`                    | status, immediate                     | OS Bluetooth queue              | Retain.                                                                                    |
|   3 | Bluetooth connect     | `app-backend`                    | currently delayed until ready/failure | OS Bluetooth queue              | Retain; completion becomes OS acceptance and Rust owns readiness.                          |
|   4 | Bluetooth disconnect  | `app-backend`                    | status                                | OS Bluetooth queue              | Retain.                                                                                    |
|   5 | Bluetooth write       | `app-coordinator`                | status                                | OS Bluetooth queue/backpressure | Retain.                                                                                    |
|   6 | Location start        | `app-backend`                    | status                                | OS location                     | Retain.                                                                                    |
|   7 | Location stop         | `app-backend`                    | status                                | OS location                     | Retain.                                                                                    |
|   8 | Background logging    | `app-backend`                    | status                                | OS service/lifecycle            | Retain.                                                                                    |
|   9 | Read resource         | `host-runtime` bootstrap only    | bytes, maximum 8 MiB                  | bundle/AssetManager             | Replace bootstrap use; retain a narrower asset path only if still reachable.               |
|  10 | Stage document        | none                             | asynchronous status                   | UI/document API                 | Wire from contract before retaining, otherwise delete.                                     |
|  11 | Resolve storage root  | `host-runtime` bootstrap only    | absolute UTF-8 path                   | portable startup                | Move to startup configuration.                                                             |
|  12 | Available space       | none                             | big-endian `u64`                      | portable filesystem             | Delete unless a consumer is added and Rust owns it.                                        |
|  13 | Monotonic clock       | none                             | big-endian `u64`                      | portable                        | Delete; Rust clock owner.                                                                  |
|  14 | Wall clock            | none                             | big-endian `i64`                      | portable                        | Delete; Rust clock owner.                                                                  |
|  15 | Secure random         | `host-runtime` bootstrap only    | bytes, maximum 256                    | portable                        | Move to Rust random owner.                                                                 |
|  16 | Diagnostic            | none                             | status                                | OS logging                      | Delete; Rust emits bounded codes through the retained diagnostic seam only when reachable. |
|  17 | Present permission    | `app-backend`                    | status                                | UI/permission API               | Retain.                                                                                    |
|  18 | Present settings      | `app-backend`, `app-coordinator` | status                                | UI/settings API                 | Retain.                                                                                    |
|  19 | Present theme         | none                             | status                                | UI                              | Delete unless a requester is added.                                                        |
|  20 | Present service state | none                             | status                                | UI/service                      | Delete unless a requester is added.                                                        |
|  21 | Present export        | none                             | asynchronous status                   | UI/document API                 | Wire from contract before retaining, otherwise delete.                                     |
|  22 | Runtime ready         | `host-runtime` startup/resume    | status                                | presentation                    | Retain.                                                                                    |

Current command payload round-trip coverage lives in `host-ports` unit tests, `FloatViewBackendTests/RustCorePlatformAdapterTests.swift`, `FloatViewCoreBridgeTests`, Android `RustBluetoothPlatformAdapterTest`, `RustAndroidPlatformAdapterTest`, and JNI instrumentation. Reverse host codecs exist mainly to support those tests and are not production call paths.

## Current event reachability

| Event                  | Production source                             | Bound                                                 | Decision                              |
| ---------------------- | --------------------------------------------- | ----------------------------------------------------- | ------------------------------------- |
| Bluetooth state        | OS adapter initial/radio/permission callbacks | fixed enum values                                     | Focused submit.                       |
| Bluetooth device       | scan callback                                 | association <=256 bytes, name <=128 bytes, `i16` RSSI | Focused submit.                       |
| Bluetooth connection   | GATT callbacks                                | fixed generation/state                                | Focused submit.                       |
| Location authorization | location provider                             | fixed enum                                            | Focused submit.                       |
| Location observation   | location provider                             | finite bounded coordinates/accuracy                   | Focused submit.                       |
| Lifecycle              | app/activity/scene callbacks                  | two booleans                                          | Focused submit.                       |
| Logging service        | Android foreground service / host state       | boolean                                               | Focused submit.                       |
| Theme                  | native presentation/lifecycle observation     | fixed enum                                            | Focused submit only while reachable.  |
| Bluetooth notification | GATT notification callback                    | 1..4096 raw bytes                                     | Keep existing dedicated hot-path ABI. |

## Firmware UUID evidence

- Upstream `vendor/bldc/comm/commands.c` handles `COMM_FW_VERSION` by copying exactly 12 bytes from `STM32_UUID_8`; motor thread two increments the final byte so configuration backup/restore identities remain distinct.
- Upstream `vendor/bldc/conf_general.h` defines `STM32_UUID_8` over the MCU UUID address, and CANard also treats it as the unique ID source.
- `backends/core/crates/protocol-vesc/src/firmware.rs` reads the first 12 bytes after the terminated hardware name into `FirmwareRecord.controller_uuid: Option<[u8; 12]>`.
- Existing protocol fixtures prove that structurally valid firmware may contain `[0; 12]`; `controller_discovery::meaningful_uuid` already rejects all-zero values for identity continuity.

Decision: the meaningful non-zero `[u8; 12]` is the private stable saved-board key. All-zero is unavailable and uses the existing Rust-owned private fallback. Platform transport associations remain separate and are revalidated against firmware identity after connection.

## Platform-event ABI comparison

| Option                                      | Host code                                                                                                                 | Rust/C/JNI surface                                                                       | Safety/audit result                                                                                                                      |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| One generic fixed event header plus payload | One call, but hosts pack overloaded integer slots, floating-point bits, optional values, and multi-value device payloads. | One symbol and a 64+ byte header with kind-dependent semantics.                          | Small symbol count but recreates a manual codec and weakens type review. Rejected.                                                       |
| Focused event exports                       | Direct primitive/byte calls; no event writer/reader or tagged envelope.                                                   | Seven or eight small C/JNI entry points delegating immediately to Rust typed validation. | Slightly larger symbol table, materially less Swift/Kotlin code, explicit per-event bounds, and easier call-site reachability. Selected. |

The selected focused exports preserve caller-owned inputs copied before return, fixed-width C values, no allocator sharing, no callback objects, no retained `JNIEnv`, and the existing dedicated BLE-byte path. Command retrieval continues to use the existing 40-byte caller-owned `FloatViewCommandHeader`, with its `kind` changed from generic platform value to the actual command discriminator.
