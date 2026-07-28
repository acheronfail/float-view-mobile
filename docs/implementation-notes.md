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

## Android 10 VESC discovery, primary-controller selection, and durable close

Recorded: 2026-07-28. Status: partial R6/R7 hardware evidence; telemetry remains
pending.

The Samsung Galaxy S9+/Android 10 test device granted the legacy location-based
BLE scan permission, discovered one VESC UART transport advertising as
`ExpressT`, connected over GATT, discovered the Nordic UART service and both
required characteristics, enabled local notifications, and received a
successful CCCD write callback. The connection then reached `ready` and
automatically created a durable session.

The first ready attempt exposed two Android SQLite differences that the JVM
tests could not detect: query-style PRAGMAs cannot be passed to `execSQL`, and
`SQLiteOpenHelper.onCreate` runs inside a transaction where journal mode cannot
be changed. Android now configures catalog/session WAL explicitly, filters
schema PRAGMAs from DDL execution, removes partial session databases on create
failure, and transitions the connection to `failed` rather than claiming a
logging-capable ready state.

After the fix, an explicit disconnect produced:

- SQLite `integrity_check = ok` for both the catalog and per-session database;
- one catalog session in `completed`/`complete` state;
- `close_reason = explicit_disconnect` with a non-null end time;
- a flushed, readable session schema;
- an Android Bluetooth client disconnect and GATT client unregister.

The session contains zero telemetry samples because this test intentionally
preceded the first telemetry command. The read-only firmware sequence is now
implemented in both native cores and was verified on the same hardware:

- the directly connected VESC Express transport reported `VESC Express T`,
  VESC 6.05, hardware type `CUSTOM_MODULE`;
- `COMM_PING_CAN` returned the available CAN nodes;
- sequential `COMM_FORWARD_CAN` firmware requests retained their CAN identity;
- the first hardware type `VESC` node was selected as the primary controller;
- the primary reported `Thor300`, VESC 7.00 before the fixture was reflashed;
- the UI displayed the primary controller rather than the transport module.

The forwarding constants, hardware-type values, and packet layout were checked
against the pinned VESC Tool source. Refloat package identification uses the
owner-authorized pinned Floaty reference: forwarded
`COMM_CUSTOM_APP_DATA [101, GET_INFO]`, with a response byte encoded in tenths.
Both native cores now test the complete ordered Express → CAN nodes → primary
VESC → Refloat version sequence. Physical Refloat 1.2 confirmation initially
waited on the fixture update to VESC 6.06 and Refloat 1.2.

The fixture was subsequently updated and Android physically verified:

- the primary reports `Thor300`, VESC 6.06;
- Refloat INFO reports the 1.2 release line;
- the UI reports the combination as supported;
- the automatic session closes without reconnecting.

The first telemetry implementation used the supplementary legacy
`GET_ALL_DATA` response. It proved a 2 Hz idle request cadence, 1 Hz adaptive
durable persistence, sequence gaps that preserve dropped/non-persisted sample
identity, and a clean SQLite checkpoint: 175 samples over 228 seconds,
`committed_sequence = 451`, and `integrity_check = ok`.

The test controller is known to report invalid sensor values, so those numeric
values are not sensor-accuracy evidence. They remain useful transport data only.
Value accuracy, real riding-state transitions, and threshold alerts require a
known-good controller. Until then, hardware acceptance on this fixture is limited
to discovery, framing, dynamic field identification, response ordering, bounded
cadence, durable persistence/integrity, lifecycle recovery, and deterministic
disconnect. Tests must not add plausibility ranges derived from this fixture.

Inspection of the pinned Refloat v1.2.0 source then identified the canonical
modern interface: versioned INFO v2, `REALTIME_DATA_IDS` (32), and
`REALTIME_DATA` (31). The native implementations now request the dynamic field
IDs, decode Refloat's extended float16 values by ID, extract package RUNNING and
footpad states from their documented flag bytes, probe at 2 Hz idle, switch to
10 Hz riding, and feed one canonical sample into adaptive durable logging.
Swift and Kotlin cover the byte layout and rate transition deterministically.
A subsequent Android hardware run confirmed versioned INFO `1.2.3`, the complete
dynamic layout (16 persisted fields plus 10 runtime fields), fragmented
`REALTIME_DATA` responses, and clean disconnect. The sanitized capture is
committed as `fixtures/refloat/rtdata-1.2.3-android-hardware.json` and is decoded
across its original notification boundaries by both Swift and Kotlin tests.

