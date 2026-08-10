## Why

Apple and Android currently clear a pending flywheel action as soon as telemetry reports flywheel mode, before the mutation coordinator can use that same authoritative telemetry to confirm flywheel enable. The caller consequently times out, and repeated flywheel telemetry can also cancel an eligible pending disable that must remain reachable for safety.

## What Changes

- Make both native application services observe authoritative telemetry before applying flywheel-mode cancellation and configuration invalidation.
- Complete a confirmed flywheel-enable waiter successfully, preserve a pending flywheel-disable waiter while mode remains flywheel, and fail only prohibited competing mutations.
- Make generation-scoped pending failure a no-op when no Refloat action remains, preventing late cancellation or timeout callbacks from overturning a completed result.
- Add equivalent Apple and Android application-service tests for enable confirmation, repeated-flywheel telemetry during disable, normal-mode disable confirmation, and stale/no-pending failure callbacks.
- Run `just verify` plus direct/CAN physical Refloat control scenarios on Android, iOS, and macOS before resuming the paused fixture and Rust changes.
- Do not change control command bytes, the frontend contract, persistence, BLE ownership, Rust ownership, configuration eligibility, or flywheel mutation exclusions.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-live-controls`: Define authoritative flywheel enable/disable confirmation ordering and preserve the sole eligible disable across repeated flywheel telemetry.
- `backend-conformance`: Require equivalent native regression evidence and all-host direct/CAN physical control completion for the ordering defect.
- `platform-hosts`: Keep Apple and Android application-service mutation lifecycle behavior equivalent without changing their platform adapters.

## Impact

The change affects the hand-written Apple and Android application services, their shared-shape mutation coordinators, and focused native tests. It changes no wire/API or durable data shape, adds no dependency, and removes the premature pending-action cancellation behavior.
