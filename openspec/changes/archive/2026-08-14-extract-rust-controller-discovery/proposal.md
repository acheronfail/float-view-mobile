## Why

Apple and Android still duplicate the complete pure controller-discovery state machine: direct-versus-CAN routing, configured and learned route recovery, bounded enumeration, continuity, candidate selection, and Refloat-readiness decisions. The newly extracted safe Rust firmware identity and compatibility components now make this the next coherent application-behavior slice that can move to Rust without adding another temporary host bridge.

## What Changes

- Add an unselected, unsafe-free `float-view-controller-discovery` Rust candidate that directly composes safe firmware identity, VESC compatibility, and Refloat compatibility models.
- Reproduce the complete selected Swift/Kotlin discovery policy as typed state transitions and bounded host effects: direct admission, configured/hinted CAN probing, normalized enumeration, continuity recovery, candidate publication/selection, timeout/reset generations, Refloat profile and layout readiness, and stable failures.
- Add source-derived shared-vector, exhaustive-boundary, adversarial-lifecycle, dependency, isolation, and diagnostic-privacy evidence.
- Keep Swift and Kotlin discovery selected and unchanged until a later unified-runtime cutover proves production Rust execution and deletes both native implementations.
- Add no Apple FFI, Android JNI, callback, generated binding, package input, persisted format, frontend contract, or bridge-catalogue entry.
- Preserve all current OS-owned Bluetooth, background execution, location, lifecycle, permissions, native files, theme, WebView, SQLite, loopback, and UI behavior.

Non-goals: host cutover; BLE, clock, timer, scheduler, persistence, Board Profile JSON, routing-hint storage, request-byte construction, protocol decoding, auxiliary BMS discovery, telemetry polling, mutation execution, loopback serving, or deletion of existing native discovery code or protocol bridges.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define complete parity and the direct-composition boundary for the safe Rust controller-discovery candidate.
- `platform-hosts`: Locate the unselected candidate while retaining native production and OS-capability ownership.
- `backend-conformance`: Require source-derived, adversarial, isolation, software, and all-host physical evidence before later runtime composition.

## Impact

Adds one focused crate under `backends/core`, locked workspace metadata, focused root verification/audit recipes, documentation, and OpenSpec evidence. Native application sources, host package graphs, the ten Apple/Android bridge families, wire contracts, persistent data, and production behavior remain unchanged.