The same run durably committed 32 idle samples from received sequences 1 through
78 over 41,918 milliseconds. Sequence gaps reflect the configured 1 Hz idle
persistence policy rather than packet loss. The closed database reported
`integrity_check = ok`, `committed_sequence = 78`, and a populated extension
payload. The reproducible evidence commands are
`just android-device-protocol-capture` and
`just android-device-verify-latest-session`.

The identical read-only workflow then passed on an iPhone 12: it discovered and
connected through the Express transport, selected the Thor300 primary, reported
VESC 6.06 and Refloat 1.2.3, and disconnected cleanly. A CoreDevice copy of the
app-private store showed catalog and session `integrity_check = ok`, a
`completed` session with `close_reason = explicit_disconnect`, four persisted
idle samples from received sequences 1 through 10 over 7,525 milliseconds, and
`committed_sequence = 10`. The final sample contained the same current/ERPM/speed
extension fields as Android. Reproduce the durable inspection with
`just ios-device-verify-latest-session <device-id>`.

The macOS host then passed the same physical path. Its intended 29.9-second run
closed as `completed`/`explicit_disconnect`, persisted 20 idle samples from
received sequences 1 through 55, and reported catalog/session
`integrity_check = ok`. A user-initiated second reconnect began three seconds
later and closed after 10.2 seconds with zero samples; this was not mistaken for
the intended evidence run. It exposed that the dynamic field-layout request was
one-shot. Both native cores now retry that read-only request at one-second
intervals, stop after five attempts, close the logging session with
`refloat_telemetry_layout_timeout`, and fail/disconnect visibly instead of
leaving a zero-sample session presented as healthy. Deterministic Swift/Kotlin
tests cover both recovery and bounded failure. Reproduce local durable
inspection with `just macos-device-verify-latest-session`.

Unexpected connection loss no longer calls the platform adapter in an immediate,
unbounded loop. Both application services now retain the active durable session,
enter `reconnecting`, schedule one-shot attempts with exponential delays from
one second up to a 30-second cap, and stop at the configured reconnection window
(five minutes by default). A successful `ready` transition keeps the same
session. Window expiry closes it with `reconnect_timeout`; unavailable scheduling
and disabled reconnection also terminate explicitly. Explicit user disconnect
cancels every pending reconnect before closing. Fake-clock Swift/Kotlin tests
cover recovery, backoff bounds, timeout closure, and the no-reconnect explicit
path.

Android runtime permission selection is now an independently tested policy:
API 29/Android 10 requires fine location for BLE scanning and reports connect
permission as `not_required` because `BLUETOOTH` is manifest-only; API 31 and
newer require both `BLUETOOTH_SCAN` and `BLUETOOTH_CONNECT`. The production
adapter consumes this policy instead of duplicating SDK checks.

Android also persists whether each runtime permission request has resolved, so a
missing permission remains `not_determined` before the prompt and becomes
`denied` after an actual refusal. Both native application services treat a
denied/restricted connect permission as terminal for an active session, close it
with `bluetooth_permission_denied`, release the adapter, and schedule no retry.

The R6 physical failure gates now use one shared declarative scenario file and a
redaction-enforcing evidence runner. `just hardware-r6-guide` prints the
permission, radio-cycle, out-of-range, explicit-disconnect, and local-profile
override procedure and optionally creates a non-overwriting template;
`just hardware-r6-verify` accepts it only when every expected state is present.
Android permission denial/restoration is reproducible with
`just android-r6-permission`; iOS/macOS privacy and radio changes remain explicit
user actions.

Physical Android 10 failure qualification then found and fixed three integration
gaps:

- Samsung silently restored fine location when the test helper left the
  implicitly granted coarse-location group member; reset now revokes both;
- the shared UI displayed the same generic message after denial and grant; it
  now polls the request result, reports denied/restricted explicitly, and begins
  scanning immediately after a grant;
- Android GATT did not publish adapter radio changes and a reconnecting
  `connectGatt` could wait forever for a callback. The adapter now observes
  `ACTION_STATE_CHANGED` and gives every GATT setup attempt a ten-second
  watchdog that releases stale resources and emits a terminal attempt result.

