## 1. Shared percentage presentation

- [ ] 1.1 Extend the shared battery presentation helper with estimate-first selection, saved-board `not_configured` fallback, source, and setup-needed state; verify focused unit cases for differing percentages, rounding, zero, absent/invalid reported values, and every excluded unavailable reason.
- [ ] 1.2 Apply the presentation to both live views, including fill, colors, source-aware labels, warning placement, and existing voltage/charging/stale behavior; verify component coverage for configured, partially configured, and unconfigured boards plus guests and BMS-only connections.

## 2. Reusable battery setup

- [ ] 2.1 Extract reusable board battery fields and validation and provide a battery-only editor mode using the existing profile lifecycle and revisioned save queue; verify equivalent validation, prepopulation, and saves in the full editor and battery-only mode without unrelated fields.
- [ ] 2.2 Connect both warning buttons to one Live configuration modal mode bound to the opened saved-board identity; verify primary pointer and keyboard activation occur once, non-primary/cancelled gestures do nothing, and only one editor mounts.
- [ ] 2.3 Implement accessible modal focus and dismissal, blur/flush close handling, visible save/conflict failures, and board-change invalidation; verify persistence after reopen, no cross-board queued saves, and focus restoration after the warning disappears.

## 3. Integration and documentation

- [ ] 3.1 Exercise browser/mock flows under ongoing telemetry: fallback in both views, battery-only setup, authoritative switch to estimate, missing reported value, validation/save errors, reconnect, board switch, and view recreation; record observed results and add regression coverage using the existing browser test conventions.
- [ ] 3.2 Update frontend ownership documentation to describe the conditional reported fallback and shared battery editor; verify it agrees with the delta spec and preserves native estimation ownership.
- [ ] 3.3 Run affected frontend Vitest tests, `npm run check`, and relevant formatting/lint checks from `frontend`; record results. Broaden checks only if implementation changes the expected frontend-only scope.
