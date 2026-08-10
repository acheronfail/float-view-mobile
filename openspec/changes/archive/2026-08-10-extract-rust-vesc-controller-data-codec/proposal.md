## Why

Apple and Android now share Rust ownership for framing, firmware identity, custom-configuration envelopes, BMS values, and Refloat controls, but still duplicate the same stateless VESC controller read-data parsing. The safest next increment is to prove the small `COMM_GET_VALUES` realtime subset and `COMM_GET_MCCONF` current-limit subset in safe Rust before adding either host bridge or moving Refloat telemetry, application behavior, persistence, or transport.

## What Changes

- Add one safe, unselected Rust candidate under `backends/core` with an empty production dependency graph for the standard VESC realtime scalar subset and motor-configuration current-limit subset already consumed by both hosts.
- Preserve command validation, signed fixed-point and VESC auto-float decoding, selected realtime fields, raw fault code plus stable fault classification, firmware-signature-specific battery-series-cell offsets, optional/unrecognized cell-count behavior, finite/nonzero limit validation, and bounded payload failures.
- Verify the candidate against immutable repository fixtures, pinned upstream field ordering, every supported motor-configuration signature, exact scalar boundaries, truncation at every consumed field, malformed numeric values, wrong commands, optional tails, deterministic output, and payload-safe diagnostics.
- Add locked formatting, warnings-denied linting, dependency, workspace-layout, privacy, and production-isolation audits without adding or changing any Apple C ABI or Android JNI symbol.
- Keep Swift and Kotlin as the only production controller-data decoders. Separate later OpenSpec changes will cut over Apple and Android independently and delete each native decoder only after bridge, packaging, canonical, and physical verification.
- Run the complete software gate and focused physical identity/direct-CAN/background regression on the connected Android device, iOS device, and macOS host with the ESP32 fixture. These runs exercise the selected native decoders and do not claim that the Rust candidate executed.
- Non-goals: VESC firmware compatibility or routing, request construction, framing, Refloat telemetry/layouts, BMS, controller shutdown, scheduling, application behavior, SQLite, local webserver, Bluetooth, frontend contract, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define bounded parity, ownership, and error requirements for an unselected Rust VESC controller-data codec candidate.
- `platform-hosts`: Locate the focused candidate while preserving Swift and Kotlin production ownership and every native service boundary.
- `backend-conformance`: Require upstream-derived/adversarial component evidence, complete software verification, and all-host physical controller regression without presenting the candidate as a supported backend path.

## Impact

This affects the locked `backends/core` workspace, one new safe Rust crate, focused tests over existing VESC fixtures and pinned upstream layouts, core ownership documentation, root workspace/dependency audits, `just verify`, and OpenSpec requirements. It adds no normal or third-party development dependency, generated binding, native bridge function, packaged library, frontend-contract change, database change, production runtime change, or user-visible behavior. No Swift or Kotlin production code is removed in this step.
