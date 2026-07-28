# T1 production-shaped VESC GATT transport evidence

Status: complete.

Date: 2026-07-28

## Fixture and toolchain

- Board: generic ESP32-S3 Super Mini
- Chip observed while flashing: ESP32-S3 QFN56 revision 0.2
- Flash/PSRAM observed: 4 MB embedded flash, 2 MB PSRAM
- USB transport: native USB Serial/JTAG
- ESP-IDF: v6.0.2
- ESP-IDF commit: `7101770dc6db2667b3c477cc31365dd1acd6db4e`
- Firmware binary size: 445,504 bytes
- Firmware binary SHA-256:
  `5907a38236ab02ae1b664f4f1f92d81939368e1126be3c2f2ea833766ff2ad83`

The fixture USB serial, BLE address, and transient `/dev/cu.*` path are local
lab identifiers and are intentionally omitted.

## Checks passed

The platform-independent parser/decoder suite passed:

```sh
sh tools/vesc-test-device/firmware/test/run.sh
```

The suite covers valid and invalid hexadecimal payloads, unsigned integer
bounds, negative and fractional values, trailing data, duplicate fields,
escapes, nested values, and output-size limits.

The final source built with ESP-IDF v6.0.2, fit the one-megabyte application
partition with 58% free, flashed successfully, and passed the physical macOS
USB/CoreBluetooth acceptance using Float View's production
`CoreBluetoothAdapter` source:

```json
{
  "schemaVersion": 1,
  "kind": "vesc-test-device-t1-macos",
  "passed": true,
  "gattWriteHex": "102030405060",
  "notificationHex": "deadbeef0102",
  "clientEventKinds": [
    "radioReady",
    "discovered",
    "diagnostic",
    "connectionState",
    "connectionState",
    "connectionState",
    "connectionState",
    "writeIssued",
    "notificationReceived",
    "connectionState",
    "passed"
  ],
  "safeAfterHostLoss": true
}
```

The production Android adapter instrumentation APK built successfully and
passed on a physical Samsung Galaxy S9 running Android 10/API 29:

```json
{
  "schemaVersion": 1,
  "kind": "vesc-test-device-t1-android",
  "passed": true,
  "apiLevel": 29,
  "gattWriteHex": "102030405060",
  "notificationHex": "deadbeef0102",
  "instrumentation": "OK (1 test)"
}
```

The production iOS adapter source built in a dedicated signed XCTest bundle and
passed on a physical iPhone 12 running iOS 18.6.1:

```json
{
  "schemaVersion": 1,
  "kind": "vesc-test-device-t1-ios",
  "passed": true,
  "gattWriteHex": "102030405060",
  "notificationHex": "deadbeef0102",
  "xcodeResult": "TEST SUCCEEDED"
}
```

This proves on the physical fixture that:

- the explicit test name and canonical VESC service are discoverable through
  Float View's production filtered CoreBluetooth and Android BLE scans;
- one central can connect, discover the canonical write/notify
  characteristics, and subscribe on macOS, Android, and iOS;
- client bytes arrive unchanged in the USB `gattWrite` event;
- host-supplied bytes arrive unchanged in the client notification;
- host-commanded disconnect completes;
- a lost USB host leaves the fixture unconfigured, disarmed,
  non-advertising, disconnected, and with no pending notification.

Python sources compile, the Swift client type-checks, and the touched files pass
`git diff --check`.

## Exit-gate matrix

| Gate | Status | Evidence or blocker |
| --- | --- | --- |
| Firmware deliverables | Pass | Implemented under `firmware/`; parser suite passes |
| Float View production scan/adapter on macOS | Pass | Physical acceptance compiles and runs the shared `CoreBluetoothAdapter` source |
| Float View production scan/adapter on Android | Pass | Production `AndroidBluetoothAdapter` instrumentation on Galaxy S9/API 29 |
| Float View production scan/adapter on iOS | Pass | Signed XCTest using shared `CoreBluetoothAdapter` source on iPhone 12/iOS 18.6.1 |
| Exact bytes, client to host | Pass on all three platforms | `102030405060` |
| Exact bytes, host to client | Pass on all three platforms | `deadbeef0102` |
| Safe state after host loss | Pass | Verified after a six-second host absence |

All T1 exit gates are satisfied. The standalone CoreBluetooth client remains
useful as an independent fixture diagnostic; the recorded macOS and iOS
acceptance runs use the shared production adapter source.
