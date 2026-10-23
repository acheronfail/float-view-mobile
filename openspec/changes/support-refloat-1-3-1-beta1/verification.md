# Verification — 2026-10-23

Implementation is in the working tree based on `fae15048`. Refloat source is
pinned to `93f0f1f6026531302e75c7f900261b55a6b84b3a` (`v1.3.1-beta1`).
No application data reset, schema-layout change, storage-marker change, archive
format change, Tune conversion, commit or archive was performed.

Passed:

- Focused `cargo test --locked --offline` runs for compatibility, dynamic
  telemetry, configuration codec/service, telemetry service, mutation coordinator,
  session SQLite, product store, connection coordinator, application runtime,
  backend service and loopback server. This includes exact suffix rejection,
  source vectors, mixed timeout/fault recovery and deduplication, mutation gates,
  transactional rollback, timeout-only ride retention, supported baseline catalog
  and archive fixtures, and archive round trips containing timeout records.
- Workspace/all-target warnings-denied Clippy and Rust formatting checks.
- Configuration fixture regeneration is byte-for-byte reproducible, with all
  preexisting schema/value files unchanged. Beta schema has 172 fields,
  fingerprint `25e21bbfbe89af1dfb9dd47b21e774d68d8d4a51d635526533d8683a5a55493d`
  and signature `0x191a6c1b`.
- `npm --prefix conformance run check` and `npm --prefix conformance test`:
  TypeScript/fixture UI checks clean; all 218 harness/fixture tests pass.
- Canonical `npm --prefix conformance run mock` and `run direct`: passed,
  including older firmware and public API regression paths.
- Exact beta scenario with `FLOAT_VIEW_FIXTURE_REFLOAT=1.3.1-beta1` and
  `FLOAT_VIEW_CONFORMANCE_SCENARIOS=refloat-beta1` against `run direct` and
  `run macos`: passed. These exercise exact admission, schema loading, saved and
  guest connections, and recovered timeout activation/clear without firmware
  fault pollution. Direct/CAN telemetry, configuration and verified readback also
  pass source-derived simulator tests.
- Six affected frontend test files: all 82 tests pass. Frontend Svelte checks,
  build, changed-file ESLint and Prettier checks pass.
- Playwright `fault-warning.browser.ts` and
  `refloat-compatibility.browser.ts`, serialized outside the macOS sandbox after
  building: all three tests pass, including Chromium/WebKit compatibility UI,
  active timeout alongside firmware fault, reload, clearing and exact beta
  supported-version guidance.
- `just test apple`: 35 Swift tests pass after rebuilding the Rust runtime.
- `just android-rust-codec-test` using installed OpenJDK 21: Gradle/JNI test
  passes. No Android emulator was connected for beta software-host conformance.
- `git diff --check` and strict telemetry-disabled OpenSpec validation pass.

Loopback socket tests required an unsandboxed rerun; the rerun passed. Initial
beta conformance exposed a numeric-only saved-session provenance validator;
Rust and frontend provenance validators now admit the exact beta with regression
coverage. The fixture matrix was extended to treat the beta as selectable
telemetry, retaining all older profiles.

Physical checkpoint passed on 2026-10-23 with actual Thor300/VESC 6.06 running
exact `1.3.1-beta1`, through CAN over production macOS BLE. The same working-tree
build admitted the package, advanced fast/slow telemetry, loaded all 172 current
and default configuration fields, verified a non-actuating startup LED change,
restored and freshly verified the complete original values, exercised/restored
runtime lighting, confirmed idempotent package control, read durable samples and
reconnected with the same audited identity/route and restored configuration.
Firmware cleared `meta.is_default` on write as documented; every other restored
value matches the original. The device was disconnected and isolated host/storage
cleaned up. The initial attempt stopped before any write because an exact-zero
eRPM test rejected stationary sub-eRPM measurement noise; the harness then used
an absolute eRPM limit below one and the complete run passed.

This establishes real firmware/BLE interoperability on the available macOS CAN
path. Direct-controller BLE and mobile hosts were not physically exercised;
actual IMU failure was not induced. Timeout activation/clear and mixed fault
history retain their source-derived software evidence. No motor, flywheel,
remote, Handtest, firmware-write or shutdown command was requested. See
[the checkpoint](../../../conformance/physical-checkpoints/refloat-1-3-1-beta1.md).
Detailed device/route records, complete backup/restored snapshots and raw results
are retained in ignored `.local/physical-testing/refloat-131-beta1-20261023/`.
No unrelated physical matrix or `just verify` run is required by the impact-based
verification policy.