After those fixes, the Galaxy S9 physically passed permission denial, radio
off/on, VESC power-loss recovery, and explicit disconnect. Radio-off produced no
adapter connect request; radio-on returned to ready without creating a second
session. VESC power loss retried at approximately 1/2/4/8-second delays and
returned to the same session. Explicit disconnect produced no later attempt, and
a temporary catalog/WAL/SHM copy passed integrity with
`completed`/`explicit_disconnect` at committed sequence 177. The redacted
failure evidence remains intentionally incomplete because native
`boardProfiles.get/update` is not implemented; profile-override isolation must
not be claimed from mock-only behavior.

Physical iPhone 12/iOS 18.6.1 qualification then passed the same four native
backend scenarios. It found that CoreBluetooth reports an authorization-denied
manager as an unknown radio plus denied permission; discovery now gives
permission precedence and returns `permission_required` instead of falsely
claiming Bluetooth is powered off. Identifier-free debug traces proved that
radio-off entered reconnecting with zero connection requests, radio-on issued
one request and returned to ready, and VESC power loss/restoration completed the
single OS-owned pending CoreBluetooth connection without replacing the durable
session. Explicit disconnect produced no later request; copied catalog/session
databases passed integrity and reported `completed`/`explicit_disconnect`.

This qualification originally exposed two lifecycle gaps for R7. The API store
first learned to reconnect a closed socket when the page survived, and the Apple
hosts now go further: a shared frontend lifecycle coordinator owns only the
disposable loopback server and issues a fresh port, token, and generation for
each frontend. On iOS, entering the background destroys the server and
WKWebView; foreground activation creates both again while the native Bluetooth,
application-service, and SQLite owners remain alive. A physical 60-second
iPhone 12/iOS 18.6.1 background run traced generation 1 stopping and generation
2 starting, and the UI reconstructed the same connected durable session after a
brief Backend Unavailable state. Both production stores also mark catalog rows
left active by process termination as
`interrupted`/`process_interruption` at backend startup, with deterministic Apple
persistence coverage.

iOS now declares `bluetooth-central`, supplies a stable CoreBluetooth restoration
identifier, and safely accepts a restored peripheral binding without claiming
that process-level job restoration already exists. The exported physical
background session passed catalog/session integrity and reported
`completed`/`explicit_disconnect`, 391 committed observations, 143 adaptively
persisted samples, and 197910 ms elapsed. Its largest gap between persisted
samples was 1561 ms, so the 60-second background interval did not suspend
idle-rate native polling on this iPhone 12/iOS 18.6.1 run.

The same coordinator now powers the macOS host. A real signed development
`.app` contains the shared web assets and SQL migrations, installs at
`~/Applications/Float View.app`, and has its own persistent Bluetooth TCC
identity. Closing the window destroys its WKWebView and loopback server but
leaves the native owner and Dock process alive; reopening creates generation 2
and reconstructs the same connected session. The physical macOS run passed
permission denial/restoration, radio off/on recovery, VESC power-loss recovery,
explicit disconnect, and window close/reopen. Its copied databases passed both
integrity checks and reported `completed`/`explicit_disconnect`, 306 committed
observations, 106 adaptively persisted samples, and 218572 ms elapsed. Startup
also recovered each session interrupted by the two window-ownership crashes
found during bring-up. The crash was fixed by making the programmatic NSWindow
owner-managed rather than allowing AppKit and Swift to release it independently.

The cumulative `just verify-r6` software gate passes with these changes,
including generated-model drift, boundary checks, web type/lint/unit tests,
Swift and Kotlin protocol/application/storage/alert/Bluetooth tests, all three
native builds, mock and macOS black-box conformance, and validation of the three
recorded hardware-smoke evidence files.

The canonical sample now preserves Refloat motor current, battery current, ERPM,
and speed in the session schema's `extension_json`, using the same API field
names on Apple and Android. Voltage, duty cycle, temperatures, ride state, and
footpad state remain indexed first-class columns. This keeps the initial schema
generic while retaining every currently decoded value for later review and
diagnosis.

Catalog and per-session schema version 2 make those canonical samples directly
queryable without whole-ride WebView loading. Each durable append also advances
indexed catalog summaries for sample/riding/total duration, distance, maximum
speed, and maximum duty. Native `sessions.list` uses opaque keyset cursors with
device/date/duration/distance/state/integrity filters and supported sort fields.
Review queries aggregate into a caller-bounded number of SQLite buckets;
diagnostic queries return at most 10,000 lossless rows from a bounded
ten-minute window. The same operations now run through the Swift and Kotlin
application-service boundaries, the mock, and the Svelte/SVG review UI. The
macOS loopback conformance seed and Galaxy S9 instrumentation suite exercise
these paths against real SQLite databases.

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
