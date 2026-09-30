# Permissions Settings verification — 2026-09-30

Implementation uses one complete permission inventory and one recovery presentation
slot in the shared Rust service. Recording eligibility and the inventory derive
from the same OS observations. Recovery neither enables recording nor starts
scanning/capture. iOS retains Always access for rides that begin while locked,
as explicitly confirmed during implementation.

## Software evidence

- Frontend focused Vitest checks: 51 passing across client, state, mock, permission
  decoding and pointer activation. All five platform inventories are decoded;
  unknown values, duplicate rows, oversized labels, stale actions and pending
  conflicts are rejected. Presentation alone never invents a grant.
- Frontend type check, changed-file formatting/lint and production build pass. The type check retains the
  pre-existing non-reactive `batteryFields` warning in BoardEditor.svelte.
- Chromium and WebKit production-UI tests pass: applicable rows, optional recording
  context with recording enabled and disabled, primary touch, non-primary rejection, cancellation, keyboard and
  assistive activation, duplicate suppression, stale errors, pending controls,
  transport disconnect and reconnect.
- Shared backend and loopback tests pass, including the permission matrix,
  approximate access, individual Bluetooth grants, disabled services, unknown
  observations, restrictions, background prerequisites, stale action validation,
  one presentation slot, client abandonment, timeout, failed navigation and late
  completion. Affected C/JNI consumer tests and warnings-denied Clippy pass.
- Apple tests pass (8 tests), including service-independent authorization and
  accuracy, existing Always/When In Use behavior, and loopback reconstruction.
- Android tests and the conformance build pass, including first-request history,
  rationale retry, blocked retry, restriction, absent Activity admission,
  timeout and stale native completion. API-dependent projection is covered in
  the Rust matrix; Android 29/31 grant requirements are covered in Android tests.
- Canonical mock and direct conformance pass with mandatory inventory decoding,
  malformed/stale recovery rejection and existing authentication/bounds coverage.
- Conformance TypeScript and fixture UI checks pass.

Commands: focused `npm test` under frontend, `npm run check` and `npm run build`;
`node --strip-types --test conformance/test/permissions.browser.ts`;
`cargo test --locked --offline` for backend-service, loopback-server and affected
application-runtime host/FFI/JNI crates; affected warnings-denied `cargo clippy`;
`just test apple`; Android `testDebugUnitTest assembleConformance` with the installed
SDK; and conformance `mock`, `direct`, and `check`.

## Connected Android — Galaxy S9+, Android 10 / API 29

The real OS displayed the separate background-location dialog. Denial remained
visible; rationale and an explicit retry reached Always access. A subsequent
blocked retry advertised Open settings. Navigation reached the app's Android
Settings page; returning without a change retained denied access. An external
OS grant was reflected on activation while recording remained disabled.

The checkpoint navigates to Settings and waits for Home's existing discovery scan
to finish before asserting recovery has no scanning side effect. Earlier test
setup failures (Home scanning, Samsung task-stack return, unavailable API 29 shell
flag commands) were corrected; no unsupported shell flag manipulation is retained.
The final Settings-return/external-grant run passed. Real denial/retry assertions
passed in the preceding run before its task-stack return failure.

The original background-location grant was restored and verified after every
run. OS request/denial history is intentionally retained; unrelated app storage
and global permissions are not reset. The scoped UI dump was removed.

Reproduction: `conformance/src/permissions-physical-android.ts <device-serial>`.
It requires API 29 and an initially granted background permission for a defined
restore. If OS history already blocks prompting, it exercises Settings recovery
and reports that condition instead of resetting OS history or fabricating a
fresh-request test.

## Connected iPhone — development iPhone 12, iOS 18.7.8

`FloatViewLifecycleTests.testPermissionsRecovery` passed on the physical iPhone.
It exercised a fresh Location prompt, denial, app Settings navigation, return
without changes, and granting Always in Settings. Returning reconstructed the
frontend and displayed Always access. The test restores Always access.

On this phone XCTest's authorization reset could retain a prior denial. The test
uses the app's real Location > Ask Next Time control when necessary, then checks
the fresh prompt. The first test attempts failed on system-button selection and
retained denial; the corrected test passed.

## Connected ESP32

The focused `identity` physical conformance scenario passed for both Android and
iOS with the production Bluetooth adapters. It verifies discovery, connection,
identification and telemetry. The final Android checkpoint also decodes the
complete permission inventory. Fixture safety checks, disconnect, RESET/READY,
LED-off cleanup, and native tunnel cleanup completed. No firmware change or
unrelated control/configuration matrix was needed.

## Remaining physical acceptance

- macOS native inventory was observed (Bluetooth granted; Location unresolved).
  An unanswered Location request returned `unavailable` after the bounded timeout,
  cleared pending state and did not claim a grant. The desktop was locked and the
  UI automation tool could not unlock it. Dialog inspection, Privacy & Security
  pane navigation and interactive activation recovery remain pending an unlocked
  desktop. The user has been asked to unlock it.
- No connected Android API 30+ or 31+ hardware was available. Their background
  Settings and nearby-device grouping paths have software coverage, not physical
  validation. Approximate location and actual one-time-grant expiry were not
  claimed as physical results.

The change remains open until the remaining macOS checkpoint is resolved and
acceptance is recorded; canonical spec sync and archive are deliberately pending.
