# Shared BLE transport physical checkpoint

Date: 2026-09-29. Status: complete for the requested BLE-focused checkpoints on all three hosts.

The ESP32 is the existing protocol-agnostic serial-controlled BLE byte bridge.
Conformance owns the VESC/Refloat simulation. These runs use production native
Bluetooth adapters and the process-owned Rust transport policy, with unreachable
software-relay endpoints as required by the physical targets.

## Completed evidence

- macOS passed identity, guest, direct telemetry, direct and CAN configuration /
  controls, and unexpected-disconnect recovery. Configuration checks included
  restoration and complete readback. An initial second-process launch retained
  radio `unknown` and timed out before connecting; a clean six-scenario rerun
  passed. Diagnostics reported with-response, negotiated payload 512 and shared
  cap 244. The supplemental control/reconnect run measured an actual maximum received payload of 244 bytes and passed generation-qualified cancellation.
- Android SM-G965F / API 29 passed the same six scenarios plus background. BLE
  telemetry and durable session continuity survived backgrounding and recovery;
  the location checkpoint recorded one new persisted sample during the 10-second
  dwell. The supplemental direct configuration/control run measured an actual
  maximum BLE write payload of 20 bytes at the ESP32. Native diagnostics reported
  with-response and ceiling 20. A final reconnect rerun passed after qualifying
  native cancellation by lifetime and generation.

- iPhone 12 passed identity, guest, direct telemetry, direct and CAN
  configuration/controls, unexpected disconnect and background. Its measured
  maximum incoming write was 244 bytes on both configuration routes. The first
  background run persisted two new location samples; final reconnect/background
  regressions also passed with qualified cancellation.

All three hosts used the fixture's profile with both write properties and the
shared with-response preference. macOS and Android diagnostics directly confirmed
that mode; the iOS mode follows the same shared profile decision (its launch
harness does not capture application stdout). This does not claim an independent
over-the-air ATT trace for any host.

Reports are retained in [the checkpoint directory](2026-09-29-share-ble-transport-policy/).
The runner now records maximum incoming BLE write size independently of VESC
frame reconstruction, and physical acceptance asserts the host-specific ceiling.
No firmware or native protocol simulation was added for this instrumentation.

## Coverage limits

The fixture profile selected with-response on the measured hosts. Physical
write-without-response pacing/readiness, missing CCCD, intentionally missing OS
write completions, queue saturation, and injected duplicate native callbacks
were not exercised by this firmware configuration. They are not reported as
physical passes. Deterministic shared reducer / OS-exchange tests cover both
write modes, separate submission and completion, stale/duplicate generations and
operations, atomic queue rejection, cancellation, missing results, deadlines and
cleanup admission. Android's absent-CCCD routing remains native and unchanged in
meaning; Android keeps MTU 23 / 20-byte payload / 20 ms no-response pacing, while
Apple uses negotiated capability and its native readiness callback.

This is the requested BLE-focused checkpoint, not a claim that every unrelated
scenario in the larger physical suite was rerun. Canonical software conformance
and the full repository software gate cover the wider unchanged product surface.
