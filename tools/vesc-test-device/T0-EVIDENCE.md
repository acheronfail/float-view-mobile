# T0 board and toolchain evidence

Status: complete on 2026-07-28.

## Fixture

- Carrier: generic ESP32-S3 Super Mini
- MCU observed by esptool: ESP32-S3 QFN56 revision 0.2
- Flash: 4 MB embedded XMC flash
- PSRAM: 2 MB embedded QSPI PSRAM
- Crystal: 40 MHz
- USB: native ESP USB Serial/JTAG, VID:PID `303a:1001`
- Antenna: onboard antenna; generic-board batch and orientation caveats apply
- Host: macOS development machine

The unique USB serial and Bluetooth address are intentionally excluded.

## Reproducible inputs

- ESP-IDF tag: `v6.0.2`
- ESP-IDF commit: `7101770dc6db2667b3c477cc31365dd1acd6db4e`
- Bluetooth host: ESP-IDF NimBLE
- Firmware target: `esp32s3`
- Firmware image size: 439,616 bytes
- Firmware SHA-256:
  `51261ad48bf74564cf2511d771c79c6108ae67f8a88bd11241087ace22212368`

The image digest identifies this local evidence run. It is not a release
artifact and will change when source metadata or the worktree changes.

## Gate results

Build and flash:

- ESP-IDF build passed.
- esptool detected the expected chip, flash, PSRAM, crystal, and native USB
  interface.
- Bootloader, partition table, and application writes passed hash verification.
- Reset and ROM-download recovery were exercised.
- Panic behavior is configured to halt rather than enter a USB re-enumeration
  loop.

BLE smoke:

```json
{
  "kind": "vesc-test-device-t0-ble",
  "name": "FloatView Test VESC",
  "passed": true,
  "rssi": -51,
  "schemaVersion": 1,
  "serviceUuid": "6e400001-b5a3-f393-e0a9-e50e24dcca9e"
}
```

BLE discovery also passed at RSSI -53 dBm while the ten-minute USB soak was
actively exchanging commands.

USB smoke:

```json
{
  "schemaVersion": 1,
  "kind": "vesc-test-device-t0-usb",
  "durationSeconds": 5,
  "eventCount": 25,
  "eventKinds": ["heartbeat", "pong", "state"],
  "parseErrors": 0,
  "advertisingObserved": true,
  "passed": true,
  "missing": []
}
```

Concurrent ten-minute soak:

```json
{
  "schemaVersion": 1,
  "kind": "vesc-test-device-t0-usb",
  "durationSeconds": 600,
  "eventCount": 2760,
  "eventKinds": ["heartbeat", "pong", "state"],
  "parseErrors": 0,
  "advertisingObserved": true,
  "passed": true,
  "missing": []
}
```

The fixture stayed on one USB session for the full soak. There was no reset,
write timeout, USB re-enumeration, stream corruption, or macOS accessory-prompt
churn.

## Findings carried into T1

- Native USB Serial/JTAG is the selected control interface; this carrier has no
  separate USB-to-UART bridge.
- Host tooling must locate the port from stable USB identity rather than embed a
  `/dev/cu.usbmodem*` path.
- Do not call `tcdrain`/pyserial `flush()` on this native-USB control path;
  bounded writes and reads are sufficient.
- ESP-IDF 6 requires a non-null access callback even for a notify-only NimBLE
  characteristic.
- Generic ESP32-S3 Super Mini antenna quality is fixture-specific and must not
  substitute for real-board Bluetooth qualification.
