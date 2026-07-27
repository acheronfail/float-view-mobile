# Implementation ambiguities and resolutions

This log records issues encountered during the R0-R5 implementation pass that
could not safely remain implicit.

## Android 10 BLE scanning and location permission

Recorded: 2026-07-28. Status: resolved for implementation; product copy remains
to be reviewed.

The scope lock defers GPS/location recording and the R7 verification text said
the first-release build should not request location permission. Android 10
(API 29), however, requires location permission for BLE scan results. The
Galaxy S9/API 29 support requirement therefore conflicts with a literal
cross-platform ban on the permission.

Resolution:

- keep `ACCESS_FINE_LOCATION` only through API 30 using
  `android:maxSdkVersion="30"`;
- request it only when starting BLE discovery on those OS versions;
- never collect, store, or expose geographic location;
- Android 12+ uses `BLUETOOTH_SCAN`/`BLUETOOTH_CONNECT`;
- iOS declares no Core Location usage description in the first release.

R7 verification should assert absence of GPS/location data collection and no iOS
location permission, while accepting the legacy Android BLE scan permission.

## iOS host environment and physical smoke test

Recorded: 2026-07-28. Status: physical iOS 18 shell/bootstrap smoke passed.

The earlier simulator/device environment blocker was resolved by installing the
iOS 26.3.1 simulator runtime and enabling Developer Mode on the physical test
device. A signed Debug build was then built, installed, and launched on an
iPhone 12 running iOS 18.6.1.

The physical smoke test verified:

- development signing, provisioning, installation, and launch;
- bundled Svelte assets served from the authenticated loopback origin;
- WKWebView rendering and the Device/Sessions navigation;
- bootstrap retrieval, WebSocket upgrade, `system.hello`,
  `system.getSnapshot`, `savedDevices.list`, and `sessions.list`;
- stable empty Device and Sessions states.

The test exposed and resolved two host defects: the iOS shell looked for bundled
assets under `web/` while Xcode copied them as `build/`, and the native host did
not yet return the empty collection responses required by UI startup. This is
R2 shell evidence only. It does not verify CoreBluetooth discovery, a VESC
connection, logging, background behavior, or clean disconnect.

## Apple loopback WebSocket repeated-frame trap

Recorded: 2026-07-28. Status: resolved.

The initial Network.framework host trapped on the second WebSocket frame because
`Data.removeFirst` advances `startIndex` and the incremental parser later used
zero-based integer subscripts. The parser now normalizes the buffer before
indexing. The authenticated Apple loopback host passes all four system
conformance scenarios over one persistent connection.

The iOS and macOS shells now consume this shared `FloatViewHost`; Android keeps
an equivalent platform implementation behind the same API contract.

## Android 10 physical shell smoke test

Recorded: 2026-07-28. Status: physical Android 10 shell/bootstrap smoke passed.

A Debug APK was built, installed, and launched on a Samsung Galaxy S9+
(`SM-G965F`) running Android 10/API 29. The test verified the application-owned
loopback server, WebView startup, authenticated WebSocket bootstrap, Device and
Sessions empty states, tab navigation, and a Home/reopen lifecycle cycle without
changing the application process or emitting an AndroidRuntime/Chromium error.

The first physical run exposed a `NoSuchMethodError` because
`BufferedInputStream.readNBytes` is unavailable on API 29 even though the app
compiled against a newer Java/Android API. The loopback WebSocket reader now
uses a bounded compatibility read loop and successfully boots on API 29. The
same run also exposed and fixed a duplicate `android:networkSecurityConfig`
attribute that made the app manifest unparsable.

This S9+ is representative API 29 evidence but does not replace the explicit
Galaxy S9 qualification target. This is R2 shell evidence only; Bluetooth,
runtime permission behavior, VESC connection, durable logging, background
service behavior, and disconnect cleanup remain unverified.

## Synthetic COMM_GET_VALUES fixture layout

Recorded: 2026-07-28. Status: bounded pending R6 capture.

The R0 synthetic `COMM_GET_VALUES` fixture places input voltage and fault
immediately after ERPM, matching its declared expected data but not proving the
complete canonical 6.06 response tail. Swift and Kotlin deliberately agree with
the checked fixture for R4 parity. R6 must replace or supplement it with a
sanitized raw 6.06 hardware capture checked against pinned VESC Tool before
claiming full telemetry compatibility.

## R3/R5 breadth not yet represented in executable schemas

Recorded: 2026-07-28. Status: implementation backlog; blocks R3/R5 completion.

The product contract and stateful mock cover the primary device, settings,
profiles, session list/review/diagnostic, export, alert, storage, and firmware
operations. Remaining accepted behaviors still need first-class contract
methods and scenarios: bounded telemetry subscribe/unsubscribe, active logging
query/stop, Board Profile discovery/reset/precedence, session filter options,
session-group get/unlink, exact riding intervals, and recovery catch-up by
committed sequence. These must not be invented independently in native code.
