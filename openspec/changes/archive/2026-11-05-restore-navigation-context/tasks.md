# Tasks

## 1. Retain browsing context

- [x] 1.1 Add bounded shared route/anchor restoration and session page retention; verify cache bounds with focused unit coverage and invalidation in browser coverage.
- [x] 1.2 Integrate shell navigation and list readiness; cover deep positions through header, browser and native back, independent boards/tabs, keyboard focus and changed catalogs in Chromium and WebKit.

## 2. Verify integration

- [x] 2.1 Run frontend type, formatting/lint, focused unit, build and relevant existing browser checks; record actual results.
- [x] 2.2 Validate changed OpenSpec requirements and synchronize canonical specs after checks pass.

## Workflow follow-up

- Archive the completed change locally after verification and sync.

## Verification evidence

- `cd frontend && npm run check`: zero errors/warnings.
- Focused Vitest navigation-context, navigation, connection-navigation, ui-restoration and activation suites: 34 tests passed.
- `cd frontend && npm run build`: passed; existing single-bundle size warning remains.
- Changed frontend files: Prettier and ESLint passed. Conformance TypeScript and changed browser-file formatting passed.
- Built frontend browser checks, run outside the sandbox and serially: `navigation-restoration.browser.ts` (2), `navigation.browser.ts` (2), `ui-restoration.browser.ts` (2), `session-list-conflicts.browser.ts` (8), all passed in Chromium/WebKit. The final navigation rerun passed after correcting its mock port and explicitly ending the paused ride.
- Restoration coverage includes third-page positions, same-date pagination rendering, header/browser/native back, keyboard focus, independent boards/tabs, inserted/deleted anchors, delayed loading without a top flash, failure/retry, navigation during loading, and resuming the intended loaded extent.
- OpenSpec delta validation has no issues; canonical app-navigation validation retains only its three pre-existing requirement-length warnings. Canonical additions match the delta; existing requirements are unchanged.
