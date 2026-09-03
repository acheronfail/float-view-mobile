# Apple unified Rust runtime iOS checkpoint

Date: 2026-09-03

The canonical physical suite ran on the connected iPhone against the flashed
ESP32 fixture after Apple selected the unified Rust application runtime. Android
physical conformance was intentionally skipped for this host-only cutover.

Command:

```sh
FLOAT_VIEW_DEFER_PHYSICAL_LOCATION=1 just conformance-physical ios \
  /dev/cu.usbmodem111401 00008101-00195021268B001E
```

All 17 scenarios passed: identity, direct, direct-legacy, control-direct,
control-can, shutdown-direct, shutdown-can, shutdown-no-op, explicit-can,
generic-gateway, ambiguous, bms-direct-controller, bms-express, bms-standalone,
ride, unexpected-disconnect, and background.

Location authorization/acquisition was deferred because the connected device
had foreground-only permission. The background scenario exercised the bounded
disabled/unavailable checkpoint and confirmed no rows or sequences were
invented; full background location acquisition remains a manual permission
checkpoint rather than an automated claim.
