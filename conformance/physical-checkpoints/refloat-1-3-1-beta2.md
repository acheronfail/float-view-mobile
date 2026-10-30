# Refloat 1.3.1-beta2 physical checkpoint

The audited Refloat source is `v1.3.1-beta2` at commit
`ebe3eb5886b9f7c6247d9a06abeafa8415a94542`.

On the available production host and board route, record the app build and
firmware identity, then verify discovery and connection; advancing fast and slow
telemetry; saved ride logging and reopen; complete schema, current, and default
configuration reads; a backed-up non-actuating configuration write with full
readback and restoration; eligible same-version Tune application with restoration;
and disconnect/reconnect. Check the exact INFO suffix and schema fingerprint
against the pinned source. Verify no pending write or unsafe state before each
mutation. Keep original and restored configuration snapshots, device and route
inventory, scripts, and raw results under ignored `.local/physical-testing/`.
Publish a brief result here after the run, with the actual host/route and any
coverage limits.

Do not induce a real IMU failure. Its activation, clearing, malformed history,
and mutation gates are covered through source-derived simulation and software
checks. Do not run motor-control, remote, firmware-write, or shutdown commands
for this checkpoint.

## Result — 2026-10-31

The production macOS BLE host built from the uncommitted implementation on
`fae1ead076a664ba30c4c44a0e6fdaa0a758be7f` connected to the user's
ExpressT bridge and its CAN-routed controller. The controller reported Refloat
`1.3.1-beta2`. Fast and slow telemetry advanced with an inactive IMU timeout.
The app read the audited 172-field schema, complete current and default values,
and the expected schema fingerprint and wire signature. A backed-up startup LED
setting was written, fully read back and restored. An exact-version Tune made
from unchanged portable values was applied, fully read back and restored. A
stationary session persisted samples, reopened after disconnect, and the app
reconnected through the same route. The original configuration captured before
the first write and the final restored configuration had no field differences.
Private scripts, backups and raw results are in ignored
`.local/physical-testing/refloat-131-beta2-20261031/`.

ExpressT's controller has damaged MOSFETs and unreliable duty/eRPM readings, so
this checkpoint makes no claim about motor output, ride-state transitions or
loaded riding telemetry. It covers one macOS BLE/Express/CAN route; iOS,
Android, other routes and a real-controller IMU failure were not physically
tested. The app's software fixtures cover those protocol states and write gates.

A separate source-derived IMU timeout scenario passed through the production
macOS BLE host and connected ESP32 byte bridge on the direct fixture route. It
confirmed the active warning, blocked package/flywheel/configuration writes,
clearing and transient recovery, and four durable timeout history events without
a firmware fault. The fixture returned to READY. This verifies the native BLE
path for those responses without claiming a physical IMU sensor failure.
