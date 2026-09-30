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

## 4. Basic and Advanced preset setup

- [ ] 4.1 Integrate the shared preset catalog/identity/capacity contract after `stabilize-battery-state-of-charge` tasks 1.1–1.3; verify typed mock fixtures and snapshot consumption without a frontend-owned preset policy or duplicate catalog.
- [ ] 4.2 Add accessible Basic/Advanced segments and manufacturer-grouped native select to the shared battery form; verify identical full-editor/modal behavior, initial mode for unconfigured/named/manual profiles, primary/non-primary/cancelled input, keyboard/assistive activation, and no duplicate activation under telemetry.
- [ ] 4.3 Apply presets with one serialized revisioned selection request and pending/error feedback; verify complete defaults, successful persisted identity, rapid selections, conflicts, failed saves, no partial estimator configuration, and no queued edit reaching another board.
- [ ] 4.4 Preserve drafts on mode changes without toggle-induced blur saves; add Custom behavior, actual-value summary, manual override handling, and optional Advanced capacity input. Verify toggles issue no requests, invalid drafts survive, 3.0 V help remains in Advanced, reopening keeps names, count edits invalidate prior capacity, and the explicit reset action restores preset defaults without relying on native select re-selection.
- [ ] 4.5 Cover a known-curve preset and XR/Pint or P30B/RS50 preset mapped internally to generic; verify Basic has no generic/approximation label, missing-curve warning, or extra confirmation while Advanced exposes the actual profile and accepted setup removes only the missing-configuration warning.
- [ ] 4.6 Update frontend ownership documentation and extend browser/mock coverage for both editors, touch/keyboard mode changes, reconnect, view recreation, custom/cleared capacity, and save failures; verify the shared catalog remains service-owned and reported fallback stays limited to `not_configured`.
- [ ] 4.7 Run affected frontend unit/component tests, type checks, relevant formatting/lint checks, and battery browser scenarios; validate both OpenSpec changes and record new results separately from the completed implementation evidence below. Cross-platform contract verification belongs to the companion change.

## Verification results — completed fallback/modal implementation

These results predate the Basic/Advanced preset extension and do not verify tasks 4.1–4.7.


- Frontend Vitest: 205 tests passed in the full run; the four loopback authentication tests were sandbox-blocked and then all passed in an elevated focused rerun (209 total).
- `npm run check`: zero errors or warnings. `npm run build`: passed.
- ESLint and Prettier checks for all changed frontend files: passed. `git diff --check`: passed.
- Conformance `tsc --noEmit`: passed.
- `node --strip-types --test test/battery.browser.ts`: Chromium and WebKit passed against the final frontend build, with ongoing mock telemetry. Coverage includes fallback/missing reported values, pointer cancellation and keyboard/touch opening, modal subset and reopening, accepted profile changes, revision conflict recovery, save failure and explicit dismissal, retained drafts during projected reconnect snapshots, board-identity invalidation, focus restoration, and view recreation.
- Shared-service algorithms, firmware protocols, public contract, and native host code are unchanged. Reconnect/board-switch browser cases exercise frontend snapshot reactions; they do not claim physical Bluetooth validation.
