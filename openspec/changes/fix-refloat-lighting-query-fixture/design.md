## Context

Both production backends periodically request current Refloat lighting state. Refloat 1.0.x/1.1.x command 202 uses a one-byte mask and Refloat 1.2.x command 20 uses a four-byte mask; upstream source treats a zero mask as a read-only query, ignores the supplied flags, and returns current lighting state. The shared conformance decoder currently maps mask zero to an LED-off control. Its simulator then applies the flywheel mutation exclusion before responding, so an auxiliary status refresh can fail an otherwise valid physical control sequence.

The same deterministic request transcript reproduced on Android and macOS. The ESP32 remains a byte bridge, and neither native backend is implicated: both emit the source-defined query shape.

## Goals / Non-Goals

**Goals:**

- Represent zero-mask lighting status queries distinctly from nonzero-mask mutations.
- Return current simulated LED and headlight state for either supported query shape without changing model state.
- Preserve strict route, firmware-access, length, command-family, and value bounds.
- Preserve flywheel rejection for every actual mutation while permitting scheduled read-only lighting refresh.
- Prove the correction in deterministic tests and the direct/CAN physical control checkpoints on all native hosts.

**Non-Goals:**

- Changing Swift or Kotlin codecs, schedulers, mutation coordinators, or application behavior.
- Changing the frontend contract, persistence, BLE firmware, or the paused Rust migration.
- Relaxing any nonzero-mask mutation, test-version, wrong-route, malformed, or unsupported-command rejection.

## Decisions

### Decode query intent explicitly

The conformance decoder will represent mask-zero command 202/20 as a lighting query rather than fabricating an LED control with `enabled = false`. This keeps the safety model semantic: mutation gates apply only to decoded mutations. Checking `lightingMask === 0` only inside the state model was rejected because callers and tests would continue to observe a misleading decoded operation.

### Preserve the current response path

The state model will answer a lighting query with the exact existing four-byte response derived from current disposable state. It will not call either lighting setter, change flags, or bypass route/test-version/shape validation. A parallel simulator-only query implementation was rejected because it would duplicate decoding rules between direct and CAN paths.

### Cover the race, not timing

Software tests will issue legacy and stable zero-mask queries while normal and flywheel state are active, assert state immutability, and exercise direct and CAN routing. The physical gate will rerun `control-direct` and `control-can` on Android, iOS, and macOS. No sleep or retry allowance will hide the original auxiliary-refresh race.

## Risks / Trade-offs

- **Risk: Treating malformed masks as queries.** → Query classification occurs only after the existing exact payload-length, command-family, route, firmware, mask, and flag validation.
- **Risk: Weakening flywheel safety.** → Only mask zero is read-only; every mask containing LED or headlight mutation bits follows the unchanged rejection path.
- **Risk: A native issue remains after the simulator correction.** → Require both direct and CAN physical scenarios on every host before archiving; any remaining failure stays visible.

## Migration Plan

1. Add failing unit coverage for zero-mask query semantics and the flywheel race.
2. Correct the shared decoder/model and run focused conformance tests.
3. Run `just verify`.
4. Run direct/CAN physical control scenarios on Android, iOS, and macOS against the ESP32 fixture.
5. Sync and archive this change, then resume the paused Rust extraction and repeat its original physical gate.

Rollback is a single fixture-model commit; no production data or API migration is involved.

## Open Questions

None. Vendored Refloat v1.0.0 and v1.2.3 source both define zero-mask lighting requests as non-mutating status queries.
