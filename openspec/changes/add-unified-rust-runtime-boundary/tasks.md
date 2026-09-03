## 1. Baseline and workspace

- [ ] 1.1 Add a reproducible migration-baseline audit that reports tracked non-test production LOC by Rust/Apple/Android owner, workspace members, temporary bridge crates, selected protocol families, and Android/Apple bridge artifacts without counting generated output.
- [ ] 1.2 Capture the pre-implementation baseline and exact future deletion inventory in the core documentation, and make the audit fail if a new sideways candidate or protocol bridge appears.
- [ ] 1.3 Add the safe `application-runtime` crate to the locked workspace with direct dependencies only on existing framing, coordinator, telemetry, and session-SQLite owners.

## 2. Runtime composition

- [ ] 2.1 Implement the closed runtime command, effect, snapshot, diagnostic, and failure model with explicit bounds and payload-redacted diagnostics.
- [ ] 2.2 Own bounded VESC stream lifetime, submit every complete payload to the connection coordinator in order, and frame each scheduled outbound write exactly once.
- [ ] 2.3 Implement deterministic translation of coordinator activation, timers/deadlines, routing hints, configuration cache, state, telemetry, BMS, mutation, failure, and diagnostic effects without callbacks or host references.
- [ ] 2.4 Add explicit session attach/open/detach lifecycle and persist normalized samples before publication, preserving marker ordering and atomic failure behavior.
- [ ] 2.5 Add location persistence and safe direct historical reader/pruner entry points that preserve existing bounds, published-boundary, interruption, preview, and atomic-commit semantics.

## 3. Evidence and integration guardrails

- [ ] 3.1 Add exact composition vectors covering fragmented/noisy/multiple frames, framed writes, activation, timers/deadlines, configuration/cache work, telemetry/BMS publication, reset, and effect ordering.
- [ ] 3.2 Add persistence tests covering attached and intentionally absent sessions, commit-before-publication, location markers, reopen, failure recovery, and no partial publication.
- [ ] 3.3 Add adversarial tests for stale generations, invalid clocks/identifiers/primitives, oversized streams/effects, malformed frames/responses, redaction, and reusable reset state.
- [ ] 3.4 Add root focused test and audit recipes proving the runtime has no unsafe code, host package selection, callback/executor/thread/OS dependency, generated binding, new bridge, copied algorithm, or unbounded collection.
- [ ] 3.5 Update core ownership documentation to mark the runtime as the final unselected composition candidate and identify Apple cutover, Android cutover, and bridge retirement as the only remaining migration stages.

## 4. Verification

- [ ] 4.1 Run formatting, locked focused runtime tests, the reproducible convergence audit, warnings-denied workspace linting, and `git diff --check`.
- [ ] 4.2 Run strict OpenSpec validation for this change and the complete `just verify` software gate.
- [ ] 4.3 Confirm the production-isolation diff changes no selected Apple/Android/frontend/contract/schema implementation, then record the post-change baseline delta for later host proposals.
