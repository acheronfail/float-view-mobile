# Tasks

## 1. Tune confirmation and comparison

- [x] 1.1 Share the compact configuration-write/apply/overwrite comparison presentation; verify labelled current/saved values and no-differences rendering with focused frontend tests.
- [x] 1.2 Separate preview from confirmed application, retain fresh-read and live safety guards, and update frontend documentation; verify browser coverage for idle/riding confirmation, cancellation, matching values, changed values and failed retries.
- [x] 1.3 Extend long-list browser coverage to apply and verify fixed actions, keyboard scrolling, pointer guards and narrow/desktop layouts.
- [x] 1.4 Remove the tune-only renderer and reuse the compact configuration diff in all three confirmations; verify focused unit tests plus configuration-editor and Tunes browser coverage.

## 2. Integration verification

- [x] 2.1 Run focused unit/browser tests, frontend and conformance type checks, formatting/lint, frontend build and strict OpenSpec validation; record actual outcomes and unavailable coverage.

## Verification outcome

2026-10-25: All 43 focused configuration-controller/Tune capture/diff/component unit tests and all four Chromium cases in `conformance/test/refloat-tunes.browser.ts` and `conformance/test/refloat-editor.browser.ts` pass. Configuration writes, Tune application and overwrite use the same compact label: before → after renderer; the superseded Tune-only component has been removed. Browser coverage includes idle/riding confirmation, current-to-saved values, cancellation and focus restoration, non-primary pointers, live safety gating, read/apply failure retries, changed board values, matching tunes, route replacement, native Back dismissal, and keyboard scrolling with fixed actions at 320×420 and 900×700. Screenshots were visually reviewed. Frontend Svelte checks (zero errors/warnings), conformance TypeScript, changed-file Prettier/ESLint, production build, diff whitespace and strict OpenSpec validation pass. WebKit remains unavailable because this Linux host lacks required runtime libraries.
