# Tasks

## 1. Durable classification

- [x] 1.1 Persist bounded interruption reasons through the shared writer/runtime/service, recover pending causes on reopen, and verify deliberate pause, connection loss, restart, precedence and write failures.
- [x] 1.2 Project reason-bearing break metadata and validate it in the client/mock; verify old boundaries remain unknown and reasons survive archive/pruning round trips.

## 2. Chart behavior

- [x] 2.1 Compact and mark only deliberate pauses using the existing shared time mapping; verify mixed reasons, unchanged unknown/connection-loss gaps and mouse/touch interactions.

## 3. Verification

- [x] 3.1 Run affected Rust crates/consumers, frontend checks/build, canonical mock/direct conformance, browser tests, formatting and strict OpenSpec validation. Document shared-host coverage and compatibility.
- [x] 3.2 Import the supplied historical archive through the real backend, confirm all six causes remain unknown and no pause markers render, and capture the result locally.

## Verification outcome

- SQLite, product-store, runtime, service and loopback suites passed, including frozen baseline compatibility, reason preservation through pruning/import/re-export, interruption capacity/idempotence, restart, precedence and atomic failure tests. Existing on-disk scale benchmarks remained ignored. Loopback listener tests passed outside the sandbox after sandbox bind failures.
- Shared host, Apple FFI and Android JNI tests passed. Both native hosts use the same reason-bearing metadata projection; no native adapter or binding contract changed. Native packaging, simulator/emulator and device runs were not performed.
- 40 focused frontend tests plus the three session-controller tests passed; Svelte and conformance TypeScript checks, affected ESLint checks and the production build passed. All 12 Chromium/WebKit pause, connection-loss and unknown mouse/touch cases passed.
- Focused canonical connection-context conformance passed against mock and direct Rust targets, including deliberate-pause provenance. Rust formatting, warnings-denied affected-crate Clippy and strict OpenSpec validation passed.
- The supplied historical archive imported through the production Rust service with six `unknown` reasons and zero pause separators. Desktop/mobile screenshots and private preview data remain local and were not added to source control.
- The preceding archived change remains historical; this follow-up records the final implementation and verification.

## 4. Disconnect despite cleanup failure

- [x] 4.1 Update disconnect and writer detach to release the connection and recording ownership even after persistence/catalog failures, preserving supported data and reporting cleanup errors afterwards.
- [x] 4.2 Verify explicit and native stop failure paths, independent cleanup stages, stale-event rejection and resumed unknown provenance after an unsuccessful reason write; run affected Rust/service/binding checks and focused canonical conformance.

### Disconnect failure verification

- All 132 affected SQLite/runtime/service library tests passed. New cases cover explicit/native stop with failed reason writes, marker updates, pause updates, draft deletion and catalog reads; simultaneous failures during close; expired-ride preflight; full effect queues; stale callbacks; and unknown resumed provenance after a failed reason write. A real SQLite checkpoint error also verifies writer close and preservation of committed rows.
- All five shared host/Apple FFI/Android JNI library tests passed. Connection-context and ride-continuity conformance passed against both direct Rust and mock targets. Warnings-denied affected-crate Clippy, Rust formatting, diff checks and strict OpenSpec validation passed.
- This revision changes shared cleanup policy and writer ownership, with no frontend, native adapter or OS effect contract change. No new browser, packaging or physical-device run was required; the earlier UI verification remains applicable.
