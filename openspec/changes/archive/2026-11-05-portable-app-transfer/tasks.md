# Tasks

## 1. Portable storage work

- [x] 1.1 Preserve recording UUIDs and verify repeat individual import/re-export, existing baseline archives and collision rejection.
- [x] 1.2 Add bounded manifest/bundle export and full validation/atomic merge; verify populated/empty boards, profiles, tunes, settings remapping, duplicate import, invalid ZIPs, bounds and rollback/restart safety.
- [x] 1.3 Add explicit product clear using restartable deletion and verify preserved user exports/permissions boundaries and re-import.

## 2. Service and UI

- [x] 2.1 Integrate authenticated bounded app transfers and idle guards through shared service/contract/mock; verify affected Rust and frontend tests.
- [x] 2.2 Add Settings export/import/preferences-choice/clear confirmation and summaries; verify frontend checks and touch/keyboard browser flows.

## 3. Integration

- [x] 3.1 Verify canonical mock/direct and affected native delivery consumers, formatting/lint/specs, and record actual results.

## Workflow follow-up

- Sync specs and archive after required verification passes.

## Verification evidence

- Affected Rust product-store, backend-service and loopback-server test suites passed outside the sandbox where socket binding is required. The archive suite passed all 24 cases, including frozen baseline import, pruning-preserving repeat import, identity collision, malformed packages, rollback, clearing and bounds. Backend-service passed 139 unit tests including commit-time idle rechecks and file lease admission. Doc checks passed on a serialized rerun.
- Warnings-denied Clippy for all targets of those three crates and workspace Rust formatting passed.
- Frontend Svelte check/build and Prettier/ESLint passed. Affected state, mock/auth and transfer clients passed 78 tests; the final export-client/Vite configuration rerun passed 7 tests. Conformance TypeScript/fixture checks passed.
- Chromium and WebKit each passed app transfer and existing board export browser scenarios (4 cases total), covering touch/keyboard, duplicate-safe import, clear confirmation, preference choice, failure, export progress and cancellation.
- Full canonical mock and direct suites passed. Focused `FLOAT_VIEW_CONFORMANCE_SCENARIOS=app-transfer` passed on mock, direct, macOS and Android emulator, including native ZIP presentation and merge/clear/re-import through the shared service. Native presentation runs last because picker dismissal and lease acknowledgement are asynchronous.
- Change and new app-data-transfer spec validate strictly. Affected canonical specs validate with 36 pre-existing backend-contract length warnings and one pre-existing saved-board-session-history length warning; their previous text is unchanged. No warnings were introduced by these requirements.
- Individual archive version and owned-storage baseline remain unchanged; no dependencies or host bindings were added. The existing per-session archive ceiling still applies inside the larger app ZIP.
