# Refloat 1.3.1-beta1 checkpoint

Passed on 2026-10-23 with an actual Thor300 running VESC 6.06 and exact
`1.3.1-beta1`, reached through a CAN controller route over production macOS BLE.
The Debug working-tree build was based on `fae15048`; audited Refloat source was
`93f0f1f6026531302e75c7f900261b55a6b84b3a` (`v1.3.1-beta1`).

Exact identity admission, advancing fast/slow telemetry, inactive IMU-timeout
state, complete 172-field schema/current/default reads, and the audited schema
fingerprint/signature passed. One non-actuating startup LED setting was changed
and its complete readback verified. The complete original configuration was
restored and freshly verified, apart from firmware's documented clearing of
`meta.is_default` on any configuration write. Runtime LED control and restoration,
idempotent package control, durable session sample queries, and BLE
disconnect/reconnect with route and restored-configuration revalidation passed.
The connection was disconnected and the isolated test host/storage cleaned up.

Coverage is real hardware on macOS through CAN. Direct-controller BLE and mobile
hosts were not physically exercised in this checkpoint. Actual IMU failure was
not induced; timeout activation, clearing, mixed fault history, and malformed
responses remain established by source-derived simulation and software tests.
No flywheel, motor-control, remote, Handtest, firmware-write or shutdown command
was requested.

Full original/restored configuration snapshots, device/route details, script,
raw logs and results are retained under ignored `.local/physical-testing/`.
Future repeats should preserve the same full backup/readback/restore procedure;
do not induce a hazardous IMU failure merely to expand coverage.
