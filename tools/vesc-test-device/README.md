# Float View VESC test device

This directory contains the programmable BLE test peripheral described in
[`../../docs/vesc-test-device-plan.md`](../../docs/vesc-test-device-plan.md).

T0 and T1 completed on 2026-07-28. The current implementation:

- targets ESP32-S3 with ESP-IDF v6.0.2 and NimBLE;
- advertises the canonical VESC UART service UUID;
- exposes the canonical write and notify characteristics;
- boots disarmed and non-advertising;
- accepts sequenced, versioned JSON-line commands over USB Serial/JTAG;
- emits sequenced raw GATT-write and connection events;
- sends host-supplied notification bytes and host-commanded disconnects;
- enforces one central, bounded input/output, and one pending notification;
- returns to a disconnected, non-advertising state after host loss.

It is a production-shaped T1 transport, not yet a VESC protocol emulator. T0
results are in [`T0-EVIDENCE.md`](T0-EVIDENCE.md); current T1 results and the
remaining platform gates are in [`T1-EVIDENCE.md`](T1-EVIDENCE.md).

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
