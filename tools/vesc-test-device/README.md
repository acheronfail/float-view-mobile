# Float View VESC test device

This directory contains the programmable BLE test peripheral described in
[`../../docs/vesc-test-device-plan.md`](../../docs/vesc-test-device-plan.md).

T0, T1, and T2 completed on 2026-07-28. The current implementation:

- targets ESP32-S3 with ESP-IDF v6.0.2 and NimBLE;
- advertises the canonical VESC UART service UUID;
- exposes the canonical write and notify characteristics;
- boots disarmed and non-advertising;
- accepts sequenced, versioned JSON-line commands over USB Serial/JTAG;
- emits sequenced raw GATT-write and connection events;
- sends host-supplied notification bytes and host-commanded disconnects;
- lights the onboard status LED while a BLE central is connected and turns it
  off on disconnect;
- enforces one central, bounded input/output, and one pending notification;
- returns to a disconnected, non-advertising state after host loss.
- runs a schema-validated, deterministic TypeScript scenario engine;
- assembles and validates VESC frames independently of product code;
- models a read-only VESC Express/Thor300 CAN topology with Refloat 1.2.3;
- rejects and records unknown or mutating commands;
- produces bounded transcripts and machine-readable verification manifests.

T0 results are in [`T0-EVIDENCE.md`](T0-EVIDENCE.md), T1 results are in
[`T1-EVIDENCE.md`](T1-EVIDENCE.md), and T2 host and physical evidence are in
[`T2-EVIDENCE.md`](T2-EVIDENCE.md).

## T0 board

