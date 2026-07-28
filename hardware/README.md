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

## R6 failure-state qualification

The remaining permission, radio-cycle, out-of-range, explicit-disconnect, and
local-profile-override gates share one checked-in scenario definition:
`hardware/scenarios/r6-failure-states.json`. Generate a new evidence template
without overwriting an existing file:

```sh
just hardware-r6-guide ios vesc-6_06-refloat-1_2 /tmp/ios-r6-failures.json
```

The command prints the ordered procedure to stderr and creates a redacted JSON
template. After filling only its observations and notes, validate it with:

```sh
just hardware-r6-verify ios vesc-6_06-refloat-1_2 /tmp/ios-r6-failures.json
```

The validator requires every declared terminal state, rejects incomplete
scenarios, and refuses fields that could contain device/session identifiers,
tokens, raw packets, or telemetry.

Android permission setup can be reproduced without navigating system settings:

```sh
just android-r6-permission status
just android-r6-permission deny
just android-r6-permission grant
just android-r6-permission reset
```

These reversible commands are intentionally manual and never run inside
`verify-r6`. `reset` moves only Float View's permission-history preference aside
and restores the OS permission to its pre-prompt state; on Android 10 it revokes
both members of the legacy fine/coarse location permission group so Samsung
cannot silently restore fine location. Session databases are untouched. iOS and
macOS permission/radio changes remain user actions because the applications
must not automate or bypass their platform privacy controls.