## Review follow-up

The mutation coordinator now preserves unknown timeout state and requires a
confirmed inactive state for guarded beta writes. Regression cases cover unknown,
active and inactive states on direct/CAN routes; older profile eligibility and
flywheel escape retain their existing coverage. Sampled timeout activation and
clear events now appear in review across page boundaries, initial healthy clears
are excluded, and recovered events take precedence over duplicate sampled/stop
observations. Sampled timeout-only rides survive distance filtering and restart.

The shared snapshot now carries a separate bounded `imuTimeoutHistory` collection,
including guest history after clearance and frontend recreation. A retained IMU
stop establishes history with unavailable occurrence time, without establishing
active timeout state; subsequently recovered records replace that fallback.
Healthy beta snapshots no longer make Fault history permanently visible. The menu
also respects an independently reported fatal latch. The mock's preexisting extra
live `estimatedBatteryLevel` field was removed after exact contract conformance
caught it; the recorded-session estimate field remains supported.

Follow-up checks passed: affected Rust crates and consumers (including retention
and baseline/archive cases), loopback tests, workspace/all-target warnings-denied
Clippy, 86 frontend tests in six affected files, clean Svelte/TypeScript checks,
frontend build, all 218 conformance tests, exact beta direct conformance (saved and
guest snapshot history assertions), three Chromium/WebKit browser tests including
healthy menu hiding and cleared guest history after reload, 35 Apple tests, and
Android Rust/JNI integration tests. Socket/browser and native-cache checks were
rerun outside the sandbox where required. Formatting, changed-file lint, diff
whitespace and strict OpenSpec validation pass.

No additional physical IMU failure was induced. The earlier scoped physical
interoperability checkpoint remains the hardware evidence; these corrections
were verified through software fault/state injection. Beta1 retirement in favor
of audited stable 1.3.1 is documented as a future replacement, preserving
supported stored data and keeping Tune conversion out of scope.

## Second review regression fixes — 2026-10-23

Fixed independently expiring inactive IMU-timeout authority. Successful command-35
reads record a monotonic observation time; inactive authority expires after the
larger of idle/history cadence plus two request timeout budgets. Failed pending
alert reads invalidate it immediately, while active warnings remain until a valid
clear. Fast samples project unavailable inactive state as null without generating
a fictional clear event. Package/flywheel-enable and configuration/Tune submissions
query current alert authority even before another sample publishes. Older profiles
and flywheel disable retain their existing eligibility.

Recorded retained-stop fallback rows now keep occurrence time null and preserve
observation time separately. Timeout deduplication matches ordered transitions
one-to-one within state-observation windows, instead of applying the firmware
fault list's two-second proximity tolerance. Recovered records remain intact.
Coverage includes delayed activation/clear across pages, undated recovery,
separate sampled episodes, retained-stop uncertainty, and an older recovered
episode that must not hide a later sampled activation.

Passed in this follow-up:

- `cargo test --locked --offline` for `float-view-refloat-telemetry-service`,
  `float-view-connection-application-coordinator`,
  `float-view-device-mutation-coordinator`, `float-view-application-runtime`
  and `float-view-backend-service`. Four new Rust regressions cover cadence
  boundaries on direct/CAN routes, failed-read invalidation, active-warning
  retention, recovery, reset, stale-generation failures, immediate guarded-write
  rejection and fresh inactive write eligibility.
- Warnings-denied all-target Clippy for those five crates and workspace
  `cargo fmt --all -- --check`.
- Four affected frontend files: 57 tests pass, including six new recorded-history
  regressions. Changed-source ESLint and Prettier pass; `npm run check` reports
  zero errors/warnings, and `npm run build` passes (Vite retains its bundle-size
  advisory).
- Exact beta canonical direct conformance with
  `FLOAT_VIEW_FIXTURE_REFLOAT=1.3.1-beta1` and
  `FLOAT_VIEW_CONFORMANCE_SCENARIOS=refloat-beta1`: passed (revision 189),
  outside the sandbox for loopback sockets.
- `fault-warning.browser.ts` Chromium smoke coverage passes outside the macOS
  sandbox after building. A repeat exposed a reload/menu synchronization race;
  the test now waits for menu closure and restored live state before tapping,
  and the final-build rerun passes.
- `git diff --check` and strict telemetry-disabled OpenSpec validation pass.

