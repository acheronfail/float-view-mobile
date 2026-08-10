## Why

VESC framing and firmware identity are now Rust-owned on every host, but the next protocol layer still contains duplicated Swift and Kotlin encoders and decoders for custom-configuration commands 92–95. The safest next increment is to prove that small, stateless envelope codec in safe Rust before adding another host bridge or moving the much larger Refloat schema and application lifecycle.

## What Changes

- Add one safe Rust component under `backends/core` with an empty production dependency graph for strict VESC custom-configuration request encoding and response decoding only.
- Reproduce the existing command-92 schema request/chunk layout, command-93 current and command-94 default value layouts, command-95 write/acknowledgement layout, signed big-endian extent rules, index handling, payload bounds, and established failure classes.
- Verify the candidate against all nine immutable Refloat configuration releases and adversarial boundary cases without parsing compressed schemas or typed Refloat values in this step.
- Add the candidate to locked formatting, warnings-denied linting, dependency, workspace-layout, privacy, and unit-test verification while keeping every existing Apple and Android bridge symbol and package unchanged.
- Retain the Swift and Kotlin custom-configuration codecs as the only production owners. Separate later OpenSpec changes will cut over Apple and Android independently and delete each superseded codec only after its own bridge, packaging, canonical, and physical checks pass.
- Correct the core ownership documentation to reflect the already-completed Android firmware-identity cutover while documenting the new candidate-only boundary.
- Run the complete software gate and focused ESP32 physical regression on the connected Android device, iOS device, and macOS host; these physical runs continue to exercise the selected native custom-configuration and Refloat paths rather than claiming the candidate executed.
- Non-goals: adding or changing FFI/JNI symbols, selecting Rust from a host, deleting Swift/Kotlin code, parsing Qt-compressed XML or typed Refloat values, moving configuration assembly/safety/scheduling, CAN routing, telemetry, BMS, Bluetooth, application behavior, SQLite, the loopback server, the frontend contract, stored data, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define bounded parity, error, dependency, and candidate-ownership requirements for the Rust VESC custom-configuration envelope codec.
- `platform-hosts`: Locate the focused candidate while preserving Swift and Kotlin production ownership until separate host cutovers.
- `backend-conformance`: Require immutable fixture parity, adversarial component evidence, complete software verification, and all-host physical regression without presenting the candidate as a supported backend path.

## Impact

This affects the locked `backends/core` workspace, one new safe Rust crate, focused tests over existing configuration fixtures, core ownership documentation, root workspace/dependency audits, `just verify`, and OpenSpec requirements. It adds no normal or third-party development dependency, generated binding, native bridge function, packaged library surface, frontend-contract change, database or stored-data change, production runtime change, or user-visible behavior. No Swift or Kotlin production source is removed in this step.
