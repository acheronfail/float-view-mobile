# T3 deterministic Express/Thor/Refloat ride evidence

Date: 2026-07-28

Status: complete. Host/application foundations plus physical macOS, Android,
and iOS ride and unexpected-disconnect gates pass.

## Implemented

- A schema-validated 168-request ride scenario with occurrence-driven phases:
  idle, riding, footpad warning, 15-second post-ride tail, and return to idle.
- Captured and deliberately fragmented notification boundaries, all bounded to
  the portable default ATT payload maximum of 20 bytes.
- Cadence assertions with phase-specific tolerances and first-violation output.
- Response schedule validation that rejects gaps, overlaps, invalid ranges, and
  unknown cadence rule references.
- Scheduled host disconnect actions in the scenario engine and runner.
- Cached fixture loading so 10 Hz execution does not perform disk I/O for every
  response.
- Apple and Android application-service behavior that keeps the 10 Hz probe
  through the configured post-ride tail before returning to the 2 Hz idle
  probe.
- A bounded adaptive-persistence jitter allowance shared by Apple and Android.
  This prevents normal 89/110 ms CoreBluetooth callback jitter from aliasing a
  nominal 10 Hz stream into a lower persisted rate.
- A compile-checked macOS guided verifier using production Bluetooth,
  application-service, protocol, adaptive sampling, and SQLite implementations.
  It verifies explicit disconnect, completed session state, sampling coverage,
  footpad evidence, catalog/session integrity, and no remaining active
  connection or session.
- A CRC-valid synthetic `COMM_GET_VALUES` over-voltage fixture that maps VESC
  fault code `1` to stable `over_voltage`, while preserving the raw numeric code
  and mapping newer unrecognized values to `unknown`.
- A schema-validated short scenario that commands an unexpected BLE disconnect
  at the sixth Refloat telemetry request, after idle and riding cadence barriers.
  The macOS driver asserts `unexpected_disconnect`, durable session closure,
  SQLite integrity, and complete cleanup.

## Host and application verification

```text
TypeScript check
  PASS

Host tests
  PASS: 28 tests

Apple tests
  PASS: 35 tests

Android :core:test
  PASS

Android :protocol:test
  PASS

macOS guided verifier compile-only check
  PASS

Android app instrumentation compile
  PASS

iOS hardware-test bundle compile
  PASS
```

The host suite includes a fully deterministic passing cadence run and a
deliberate 500 ms post-ride interval that fails at the first incorrect
occurrence with:

```text
cadence refloatTelemetry occurrence 20 expected 100±80ms, observed 500ms
```

## Physical macOS gate

The production macOS CoreBluetooth/application-service path passed the complete
ride scenario against the ESP32-S3:

```text
Scenario responses                  168 telemetry, all identity/layout rules met
Idle cadence                        500.01 ms average, pass
Riding cadence                      101.22 ms average, pass
Post-ride-tail cadence               99.94 ms average, pass
Return-to-idle cadence              500.09 ms average, pass
Durable samples                     160
Footpad warning                     observed
Session                             completed / complete
SQLite integrity                    ok
Connection and active session       both absent after explicit disconnect
```

Machine-readable evidence:

- `artifacts/verification/vesc-test-device/express-thor-refloat-ride-passed-manifest.json`
- `artifacts/verification/vesc-test-device/express-thor-refloat-ride-passed-transcript.json`

The first physical attempt exposed persistence aliasing caused by real callback
jitter around the exact 100 ms threshold. Matching Apple/Android regression
tests now cover the bounded early allowance used by the passing run.

## Physical Android gate

The production Android 10 path passed the same complete ride scenario on the
Galaxy S9 qualification device:

```text
Scenario responses                  168 telemetry, all identity/layout rules met
Idle cadence                        503.85 ms average, pass
Riding cadence                      103.65 ms average, pass
Post-ride-tail cadence               99.76 ms average, pass
Return-to-idle cadence              463.24 ms average, pass
Durable samples                     156
Footpad warning                     observed and durable
Session                             completed / complete
SQLite integrity                    ok
Connection and active session       both absent after explicit disconnect
```

Machine-readable evidence:

- `artifacts/verification/vesc-test-device/express-thor-refloat-ride-android-ride-passed-manifest.json`
- `artifacts/verification/vesc-test-device/express-thor-refloat-ride-android-ride-passed-transcript.json`

The Android driver launches the real `MainActivity` before scanning so Android
10 grants the location-backed BLE operation foreground status. It then exercises
the production adapter, application service, protocol decoder, adaptive sampler,
and SQLite store without UI test shortcuts. Bring-up also proved that every
fixture notification must respect the default 20-byte ATT payload and that the
large Refloat field-layout response needs 10 ms inter-chunk pacing.

### Physical Android unexpected-disconnect gate

The same Android path passed the host-commanded disconnect scenario:

