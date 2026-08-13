## Why

Apple and Android still duplicate the same negotiated Refloat command-32 layout and command-31 realtime decoder even though their bounds, field order, numeric encoding, state bits, and alert tails are defined by pinned upstream source and immutable fixtures. The safest next increment is to prove that complete stateful protocol boundary in safe Rust before adding another temporary host bridge or changing either production path.

## What Changes

- Add one safe, unselected Rust candidate under `backends/core` for Refloat command-32 field-layout negotiation and command-31 dynamic realtime decoding.
- Preserve exact custom-app-data, package, and command envelopes; bounded UTF-8 field lists; negotiated always-present and running-only order; package state, mode, footpad, stop, adjustment, alert, charging, active-alert, fatal-latch, firmware-fault, and extension semantics; and source-defined finite float16 values across their complete range.
- Verify every committed 1.1.x and 1.2.x source fixture plus malformed envelopes, layouts, masks, extents, identifiers, values, flags, tails, duplicate fields, unknown bounded extensions, deterministic resets, and payload-safe failures.
- Extend the locked workspace, dependency, unsafe-code, layout, privacy, and production-isolation audits without adding an Apple C ABI, Android JNI symbol, packaged native library, generated binding, or selected host call path.
- Keep Swift and Kotlin as the only production dynamic telemetry decoders. Separate later OpenSpec changes will cut over Apple and Android independently and delete each native parser only after bridge, packaging, canonical, and physical verification.
- Run `just verify` and focused physical identity, direct/CAN, telemetry, ride, unexpected-disconnect, and background/session regression on the connected Android device, iOS device, and macOS host with the ESP32 fixture. Only bounded location-sample acquisition may remain deferred manual coverage.
- Non-goals: host bridges or cutover, native parser deletion, legacy command-10 decoding, compatibility selection, polling, compact-cache merging, application state, persistence, SQLite, local webserver, Bluetooth, framing, routing, frontend contract, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define parity, bounded-state, isolation, and ownership requirements for an unselected safe Rust Refloat dynamic telemetry candidate.
- `platform-hosts`: Locate the candidate while preserving Swift and Kotlin production ownership and every native service boundary.
- `backend-conformance`: Require upstream-derived and adversarial component evidence, the complete software gate, and all-host physical regression without claiming candidate execution.

## Impact

This affects the locked `backends/core` workspace, one new dependency-free safe Rust crate, focused tests over committed Refloat fixtures and pinned upstream source, core ownership documentation, root workspace/dependency audits, `just verify`, and OpenSpec requirements. It adds no runtime dependency, generated binding, native bridge function, packaged library, frontend-contract change, database change, selected production behavior, or user-visible behavior. No Swift or Kotlin production code is removed in this step.
