# Shared host and Android cutover iOS checkpoint

Date: 2026-09-03

The canonical physical suite ran on the connected iPhone against the flashed
ESP32 fixture after the shared application-runtime host extraction and Android
cutover. Android physical conformance was intentionally skipped as requested;
its clean Debug/Release four-ABI packaging and software conformance passed in
`just verify`.

Command:

```sh
FLOAT_VIEW_DEFER_PHYSICAL_LOCATION=1 just conformance-physical ios \
  /dev/cu.usbmodem111401 00008101-00195021268B001E
```

All 17 scenarios passed: identity, direct, direct-legacy, control-direct,
control-can, shutdown-direct, shutdown-can, shutdown-no-op, explicit-can,
generic-gateway, ambiguous, bms-direct-controller, bms-express, bms-standalone,
ride, unexpected-disconnect, and background.

Location authorization/acquisition was explicitly deferred. The background
scenario exercised the bounded disabled/unavailable checkpoint and confirmed
that no rows or sequences were invented.

The final convergence audit recorded 27,254 Rust, 11,329 Apple, and 10,624
Android production lines (49,207 combined, 10,090 below the baseline), 25 Rust
workspace members, two host-binding crates, zero temporary protocol bridges,
and eight Android runtime target artifacts. The frontend wire contract,
canonical session schema, and Apple production runtime selection did not
change. Rust's session validator now admits only Android's exact bounded
`android_metadata(locale TEXT)` operating-system table alongside the canonical
schema; it does not permit application schema extensions.
