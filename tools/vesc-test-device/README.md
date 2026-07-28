# Programmable VESC fixture

An ESP32 running `firmware/` emulates a read-only VESC/Refloat board over BLE.
The TypeScript code in `host/` drives deterministic ride, disconnect, background,
and soak scenarios from the shared fixtures in `fixtures/`.

## Daily commands

From the repository root:

```sh
just vesc check
just vesc doctor
just vesc flash /dev/cu.usbserial-…
just vesc android ride
just vesc ios <device-id> ride
just vesc macos ride
```

`check` type-checks and unit-tests the host scenario engine without hardware.
`doctor` reports local ESP-IDF, serial, Android, and Apple prerequisites. `flash`
builds and flashes the ESP32; source the ESP-IDF `export.sh` first.

Platform scenarios expect a flashed fixture advertising the configured test
service. They exercise production Bluetooth and application-service code and are
read-only: no firmware, configuration, or motor-control command is allowed.

Supported scenario names:

- Android: `ride`, `unexpected-disconnect`, `background`,
  `same-process-soak`, and `soak`.
- iOS: `ride` and `unexpected-disconnect`.
- macOS: `ride` and `unexpected-disconnect`.

Set `FLOAT_VIEW_ESP_PORT` to omit the port from `just vesc flash`. The host tool
keeps platform-specific launch details internally so normal development needs
only the commands above.

The retained design/evidence record is [`T3-EVIDENCE.md`](T3-EVIDENCE.md).
