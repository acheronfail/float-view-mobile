# Verification

Implementation provides Settings → Diagnostics → Export diagnostics, native ZIP
presentation, independent bounded local incidents, and manual GitHub reporting
instructions. There is no issue prefill, composer launch, submission or upload.

## Passed software checks

- Shared Rust backend tests: the affected library run passed 85 tests, followed
  by focused runs including all 16 diagnostic/initial-connection regressions as
  final cases were added. Coverage includes saved/guest failure before recording,
  closed privacy fields, journal reopen/expiry, unknown versions/corruption,
  symlink/hardlink rejection, reserved cleanup admission, immutable ZIP contents,
  original incident compatibility facts, handle ownership/replacement, clear
  races and disconnected preparation.
- `cargo test --locked --offline -p float-view-loopback-server -p
float-view-product-store -p float-view-session-sqlite`: passed outside the
  sandbox, including transport authentication and frozen baseline reopen/import
  and historical archive tests. Two existing scale benchmarks remain ignored.
- Warnings-denied Clippy for changed Rust crates and all their targets, and Rust
  formatting: passed. Baseline 100, archive format 4 and frozen fixtures remain
  unchanged.
- Frontend tests: 409 tests passed inside the sandbox; the four authentication
  tests requiring local listeners passed outside it. Final focused diagnostics,
  client and mock tests passed again (39 tests).
- Frontend Svelte/type checks, production build, changed-file ESLint/Prettier,
  and conformance TypeScript checks: passed. The production build retains its
  existing large-bundle advisory.
- Canonical direct and mock conformance: passed, followed by focused diagnostic
  conformance on both targets after lifecycle/handle fixes.
- Chromium and WebKit production-UI diagnostic checks: passed for desktop/touch,
  non-primary rejection, keyboard activation, immutable preview, native
  presentation simulation, manual-only Issues link and clear. Mobile full-page
  visual inspection completed. Browser tests ran outside the macOS sandbox.
- Apple host tests (`just test apple`): passed. Focused diagnostics conformance
  through the freshly linked macOS host: passed. The attempted full macOS suite
  timed out at the unrelated `location.requestPermission` step; focused coverage
  avoids prompting for unrelated OS permissions.
- Android host/JNI tests: passed using installed OpenJDK 21, including an explicit
  `./gradlew --offline testDebugUnitTest --rerun-tasks` so the tests execute with
  the rebuilt native library rather than relying on Gradle's external-library
  up-to-date result.
- Third-party notice regeneration/check for pinned `zip` 6.0.0 and resolved
  dependencies: passed. Existing `getrandom` and SQLite dependencies are reused.
- Strict OpenSpec validation and `git diff --check`: passed.

## Remaining native presentation checkpoint

Task 6.3 remains open. No Android device was attached and no Apple simulator was
booted during this session. Real iOS/Android save/share UI, and real-device
background/file-protection behavior of the new diagnostic directory, have not
been physically validated. The macOS diagnostic contract and native host tests
are software validation, not evidence of an interactive save-panel check.

On the relevant native installations, export diagnostics, save the ZIP,
verify the three named members open, cancel/retry sharing, and follow the plain
Issues-list link. Verify that no issue is prefilled/submitted, that clearing does
not break an already open presentation, and that interrupted/protected-storage
captures report their limitations after reopening. No motor movement, new
firmware interoperability or expanded radio matrix is needed. Keep detailed
physical evidence and device inventories in ignored local testing directories.

The implementation was kept active with this limitation explicit. The owner
subsequently requested sync/archive; see the archive disposition below.

## Simplified export UX follow-up

Settings now has one export button, without preview, incident selection, manual
mark, copy, refresh or clear controls. It prepares the newest retained incident
(or current context when empty) and immediately opens native save/share. Static
copy retains manual GitHub attachment instructions and public visibility.
Backend methods and automatic collection are unchanged.

Svelte checks, production build, changed-component lint/formatting, focused
frontend diagnostic tests and Chromium/WebKit browser checks passed. Browser
coverage verifies one button, primary/touch/keyboard activation, no double
activation, repeated exports beyond the two-report capacity, no visible preview
and the plain Issues link without automatic GitHub requests. The existing build
size advisory remains. This UI follow-up does not establish the remaining native
checkpoint above.

## Central request failure capture follow-up

