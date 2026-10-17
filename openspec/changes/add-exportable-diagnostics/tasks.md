# Tasks

## 1. Typed diagnostic journal and retention

- [x] 1.1 Define closed diagnostic events, environment/compatibility facts, run/generation/operation correlation and centralized numeric limits in the existing shared service; verify serializer tests reject arbitrary strings/payloads and enforce event/collection bounds.
- [x] 1.2 Implement an independently versioned, exclusively owned diagnostic directory with bounded append/rotation, atomic metadata publication and validated-path cleanup; verify temporary-directory tests cover links, unknown files/versions, truncated tails, corruption, reopen, seven-day expiry and all byte/count limits without product-file mutation.
- [x] 1.3 Connect the nonblocking writer with bounded batches/retries/shutdown and memory-only degradation; verify injected full/locked/unavailable storage and queue saturation count losses, avoid recursion and leave telemetry/product operations responsive.
- [x] 1.4 Implement automatic/manual incident windows, coalescing, rate limits, restart completeness and generation-safe clearing; verify deterministic-clock tests cover a failed initial connection, error storms, manual marking, background interruption, stale workers and clear-versus-capture races.
- [x] 1.5 Document the diagnostic event/retention format and independent root in reusable repository documentation and `docs/storage-baseline.md`; verify links, bounds and product reset exclusions match the implementation.

## 2. Integrate shared producers and native environment facts

- [x] 2.1 Initialize diagnostics before discovery and instrument setup, selection, compatibility, protocol/configuration, request timing, lifecycle/permission, queue and storage/export failure boundaries with typed facts; verify shared-service tests cover failed pre-session setup, guest mode, reconnect generations and unknown firmware/schema facts.
- [x] 2.2 Supply fixed private diagnostic roots and normalized OS/build/model facts through existing Apple C and Android JNI capability boundaries; verify focused host/binding tests cover root isolation, unavailable diagnostic storage and stale native completions without adding product policy to hosts.
- [x] 2.3 Replace the session-leased diagnostic capture sink and ride-close ownership with the app-wide owner; verify session deletion/retention does not delete incidents, old metrics remain covered, and no duplicate diagnostic writer or second runtime remains.
- [x] 2.4 Preserve supported historical diagnostic tables, schemas, imports and recording export behavior; verify existing product-store/session tests and frozen baseline-100 reopen/import tests pass without editing fixtures or incrementing product/archive markers.

## 3. Immutable report preparation and native export

- [x] 3.1 Select and pin a maintained minimal-feature ZIP writer, document why it replaces custom encoding and update lockfile/licensing artifacts; verify the dependency builds for existing consumers with compression/reader features disabled where supported.
- [x] 3.2 Implement bounded immutable report preparation with exactly three fixed ZIP members, report-local aliases, relative timestamps and explicit omissions/loss/completeness metadata; verify an independent ZIP reader can open output, members match the previewed report and adversarial sentinel values never occur in files or filenames.
- [x] 3.3 Generalize existing file-worker/staging/presentation ownership for diagnostic files independent of the catalog; verify session/catalog failures do not prevent report export when diagnostic storage works, and test worker capacity, 20-MiB total storage, partial cleanup and pinned presentation leases.
- [x] 3.4 Implement client-owned report handles, ten-minute expiry, two-report admission and generation-safe invalidation; verify foreign/expired/cleared handles fail, new events cannot alter previewed bytes, and frontend loss/cancellation/duplicate completions release resources boundedly.

## 4. Contract, client, mock and canonical scenarios

- [x] 4.1 Add the six mandatory methods and bounded typed results to `frontend/src/lib/backend/contract.ts`, client/mock and shared loopback dispatch; preserve `diagnostics.report` and existing stable errors. Verify contract/client and server tests cover authentication, unknown/oversized input, result limits and no diagnostic bytes/history in `AppState`.
- [x] 4.2 Implement mapped frontend exception/rejection and feature-error ingestion with closed categories/source positions, a bounded temporary queue and recursion guard; verify unit tests cover private Error messages/stacks, disconnected reporting, rate limits and reporting failures.
- [x] 4.3 Add canonical conformance cases for initial handshake failure, guest isolation, manual capture, reconnect ordering, immutable preview/export, clear races, loss reporting and malicious private inputs; run affected mock/direct scenarios and verify both native host integrations use identical mandatory semantics.
- [x] 4.4 Document diagnostic method examples and error/limit semantics with the contract guidance; verify examples against client/mock tests and the selected shared implementation.

## 5. Reporting UI and GitHub form

- [x] 5.1 Add the shared reporting feature to Settings and relevant connection/configuration/storage error surfaces, with incident selection/current context, manual mark, preview, save/share, copy summary and clear; verify component/mock tests cover empty/unavailable/partial/expired states and unknown hardware.
- [x] 5.2 Implement primary-pointer/cancellation and guarded keyboard activation for reporting controls; build the frontend and run affected Playwright browser tests outside the macOS sandbox, verifying touch/desktop layouts, non-primary rejection and no double activation.
- [x] 5.3 Add `.github/ISSUE_TEMPLATE/bug_report.yml` and manual reporting instructions linking only to the repository Issues list; verify YAML field IDs, duplicate-check/reproduction/unknown-hardware guidance, public attachment notice, absence of prefilled composer links and copy-summary fallback with focused tests.
- [x] 5.4 Update `PRIVACY.md` and reusable support instructions for local retention/deletion, report exclusions, manual public attachment and unavailable reports; verify documentation links and that UI wording never claims upload or issue submission occurred.

## 6. Affected integration verification

- [x] 6.1 Run focused locked/offline Rust tests for changed crates and affected consumers, formatting and warnings-denied Clippy, plus frontend tests/type checks/build and affected mock/direct conformance per `CONTRIBUTING.md`; record actual commands/results and resolve failures before widening coverage.
- [x] 6.2 Run affected Apple/Android host and binding integration tests plus applicable native conformance scenarios; verify complete mandatory contract behavior, independent staging, presentation lifetimes and baseline-data preservation without requiring unrelated native packaging or firmware matrices.
- [ ] 6.3 Perform scoped native save/share and Issues-list link checks on iOS, macOS and Android, and physical lifecycle/file-protection checks only where software cannot establish the affected behavior; preserve detailed evidence locally and report any unavailable required checkpoint explicitly.
- [x] 6.4 Review for private-value leakage, unbounded diagnostic collections, stale/dead session capture paths and unintended baseline changes; run strict OpenSpec validation and `git diff --check`, and record concise implementation verification outcomes and material coverage limits.

## Workflow follow-up

- Commit completed planning artifacts before implementation, then make coherent buildable implementation commits with corresponding task updates.
- After implementation and relevant checks, sync delta specs and archive the change together in the final commit; keep private device inventories and raw physical reports in ignored local evidence directories.
