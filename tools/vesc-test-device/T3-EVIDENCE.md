# T3 deterministic Express/Thor/Refloat ride evidence

Date: 2026-07-28

Status: host/application foundations and the physical macOS happy path pass;
Android, iOS, and injected-fault gates remain.

## Implemented

- A schema-validated 168-request ride scenario with occurrence-driven phases:
  idle, riding, footpad warning, 15-second post-ride tail, and return to idle.
- Captured 20-byte notification boundaries plus single-notification and
  deliberately fragmented 1/2/3/5/8/13/27-byte patterns.
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

## Host and application verification

```text
TypeScript check
  PASS

Host tests
  PASS: 26 tests

Apple tests
  PASS: focused adaptive-sampling tests (2)

Android :core:test
  PASS

macOS guided verifier compile-only check
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

## Pending physical gates

- Android run using the same scenario and response assets;
- iOS run using the same scenario and response assets;
- explicit unexpected-disconnect run and cleanup evidence;
- explicit normalized firmware-fault fixture/evidence.

Each additional physical run requires an operator prompt immediately before
execution. The passing macOS command was:

```sh
cd tools/vesc-test-device/host
npm run verify:macos:ride -- --port "$FLOAT_VIEW_ESP_PORT"
```
