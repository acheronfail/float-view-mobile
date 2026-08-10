## Why

The conformance Refloat control model misclassifies the source-defined zero-mask lighting status query as an LED mutation. When a native backend's bounded lighting refresh races with active flywheel mode, the fixture therefore rejects a read that upstream Refloat accepts, causing the direct physical control checkpoint to fail identically on Android and macOS.

## What Changes

- Model legacy command 202 and stable command 20 with a zero lighting mask as read-only status queries that return current LED and headlight state without mutating simulated state.
- Keep every nonzero lighting mask subject to the existing mutation and flywheel-mode exclusion rules.
- Add source-derived software coverage for zero-mask queries across legacy/stable releases, direct/CAN routes, and normal/flywheel modes.
- Re-run the focused direct and CAN Refloat physical control checkpoints on Android, iOS, and macOS before resuming the paused Rust custom-configuration extraction.
- Do not change native Swift/Kotlin request encoding, production application behavior, the frontend contract, persistence, Rust ownership, or the ESP32 byte bridge.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-live-controls`: Distinguish source-defined read-only lighting status queries from lighting mutations in every supported wire family and package mode.
- `backend-conformance`: Require the canonical simulator and physical checkpoint to accept zero-mask lighting refreshes without weakening mutation rejection.
- `interactive-fixture-development`: Preserve read-only lighting refresh availability while simulated flywheel mode continues to reject nonzero-mask lighting mutations.

## Impact

The change is limited to the conformance-owned Refloat control model, its deterministic tests and fixtures, and the corresponding specifications. It adds no dependency or compatibility layer and removes the incorrect query-as-mutation branch from the simulator's observable behavior.
