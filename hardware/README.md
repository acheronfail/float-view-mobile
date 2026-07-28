# Hardware qualification

Hardware profiles declare exactly which read-only commands, firmware/package
ranges, transport/controller identities, and durable-session outcomes a physical
smoke run may use. `scripts/hardware-smoke.mjs` has its own fixed operation
allowlist and rejects a profile merely claiming that an unknown operation is
read-only.

Evidence files are redacted acceptance records produced after the corresponding
platform workflow and durable-store inspection. They contain no BLE address,
CoreBluetooth peripheral UUID, session ID, token, or raw user telemetry. A
known-bad controller may prove transport and lifecycle behavior, but its evidence
must leave `numericPlausibilityValidated` false.

Validate a recorded run from the repository root:

```sh
just hardware-smoke android vesc-6_06-refloat-1_2
just hardware-smoke ios vesc-6_06-refloat-1_2
just hardware-smoke macos vesc-6_06-refloat-1_2
```

To validate a newly captured evidence file before replacing the accepted record:

```sh
just hardware-smoke android vesc-6_06-refloat-1_2 /tmp/android-evidence.json
```

The current command validates safety and evidence. Device launch, user-confirmed
scan/connect/disconnect, and platform-specific database extraction remain the
reproducible `*-device-*` recipes in the `justfile`. A later test-control host
will orchestrate those pieces without exposing control or unsafe commands in a
release build.
