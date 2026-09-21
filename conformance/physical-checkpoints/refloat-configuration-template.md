# Refloat configuration physical checkpoint

Date: YYYY-MM-DD  
Native target: Apple or Android, device/build identifier  
BLE fixture: ESP32 serial identifier  
Routes: direct and once-forwarded CAN  
Exact Refloat release/fingerprint/signature: record from the audited UI

This checkpoint is intentionally outside `just verify`. Run it only with a
fixture-safe, non-actuating schema field and retain the full original
configuration until restoration has been verified. Compare every restored field
against the original plus documented firmware normalization (for example,
`meta.is_default` is cleared by a write); record any such normalization explicitly.

## Required evidence

- [ ] The native backend read a complete current configuration and defaults on
      the direct route.
- [ ] One fixture-safe non-actuating field changed, command 95 was acknowledged,
      and command 93 verified the complete expected readback.
- [ ] The complete original direct-route configuration was restored and a fresh
      command-93 read verified every field. Any failure here fails the checkpoint.
- [ ] The same current/default/change/full-readback/full-restore sequence passed
      through exactly one `COMM_FORWARD_CAN` wrapper on the selected CAN route.
- [ ] Disconnect/reconnect reset transient operation state and retained no stale
      current write authority.
- [ ] Fixture counters and diagnostics show only discovery, telemetry, commands
      92–95, and any explicitly exercised semantic controls.
- [ ] No motor/remote, Handtest, recording/history, arbitrary app-data tuning,
      firmware write, or other actuation command was observed.
- [ ] Runtime lighting remained distinct from persistent startup lighting fields.

Record timeouts, readback mismatch, restoration retry, or unavailable hardware
verbatim. Do not mark a route or platform passed using mock/software evidence.
