## 1. Candidate Model and Boundary

- [x] 1.1 Add the safe `refloat-telemetry-service` workspace crate with only the three named Rust component dependencies and document its unselected direct-composition boundary.
- [x] 1.2 Define bounded service generation, configuration, timer token, decoded input, normalized sample, snapshot, command, failure, diagnostic, and ordered-effect types with validated public construction.
- [x] 1.3 Implement exhaustive semantic conversion from legacy/dynamic decoded models without bridge records, payload logging, lossy enum fallbacks, or unbounded extensions.

## 2. Telemetry State Machine

- [x] 2.1 Implement exact direct/CAN layout, legacy, realtime, and compact request facts plus immediate layout attempt, bounded retry, establishment, timeout, stop, and reset transitions.
- [x] 2.2 Implement one pending semantic telemetry request, compact due selection, exact response/completion attribution, timeout recovery, adaptive ride/idle/post-ride cadence, and stale-event rejection.
- [x] 2.3 Implement compact-cache replacement/merge, active-fault handling, complete normalized publication, source-equivalent sequence/elapsed time, settings updates, atomic validation, checked time arithmetic, and deterministic effect ordering.

## 3. Parity and Isolation Evidence

- [x] 3.1 Add source-derived tests covering every duplicated Swift/Kotlin transition, exact request bytes/facts, semantic enum and field, legacy/dynamic family, direct/CAN route, cadence boundary, cache behavior, and ordered effect.
- [x] 3.2 Add adversarial tests for invalid settings/time arithmetic, maximum values/extensions, stale/repeated/mismatched input, timeout/stop/reset races, active faults, decreasing totals, replacement caches, and counter/token rollover.
- [x] 3.3 Update workspace locks, core ownership documentation, and repository audits to require the safe unselected three-dependency candidate and forbid host selection, bridge crates/symbols, generated bindings, callbacks, executors, sockets, persistence, or packaged artifacts.

## 4. Verification

- [x] 4.1 Run focused Rust tests, formatting, warnings-denied linting, dependency/ownership audits, and strict change validation.
- [x] 4.2 Run `just verify` with the selected native telemetry services unchanged.
- [x] 4.3 Run the canonical ESP32 physical matrix on the connected Android device, connected iOS device, and macOS host; require every non-location checkpoint and defer only bounded location acquisition per the agreed manual follow-up.
