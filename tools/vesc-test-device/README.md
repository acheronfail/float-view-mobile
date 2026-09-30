# VESC test device

Pure ESP-IDF firmware for an ESP32-S3 that exposes the canonical VESC UART BLE
service. It is deliberately a byte bridge: VESC interpretation, boards,
scenarios, cadence, and safety policy live in `conformance/`.

Input lines:

```text
RESET
ADVERTISE
NOTIFY <HEX>
DISCONNECT
LED <RRGGBB>
```

Output lines:

```text
READY
CONNECTED
DISCONNECTED
WRITE <HEX>
ERROR <CODE>
```

Notification failures report `NOTIFY_FAILED_<status>` with the NimBLE status
number. Transient enqueue failures use a one-second retry budget, yielding at least
one scheduler tick between attempts. Successful enqueues retain 20 ms pacing and
are never replayed; NimBLE's notification callback reports enqueue status, not a
remote acknowledgement.

The target is the ESP32-S3 Super Mini with an ESP32-S3FH4R2: 4 MB QIO flash,
2 MB QSPI PSRAM, and native USB. The firmware does not use PSRAM.

PlatformIO installs the pinned ESP-IDF toolchain and handles the device:

```sh
just fixture setup
just fixture build
just fixture ports
just fixture flash /dev/cu.usbmodemXXXX
just fixture monitor /dev/cu.usbmodemXXXX
just fixture dev /dev/cu.usbmodemXXXX
```

Only PlatformIO Core (`pio`) must already be installed; ESP-IDF does not need to
be installed or sourced separately.

If the native USB port does not appear, hold BOOT while connecting the board.
The host-buildable parser test is `just fixture check`.
The `dev` action starts the host-side SvelteKit controller from `conformance/`;
it does not add JavaScript, TypeScript, scenarios, or VESC interpretation to
this firmware.
