## Why

Apple and Android still duplicate the same Refloat INFO identity decoder, exact supported-version catalogue, telemetry/control profile selection, and required dynamic-layout admission policy. With both telemetry wire decoders already selected from Rust, the safest next increment is to prove this remaining pure compatibility decision boundary in one isolated safe component before changing either production host or adding another temporary bridge.

## What Changes

- Add one dependency-free, unsafe-free, unselected Rust candidate under `backends/core` for the exact Refloat INFO request and response identity, supported package catalogue, telemetry/control profile selection, and required dynamic-layout admission policy.
- Preserve legacy compact wire identity `1.0` without inventing patch `.0`; exact audited 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3 catalogue entries; legacy, dynamic, and dynamic-alert profiles; lighting/fatal-clear capabilities; and required always-present/runtime field sets.
- Verify source-derived INFO response forms, exact package/profile decisions, unknown and prerelease rejection, complete layout admission, duplicates/extras/order tolerance, bounds, malformed input, deterministic output, and payload-safe failures.
- Extend locked workspace, dependency, unsafe-code, ownership, production-isolation, bridge-symbol, and packaged-artifact audits without adding an Apple C ABI, Android JNI symbol, generated binding, packaged native library, or selected host call path.
- Keep Swift and Kotlin as production owners of Refloat INFO decoding and compatibility admission. Separate later OpenSpec changes will cut over Apple and Android independently and delete each superseded implementation after complete bridge, packaging, canonical, and physical verification.
- Run `just verify` plus physical identity, direct/CAN, telemetry, controls, ride, unexpected-disconnect, and background/session regression on the connected Android device, iOS device, and macOS host with the ESP32 fixture. Only bounded location-sample acquisition may remain deferred manual coverage.
- Non-goals: host bridges or cutover, native-code deletion, connection state-machine ownership, compatibility-driven routing or scheduling, telemetry decoding, VESC framing, application state, persistence, SQLite, local webserver, Bluetooth, frontend contract, or any OS-owned capability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define parity, bounded input/output, isolation, and retained native ownership for an unselected safe Rust Refloat compatibility candidate.
- `platform-hosts`: Locate the focused candidate while preserving native production admission, application coordination, and OS capability ownership.
- `backend-conformance`: Require source-derived and adversarial candidate evidence, complete software verification, and all-host physical regression without claiming candidate execution.

## Impact

This affects the locked `backends/core` workspace, one new dependency-free safe Rust crate, focused tests over pinned Refloat source and committed compatibility fixtures, core ownership documentation, root workspace/dependency audits, `just verify`, and OpenSpec requirements. It adds no runtime dependency, native bridge function, generated binding, packaged library, frontend-contract or persistence change, selected production behavior, or user-visible behavior. No Swift or Kotlin production code is removed in this step.
