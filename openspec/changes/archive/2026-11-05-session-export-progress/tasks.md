# Tasks

## 1. Shared progress contract

- [x] 1.1 Add worker counts, bounded correlation and cancellation with cleanup/race tests; run affected Rust checks.
- [x] 1.2 Update HTTP/control request validation, contract/mock and canonical progress assertions; verify frontend and direct/native consumers.

- [x] 1.3 Add larger bulk disk budget, preflight and storage/size errors; verify free-space boundaries and a bundle exceeding 128 MiB.

## 2. Modal

- [x] 2.1 Add modal, sequential progress polling, cancellation and distinct outcomes; verify Svelte and browser touch/keyboard/failure/cancellation tests.

## 3. Integration

- [x] 3.1 Run focused mock/direct/Apple/Android conformance, formatting, lint and spec validation; record actual evidence.

## Workflow follow-up

- Sync canonical specs and archive after required checks pass.

## Verification evidence

- Frontend Svelte checks: zero errors/warnings; production build passed with the existing bundle-size advisory.
- Focused frontend export/mock/licence tests: 43 passed. Affected ESLint and Prettier checks and conformance TypeScript checks passed.
- Chromium and WebKit: progress counts, blocking modal/focus, touch/keyboard/non-primary handling, keyboard/touch/Escape cancellation, repeat export, failure and empty history passed.
- Product-store, backend-service and loopback-server suites passed; scale benchmarks retain their existing ignored status. Focused tests cover correlated cancellation, idempotence, stale IDs, completion races, resource cleanup, free-space boundaries, a ZIP larger than 128 MiB, and mid-export cancellation preserving sources.
- Focused board-export conformance passed mock, direct, macOS and Android, exercising status and late-cancel semantics through shared native integrations. Android rebuilt all four ABIs and used real scoped save pickers; these are software host checks.
- Rust formatting and warnings-denied Clippy passed. Bundled dependency notices were regenerated and verified offline for the safe filesystem-space dependency.
- One final macOS attempt timed out at startup under concurrent builds; its serial rerun passed. The Android emulator crashed after an earlier successful run; restarting the existing AVD without its snapshot allowed the final Android run to pass.
- Bulk ZIPs are bounded at 2 GiB and 10,000 sessions. Individual session artifacts retain their 128 MiB compatibility bound. Free-space estimates and repeated checks cannot reserve space against unrelated applications; write failures still discard the partial output.
- Canonical specs validate with 36 unchanged requirement-length warnings in backend-contract and one in saved-board-session-history. New requirements have no warnings.
