# Tasks

## 1. Contract and persistence

- [x] 1.1 Extend Live fields and custom actions in TypeScript and Rust; verify validator tests, Rust persistence reopen and canonical mock/direct conformance.

## 2. Dashboard interactions

- [x] 2.1 Add shared appearance toggle controller/display, Live selection and Custom picker/configuration; verify frontend checks and focused unit tests.
- [x] 2.2 Cover primary, non-primary, cancelled and keyboard input, system resolution, persistence, pending/failure and edit suppression with browser tests.

## 3. Integration

- [x] 3.1 Verify formatting, lint, build and affected spec validation; inspect host JSON pass-through and record any unavailable required native checks.

## Workflow follow-up

- Sync canonical specifications and archive after required verification passes.

## Verification evidence

- Frontend: 42 focused Vitest tests passed; Svelte check had zero errors/warnings; production build passed (existing bundle-size advisory); changed frontend files passed ESLint and Prettier.
- Rust: all 115 product-store tests passed, including dashboard reopen and baseline fixtures; rustfmt and warnings-denied product-store Clippy passed.
- Canonical mock and direct Rust conformance passed, including theme action/Live slots and rejection as a numeric reading. Conformance TypeScript check passed.
- Chromium and WebKit: theme and marker browser tests passed (4 cases), covering tap, Enter/Space, non-primary and cancelled input, hold/edit suppression, failed/pending writes, system changes, reload, picker addition and resized persistence. WebKit used libraries extracted into /tmp on this Linux host, without system installation.
- Apple and Android listener adapters both borrow the shared Rust server and do not decode dashboard content; no host or ABI change was needed. Native platform builds were not run; the changed persistence/dispatch boundary was exercised by direct shared-service conformance.
- Reviewed the rendered compact widget screenshot and final diff; no storage migration or fixture rewrite.

Affected canonical specs validated without errors. The two appearance requirement-length warnings belong to unchanged pre-existing requirements (measurement presentation and continuous startup appearance); the new requirements have no warnings.
