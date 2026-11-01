# Refloat 1.3.1-beta3 physical checkpoint

On 2026-11-02 the production macOS BLE host, built from the uncommitted beta3
implementation on `fae1d4a3`, admitted Refloat `v1.3.1-beta3` at
`e6388f75e63919bc42a3e10cecb68824ac413ae4` through ExpressT's Express/CAN route.
Fast/slow telemetry advanced, all 172 configuration fields/current/defaults matched
the audited schema, and a startup LED write and unchanged same-version Tune passed
complete readback and restoration. Saved stationary samples reopened after
disconnect and the app reconnected. The first backup and final freshly read
configuration had no field differences; isolated host/storage cleanup completed.

The later fixture/profile consolidation leaves production behavior unchanged and
retains this checkpoint. The damaged motor stage and unreliable duty/eRPM exclude
motor output, loaded riding and ride-state claims. Other real-device routes,
iOS/Android and induced IMU failure were not physically checked. Follow the
[configuration procedure](refloat-configuration-template.md) for future runs.
Detailed source copies/hashes, attempt logs, scripts and backups remain under
ignored `.local/physical-testing/refloat-131-beta3-20261102/`.
