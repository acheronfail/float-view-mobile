## Why

VESC framing, firmware identity, custom-configuration envelopes, and BMS values decoding are now Rust-owned on every host, while Refloat live-control wire encoding and lighting-response decoding remain duplicated in Swift and Kotlin. The safest next increment is to prove this small stateless, safety-sensitive codec in safe Rust before adding either host bridge or moving compatibility, routing, authorization, application state, or telemetry.

## What Changes

- Add one safe Rust candidate under `backends/core` with an empty production dependency graph for exact Refloat package, flywheel, legacy/stable lighting, and fatal-clear payload encoding plus lighting-response decoding.
- Preserve the custom-app-data command, Refloat package ID, command and parameter bytes, lighting mask/flag behavior, legacy/stable profile distinction, fatal-clear availability, exact response extent, and fail-closed errors.
- Verify the candidate against the immutable source-derived Refloat product-control fixtures plus query, state-combination, malformed, wrong-profile, unsupported, boundary, and privacy cases.
- Add locked formatting, warnings-denied linting, dependency, workspace-layout, privacy, and production-isolation audits without adding or changing any Apple C ABI or Android JNI symbol.
- Keep Swift and Kotlin as the only production Refloat control codecs. Separate later OpenSpec changes will cut over Apple and Android independently and delete each native codec only after bridge, packaging, canonical, and physical verification.
- Run the complete software gate and focused Refloat control physical regression on the connected Android device, iOS device, and macOS host with the ESP32 fixture. These runs continue to exercise the selected native codecs and do not claim the Rust candidate executed.
- Non-goals: Refloat compatibility catalogue, request routing or framing, mutation authorization, scheduling, telemetry or configuration, application behavior, SQLite, local webserver, Bluetooth, frontend contract, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define bounded parity, ownership, and error requirements for an unselected Rust Refloat live-control codec candidate.
- `platform-hosts`: Locate the focused candidate while preserving Swift and Kotlin production ownership and every native service boundary.
- `backend-conformance`: Require source-derived/adversarial component evidence, complete software verification, and all-host physical Refloat control regression without presenting the candidate as a supported backend path.

## Impact

This affects the locked `backends/core` workspace, one new safe Rust crate, focused tests over `backends/fixtures/refloat/product-controls.json`, core ownership documentation, root workspace/dependency audits, `just verify`, and OpenSpec requirements. It adds no normal or third-party development dependency, generated binding, native bridge function, packaged library, frontend-contract change, database change, production runtime change, or user-visible behavior. No Swift or Kotlin production code is removed in this step.
