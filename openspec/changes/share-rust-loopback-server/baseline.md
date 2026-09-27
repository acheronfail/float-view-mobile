# Loopback migration baseline

Captured before implementation at `fae1a4d4` on 2026-09-27. This is an inventory, not passing migration evidence.

## Routes

| Route | Authentication | Payload and result |
| --- | --- | --- |
| GET / | None | Identical embedded HTML; text/html; no-store |
| GET /api/bootstrap | None | webSocketUrl, sessionDataUrl, sessionExportUrl, sessionImportUrl, sessionToken, platform; application/json; no-store |
| GET /api/ws | Exact Origin and query token | Correlated id/result/error and complete state.changed; 65,536-byte messages |
| POST /api/session-data | Exact Origin and X-Float-View-Token | 16,384-byte request; 4 MiB decoded JSON response; optional gzip |
| POST /api/session-import | Same header authentication | 128 MiB archive; JSON sessionId on success |
| POST /api/session-export | Same header authentication | 16,384-byte JSON request; CSV or application/octet-stream with attachment filename |

Ports: macOS 11010, iOS 11011, Android 11012; bind 127.0.0.1. Protected HTTP rejection is 403; size rejection is 413; invalid session request is 400. Unexpected session-data failures use 500. Unknown content paths must return 404.

## Control method and host-call inventory

All 34 BackendMethods keys occur in both native dispatchers and canonical mandatoryMethods. Presence in mandatoryMethods is not proof that every failure path is exercised. Calls below are mechanically extracted from each complete dispatch branch, including reads used for validation; private helpers are inventoried separately.

| Method | Swift application calls | Kotlin application calls |
| --- | --- | --- |
| `state.get` | Shared projection/private helper | Shared projection/private helper |
| `bluetooth.requestPermission` | `requestBluetoothPermission` | `requestBluetoothPermission` |
| `bluetooth.scan` | `startDiscovery`, `stopDiscovery` | `startDiscovery`, `stopDiscovery` |
| `location.requestPermission` | `requestLocationPermission` | `requestLocationPermission` |
| `location.openSettings` | `openLocationSettings` | `openLocationSettings` |
| `device.connect` | `activeConnectionSnapshot`, `connect` | `activeConnectionSnapshot`, `connect` |
| `device.selectController` | `selectController` | `selectController` |
| `device.disconnect` | `disconnect` | `disconnect` |
| `controller.shutdown` | `shutdownController` | `shutdownController` |
| `refloat.control.set` | `setRefloatControl` | `setRefloatControl` |
| `refloat.fatalError.clear` | `clearRefloatFatalError` | `clearRefloatFatalError` |
| `refloat.configuration.open` | `refloatConfigurationSummarySnapshot` | `refloatConfigurationSummarySnapshot` |
| `refloat.configuration.schema` | Shared projection/private helper | Shared projection/private helper |
| `refloat.configuration.read` | `readRefloatConfiguration` | `readRefloatConfiguration` |
| `refloat.configuration.write` | `writeRefloatConfiguration` | `writeRefloatConfiguration` |
| `refloat.tune.list` | `refloatTunes` | `refloatTunes` |
| `refloat.tune.get` | `refloatTune` | `refloatTune` |
| `refloat.tune.create` | `createRefloatTune` | `createRefloatTune` |
| `refloat.tune.update` | `updateRefloatTune` | `updateRefloatTune` |
| `refloat.tune.delete` | `deleteRefloatTune` | `deleteRefloatTune` |
| `refloat.tune.apply` | `applyRefloatTune` | `applyRefloatTune` |
| `savedDevice.update` | `updateSavedDevice` | `updateSavedDevice` |
| `savedDevice.delete` | `deleteSavedDevice` | `deleteSavedDevice` |
| `settings.update` | `updateSettings` | `updateSettings` |
| `boardProfile.get` | `boardProfile` | `boardProfile` |
| `boardProfile.update` | `updateBoardProfile` | `updateBoardProfile` |
| `session.list` | `savedDevices`, `sessions` | `savedDevices`, `sessions` |
| `session.get` | `session`, `sessionExtensionFields`, `sessionPruneMetadata`, `sessionSnapshot` | `session`, `sessionExtensionFields`, `sessionPruneMetadata`, `sessionSnapshot` |
| `session.export` | `exportSession` | `exportSession` |
| `session.delete` | `deleteSession` | `deleteSession` |
| `session.delete.cancel` | `cancelPendingDelete` | `cancelPendingDelete` |
| `session.prune.preview` | `maintenanceJob`, `prunePreview`, `saveMaintenanceJob` | `maintenanceJob`, `prunePreview`, `saveMaintenanceJob` |
| `session.prune.start` | `maintenanceJob`, `pruneSession`, `saveMaintenanceJob` | `maintenanceJob`, `pruneSession`, `saveMaintenanceJob` |
| `session.prune.cancel` | `cancelMaintenanceJob` | `cancelMaintenanceJob` |