The physical fixture is a generic
[ESP32-S3 Super Mini](https://www.espboards.dev/esp32/esp32-s3-super-mini/).
The board uses native USB over USB-C rather than a separate USB-to-serial chip,
and provides BOOT and RESET buttons. The connected unit reports an ESP32-S3
QFN56 revision 0.2 with 4 MB embedded flash and 2 MB embedded QSPI PSRAM,
consistent with the commonly fitted ESP32-S3FH4R2.

The carrier has an onboard antenna. Generic Super Mini batches vary, and some
have been reported with a reversed antenna, so T0 BLE range results apply to
this individual fixture rather than every board sold under the same name.

The board's one-pixel WS2812 status LED is connected to GPIO48. Firmware drives
it through ESP-IDF's RMT peripheral at low white brightness: off at boot and
while advertising, on for the lifetime of a BLE connection, and off again on
disconnect or Bluetooth stack reset. This mirrors the visible connection
indicator on VESC Express devices.

## Toolchain

Install and export ESP-IDF v6.0.2:

```sh
git clone --filter=blob:none --branch v6.0.2 --depth 1 \
  --recurse-submodules --shallow-submodules \
  https://github.com/espressif/esp-idf.git /path/to/esp-idf-v6.0.2
cd /path/to/esp-idf-v6.0.2
./install.sh esp32s3
source ./export.sh
```

The local T0 run used:

```text
IDF path: /Users/cosmotherly/esp/esp-idf-v6.0.2
USB VID:PID: 303a:1001
USB serial: <local lab identity; intentionally omitted>
```

The USB serial is local lab identity and must not be copied into public test
artifacts.

If faulty firmware repeatedly disconnects native USB, recover without running
the application: hold BOOT while connecting USB (or while tapping RESET), then
release BOOT after the ROM download port appears. T0 builds halt after a panic
instead of rebooting, preventing repeated macOS accessory prompts.

## Build, flash, and verify

From `tools/vesc-test-device/firmware`:

```sh
idf.py set-target esp32s3
idf.py build
idf.py -p "$FLOAT_VIEW_ESP_PORT" flash
```

Set `FLOAT_VIEW_ESP_PORT` to the board's current `/dev/cu.usbmodem*` path for
flashing; the path can change after reconnecting. Keep the board's USB serial in
the local `FLOAT_VIEW_ESP_USB_SERIAL` environment variable; do not commit it.

Run the platform-independent control parser tests:

```sh
sh tools/vesc-test-device/firmware/test/run.sh
```

Run the T1 macOS USB/CoreBluetooth transport acceptance:

```sh
python tools/vesc-test-device/host/t1_verify.py \
  --usb-serial "$FLOAT_VIEW_ESP_USB_SERIAL"
```

The verifier compiles Float View's shared production `CoreBluetoothAdapter`
directly from `apple/`, then configures and arms the fixture, scans with its
canonical service filter, connects, subscribes, verifies exact bytes in both
directions, disconnects, and proves that loss of the USB host returns the
fixture to its safe state. `probe.py` and `scan.swift` remain as immutable T0
diagnostics and target protocol version 0.

Build, install, and run the opt-in Android production-adapter acceptance:

```sh
cd android
./gradlew :bluetooth:assembleDebugAndroidTest
adb -s "$ANDROID_SERIAL" install -r \
  bluetooth/build/outputs/apk/androidTest/debug/bluetooth-debug-androidTest.apk
cd ..
python tools/vesc-test-device/host/t1_android_verify.py \
  --usb-serial "$FLOAT_VIEW_ESP_USB_SERIAL" \
  --android-serial "$ANDROID_SERIAL"
```

The Android orchestrator grants only the installed test package the runtime
permissions needed for BLE scanning. On Android 10 it also sets location
app-ops to `allow`, because a headless instrumentation APK has no foreground
Activity and some vendor stacks otherwise suppress scan results. Uninstalling
`fail.acheron.floatview.bluetooth.test` removes that test-package state.

Run the physical iOS production-adapter acceptance:

```sh
python tools/vesc-test-device/host/t1_ios_verify.py \
  --usb-serial "$FLOAT_VIEW_ESP_USB_SERIAL" \
  --ios-device-id "$FLOAT_VIEW_IOS_DEVICE_ID"
```

The iOS orchestrator builds and signs the `float-view-hardware` scheme before
arming the fixture. The scheme uses a minimal test-only host app and compiles
the shared production `CoreBluetoothAdapter` source directly into its XCTest
bundle; it does not start the Float View UI, backend, storage, or WebView.
`FLOAT_VIEW_IOS_DEVICE_ID` is the physical device identifier accepted by
`xcodebuild`, and must remain local lab configuration.

## T1 USB control protocol

Every command is one compact JSON object followed by `\n`, with
`protocolVersion: 1`, a monotonically increasing unsigned `sequence`, and a
`kind`. The firmware accepts:

- `hello`
- `reset`
- `configureGattProfile` with `profile: "canonicalVescUart"`
- `startAdvertising` and `stopAdvertising`
- `setWritePolicy` with `policy: "accept"` or `"reject"`
- `notify` with a lower- or upper-case hexadecimal `hex` payload
- `disconnect`
- `getState`

Every firmware event includes `protocolVersion`, a monotonically increasing
event `sequence`, and `uptimeMs`. Commands terminate exactly once with
`commandCompleted` or `commandFailed`; asynchronous BLE state and raw
`gattWrite` events are separate. A new `hello` starts a host session. The host
must send a valid command at least every five seconds or the fixture disarms,
stops advertising, drops pending work, and disconnects.

Limits are deliberately small: 768 bytes per host line, 896 bytes per device
line, 256 bytes per captured write, 244 bytes per notification, one central,
and one pending notification. The fixture marker is manufacturer data
`ff ff 46 56 54 31` (`0xffff` plus `FVT1`).

## T2 scenario engine

Install and run the host-only checks:

```sh
cd tools/vesc-test-device/host
npm ci
npm run check
npm test
```

The first T2 scenario is
[`../../conformance/scenarios/vesc-test-device/express-thor-refloat-identity.json`](../../conformance/scenarios/vesc-test-device/express-thor-refloat-identity.json).
It uses committed response fixtures for Express 6.05, Thor300 6.06, Refloat
INFO 1.2.3, dynamic telemetry field IDs, and deterministic idle telemetry.
Scenario files are validated against
[`schemas/scenario-v1.schema.json`](schemas/scenario-v1.schema.json).

The opt-in macOS physical verifier compiles Float View's production
CoreBluetooth adapter and VESC protocol codec, arms the ESP, executes the
identity sequence, and writes ignored evidence under
`artifacts/verification/vesc-test-device`:

```sh
cd tools/vesc-test-device/host
npm run verify:macos -- --port "$FLOAT_VIEW_ESP_PORT"
```

This command performs real Bluetooth discovery and connection. Run it only
after the fixture port has been resolved locally and the operator has approved
starting physical verification.

## T3 deterministic ride

The T3 ride scenario is
[`../../conformance/scenarios/vesc-test-device/express-thor-refloat-ride.json`](../../conformance/scenarios/vesc-test-device/express-thor-refloat-ride.json).
Its telemetry schedule uses observable request occurrences to drive idle,
riding, footpad-warning, 15-second post-ride-tail, and return-to-idle phases.
Cadence is verified from request timestamps: 2 Hz idle and 10 Hz riding/tail.

The opt-in macOS session verifier uses the production CoreBluetooth adapter,
application service, VESC codec, adaptive sampling policy, and SQLite store. It
explicitly disconnects after the deterministic ride, then checks session
closure, sample coverage, footpad evidence, SQLite integrity, and cleanup:

```sh
cd tools/vesc-test-device/host
npm run verify:macos:ride -- --port "$FLOAT_VIEW_ESP_PORT"
```

This command performs a real Bluetooth connection and takes roughly 20 seconds.
Run it only after explicit operator approval.

The physical macOS gate passed on 2026-07-28. Its machine-readable manifest and
bounded transcript are written under
`artifacts/verification/vesc-test-device/express-thor-refloat-ride-passed-*`.

The short T3 unexpected-disconnect verifier uses the same production path but
commands the ESP to drop BLE during riding. Reconnection is deliberately
disabled for this cleanup-specific gate; reconnect success and expiry variants
remain T5 scenarios:

```sh
cd tools/vesc-test-device/host
npm run verify:macos:unexpected-disconnect -- --port "$FLOAT_VIEW_ESP_PORT"
```

This is also a physical Bluetooth run and requires explicit operator approval.

The Android T3 verifier builds and installs the debug app and instrumentation,
launches the real foreground Activity, drives the same scenarios through the
production Android GATT/application/logging stack, and pulls sanitized durable
session evidence:

```sh
cd tools/vesc-test-device/host
npm run verify:android:ride -- \
  --port "$FLOAT_VIEW_ESP_PORT" --serial "$ANDROID_SERIAL"
npm run verify:android:unexpected-disconnect -- \
  --port "$FLOAT_VIEW_ESP_PORT" --serial "$ANDROID_SERIAL"
```

Both Android 10 gates passed on 2026-07-28. The manifests and bounded
transcripts are written under
`artifacts/verification/vesc-test-device/*-android-*-passed-*`. The real
foreground Activity is required because Android 10 treats location-backed BLE
scans from a headless instrumentation process as background. Scenario
notifications remain at or below the default 20-byte ATT payload, and the large
Refloat layout uses 10 ms inter-chunk pacing.

The iOS T3 verifier builds a signed XCTest host containing the production
CoreBluetooth adapter, application service, VESC codec, adaptive sampler, and
SQLite store. It emits sanitized durable-session evidence back to the host
runner:

```sh
cd tools/vesc-test-device/host
npm run verify:ios:ride -- \
  --port "$FLOAT_VIEW_ESP_PORT" --device "$FLOAT_VIEW_IOS_DEVICE_ID"
npm run verify:ios:unexpected-disconnect -- \
  --port "$FLOAT_VIEW_ESP_PORT" --device "$FLOAT_VIEW_IOS_DEVICE_ID"
```

Both iOS gates passed on 2026-07-28. The manifests and bounded transcripts are
written under `artifacts/verification/vesc-test-device/*-ios-*-passed-*`.
Physical iOS delivery jitter produced 148 of the scenario's nominal 160 durable
samples, above the platform gate of 144 (90%), while all 168 telemetry responses
and every request-cadence phase passed.
