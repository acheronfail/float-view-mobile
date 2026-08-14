## 1. Candidate Model and Bounds

- [ ] 1.1 Add `float-view-controller-discovery` to the locked Rust workspace with unsafe code forbidden and only the safe firmware-identity, VESC-compatibility, and Refloat-compatibility production dependencies.
- [ ] 1.2 Define bounded routes, generations, routing hints, continuity facts, candidates, classified inventory, snapshots, stable issues, and ordered host effects with private identity diagnostics redacted.
- [ ] 1.3 Implement finite CAN-ID normalization, exact 64-device/16-controller limits, meaningful UUID handling, display-safe identity projection, and total invalid-input outcomes.

## 2. Discovery State Machine

- [ ] 2.1 Implement begin, direct transport admission, preferred/hinted CAN probing, learned-hint clearing, sequential enumeration, probe timeout, and stale-generation isolation.
- [ ] 2.2 Implement complete inventory publication, fresh and active-session continuity, sole-controller selection, ambiguous candidate publication, explicit selection, exact compatibility admission, and stable failure decisions.
- [ ] 2.3 Implement Refloat profile/layout readiness, issue handling, reset, repeated-event behavior, and explicit wrapping generation invalidation without executing host work.

## 3. Parity and Adversarial Evidence

- [ ] 3.1 Replay every committed shared controller-discovery TSV vector and assert exact route, effect, candidates, inventory, generations, readiness, and issue parity.
- [ ] 3.2 Cover direct/configured/hinted/enumerated/continuity/selection/Refloat paths across supported, test, unsupported, non-motor, sentinel-UUID, missing-response, and mixed-node inputs.
- [ ] 3.3 Exhaust CAN normalization and 64/65 plus 16/17 boundaries; permute stale responses/timeouts, resets, invalid selections, counter rollover, repeated transitions, and private diagnostic values.

## 4. Isolation and Repository Integration

- [ ] 4.1 Add focused root test and audit recipes and wire the candidate into workspace verification and the root audit allowlist.
- [ ] 4.2 Audit the exact safe dependency graph, absence of FFI/JNI/callback/runtime/storage/transport code, unchanged ten-entry bridge catalogues, absence from Apple/Android package inputs, and retained native discovery sources.
- [ ] 4.3 Document the candidate's direct Rust composition boundary, unchanged native owners, eventual bridge/native deletion path, and recommended next unified-runtime slice.

## 5. Verification

- [ ] 5.1 Run focused tests/audits, formatting, warnings-denied locked workspace linting, strict OpenSpec validation, and `git diff --check`.
- [ ] 5.2 Run `just verify` with both native controller-discovery implementations still selected.
- [ ] 5.3 Run the canonical ESP32 physical matrix on the connected Android device, connected iOS device, and macOS host; require every non-location product checkpoint and defer only bounded location acquisition per the agreed manual follow-up.