## Historical data and serializers

The four kinds in session-data.ts are series, samples (window), events, and locations. Every result echoes sessionId, published snapshot, and elapsed range. Series returns field descriptors and aligned elapsedMs/sequence/values/generations columns plus gapPolicy; samples returns field descriptors and aligned elapsedMs/sequence/values with nextCursor; events returns a page of elapsedMs/sequence/sourceSampleSequence/kinds/values with nextCursor; locations returns points, gaps, locationBoundary and nextCursor. History and catalog cursors bind their query and published/revision context. The runtime owns published commit boundaries; the server must not read beyond them.

Complete native helper inventory (includes serializers, parsers, cursor transforms, error projection, and lifecycle helpers; delete or relocate responsibilities during cutover):

### LoopbackServer.swift

`loopbackStartupFailure`, `start`, `stop`, `handle`, `success`, `stateChanged`, `snapshot`, `bluetoothPayload`, `refloatConfigurationSummaryPayload`, `refloatConfigurationIdentityPayload`, `refloatConfigurationIdentity`, `boundedRefloatText`, `refloatTunePayload`, `refloatConfigurationSchemaPage`, `refloatConfigurationFieldPayload`, `refloatConfigurationValuePayload`, `refloatConfigurationValuesPayload`, `refloatConfigurationValues`, `devicePayload`, `connectionPayload`, `refloatControlsPayload`, `controllerActionsPayload`, `bmsConnectionPayload`, `bmsStatePayload`, `bmsSnapshotPayload`, `currentLimitsPayload`, `liveLocationPayload`, `controllerRoutePayload`, `controllerCandidatePayload`, `connectionIssuePayload`, `savedDevicePayload`, `liveSamplePayload`, `sessionEventPayload`, `boardProfilePayload`, `sessionMetadataPayload`, `sessionDataPayload`, `decodeSessionCursor`, `encodeSessionCursor`, `operationPayload`, `maintenanceJobPayload`, `pruneSessionIDs`, `gzip`, `logSessionDataTiming`, `sessionPayload`, `seriesPayload`, `settingsPayload`, `locationPayload`, `deviceState`, `failure`, `applicationFailure`, `productFailure`, `sessionFailure`, `json`, `jsonString`, `randomToken`, `encodeSessionCatalogCursor`, `decodeSessionCatalogCursor`, `makeMessages`, `invalidatesState`, `set`, `get`

All application-service calls (also covers route handlers, snapshots and nested history helpers):

`activeConnectionSnapshot`, `applyRefloatTune`, `boardProfile`, `cancelMaintenanceJob`, `cancelPendingDelete`, `clearRefloatFatalError`, `connect`, `createRefloatTune`, `deleteRefloatTune`, `deleteSavedDevice`, `deleteSession`, `disconnect`, `discoverySnapshot`, `exportSession`, `firmwareSnapshot`, `importSessionArchive`, `locationState`, `maintenanceJob`, `openLocationSettings`, `prunePreview`, `pruneSession`, `querySessionLocations`, `querySessionSeriesSource`, `readRefloatConfiguration`, `reduceSessionSeries`, `refloatConfigurationSchemaSnapshot`, `refloatConfigurationSummarySnapshot`, `refloatPackageVersionSnapshot`, `refloatTune`, `refloatTunes`, `requestBluetoothPermission`, `requestLocationPermission`, `saveMaintenanceJob`, `savedDevices`, `selectController`, `session`, `sessionEvents`, `sessionExtensionFields`, `sessionHistoryConfiguration`, `sessionPruneMetadata`, `sessionSnapshot`, `sessions`, `setRefloatControl`, `settings`, `shutdownController`, `startDiscovery`, `stopDiscovery`, `updateBoardProfile`, `updateRefloatTune`, `updateSavedDevice`, `updateSettings`, `withSessionReadCancellation`, `writeRefloatConfiguration`