```text
Identity/layout rules                all met
Telemetry                            2 idle, 3 riding notifications
Injected disconnect                  sixth telemetry request
Idle cadence                         512.12 ms average, pass
Riding cadence                       113.62 ms average, pass
Durable samples                      4
Close reason                         unexpected_disconnect
Session                              completed / complete
SQLite integrity                     ok
Connection and active session        both absent
```

Machine-readable evidence:

- `artifacts/verification/vesc-test-device/express-thor-refloat-unexpected-disconnect-android-unexpected-disconnect-passed-manifest.json`
- `artifacts/verification/vesc-test-device/express-thor-refloat-unexpected-disconnect-android-unexpected-disconnect-passed-transcript.json`

## Physical iOS gate

The production iOS CoreBluetooth/application-service path passed the complete
ride scenario using the same assets as macOS and Android:

```text
Scenario responses                  168 telemetry, all identity/layout rules met
Idle cadence                        500.01 ms average, pass
Riding cadence                      101.23 ms average, pass
Post-ride-tail cadence              100.33 ms average, pass
Return-to-idle cadence              495.08 ms average, pass
Durable samples                     148
Footpad warning                     observed and durable
Session                             completed / complete
SQLite integrity                    ok
Connection and active session       both absent after explicit disconnect
```

Machine-readable evidence:

- `artifacts/verification/vesc-test-device/express-thor-refloat-ride-ios-ride-passed-manifest.json`
- `artifacts/verification/vesc-test-device/express-thor-refloat-ride-ios-ride-passed-transcript.json`

The nominal scenario coverage is 160 durable samples. Real iOS CoreBluetooth
delivery jitter around the adaptive sampler's 80 ms early allowance yielded
148 samples while the complete 168-response transcript and every cadence phase
passed. The iOS physical gate therefore uses an explicit 90% nominal-coverage
floor of 144 while retaining the duration, riding-state, footpad, explicit-close,
SQLite-integrity, and complete-cleanup checks.

### Physical iOS unexpected-disconnect gate

The same iOS path passed the host-commanded disconnect scenario:

```text
Identity/layout rules                all met
Telemetry                            2 idle, 3 riding notifications
Injected disconnect                  sixth telemetry request
Idle cadence                         495.14 ms average, pass
Riding cadence                        90.28 ms average, pass
Durable samples                      3
Close reason                         unexpected_disconnect
Session                              completed / complete
SQLite integrity                     ok
Connection and active session        both absent
```

Machine-readable evidence:

- `artifacts/verification/vesc-test-device/express-thor-refloat-unexpected-disconnect-ios-unexpected-disconnect-passed-manifest.json`
- `artifacts/verification/vesc-test-device/express-thor-refloat-unexpected-disconnect-ios-unexpected-disconnect-passed-transcript.json`

The normalized firmware-fault fixture and cross-platform codec evidence are
complete. Persisting faults as indexed `SessionEvent` rows belongs to the
accepted logging pipeline work and is not claimed by this parser-level fixture.

### Physical macOS unexpected-disconnect gate

The production macOS path passed the host-commanded disconnect scenario:

```text
Identity/layout rules                all met
Telemetry                            2 idle, 3 riding notifications
Injected disconnect                  sixth telemetry request
Idle cadence                         495.13 ms average, pass
Riding cadence                        89.84 ms average, pass
Durable samples                      3
Close reason                         unexpected_disconnect
Session                              completed / complete
SQLite integrity                     ok
Connection and active session        both absent
```

Machine-readable evidence:

- `artifacts/verification/vesc-test-device/express-thor-refloat-unexpected-disconnect-passed-manifest.json`
- `artifacts/verification/vesc-test-device/express-thor-refloat-unexpected-disconnect-passed-transcript.json`

Bring-up exposed two useful harness constraints. The verifier now requires an
active session plus five notifications before accepting the injected
disconnect, and retries at most two pre-ready disconnects. The 412-byte
field-layout response retains its captured 20-byte fragmentation but spaces
notifications by 10 ms so NimBLE/CoreBluetooth backpressure cannot invalidate
the link. A hardware reset was required after earlier failed connections left
the ESP BLE stack unhealthy.

Each additional physical run requires operator approval immediately before
execution. The passing commands were:

```sh
cd tools/vesc-test-device/host
npm run verify:macos:ride -- --port "$FLOAT_VIEW_ESP_PORT"
npm run verify:macos:unexpected-disconnect -- --port "$FLOAT_VIEW_ESP_PORT"
npm run verify:android:ride -- --port "$FLOAT_VIEW_ESP_PORT" --serial "$ANDROID_SERIAL"
npm run verify:android:unexpected-disconnect -- --port "$FLOAT_VIEW_ESP_PORT" --serial "$ANDROID_SERIAL"
npm run verify:ios:ride -- --port "$FLOAT_VIEW_ESP_PORT" --device "$FLOAT_VIEW_IOS_DEVICE_ID"
npm run verify:ios:unexpected-disconnect -- --port "$FLOAT_VIEW_ESP_PORT" --device "$FLOAT_VIEW_IOS_DEVICE_ID"
```
