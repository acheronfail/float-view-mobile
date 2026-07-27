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

## Xcode simulator runtime unavailable

Recorded: 2026-07-28. Status: environment gate; core implementation continues.

Xcode 26.3 and the iPhoneSimulator 26.2 SDK are installed, but Xcode reports no
eligible simulator runtime/destination and the attached iOS 26.2 device also
lacks matching platform support. Consequently `xcodebuild` cannot currently
package or launch the iOS host.

The R1 implementation type-checks the complete iOS host plus shared Apple core
against the iOS 17 simulator target using the installed SDK, and the Apple Swift
package builds/tests on macOS. `just build-ios-app` remains the packaging gate
and must pass after installing an eligible simulator runtime. R0-R5 pure
protocol, application, storage, and API behavior continue to be implemented and
tested without treating a simulator skip as success.

## Apple loopback WebSocket repeated-frame trap

Recorded: 2026-07-28. Status: resolved.

The initial Network.framework host trapped on the second WebSocket frame because
`Data.removeFirst` advances `startIndex` and the incremental parser later used
zero-based integer subscripts. The parser now normalizes the buffer before
indexing. The authenticated Apple loopback host passes all four system
conformance scenarios over one persistent connection.

iOS still needs to consume the same `FloatViewHost` target, and Android needs
the equivalent host before R2 can complete.

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