### LoopbackServer.kt

`backendStartupFailure`, `totalMs`, `publishSessionCatalogChanged`, `start`, `stop`, `handle`, `success`, `requiredString`, `snapshot`, `bluetooth`, `refloatConfigurationSummary`, `refloatConfigurationIdentity`, `refloatConfigurationIdentity`, `boundedRefloatText`, `refloatTune`, `refloatConfigurationSchemaPage`, `refloatConfigurationField`, `refloatConfigurationValue`, `refloatConfigurationValues`, `parseRefloatConfigurationValues`, `device`, `connection`, `refloatControls`, `controllerActions`, `bmsConnection`, `bmsState`, `bmsSnapshot`, `savedDevice`, `boardProfile`, `session`, `sessionMetadata`, `sessionData`, `logSessionDataTiming`, `operation`, `maintenanceJobJson`, `pruneSessionIds`, `seriesPoint`, `settings`, `locationPayload`, `liveSample`, `liveLocation`, `sessionEvent`, `failure`, `accepted`, `JSONObject`, `Any`, `encodeSessionCatalogCursor`, `decodeSessionCatalogCursor`

All application-service calls (also covers route handlers, snapshots and nested history helpers):

`activeConnectionSnapshot`, `applyRefloatTune`, `boardProfile`, `cancelMaintenanceJob`, `cancelPendingDelete`, `clearRefloatFatalError`, `connect`, `createRefloatTune`, `deleteRefloatTune`, `deleteSavedDevice`, `deleteSession`, `disconnect`, `discoverySnapshot`, `exportSession`, `firmwareSnapshot`, `importSessionArchive`, `locationState`, `maintenanceJob`, `openLocationSettings`, `prunePreview`, `pruneSession`, `querySessionLocations`, `querySessionSeriesSource`, `readRefloatConfiguration`, `reduceSessionSeries`, `refloatConfigurationSchemaSnapshot`, `refloatConfigurationSummarySnapshot`, `refloatPackageVersionSnapshot`, `refloatTune`, `refloatTunes`, `requestBluetoothPermission`, `requestLocationPermission`, `saveMaintenanceJob`, `savedDevices`, `selectController`, `session`, `sessionEvents`, `sessionExtensionFields`, `sessionHistoryConfiguration`, `sessionPruneMetadata`, `sessionSnapshot`, `sessions`, `setRefloatControl`, `settings`, `shutdownController`, `startDiscovery`, `stopDiscovery`, `updateBoardProfile`, `updateRefloatTune`, `updateSavedDevice`, `updateSettings`, `withSessionReadCancellation`, `writeRefloatConfiguration`

## Public limits

Copied from the authoritative contract; names are retained to make the later boundary-test audit exact.

