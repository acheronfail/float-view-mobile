## Context

Both native application services process decoded Refloat telemetry in the same incorrect order. When `packageMode == flywheel`, they cancel flywheel-prohibited scheduler work and clear the mutation coordinator's pending action before calling `observeTelemetry`. For flywheel enable, the authoritative confirming sample is therefore discarded as confirmation evidence and the blocking request later returns `unavailable`. For flywheel disable, any intermediate sample that still reports flywheel can clear the sole safety-permitted pending action before normal-mode confirmation arrives.

The defect became visible after the conformance fixture correctly accepted the native zero-mask lighting refresh. It is independent of the paused Rust extraction and the fixture query correction.

## Goals / Non-Goals

**Goals:**

- Apply authoritative telemetry to the mutation coordinator before flywheel-mode safety cleanup on Apple and Android.
- Complete flywheel enable exactly once when telemetry first reports flywheel mode.
- Preserve a pending flywheel disable across repeated flywheel samples and complete it only when normal mode is authoritative.
- Cancel/fail only other mutations, configuration work, and shutdown intents that flywheel mode prohibits.
- Ensure late cancellation and timeout callbacks cannot fail an already completed or absent Refloat action.
- Keep both native implementations and tests behaviorally equivalent.

**Non-Goals:**

- Changing command 22 payloads, scheduler priority, confirmation deadlines, or telemetry normalization.
- Allowing any additional mutation while flywheel mode is active.
- Changing loopback/frontend contracts, persistence, BLE adapters, fixture firmware, or Rust ownership.

## Decisions

### Observe first, then enforce mode exclusions

Each application service will snapshot the pending action, call the coordinator's telemetry observer, and complete a newly satisfied waiter before applying flywheel-mode cleanup. This preserves the authoritative source of truth and avoids special-casing an optimistic enable acknowledgement. Keeping the current cleanup-first order and exempting only enable was rejected because the coordinator does not expose the expected boolean and disable would remain vulnerable.

### Preserve the flywheel action, fail other pending controls

After observation, a remaining `.flywheel` action represents an eligible disable awaiting normal mode and MUST survive repeated flywheel telemetry. A remaining package, lighting, or fatal action is prohibited by the newly authoritative mode and will be failed once. Configuration writes/current reads and shutdown intents retain their existing flywheel cleanup.

### Make absent pending failure explicit

Generation-correct `failPending` will return no result when the coordinator has no pending Refloat action. Application timeout/cancellation completion already treats a missing result as a no-op; tightening the coordinator prevents a late callback from publishing a false timeout or completing a replacement waiter after success.

### Test through the application-service boundary

Coordinator-only tests cannot reproduce the faulty ordering. Focused Apple and Android tests will run the blocking control request on a worker, observe exact command transmission, inject flywheel/normal telemetry, and assert returned state, pending-state continuity, and absence of timeout failure. Physical direct/CAN scenarios remain the end-to-end gate.

## Risks / Trade-offs

- **Risk: Flywheel disable remains pending forever.** → The existing bounded mutation timer remains active and still fails the action if normal-mode telemetry never arrives.
- **Risk: Competing mutations survive entry into flywheel.** → Only a remaining `.flywheel` action is retained; all other pending controls are explicitly failed and scheduler/configuration/shutdown cleanup remains intact.
- **Risk: Duplicate completion races.** → `failPending` returns nil without a pending action, and waiter completion continues to be consumed exactly once under the application-service lock.
- **Risk: Cross-platform drift.** → Add mirrored service tests and require both native test suites plus all three physical hosts.

## Migration Plan

1. Add failing Apple and Android application-service tests for enable and disable confirmation ordering.
2. Tighten no-pending failure semantics in both mutation coordinators.
3. Reorder both application-service telemetry paths and retain only eligible pending flywheel disable.
4. Run focused native tests, strict OpenSpec validation, and `just verify`.
5. Run direct/CAN physical controls on Android, iOS, and macOS.
6. Sync/archive this change, then resume the paused lighting-query fixture and Rust extraction changes.

Rollback is limited to the native service/coordinator commit; no stored data or wire migration exists.

## Open Questions

None.

