## 1. Candidate Boundary

- [x] 1.1 Add the safe dependency-free `vesc-request-scheduler` workspace crate and document its unselected ownership boundary.
- [x] 1.2 Define bounded route, context, request, generation, deadline, completion, diagnostic, snapshot, command, and ordered-effect types with validated public construction.

## 2. Scheduler State Machine

- [x] 2.1 Implement generation replacement, bounded enqueue, non-preemptive priority/FIFO selection, one response-bearing in-flight request, and immediate one-way completion.
- [x] 2.2 Implement exact response/context attribution, explicit deadline expiry, cancellation, deferred pumping, resume, stale-event rejection, and source-equivalent counter rollover.
- [x] 2.3 Preserve atomic validation and deterministic effect ordering without clocks, timers, writers, callbacks, host references, protocol codecs, persistence, transport, or OS dependencies.

## 3. Parity and Isolation Evidence

- [x] 3.1 Add source-derived vectors covering the complete Swift/Kotlin scheduler behavior and every request priority/context family.
- [x] 3.2 Add adversarial tests for queue/payload/value bounds, malformed facts, stale and repeated responses/deadlines, cancellation/defer sequences, one-way chains, and sequence/generation rollover.
- [x] 3.3 Update workspace locks, core ownership documentation, and repository audits to require a safe unselected candidate and forbid host selection, bridge crates/symbols, generated bindings, callbacks, or packaged artifacts.

## 4. Verification

- [x] 4.1 Run focused Rust tests, formatting, linting, dependency/ownership audits, and strict change validation.
- [x] 4.2 Run `just verify` with the selected native schedulers unchanged.
- [x] 4.3 Run the canonical ESP32 physical matrix on the connected Android device, connected iOS device, and macOS host; record all non-location checkpoints and defer only bounded location acquisition per the agreed manual follow-up.
