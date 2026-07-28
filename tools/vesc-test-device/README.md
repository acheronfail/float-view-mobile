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

Build and flash with `idf.py -C tools/vesc-test-device build flash`. The
host-buildable parser test is `tools/vesc-test-device/test/run.sh`.
