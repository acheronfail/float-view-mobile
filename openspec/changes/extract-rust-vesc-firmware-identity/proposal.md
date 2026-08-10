## Why

VESC framing is now Rust-owned on every host, but the next protocol layer still begins with duplicated Swift and Kotlin `COMM_FW_VERSION` identity parsers that decide controller, gateway, and BMS discovery. The safest next increment is to prove that bounded stateless decoder in Rust without combining parser behavior with another FFI, packaging, lifecycle, or production-ownership cutover.

## What Changes

- Add one safe Rust component under `backends/core` with an empty production dependency graph for strict VESC firmware identity decoding only.
- Reproduce the existing common identity prefix, bounded hardware and firmware names, private 12-byte UUID, test-version marker, hardware type, custom-configuration count, stock VESC BMS 6.00 tail, motor/custom-module capability tail, and optional big-endian hardware-configuration CRC behavior.
- Verify the candidate against the repository's immutable THOR300, VESC Express, stock VBMS, and VBMS32 source-derived payloads plus bounded malformed and privacy cases.
- Add the candidate to locked formatting, warnings-denied linting, dependency, workspace-layout, and unit-test verification while keeping the existing framing bridges and artifacts unchanged.
- Retain the Swift and Kotlin firmware identity decoders as the only production owners. Separate later OpenSpec changes will cut over Apple and Android independently and delete each superseded parser only after its own bridge, packaging, canonical, and physical checks pass.
- Run canonical software verification and focused physical identity regression on the connected Android device, iOS device, and macOS host through the ESP32 fixture; the physical runs continue to exercise the currently selected native parsers in this candidate-only step.
- Non-goals: adding or changing FFI/JNI symbols, selecting Rust from a host, deleting Swift/Kotlin identity code, moving compatibility policy, CAN routing, BMS values, Refloat, Bluetooth, application behavior, SQLite, the loopback server, the frontend contract, stored data, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define bounded parity, errors, privacy, dependency, and candidate-ownership requirements for the Rust firmware identity decoder.
- `platform-hosts`: Permit and locate the focused firmware identity candidate while preserving Swift and Kotlin production ownership until separate host cutovers.
- `backend-conformance`: Require source-derived component parity and all-host physical identity regression evidence without presenting the candidate as a supported backend path.

## Impact

This affects the locked `backends/core` workspace, one new safe Rust crate, focused unit/fixture tests, core ownership documentation, root workspace/dependency audits, `just verify`, and OpenSpec requirements. It adds no normal or third-party development dependency; tests may use the existing workspace-local framing crate to unwrap committed framed fixture evidence. It adds no generated binding, native bridge function, packaged library surface, frontend-contract change, database or stored-data change, production runtime change, or user-visible behavior. No Swift or Kotlin production source is removed in this step.
