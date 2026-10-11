# Tasks

## 1. Current ride review

- [x] 1.1 Add saved-session entry points and active review status/return navigation; verify saved and guest modes in browser coverage.
- [x] 1.2 Add bounded visible-page refresh and stale-response protection; verify request ownership and lifecycle with focused tests.
- [x] 1.3 Preserve chart/map state across snapshots and add pause/Jump to live behavior; verify interaction and completion in browser coverage.

## 2. Verification

- [x] 2.1 Run affected frontend tests, type checks, formatting/lint, build and browser regressions; record results.

## Workflow follow-up

- Sync the delta spec and archive after implementation verification.

## Verification results

- Frontend `npm run check`: zero errors or warnings.
- Focused session, state, session-controller and mock-handler Vitest suites: 117 tests passed across 21 files.
- Production build passed (Vite reports the bundle exceeds its 500 kB advisory threshold).
- Affected-file Prettier and ESLint checks passed; conformance TypeScript check passed.
- Chromium and WebKit current-ride browser scenarios passed, including growth, inspection freeze, field visibility, resume, reconnect presentation, refresh failure/recovery, completion, guest exclusion, Sessions entry, pointer cancellation/non-primary rejection and keyboard/touch activation.
- Existing map-filter (desktop and touch), chart-tooltip, ride-marker and session-event browser regressions passed: 10 cases, plus the two current-ride cases above.
- Lifecycle unit coverage verifies hidden/disposed view rejection, one in-flight refresh, failure retry, and superseded session opens.
- `git diff --check` and strict change validation passed. Frontend-only work; no native contract or persistence changes.
