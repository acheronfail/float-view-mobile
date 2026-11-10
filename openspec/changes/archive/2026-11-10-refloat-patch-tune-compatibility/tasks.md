# Tasks

## 1. Shared compatibility and durable data

- [x] 1.1 Reuse audited schema families in store matching, bounded pagination and target-value validation; test bidirectional patch/schema matching, negatives, preserved rows and pagination.
- [x] 1.2 Submit Tune writes with target identity and retain fresh safety/readback gates; verify shared-service cross-patch application and existing CAN configuration/readback coverage.

## 2. Frontend and mock

- [x] 2.1 Align UI/mock family checks while preserving exact public versions; test policy parity, cross-patch overwrite and invalid neighbors.
- [x] 2.2 Add browser coverage for compatible patches and incompatible disabled actions; run relevant Chromium/WebKit tests, frontend checks/build/lint and conformance type checks.

## 3. Integration and closeout

- [x] 3.1 Run affected Rust tests, formatting and warnings-denied Clippy; verify existing native-boundary/shared consumer coverage appropriate to unchanged ABI.
- [x] 3.2 Reconcile fallback draft, sync canonical requirements and validate touched specs/change with evidence recorded here.

## Workflow follow-up

- Archive after verified completion; no additional commit or online changes are implied.

## Verification evidence

- Frontend: 71 focused Tune tests passed; Svelte check reported zero errors/warnings; production build passed; changed-file Prettier and ESLint passed. Existing chunk-size advisory remains.
- Browsers: the four existing Tunes cases passed in Chromium/WebKit; both new cross-patch apply/overwrite cases passed after correcting the mock target identity. Incompatible Overwrite remains visibly disabled.
- Shared service: both persisted 1.3.0 → 1.3.1 and 1.3.1 → 1.3.0 scenarios passed, including verified apply, actual changed-value overwrite preserving source identity and stale-revision rejection. Local servers were run outside the sandbox; a cold-build startup timeout was resolved by rerunning after compilation completed.
- Canonical conformance: mock passed (revision 86); direct stable 1.3.1 passed (revision 232), including cross-patch filtered listing.
- Rust: 51 product-store tests, 24 archive tests, 153 backend-service unit tests, 18 coordinator composition tests (including CAN configuration/readback), 3 Apple FFI tests, 1 Android JNI test and shared Apple/iOS/Android Tune publication test passed. Focused warnings-denied Clippy and workspace formatting passed.
- Conformance TypeScript check passed. Fixture-manifest parity covers all audited patch-schema pairs and supported patch 255, with cross-minor/unknown fingerprint/signature negatives.
- Canonical spec validation has no errors and retains four pre-existing long-description warnings (bounded schema, signed codec, persistent write, lifecycle/cache); their description text is unchanged. The change validates without issues. The fallback draft preserves this baseline without being implemented.
