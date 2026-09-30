## 1. Shared percentage presentation

- [x] 1.1 Extend the shared battery presentation helper with estimate-first selection, saved-board `not_configured` fallback, source, and setup-needed state; verify focused unit cases for differing percentages, rounding, zero, absent/invalid reported values, and every excluded unavailable reason.
- [x] 1.2 Apply the presentation to both live views, including fill, colors, source-aware labels, warning placement, and existing voltage/charging/stale behavior; verify component coverage for configured, partially configured, and unconfigured boards plus guests and BMS-only connections.

## 2. Reusable battery setup

- [x] 2.1 Extract reusable board battery fields and validation and provide a battery-only editor mode using the existing profile lifecycle and revisioned save queue; verify equivalent validation, prepopulation, and saves in the full editor and battery-only mode without unrelated fields.
- [x] 2.2 Connect both warning buttons to one Live configuration modal mode bound to the opened saved-board identity; verify primary pointer and keyboard activation occur once, non-primary/cancelled gestures do nothing, and only one editor mounts.
- [x] 2.3 Implement accessible modal focus and dismissal, blur/flush close handling, visible save/conflict failures, and board-change invalidation; verify persistence after reopen, no cross-board queued saves, and focus restoration after the warning disappears.

## 3. Integration and documentation

- [x] 3.1 Exercise browser/mock flows under ongoing telemetry: fallback in both views, battery-only setup, authoritative switch to estimate, missing reported value, validation/save errors, reconnect, board switch, and view recreation; record observed results and add regression coverage using the existing browser test conventions.
- [x] 3.2 Update frontend ownership documentation to describe the conditional reported fallback and shared battery editor; verify it agrees with the delta spec and preserves native estimation ownership.
- [x] 3.3 Run affected frontend Vitest tests, `npm run check`, and relevant formatting/lint checks from `frontend`; record results. Broaden checks only if implementation changes the expected frontend-only scope.

## Verification results

- Frontend Vitest: 205 tests passed in the full run; the four loopback authentication tests were sandbox-blocked and then all passed in an elevated focused rerun (209 total).
- `npm run check`: zero errors or warnings. `npm run build`: passed.
- ESLint and Prettier checks for all changed frontend files: passed. `git diff --check`: passed.
- Conformance `tsc --noEmit`: passed.
- `node --strip-types --test test/battery.browser.ts`: Chromium and WebKit passed against the final frontend build, with ongoing mock telemetry. Coverage includes fallback/missing reported values, pointer cancellation and keyboard/touch opening, modal subset and reopening, accepted profile changes, revision conflict recovery, save failure and explicit dismissal, retained drafts during projected reconnect snapshots, board-identity invalidation, focus restoration, and view recreation.
- Shared-service algorithms, firmware protocols, public contract, and native host code are unchanged. Reconnect/board-switch browser cases exercise frontend snapshot reactions; they do not claim physical Bluetooth validation.
