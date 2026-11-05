# Tasks

## 1. Identity resolution

- [x] 1.1 Remove transport-based ownership fallback and implement atomic UUID reassociation; verify store tests for exact/new/imported identities, invalid UUIDs, capacity and failure rollback.
- [x] 1.2 Update synthetic callers to provide meaningful UUIDs and verify preserved profiles, sessions and supported baseline imports through affected product-store tests.

## 2. Shared activation

- [x] 2.1 Add protocol replay for controller changes behind one transport and verify separate recording ownership and published board state in backend-service tests.
- [x] 2.2 Run affected shared-service/direct conformance, formatting and warnings-denied Clippy checks; validate specs and record evidence.

## Workflow follow-up

- Sync canonical specs and archive after required verification completes.

## Verification evidence

- Product-store and backend-service suites passed, including all frozen baseline and archive import tests; existing scale benchmarks remain ignored.
- Store tests cover new-controller separation/default profiles, exact existing and imported matches, association changes after reopen, invalid/missing UUIDs, preserved opaque imported identities, full-capacity reuse and rollback after capacity or injected insertion failure.
- Shared service protocol replay verifies A → B → A through one Bluetooth device: separate session ownership, original profile restoration, old session preservation and published offline/active board associations.
- Full production direct conformance passed (`revision=476`). Its initial run exposed the old software fixture's all-zero UUID; executable synthetic controller fixtures now supply a meaningful UUID, while frozen source/baseline fixtures remain unchanged.
- Fixture/frame/profile tests passed (112 tests); software protocol/relay tests passed. Conformance TypeScript, affected Prettier, Rust formatting, warnings-denied Clippy and diff whitespace checks passed.
- Only shared identity resolution and synthetic verification changed; no native host interfaces, public wire shape, database schema or stored identity values changed. Existing mixed histories are preserved rather than guessed apart.
- Change validation has no issues; canonical history validation retains one unchanged pre-existing requirement-length warning. Prior canonical content is preserved verbatim.
