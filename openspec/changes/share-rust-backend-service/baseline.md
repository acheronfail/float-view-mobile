# Native ownership baseline

Measured at planning commit `fae1776b`, 2026-09-28. This is implementation
inventory for task 1.1, not evidence of a completed cutover. Machine-readable
measurements and input hashes are in [baseline.json](baseline.json).

## Selected call paths

- iOS: `backends/apple/ios/float-view/application/AppBackend.swift` creates
  `SQLiteLogStore`, recovers interrupted sessions, loads settings, and creates
  `ApplicationService` with Bluetooth, location and scheduler adapters. It passes
  that application owner to `LoopbackServer`.
- macOS: `backends/apple/macos/application/MacBackendRuntime.swift` performs the
  same composition and retains the application independently of its listener.
  Environment-controlled storage/schema paths and profile seeding are software
  conformance integration, not additional production owners.
- Android: `backends/android/app/src/main/java/fail/acheron/floatview/application/BackendRuntime.kt`
  creates and recovers the store and constructs the application owner with its OS
  adapters. `lifecycle/FloatViewApplication.kt` creates the listener with this
  retained runtime. Emulator Bluetooth selection is existing test integration.
- Both listeners use `transport/RustLoopbackHost` to drain Rust server operations
  through `transport/ServerOperations`. Portable requests call the native
  application/store; history handles then return to shared Rust. State publication
  reconstructs native domain facts through `transport/ServerFacts`.
- Both application owners instantiate `application/RustApplicationRuntime` once.
  `prepareRustActivation` ensures a saved board and session before attach/activate;
  guest activation skips durable storage. Android explicitly releases the
  framework SQLite session writer before Rust opens the session WAL.
- The shared runtime already owns protocol/domain kernels and the live session
  writer. Reuse this owner and the independent shared session readers. Do not
  replace a native store with a second application runtime.

## Exact deletion and extraction inventory

Paths below are relative to Apple
`backends/apple/Sources/FloatViewBackend/` and Android
`backends/android/app/src/main/java/fail/acheron/floatview/` respectively.
Counts include comments/blank lines using the convergence audit's convention.

| Path stem                            | Swift lines | Kotlin lines | Cutover action                                                                                                                                               |
| ------------------------------------ | ----------: | -----------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `storage/SQLiteLogStore`             |       2,881 |        2,638 | Delete after shared product storage/archives/recovery selection; extract Apple root protection into native startup first.                                    |
| `application/ApplicationService`     |       1,949 |        1,403 | Replace portable activation, identity, reconnect, location eligibility, request waiters and runtime-result policy with service composition and OS execution. |
| `application/RuntimeRecords`         |         245 |          124 | Delete portable runtime/product record translations; retain only types actually crossing the OS boundary.                                                    |
| `transport/ServerOperations`         |         275 |           75 | Delete domain dispatch and native archive staging/encoding; move actual file presentation/access into OS adapters.                                           |
| `transport/ServerFacts`              |          95 |           62 | Delete domain snapshot reconstruction; encode only closed OS facts in the selected host exchange.                                                            |
| `application/Domain`                 |       1,538 |          852 | Remove product, session, configuration and telemetry models/store ports; retain/rehome Bluetooth, timer, clock, location and event adapter types.            |
| `application/RustApplicationRuntime` |         112 |          160 | Adapt existing C/JNI wrapper to service lifetime; remove superseded entry points after last user is removed.                                                 |
| `storage/LogSchema`                  |          23 |           18 | Move marker version/reset and Apple storage thresholds into Rust; delete native files.                                                                       |
| `storage/StoragePolicy`              |           — |           16 | Move free-space threshold decisions into Rust; delete native file.                                                                                           |

The first five rows total **9,747** lines. They are an investigation surface,
not a promise to delete 9,747 lines: the application and operation files contain
real OS execution that must survive extraction.

Also update `transport/RustLoopbackHost`, `LoopbackServer` and Apple's
`ServerExchange` to attach the existing service handle. Replace native
`ProductStorePort`, `SessionStorePort`, `LogStorePort`, control/configuration
waiters and broad domain serialization after callers have moved. Tests that
currently exercise those owners must move with their behavior, not be discarded.

## Retained responsibilities and exceptions

