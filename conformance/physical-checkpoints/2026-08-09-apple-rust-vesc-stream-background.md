# Apple Rust VESC stream background checkpoint

Date: 2026-08-09

Native target: connected development iPhone running the production iOS backend

BLE fixture: connected canonical ESP32 byte bridge

## Focused physical result

The canonical iOS `background` physical scenario passed in 32.7 seconds after
the shared Apple backend selected ABI v2 Rust stream scanning. The installed
application used production CoreBluetooth and the shared iOS/macOS protocol
path; no test Bluetooth adapter or Swift production framing fallback was
present.

The fixture fragmented transport firmware, controller firmware, Refloat info,
the 412-byte Refloat layout, the 495-byte motor configuration, and continuing
telemetry into production notification chunks. The backend reached a ready
saved connection, retained the same active session across background and
foreground lifecycle transitions, continued accepting three- and four-fragment
telemetry responses while backgrounded, and disconnected cleanly after
foreground recovery.

Location authorization and background capability were both available. The
checkpoint persisted two new location rows and advanced location sequence from
0 to 2 during the same background interval. The final backend state revision
was 165.

This is the representative physical deletion gate for the
`cut-over-apple-vesc-stream-boundaries-to-rust` OpenSpec change. It records no
device identifier, Bluetooth address, launch token, raw frame, or full
telemetry payload.
