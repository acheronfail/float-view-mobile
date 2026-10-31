# Conformance

This directory owns canonical black-box contract tests, software VESC simulation
and physical orchestration. Host adapters launch/tunnel/clean up; scenarios and
assertions remain shared. The ESP32 is only a serial/BLE byte bridge.

## Commands and ownership

Run from `conformance/`:

```sh
npm ci
npm test
npm run mock
npm run direct
npm run physical -- macos <fixture-port>
npm run physical -- android <fixture-port> <adb-serial>
npm run physical -- ios <fixture-port> <device-id>
```

`direct` uses the production Rust service with synthetic OS facts. `macos`, `ios`
and `android` use software native hosts. Select coverage with the root
[verification policy](../AGENTS.md#verification).

| Location                | Responsibility                                   |
| ----------------------- | ------------------------------------------------ |
| `src/suite.ts`          | Canonical public-contract scenarios              |
| `fixture/`              | Reusable protocol simulation and topology        |
| `fixture-dev/`          | Manual fixture controller                        |
| `test/*.test.ts`        | Harness and simulator tests                      |
| `test/*.browser.ts`     | Chromium/WebKit interaction and rendering checks |
| `physical-checkpoints/` | Reusable procedures and brief outcomes           |

Build the frontend before browser tests; use the
[development guide](../docs/development.md#verification) for commands, shared-port
serialization and macOS sandbox requirements. Test files own their detailed
assertion inventories; public behavior is specified under [OpenSpec](../openspec/specs/).

## Live viewport rendering

`npm run test:live-viewports` builds and captures the Live and detailed views in
Chromium/WebKit. Open `generated/live-viewports/index.html` for the gallery, PNGs,
contact sheets and geometry reports; `FLOAT_VIEW_VIEWPORT_OUTPUT` overrides output.
For Custom, run `node --strip-types --test test/custom-dashboard.browser.ts` after
building. Its captures go to `generated/custom-dashboard/`.

Viewport emulation does not establish physical safe areas, hinge behavior or
fold-device gesture integration.

## On-demand native layout captures

`Capture iOS layout` is manually dispatched for pushed `apple-duo/*` branches;
other branches/tags are rejected by the job guard. It is separate from ordinary
checks and release builds:

```sh
gh workflow run capture-ios-layout.yml --ref apple-duo/side-controls
```

Pass `-f verify_device_build=true` for the optional unsigned device-SDK build.
Download `ios-layout-<run-id>` for native screenshots and geometry/source reports;
keep useful captures under ignored `.local/physical-testing/ios-layout/`.
The diagnostic accepts the agreement only in its opted-in Debug simulator.
It performs no device-data, release-signing or physical BLE work.

The [workflow](../.github/workflows/capture-ios-layout.yml) controls SDK/runtime and
artifact reuse. Frontend-only captures can reuse a native app only when its
recorded native-input key matches. Artifacts expire; do not assume a warm build.
For local frontend iterations using captured bounds, run from the repository root:

```sh
(cd frontend && npm run build)
(cd conformance && node --strip-types --test test/side-controls.browser.ts)
```

These previews exclude native status UI; simulator captures are not physical validation.

## Interactive fixture development

Run `just fixture dev [port]` beside `just dev <platform>`. The controller opens
at `http://127.0.0.1:11004`; see [fixture-dev/README.md](fixture-dev/README.md).
The manual workbench retains editable inputs and reports requests. Automated
runners own assertions. Shareable documents contain bounded public simulation
inputs; private controller identities remain server-owned.

## Physical checkpoints

Choose only affected routes, devices and scenarios. Procedures and brief outcomes
live in [physical-checkpoints/](physical-checkpoints/README.md); keep detailed
inventories, logs and completed result templates under ignored `.local/physical-testing/`.

Set `FLOAT_VIEW_PHYSICAL_REFLOAT_VERSION=1.3.0` for selectable-telemetry controller
scenarios; legacy and standalone-BMS scenarios retain their own identities.
If mobile background location cannot acquire a fix promptly, an explicit deferral is:

```sh
FLOAT_VIEW_PHYSICAL_SCENARIOS=background FLOAT_VIEW_DEFER_PHYSICAL_LOCATION=1 npm run physical -- ios <fixture-port> <device-id>
```

This leaves authorization/fix acquisition for manual verification while still
checking lifecycle, BLE and persistence. Without that explicit deferral, missing
background observations are incomplete coverage.

For control/configuration checks, use fixture-safe, non-actuating inputs. Confirm
no motor/remote/Handtest actuation occurred. Capture full current/default settings,
change one safe field, verify complete readback, then restore and verify the entire
original configuration. Failed restoration fails the run. Use the
[configuration template](physical-checkpoints/refloat-configuration-template.md).
Shutdown checks establish transmission acceptance, not observed power-off, and
must not retry. The fixture must return to RESET/READY on cleanup.

For BMS changes, select affected direct-controller/CAN-BMS, Express/CAN-BMS or
standalone-BMS routes. Record unavailable hardware rather than replacing it with
software assertions. Firmware identities and byte layouts live in source-derived
fixtures and decoder tests; see the [compatibility audit procedure](../backends/README.md#auditing-another-vbms-wire-version).

The macOS physical ride scenario opens a native Save panel in a disposable bundle:
inspect/dismiss it and press Enter within its bounded wait. Android checks the
destination picker; iOS uses the lifecycle UI runner for its share sheet.

## Focused scenarios

| Area                             | Command from `conformance/`                                                                            |
| -------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Fault latch and history over BLE | `FLOAT_VIEW_PHYSICAL_SCENARIOS=fault-direct,fault-can npm run physical -- macos <fixture-port>`        |
| Beta2 IMU timeout over BLE       | `FLOAT_VIEW_PHYSICAL_SCENARIOS=imu-timeout-beta2 npm run physical -- macos <fixture-port>`             |
| Beta2 software contract          | `FLOAT_VIEW_FIXTURE_REFLOAT=1.3.1-beta2 FLOAT_VIEW_CONFORMANCE_SCENARIOS=refloat-beta2 npm run direct` |
| Auto-connect software contract   | `FLOAT_VIEW_CONFORMANCE_SCENARIOS=auto-connect npm run direct`                                         |
| Auto-connect over BLE            | `FLOAT_VIEW_PHYSICAL_SCENARIOS=auto-connect npm run physical -- android <fixture-port> <adb-serial>`   |
| Announcement settings contract   | `FLOAT_VIEW_CONFORMANCE_SCENARIOS=announcements npm run direct`                                        |
| Automatic speech effects         | `node --strip-types --test test/announcements.test.ts`                                                 |

Use equivalent supported native/software targets where applicable. Fault scenarios
inject source-derived responses; they do not induce failures on a real board.
Auto-connect physical scenarios reject configuration/control/shutdown mutations;
macOS activation and first-use permission prompts need separate observations.
Speech effects and native-driver replay establish software behavior, not audible
output or locked-device routing. Native replay commands are in the
[development guide](../docs/development.md#verification).

## Temporal battery verification

[fixtures/battery-temporal.json](fixtures/battery-temporal.json) contains shared,
independently calculated oracles for Rust and mock tests. The tests own arithmetic,
tolerances and edge-case coverage; synthetic stability/accounting does not establish
measured absolute SOC accuracy or empirical tuning against real ride traces.

`npm run measure` writes synthetic performance results to ignored
`.local/physical-testing/software-measurements.json` by default. Pass an explicit
local output filename to retain a separate run.