```typescript
export const MAX_MESSAGE_BYTES = 65_536;
export const MAX_SESSION_DATA_REQUEST_BYTES = 16_384;
export const MAX_SESSION_DATA_RESPONSE_BYTES = 4 * 1024 * 1024;
export const MAX_SESSION_IMPORT_BYTES = 128 * 1024 * 1024;
export const MAX_SESSION_DATA_FIELDS = 16;
export const MAX_SESSION_SERIES_BUCKETS = 1_024;
export const MAX_SESSION_SERIES_OBSERVATIONS = 65_536;
export const MAX_SESSION_SAMPLE_ROWS = 2_000;
export const MAX_SESSION_EVENT_ROWS = 500;
export const MAX_SESSION_LOCATION_ROWS = 2_000;
export const MAX_SESSION_LOCATION_GAPS = 2_000;
export const MAX_SESSION_CURSOR_BYTES = 512;
export const MAX_SESSION_LIST_PAGE = 100;
export const MAX_SESSION_LIST_CURSOR_BYTES = 1_024;
export const MAX_CAN_DISCOVERY_IDS = 64;
export const MAX_CONTROLLER_CANDIDATES = 16;
export const MAX_TELEMETRY_FIELDS = 64;
export const MAX_TELEMETRY_FIELD_BYTES = 64;
export const MAX_TELEMETRY_FIELD_NAME_BYTES = 4_096;
export const MAX_TELEMETRY_EXTENSIONS = 32;
export const MAX_TELEMETRY_JSON_BYTES = 16_384;
export const MAX_REFLOAT_CONFIGURATION_FIELDS = 512;
export const MAX_REFLOAT_CONFIGURATION_PAGE_FIELDS = 32;
export const MAX_REFLOAT_CONFIGURATION_GROUPS = 16;
export const MAX_REFLOAT_CONFIGURATION_SUBGROUPS = 64;
export const MAX_REFLOAT_CONFIGURATION_ENUM_CHOICES = 128;
export const MAX_REFLOAT_CONFIGURATION_NAME_BYTES = 128;
export const MAX_REFLOAT_CONFIGURATION_DESCRIPTION_BYTES = 4_096;
export const MAX_REFLOAT_CONFIGURATION_STRING_BYTES = 4_096;
export const MAX_REFLOAT_CONFIGURATION_CURSOR_BYTES = 512;
export const MAX_REFLOAT_CONFIGURATION_JSON_BYTES = 48 * 1_024;
export const MAX_REFLOAT_TUNE_NAME_BYTES = 128;
export const MAX_REFLOAT_TUNE_DESCRIPTION_BYTES = 1_024;
export const MAX_REFLOAT_TUNE_LIST_PAGE = 50;
export const MAX_REFLOAT_TUNE_CURSOR_BYTES = 512;
```

Additional validated ranges: discovery timeout 1–60,000 ms (default 5,000); CAN ID 0–253; prune resolution 1–300 s; history gap threshold 0.1–3,600 s or null; safe nonnegative integer snapshot/range fields; nonempty unique history fields. Native snapshots cap discovered/saved lists at 50. The existing host command/result envelopes are 1 MiB/4 MiB; archive bytes cannot use those envelopes.

## Canonical evidence and gaps

- conformance/src/suite.ts: runConformance covers bootstrap, unknown method, permission, scan, frontend replacement; exerciseProductMethods covers settings/conflicts, location, saved/guest connection, profile, board-scoped catalog, history, pruning, export control and deletion; exerciseRefloatConfiguration covers configuration and Tunes. assertState validates complete snapshots.
- conformance/fixture/{refloat-controls,controller-shutdown,location,bms,telemetry-contract}.ts cover domain/physical assertions. Existing assertions remain canonical.
- Add route authentication and streaming limits, HTTP archive round trips/cleanup, fragmented WebSocket messages, multiple/slow clients, exactly-once replies, client-independent revisions, listener restart, cancellation and host-exchange races in tasks 4.1/4.2. Method enumeration alone does not cover these.

Observed migration hazards:

- Swift replies and events share AsyncStream bufferingNewest(1), allowing replies to be overwritten. Kotlin already separates responses from conflated state.
- Both native implementations increment revisions from each subscriber callback, not from state.get. Thus revision growth depends on subscriber count and stops without subscribers. Shared publication must own revision advancement once per application update.
- Native HTTP paths obtain complete bodyData/readBytes before their final size check. The shared implementation must enforce limits during reads, including chunked input.
- Kotlin checks WebSocket authorization inside the upgraded handler; shared admission must reject before upgrade/body/application work.
- Native history cursor decoding and numeric coercions are sometimes more permissive than frontend validators. Use contract limits and add regression cases, rather than carry those differences forward.

## Source and deletion baseline