| Native responsibility/location                                                                                       | OS capability retained                                                                     | Portable ownership to remove                                                                  |
| -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------- |
| `bluetooth/CoreBluetoothAdapter`, `AndroidBluetoothAdapter`, permission brokers, GATT profile/policy                 | Radio/permission observations, discovery, GATT transport, bounded byte I/O and callbacks   | Product discovery expiry, saved/guest decisions, board identity, retry budgets/backoff        |
| Clock/scheduler implementations currently in `application/Domain`                                                    | Wall/monotonic readings, arm/cancel actual OS timers                                       | Deadline selection, retry policy and generation decisions                                     |
| `location/CoreLocationAdapter`, `AndroidLocationProvider`, `LocationPermissionBroker`                                | Provider acquisition, platform authorization/lifecycle facts, settings/prompt execution    | Recording eligibility, session elapsed assignment, persistence/publication                    |
| Apple host startup; Android `BackendRuntime`/application context                                                     | App-private root selection, storage availability facts, Apple protection/backup attributes | Schema/reset, catalog/session filenames and SQL                                               |
| Apple `files` presenters/import controllers; Android `files/ExportPresentationBroker`, `SessionFileImportController` | Security-scoped/URI access, picker/save/share UI, bounded external copying                 | Manifest parsing, CSV/archive encoding, private staging and lease policy                      |
| Apple host lifecycle and Android `LoggingForegroundService`/`FloatViewApplication`                                   | OS background execution, notification/process lifetime and lifecycle observations          | Session retention/close, reconnect, portable terminal-failure decisions                       |
| Host `theme` and `ui`                                                                                                | Native appearance, WebView/window/Activity lifecycle                                       | Authoritative settings and product snapshots                                                  |
| `protocol/*/SoftwareVescPeripheral`, software Bluetooth adapters, fixture package                                    | Existing software-test integration only                                                    | No new product service in these adapters; canonical simulation/assertions stay in conformance |

Native capability branching (permission APIs, platform background availability,
scoped access and file protection) is legitimate. CoreBluetooth/CoreLocation,
Android BLE/location APIs, Foundation, UI frameworks and required lifecycle
dependencies remain. `CryptoKit`/`MessageDigest` board/cache hashing and native
SQLite imports are deletion targets after their last portable user moves.
`CSQLite` is also used by tests; remove the production dependency and separately
assess test requirements. Keep exactly the existing application-runtime Apple
and Android binding artifact families, including their audited C/JNI surfaces.

## Source, dependency and package baseline

`backends/core/audit-migration-convergence.sh` passed and reported 32,760 Rust,
9,490 Apple and 7,672 Android production lines: **49,922 total**, 26 workspace
members, two binding crates, zero temporary bridge crates, one Apple artifact
family and eight Android ABI/profile artifacts. Its existing ceiling is looser
than this change's final target; task 8.2 must tighten it to fewer than 49,922
lines and at most 28 members after native deletions.

The Rust lock contains 105 external package records. Reuse existing locked
`rusqlite 0.37.0` (`bundled`, `hooks`), `libsqlite3-sys 0.35.0`,
`serde 1.0.229`, `serde_json 1.0.151`, `sha2 0.11.0`, `getrandom 0.3.4`,
`rand 0.9.5`, `flate2 1.1.9` and `tokio 1.53.1` as needed; no dependency was
added for this inventory. Apple has a local Refloat fixture package and system
SQLite/runtime libraries. Android uses core-ktx 1.10.1 and coroutines 1.9.0;
its JVM tests additionally use JUnit and org.json. Android min SDK is 29.

Existing cached packages: Debug APK 56,030,089 bytes; unsigned Release APK
29,571,840 bytes; iOS simulator app regular files 66,828,482 bytes; iOS device
app regular files 34,402,922 bytes. The JSON records executable/archive/ABI
sizes and SHA-256 hashes separately. These are cached output measurements,
**not clean packaging evidence or proof that each binary matches this commit**.
Final comparisons must identify build mode, architecture, signing and stripping.

Hashes of the contract, both canonical SQL files, lock and native build
definitions are recorded to detect unintended format/dependency changes.
The schema marker is `11` on both hosts; archive metadata format is `1`.

## Follow-up

The behavior audit found a pre-existing board-identity mismatch resolved by the user’s
pre-release disposable-data policy; see [behavior-audit.md](behavior-audit.md).
Tasks 1.2 and 1.3 remain incomplete. No production owner has changed.
