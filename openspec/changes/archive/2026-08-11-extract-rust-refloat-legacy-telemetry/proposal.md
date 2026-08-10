## Why

Apple and Android still duplicate the same fixed-layout Refloat command-10 telemetry decoder even though its detail levels, numeric scales, semantic flags, and fault form are defined by pinned upstream source and immutable fixtures. The safest next increment is to prove that stateless legacy protocol boundary in safe Rust before adding a host bridge or taking on negotiated dynamic telemetry.

## What Changes

- Add one safe, unselected Rust candidate under `backends/core` for Refloat command-10 compact telemetry detail levels 1 through 4 and its active-firmware-fault response.
- Preserve exact package/command validation, payload lengths, signed fixed-point and auto-float decoding, speed conversion, state and flag semantics, nullable detail-level tails, firmware-fault mapping, and bounded failures.
- Treat upstream's field-weakening byte `222` as an unavailable-value sentinel rather than a measured current; this correct source-faithful behavior remains candidate-only in this change.
- Verify all committed source-derived detail-level and fault fixtures plus byte boundaries, state/mode/footpad/alert mappings, numeric extremes, invalid floating-point values, deterministic output, and payload-safe diagnostics.
- Extend locked workspace, dependency, unsafe-code, layout, privacy, and production-isolation audits without adding or changing an Apple C ABI, Android JNI symbol, packaged native library, or selected host call path.
- Keep Swift and Kotlin as the only production legacy telemetry decoders. Separate later OpenSpec changes will cut over Apple and Android independently and delete each native decoder only after bridge, packaging, canonical, and physical verification.
- Run `just verify` and focused physical identity, direct/CAN, and background/session regression on the connected Android device, iOS device, and macOS host with the ESP32 fixture. A bounded missing location sample remains deferred manual coverage only.
- Non-goals: dynamic Refloat commands 31/32, field-layout negotiation, compact-cache merging, polling, compatibility, controls, configuration, application state, SQLite, local webserver, Bluetooth, framing, routing, frontend contract, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define the parity, isolation, and ownership requirements for an unselected safe Rust Refloat legacy telemetry candidate.
- `platform-hosts`: Locate the candidate while preserving Swift and Kotlin production ownership and every native service boundary.
- `backend-conformance`: Require upstream-derived and adversarial component evidence, the complete software gate, and all-host physical regression without claiming candidate execution.

## Impact

This affects the locked `backends/core` workspace, one new dependency-free safe Rust crate, focused tests over committed Refloat fixtures and pinned upstream source, core ownership documentation, root workspace/dependency audits, `just verify`, and OpenSpec requirements. It adds no runtime dependency, generated binding, native bridge function, packaged library, frontend-contract change, database change, production behavior change, or user-visible behavior. No Swift or Kotlin production code is removed in this step.
