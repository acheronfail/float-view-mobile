## Why

VESC framing, firmware identity, and custom-configuration envelopes are now Rust-owned on every host, while command-96 BMS values decoding remains a substantial duplicated Swift/Kotlin wire parser. The safest next increment is to prove that one stateless, source-derived decoder in safe Rust before adding a host bridge or moving BMS discovery, polling, application state, or persistence.

## What Changes

- Add one safe Rust candidate under `backends/core` with an empty production dependency graph for strict `COMM_BMS_GET_VALUES = 96` response decoding only.
- Reproduce the existing signed fixed-point and auto-float rules, cell/temperature count bounds, balancing flags, reported-CAN validation, legacy and extended tails, UTF-8 status bound, exact remainder handling, finite-number checks, and established structural-versus-unsupported failures.
- Verify the candidate against the immutable source-derived VBMS/VBMS32 values fixtures plus minimum, maximum, truncation, malformed, and privacy boundary cases.
- Add locked formatting, warnings-denied linting, dependency, workspace-layout, privacy, and production-isolation audits without adding or changing any Apple C ABI or Android JNI symbol.
- Keep Swift and Kotlin as the only production BMS values decoders. Separate later OpenSpec changes will cut over Apple and Android independently and delete each native parser only after bridge, packaging, canonical, and physical verification.
- Run the complete software gate and focused standalone/direct-controller/Express BMS physical regression on the connected Android device, iOS device, and macOS host. These runs continue to exercise the selected native decoders and do not claim the Rust candidate executed.
- Non-goals: request scheduling, BMS discovery or compatibility, CAN routing, firmware identity, Refloat telemetry, application behavior, SQLite, local webserver, Bluetooth, frontend contract, stored data, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define bounded parity, ownership, and error requirements for an unselected Rust VESC BMS values decoder candidate.
- `platform-hosts`: Locate the focused candidate while preserving Swift and Kotlin production ownership and every native service boundary.
- `backend-conformance`: Require source-derived/adversarial component evidence, complete software verification, and all-host physical BMS regression without presenting the candidate as a supported backend path.

## Impact

This affects the locked `backends/core` workspace, one new safe Rust crate, focused tests over the existing `backends/fixtures/vesc-bms/source-derived.json`, core ownership documentation, root workspace/dependency audits, `just verify`, and OpenSpec requirements. It adds no normal or third-party development dependency, generated binding, native bridge function, packaged library, frontend-contract change, database change, production runtime change, or user-visible behavior. No Swift or Kotlin production code is removed in this step.
