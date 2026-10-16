# Implementation verification

Verified on 2026-09-30. All ten implementation tasks are complete.

## Ownership and behavior

The existing product store owns the pinned seven-profile catalog and validates
saved/imported configuration. The existing backend service owns the calculation,
constant-size current filter and matching accepted-sample estimate. Apple and
Android use the existing shared projection and bindings without native formulas.
The workspace remains at 28 crates and two binding families.

Tests cover every pinned source knot, independent interpolation/remapping oracles,
current filtering and ordering, reconnect/configuration resets, exact voltage
endpoints, missing inputs, raw controller-value preservation, persistence gating,
profile revision conflicts, archive round trips and invalid archive configuration.
The pinned source fixture lives in `backends/core/product-store/tests/` so tests do
not depend on this change's eventual archive location.

## Checks and results

- Rust tests passed for `float-view-product-store`, `float-view-backend-service`,
  `float-view-loopback-server`, `float-view-application-runtime`,
  `float-view-application-runtime-host`, `float-view-application-runtime-apple-ffi`
  and `float-view-application-runtime-android-jni`, using `cargo test --locked --offline`.
- Warnings-denied Clippy passed for the affected product store, service, loopback
  and application runtime crates and all their targets. `cargo fmt --all --check`
  and `backends/core/audit-ownership.sh` passed.
- Frontend `npm test`: 198 tests passed across 49 files. `npm run check` reported
  zero errors and warnings; `npm run format:check` and `npm run build` passed.
- Conformance `npx tsc --noEmit` passed.
- Canonical `npm run direct`, `npm run mock`, `npm run macos` and
  `ANDROID_HOME="$HOME/Library/Android/sdk" npm run android` passed.
  The configured-board checks validate complete snapshots, catalog bounds,
  authoritative sample/profile metadata, a numerical estimate, rejected edits,
  revision conflicts and series mismatch. Existing raw-sample/export assertions
  remain in place. Apple coverage used the macOS software host; Android used the
  dedicated Float View emulator.
- `node --strip-types --test test/battery.browser.ts` passed in Chromium and
  WebKit against the built frontend. It exercises touch menu activation, native
  selection without duplicate saves, non-primary input, keyboard input,
  validation feedback, stale-revision recovery, configuration reopening,
  live/detailed percentage consistency and frontend reload without telemetry polls.
- `OPENSPEC_TELEMETRY=0 openspec validate calculate-cell-aware-battery-percentage --strict`
  and `git diff --check` passed.

Sandbox listener restrictions were resolved by rerunning listener-based tests
with local network access. One concurrent Android run timed out during existing
fixture discovery before the battery checks; the isolated final rerun passed.
`just verify` and a separate iOS simulator run were not needed for this shared
calculation/display change; both native binding consumers and the Apple/Android
software host paths were exercised.
