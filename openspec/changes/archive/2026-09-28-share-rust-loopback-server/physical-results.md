# Physical checkpoint — 2026-09-27–28

The ESP32-S3 fixture was built and flashed successfully through
`just fixture build` and `just fixture flash /dev/cu.usbmodem111401`.
All tests use production native BLE, the selected Rust transport and canonical
`conformance/src/physical.ts` assertions.

| Platform                   | Result       | Coverage                                                                                                                                                            |
| -------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| iOS, connected iPhone 12   | PASS — 17/17 | Complete iOS matrix: identity, direct/legacy/CAN, controls/configuration/Tunes, shutdown, ambiguity, BMS, saved ride, unexpected disconnect and background recovery |
| macOS                      | PASS — 16/16 | Complete macOS matrix, including saved ride and unexpected-disconnect recovery                                                                                      |
| Android, connected Samsung | PASS — 17/17 | Complete Android matrix on Android 10, including saved ride, unexpected-disconnect recovery and background recording with location                                  |

The iOS background location checkpoint recorded rows 0 → 2 and committed
location sequence 0 → 2 with authorized background capture. The physical run
exposed a WebSocket idle timeout during slow BLE configuration; after adding
10-second ping/pong heartbeats, the entire iOS and macOS matrices passed.
Canonical transport tests additionally verify that responsive idle peers survive
and peers that ignore pings are closed within the 30-second inactivity bound.

Local run evidence: `/tmp/float-view-loopback-ios-physical.log`,
`/tmp/float-view-loopback-macos-physical.log` and
`/tmp/float-view-loopback-macos-physical.json`.

After USB-debugging authorization on September 28, the Android run exposed a
startup crash from `Class.getPackageName()`, unavailable on the phone's Android
10 runtime. The native fact encoder now obtains the package from the class name
using compatible string operations. The complete 17-scenario physical matrix
then passed. Its background checkpoint recorded location rows 0 → 1 and committed
location sequence 0 → 1 with authorized/available background capture.

Android evidence: `/tmp/float-view-loopback-android-physical.log` and
`/tmp/float-view-loopback-android-physical.json`. All three required physical
platforms passed; no coverage was deferred.