The shared dispatch boundary now records every non-diagnostic backend error code
with a closed API method name, including handled validation/state/permission,
not-found and conflict failures. Authenticated session HTTP errors use fixed
route categories. Unknown method text becomes unknown. Coalesced request failures
remain exportable after the follow-up window without extending it. The frontend
queues closed categories for transport/timeouts and session HTTP/body failures,
with ten admissions per minute and ten pending. Diagnostic failures do not recurse;
deliberately cancelled session reads do not create incidents. Raw request data,
error text, URLs, tune names and values are excluded.

Verification passed:

- Backend-service library: 89 tests, including all error codes, method allowlist
  coverage against the frontend contract, private sentinel rejection and late
  coalesced failure exportability.
- Loopback server: 12 tests, including an authenticated HTTP failure inspected in
  the exported ZIP, with private body/query sentinels excluded.
- Shared host, Apple FFI and Android JNI consumers: 10 tests.
- Frontend: all 416 tests, including caught timeout capture, diagnostic recursion
  prevention, queue bounds, reconnect retry and deliberate cancellation.
- Focused canonical diagnostics on direct and mock targets, plus Chromium and
  WebKit one-button export/browser checks.
- Svelte checks (zero errors/warnings), production build (existing size advisory),
  changed frontend lint/formatting, conformance TypeScript, warnings-denied Clippy,
  Rust formatting, strict OpenSpec validation and diff whitespace checks.

The Export logs label is included. No new OS effect or firmware behavior was
introduced, and the original broader native presentation checkpoint remains open.

## RC6 export inspection corrections

Inspection of an owner-provided RC6 export found incorrect stable/candidate
labelling, inverted small-sample timing percentiles, stripped scheduler outcomes
and transport/protocol generation namespace ambiguity. The owner confirmed there
were no app-visible errors; absent backend-failure events are expected for that
sample, while request-delay observations remain useful timing context.

Corrections preserve the full candidate tag through the native store archive
environment, use nearest-rank percentiles, allow all seven scheduler outcome codes,
and retain separate transport/application generation fields with independent
report-local aliases and explanatory report text. Older immutable captures retain
their original data. Existing bounds, privacy exclusions, baseline/archive markers
and file presentation behavior are unchanged.

Passed verification:

- Backend-service library: all 92 tests, including empty/small/bounded percentile
  samples, seven scheduler outcomes/private text rejection, and independent ZIP
  reading of generation aliases across protocol changes and transport reconnects.
- Loopback server: 12 tests; backend BLE integration: two tests; shared host/Apple
  FFI/Android JNI consumers: ten tests.
- Apple store/archive tests: 32, including full candidate metadata in frontend and
  native build environments with numeric Apple marketing/build versions retained.
- Apple upload/delivery tests: 18; focused canonical diagnostics on direct and mock.
- Warnings-denied Clippy for changed shared crates/all targets, Rust formatting,
  strict OpenSpec validation and whitespace checks.

No frontend or native presentation code changed, so browser/device presentation
checks were not repeated. A fresh signed/uploaded candidate has not been created
for these corrections; local packaging-command tests are not TestFlight delivery
evidence. The original remaining scoped presentation checkpoint remains open.

## Archive disposition — 2026-10-17

The owner explicitly requested syncing and archiving this change after successful
TestFlight use and inspection of their exported iOS diagnostic ZIP. This supplies
evidence for the iOS export path and the simplified UX; it does not establish the
entire original native checkpoint. Task 6.3 remains unchecked in the archived
record: macOS/Android interactive save/share and system-browser checks, and
scoped real-device lifecycle/file-protection behavior, were not fully verified.
Archiving is the owner's requested workflow disposition, not a claim that these
unrun checks passed. The last metadata/timing corrections remain supported by the
software verification recorded above; no newer candidate delivery is claimed.

Main support-diagnostics and backend-contract specifications are synchronized
with the final single Export logs button, manual GitHub reporting, request-failure
capture and report corrections. Historical API summary/preparation remains
supported without a visible user preview step.

Archive documentation checks: the change and support-diagnostics spec pass strict
validation. Backend-contract passes normal validation; strict mode still flags
its existing long-requirement warnings. The sync introduces no additional
long-requirement warnings. Requirement-block comparison verifies both deltas
are fully reflected in main specs, and unrelated backend requirements remain
unchanged. `git diff --check` passed. No application code changed for archival.
