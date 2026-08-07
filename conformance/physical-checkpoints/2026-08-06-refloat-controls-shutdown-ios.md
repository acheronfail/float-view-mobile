# Refloat controls and controller shutdown iOS physical checkpoint

Date: 2026-08-06  
Native target: iOS (`iPhone 12`, device ID `00008101-00195021268B001E`)  
BLE fixture: ESP32 `/dev/cu.usbmodem2101`  
Supported controller: retained THOR controller on the Thor301/VBMS32 board

## ESP32 physical conformance

The iOS production backend was built, installed, and exercised over real BLE
against the canonical ESP32 byte bridge. Sixteen physical scenarios passed,
including direct and CAN-routed Refloat controls, exact flywheel disable from
active flywheel telemetry, direct and CAN-routed controller shutdown, the
30-second unsupported-hardware shutdown no-op, and unexpected-disconnect
recovery. The fixture recorded only allowlisted control, shutdown, telemetry,
and discovery requests; no motor-motion request occurred.

The first shutdown-no-op attempt exposed a stale ten-second physical-runner
deadline after the native shutdown-intent window had increased to 30 seconds.
The runner now waits 35 seconds and has a regression test; the repeated no-op
scenario passed in 33.6 seconds while remaining connected and clearing pending
shutdown state.

The seventeenth, unrelated background-location scenario did not complete
because Xcode's physical XCTest worker never materialized. A standalone
`testBackground` invocation reproduced the Apple tooling stall before the test
body emitted output; CoreDevice reported the phone booted, paired, wired,
unlocked, and in developer mode. This is retained as an unavailable lifecycle
checkpoint rather than a Float View backgrounding result.

## Supported-controller shutdown and recovery

The user completed the manual iOS checkpoint on the supported THOR hardware
and confirmed every requested behavior:

- [x] Representative Refloat controls and the active-flywheel Disable overlay
      worked on an idle, safely supported board.
- [x] Cancelling the shutdown confirmation caused no command-side effect.
- [x] Confirmed shutdown powered off the controller and BLE transport, returned
      from LiveView to home without automatic reconnect, and did not show
      `Reconnecting`.
- [x] Manual controller power-on restored normal discovery and a fresh manual
      connection.

This completes actual supported-hardware shutdown, intentional-loss handling,
and manual recovery validation on both native mobile backends.
