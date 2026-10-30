# Verification record

Software and scoped physical verification for the beta2 implementation.

- `cargo check --workspace --manifest-path backends/core/Cargo.toml`: passed.
- Full Rust workspace tests passed before the independent review fixes; affected crate suites and new focused regressions were rerun after those fixes. Warnings-denied workspace Clippy and Rust formatting passed after the final code change.
- Focused Rust compatibility, telemetry, configuration codec/service, mutation, CAN discovery, session context, persistence and archive tests: passed.
- `npm --prefix frontend run check`, `npm --prefix frontend test`, `npm --prefix frontend run build`, and `npm --prefix frontend run format:check`: passed (574 frontend tests).
- `npm --prefix conformance run check` and conformance Prettier: passed. Beta2 fixture tests: 97 passed.
- Canonical `npm --prefix conformance run direct` and `npm --prefix conformance run mock`: passed. Beta2 direct scenario, including configuration write/readback and same-version Tune apply: passed with `FLOAT_VIEW_FIXTURE_REFLOAT=1.3.1-beta2 FLOAT_VIEW_CONFORMANCE_SCENARIOS=refloat-beta2`; it passed again after the review fixes.
- Browser tests run outside the macOS sandbox after the frontend build: timeout warning/history, live menu, and Refloat compatibility passed in their configured Chromium/WebKit projects.
- Android `just test android` with JDK 21: passed, including JNI tests. Apple shared server test: passed.
- Full Apple `just test apple`: one unrelated failure in unchanged `MacThemeTests` (`loading artwork repaints in same window when appearance changes`, `symbol.contentTintColor` was nil); the focused theme rerun failed the same way. No Refloat binding failure was reported.
- `git diff --check` and `OPENSPEC_TELEMETRY=0 openspec validate support-refloat-1-3-1-beta2 --strict`: passed.

Independent review found and prompted fixes for stale projected timeout write guidance, a combined live alert history that could exceed 20 records, and an unused sampled event query. Focused regression tests cover the first two fixes.

The beta2 direct and CAN conformance fixtures covered protocol development; canonical mock coverage was generic. The production macOS BLE host passed the [ExpressT physical checkpoint](../../../../conformance/physical-checkpoints/refloat-1-3-1-beta2.md) on the Express/CAN route: exact reported beta2 identity, advancing fast/slow telemetry, 172-field schema and current/default reads, backed-up non-actuating write/readback/restore, unchanged same-version Tune apply/readback/restore, saved stationary samples reopened after disconnect, and reconnect. The first-run original and final restored configuration matched field for field.

A separate beta2 IMU timeout scenario passed through the production macOS BLE host and connected ESP32 direct-route fixture: active warning, rejected guarded writes, clear/transient recovery and four persisted timeout records. The fixture returned to READY. Damaged MOSFETs and unreliable duty/eRPM prevent a loaded riding claim on ExpressT; iOS, Android, other routes and a real-controller IMU failure remain physically untested.

Detailed follow-up review reproduced a dispatch race: a guarded mutation accepted behind an in-flight alert read could still be transmitted after that read established active or unknown timeout state. Dispatch now rechecks timeout authority and cancels rejected operations through the existing completion path, releasing their mutation/configuration state. A 30-case coordinator regression covers package changes, flywheel enable, edited configuration and Tune writes, plus preserved flywheel-disable and lighting paths, across active, malformed, timed-out, expired and healthy evidence. The synced live-control specification includes this queued-write scenario.

After the dispatch fix, the affected mutation/coordinator, backend-service, application-runtime and loopback-server test suites passed; loopback listener tests were rerun outside the sandbox after sandbox bind failures. Affected-crate warnings-denied Clippy, Rust formatting, changed-document Prettier, OpenSpec doctor and the beta2 direct conformance flow (revision 226, outside the sandbox) passed. The updated live-control spec validates; strict repository-wide spec validation still reports existing requirement-length warnings. No additional hardware run was needed for this shared authorization fix.
