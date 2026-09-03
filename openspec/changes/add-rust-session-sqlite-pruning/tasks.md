## 1. Pruning contract and immutable evidence

- [ ] 1.1 Add literal prune input/expected fixtures for empty, dense, protected evidence, phase transitions, extrema ties, nearby/orphaned locations, initial, repeated-coarser, and no-op sessions.
- [ ] 1.2 Extend the session SQLite audit recipe to require direct history-kernel composition and reject pruning bridges, generated bindings, copied native logic, host selection, schema drift, arbitrary public SQL, and production source changes.

## 2. Safe plan and lifecycle

- [ ] 2.1 Add stable payload-free pruning errors and bounded owned preview/plan/result values with redacted diagnostics and private database identity/boundary state.
- [ ] 2.2 Add validated session-pruner open/close/drop behavior and a cloneable SQLite interrupter without filesystem discovery, callbacks, threads, async runtime, or host policy.
- [ ] 2.3 Validate target resolution, current retained-resolution alignment, schema/marker/data/count bounds, and source-row ceiling before publishing a plan.

## 3. Streaming preview selection

- [ ] 3.1 Read numeric-field, event, and marker-summary source sequences inside one deferred snapshot and protect every referenced source row.
- [ ] 3.2 Stream canonical samples ordered by elapsed/sequence directly through `session_history_kernel::PruneSelector`, collecting only sorted retained sequences and exact preview summary/boundary facts.
- [ ] 3.3 Prove exact preview parity, repeated-coarser source-only behavior, no mutation, bounded live/source state, cancellation cleanup, and stable failures with fixture and adversarial tests.

## 4. Atomic prune commit

- [ ] 4.1 Begin one immediate transaction, revalidate plan identity/marker/resolution, and reject stale, foreign, reused, or invalid plans before mutation.
- [ ] 4.2 Persist a temporary bound keep set, delete only unretained samples and out-of-neighborhood locations, and atomically update prune metadata, stored counts, commit time, and one history revision while preserving recorded counts and published maxima.
- [ ] 4.3 Preserve no-op databases exactly, drop temporary state on every exit, omit checkpoint/vacuum policy, and prove commit/reopen results plus foreign-key evidence preservation.
- [ ] 4.4 Add active-WAL, busy/locked, interruption, every injected failure point, stale writer/pruner race, rollback, and post-failure reuse tests.

## 5. Repository verification

- [ ] 5.1 Run locked offline session SQLite tests/audits, existing reader/writer/history parity, schema/dependency/ownership audits, formatting, and warnings-denied workspace linting.
- [ ] 5.2 Run strict OpenSpec validation, `git diff --check`, and the complete `just verify` software gate.

## 6. Physical regression and completion

- [ ] 6.1 Run the canonical ESP32 fixture matrix on the macOS host and record every non-location checkpoint.
- [ ] 6.2 Run the canonical ESP32 fixture matrix on the connected Android device, including background BLE/session persistence; defer only bounded location acquisition.
- [ ] 6.3 Run the canonical ESP32 fixture matrix on the connected iOS device, including background BLE/session persistence; defer only bounded location acquisition.
- [ ] 6.4 Confirm the implementation commit contains no production Apple/Android/frontend/SQL source or package input, record exact verification evidence, and mark the change ready for canonical spec sync and archive.