The existing convergence audit reports Rust 28,550 + Apple 11,338 + Android 10,635 = **50,523 production lines**. Its exclusion rules apply to the final comparison. Workspace members: 25; binding crates: 2; temporary protocol bridges: 0. Existing convergence ceiling: combined source <59,280 and members <34; this change additionally requires final combined source <50,523.

| Complete deletion/replacement target | Lines | SHA-256 |
| --- | ---: | --- |
| backends/apple/Sources/FloatViewBackend/transport/LoopbackServer.swift | 1905 | 3d01dc741ff0a1771592143a078625cd0ef687c60989c8d3cf7a8fa258a7f494 |
| backends/android/app/src/main/java/fail/acheron/floatview/transport/LoopbackServer.kt | 1794 | 9a1056d0e57f1f33f2254dbf57cad2c2f19f18a8ef6d75ad301f0fd7f0e0eb47 |

Delete superseded route/dispatch/payload/auth/gzip/cursor helpers and tests of their internals; retain native integration and fixed-port tests adapted to the shared implementation. Thin adapters may keep the LoopbackServer type name.

Dependency deletion targets: FlyingFox 0.27.1 (Swift Package.swift, both Package.resolved files, and iOS project.pbxproj package/product/build references); Ktor 3.0.3 server-cio/server-core/server-websockets (Android app/build.gradle.kts and gradle/libs.versions.toml). Swift Security and zlib imports in the server and Kotlin GZIPOutputStream cease to belong to transport adapters. Keep libraries used outside the deleted transport implementation.

Binding artifacts remain libfloat_view_application_runtime_apple_ffi.a and libfloat_view_application_runtime_android_jni.so (host JVM uses dylib on macOS). Existing JNI symbols are abiVersion/apply/copyResult/create/destroy/interrupt under Java_fail_acheron_floatview_application_RustApplicationRuntime. Extend the same families and exact-symbol audits, not a second library.

## Available build artifact sizes

These are existing local artifacts, not freshly rebuilt baseline packages; compare only after confirming equivalent build configurations. Missing macOS/iOS packages require fresh baseline measurement before claiming a package delta. Old protocol archives still present in ignored target directories are not selected or packaged artifacts.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| backends/android/app/build/outputs/apk/debug/app-debug.apk | 35831485 | 064e50e273bac3e6418556b62f294ae8aae2f27e29d72e47825d7ffc278251d9 |
| backends/android/app/build/outputs/apk/release/app-release-unsigned.apk | 23999306 | 5d355d743d0392eb12314d33be54b2194b95f6c64c47861b0631dadf002b49bd |
| backends/core/target/apple/macos/cargo/aarch64-apple-darwin/debug/libfloat_view_application_runtime_apple_ffi.a | 66985496 | 55be1ec4b911fa4bd6cc66f99abb18a8cd58ebe550b7a5c15ff96c06f01f8875 |
| backends/core/target/apple/macos/cargo/aarch64-apple-darwin/release/libfloat_view_application_runtime_apple_ffi.a | 23110480 | 1001f8772d095a93301e84e86dc839c50cf789e46405acc2b3da1be04c761631 |

Fresh Apple baseline packaging passed with `just apple-packaging` (Swift/Xcode cache access required outside sandbox). The recipe built macOS Release, iOS simulator Release and iOS device Debug without signing, then restored the macOS Debug archive. Existing deprecation/actor-isolation warnings remain.

| Fresh artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| /tmp/float-view-swift-release/out/Products/Release/FloatViewMac | 8448416 | c39a6d96cd00e86498fe00ad08895423fb22225e5142edffbb4ecb0717c727a2 |
| /tmp/float-view-ios-release/Build/Products/Release-iphonesimulator/float-view.app/float-view | 20388264 | b6a98d3797d44455a1c902a1fb26450bb51ec3c585cad44e47b042f610bb57c5 |
| /tmp/float-view-ios-device/Build/Products/Debug-iphoneos/float-view.app/float-view | 72552 | e03439f8a350b36c870590ac7778db65c4195cfc3eb4dab522fc55f990d8d032 |