No new native binding/API shape or storage-layout change was needed. These are
shared software policy and history-presentation fixes; native packaging and
physical firmware checkpoints were not repeated. No commits, spec sync or archive
were performed.

## Production-code review follow-up — 2026-10-24

Recorded history now keeps retained-stop evidence separate from sampled active
timeout state. A first healthy alert observation after a retained stop leaves the
stop undated and does not invent a clearance. Three regression cases failed
before the fix and pass afterward: single-page history, cross-page history, and
a later actual activation/clear episode following the healthy baseline.

History loading now separates pagination, event collection, timeout observation
and final reconciliation, with named state/observation types and a single shared
history-limit check. The connection coordinator resolves `imu_timeout_available`
from the compatibility catalogue for both command-35 decoding and mutation guards
instead of duplicating checks against the beta suffix. No contract or storage
format changed.

Passed for this follow-up:

- `npm test -- src/lib/features/faults/session-faults.spec.ts src/lib/features/connection/live.spec.ts src/lib/features/connection/live-status.spec.ts src/lib/backend/session-data.spec.ts` from `frontend`: 60 tests.
- `npm run check`: zero errors/warnings; changed-source ESLint and Prettier pass.
- `npm run build`: passed, retaining the existing Vite bundle-size advisory.
- `cargo test --locked --offline -p float-view-connection-application-coordinator -p float-view-backend-service` from `backends/core`: 146 unit/integration tests pass; doc tests pass.
- Warnings-denied all-target Clippy for those two crates; workspace `cargo fmt --all -- --check`.
- `FLOAT_VIEW_FIXTURE_REFLOAT=1.3.1-beta1 FLOAT_VIEW_CONFORMANCE_SCENARIOS=refloat-beta1 npm run direct` from `conformance`: passed, revision 190, outside the sandbox for loopback sockets.
- `node --strip-types --test test/fault-warning.browser.ts` from `conformance`, outside the sandbox after the build: one Chromium smoke test passed.
- Strict telemetry-disabled OpenSpec validation and `git diff --check`.

Native packaging and physical checks were not repeated for this software-only
fix/refactor. No commits, spec sync or archive were performed.

## Generic suffix identity — 2026-10-24

`SourceVersion` and `WireVersion` now carry a validated `VersionSuffix` with the
wire's fixed 20-byte zero-padded layout. Empty means stable; bounded suffixes
remain part of equality, display and exact catalogue lookup. Removed the
beta-specific boolean/constructor and the duplicate `InfoIdentity` suffix string.
The catalogue explicitly declares IMU-timeout support on every entry. Existing
stable releases and exact beta admission remain unchanged.

The internal canonical compatibility record is now version 2 with a 12-byte
header and 20 suffix bytes; byte 10 carries IMU-timeout capability. Its parser
validates padding, complete identity and capabilities atomically against the
catalogue. A repository-wide consumer audit found only this Rust crate's own
record APIs/tests, with no native or persisted consumers. Public JSON, supported
storage, Tunes and archives retain their existing identities and formats.
Activation effects box their one-time facts to avoid enlarging every effect
after adding the suffix; all consumers compile and pass their tests.

Four new test cases cover suffix length/character/padding bounds, generic
unsupported identities, every catalogue entry's capability/record round trip,
and beta-record identity/capability/version/extent corruption. Existing beta
admission and older-release rejection matrices also pass.

Final checks passed:

- `cargo test --locked --offline` for `float-view-refloat-compatibility`,
  `float-view-controller-discovery`, `float-view-device-mutation-coordinator`,
  `float-view-connection-application-coordinator`, `float-view-application-runtime`,
  `float-view-backend-service` and `float-view-loopback-server`: 238 tests, plus
  passing doc tests. The final run used escalation for loopback socket binding
  after sandbox transport tests reported `Bind`.
- Warnings-denied all-target Clippy for the same seven crates and workspace
  `cargo fmt --all -- --check`.
- `FLOAT_VIEW_FIXTURE_REFLOAT=1.3.1-beta1 FLOAT_VIEW_CONFORMANCE_SCENARIOS=refloat-beta1 npm run direct`
  from `conformance`, outside the sandbox: final run passed at revision 190.
- Changed-document Prettier checks, strict telemetry-disabled OpenSpec validation
  and `git diff --check`.

No frontend source or public contract changed, so frontend/browser, native
packaging and physical checks were not repeated. No commits, spec sync or
archive were performed.
