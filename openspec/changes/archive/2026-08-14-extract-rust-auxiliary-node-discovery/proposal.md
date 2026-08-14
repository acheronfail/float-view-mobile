## Why

Apple and Android still duplicate the complete pure auxiliary VBMS discovery state machine: exact BMS identity admission, controller-inventory reuse, preferred-route probing, bounded CAN enumeration, ambiguity, availability, and generation isolation. The safe Rust controller-discovery and firmware-identity components now provide the exact adjacent facts needed to extract this application-behavior slice without adding another temporary host bridge.

## What Changes

- Add an unselected, unsafe-free `float-view-auxiliary-node-discovery` Rust candidate that directly consumes routed identities and complete inventory from `float-view-controller-discovery`.
- Reproduce the complete selected Swift/Kotlin auxiliary decision policy as typed bounded transitions: exact stock-VBMS/VBMS32 compatibility, direct and preferred routes, complete-inventory reuse, fallback CAN ping and sequential probes, timeout handling, unique/ambiguous/not-found selection, liveness availability, snapshot-presence retention, reset, and stale-generation isolation.
- Add shared source-derived vectors plus exhaustive-boundary, adversarial-lifecycle, dependency, isolation, privacy, software, and all-host physical evidence.
- Keep Swift and Kotlin auxiliary discovery selected and unchanged until a later unified-runtime cutover proves production Rust execution and deletes both native implementations.
- Add no Apple FFI, Android JNI, callback, generated binding, bridge catalogue entry, package input, persisted format, or frontend-contract change.
- Preserve all current BLE, scheduling, BMS payload decoding, polling cadence, observation time, application publication, controller readiness, session, SQLite, loopback, OS-capability, and UI behavior.

Non-goals: host cutover; BMS request or frame construction; firmware or command-96 decoding; BLE, clocks, timers, request scheduling, polling cadence, observation payload ownership, persistence, frontend state serialization, loopback serving; or deletion of native auxiliary discovery and existing protocol bridges.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Define complete parity and the direct-composition boundary for the safe Rust auxiliary-node discovery candidate.
- `platform-hosts`: Locate the unselected candidate while retaining native production and OS-capability ownership.
- `backend-conformance`: Require shared, adversarial, isolation, software, and all-host physical evidence before later unified-runtime composition.

## Impact

Adds one focused crate under `backends/core`, locked workspace metadata, shared test vectors, focused root verification/audit recipes, documentation, and OpenSpec evidence. Native application sources, host package graphs, the existing Apple/Android bridge families, wire contracts, persistent data, and production behavior remain unchanged.
