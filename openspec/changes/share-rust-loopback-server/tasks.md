# Tasks

## 1. Baseline and dependency selection

- [x] 1.1 Inventory every current route, control method, session-data kind, serializer, host call, and limit. Check the inventory against the TypeScript contract and canonical scenarios.
- [x] 1.2 Record production source, dependencies, package sizes, binding artifacts, and deletion targets. Check that the baseline includes both complete native server files.
- [x] 1.3 Select and lock the Rust server dependencies with required features only. Record licenses and replacement rationale, then compile for all supported Apple and Android targets.
- [x] 1.4 Document numeric connection, queue, worker, transfer, and shutdown bounds. Check that every bound has an assigned boundary test and preserves existing contract limits.

## 2. Shared host exchange

- [x] 2.1 Add the safe shared server crate above the application runtime. Check dependency direction and confirm no networking dependency enters domain crates.
- [x] 2.2 Add versioned start/stop, operation drain/wait, completion, publication, and cancellation records. Test malformed envelopes, stale generations, duplicate completions, queue saturation, and idempotent destruction.
- [x] 2.3 Extend the existing Apple and Android binding families with caller-owned bounded exchange. Test invalid handles, buffer extents, panic containment, interruption, and exact exported symbols.
- [x] 2.4 Add bounded archive transfer handles and chunks with host-owned temporary storage. Test envelope-size independence, cancellation, cleanup, and stale or foreign handle rejection.
- [x] 2.5 Add worker integration without callbacks or UI waits. Test concurrent completion, runtime re-entry, publication, and stop for deadlocks and bounded termination.

## 3. Shared server and public contract

- [x] 3.1 Implement loopback bind, root HTML, bootstrap, token lifetime, and startup failure. Test fixed ports, byte-identical HTML, unknown paths, and failed-bind cleanup.
- [x] 3.2 Implement authenticated HTTP/WebSocket admission and streaming limits. Test missing/wrong Origin or token, fragmented frames, absent Content-Length, exact bounds, and redacted errors.
- [x] 3.3 Implement control dispatch for state, Bluetooth, connection, permissions, settings, saved boards, and profiles through typed host operations. Run canonical success and failure scenarios for each method.
- [x] 3.4 Implement control dispatch for Refloat controls, shutdown, configuration, and Tunes. Test revisions, confirmation, stale requests, and disconnect without duplicate mutations.
- [x] 3.5 Implement session list/get/delete/cancel, prune preview/start/cancel, and export control operations. Test existing pagination, job, conflict, and cancellation semantics.
- [x] 3.6 Implement all session-data kinds, serialization, and compression. Test published boundaries, decoded response limits, two-read admission, and interruption during read/encode/compress.
- [x] 3.7 Implement HTTP import/export through bounded host transfers and existing archive validation. Test valid large archives, malformed inputs, identity preservation, and abandoned transfer cleanup.
- [x] 3.8 Implement complete shared state projection and process-owned revision tracking. Test multiple/slow clients, response correlation, stale facts, committed saved samples, and guest isolation.
- [x] 3.9 Implement listener stop/restart independently of the application runtime. Test worker shutdown, old completions, token continuity, revision continuity, and ongoing recording.

## 4. Canonical parity before host selection

- [x] 4.1 Add shared-server coverage to canonical scenarios with test-only host facts where needed. Keep the fixture adapter limited to launch/setup and reuse `conformance` assertions.
- [x] 4.2 Add adversarial transport and lifecycle scenarios for all routes. Check rejected input performs no storage lookup or board mutation and accepted connected requests receive one terminal result.
- [x] 4.3 Run focused locked Rust tests, warnings-denied linting, mock conformance, and cross-target packaging. Confirm native production still selects its current server at this checkpoint.

## 5. Apple migration and deletion

- [x] 5.1 Replace the shared Swift server with a thin adapter to Rust. Test typed host operations, native executor selection, runtime identity, and catalog/file integration on iOS and macOS.
- [x] 5.2 Connect shared publication and transport lifecycle to Apple application facts. Test frontend recreation and listener stop/restart without resetting BLE, recording, or revisions.
- [x] 5.3 Delete superseded Swift routes, payload builders, auth/compression helpers, and obsolete private tests. Remove unused FlyingFox package references and check forbidden imports and source inventory.
- [x] 5.4 Run focused Swift/FFI tests, clean macOS/iOS simulator/device packaging, and macOS/iOS canonical software conformance. Commit the verified Apple slice with its task updates.

## 6. Android migration and deletion

- [x] 6.1 Replace the Kotlin server with a thin adapter to Rust. Test host operation ordering, native executor selection, runtime identity, and catalog/file integration through linked JVM tests.
- [x] 6.2 Connect shared publication and listener lifecycle to Android application and foreground-service ownership. Test activity recreation, cancellation, reconnect, and continued recording.
- [x] 6.3 Delete superseded Kotlin routes, payload builders, auth/compression helpers, and obsolete private tests. Remove unused Ktor server declarations and check forbidden imports and source inventory.
- [x] 6.4 Run Android canonical software conformance and clean host/four-ABI Debug/Release packaging. Check one binding library per ABI and commit the verified Android slice with task updates.

## 7. Ownership and physical checkpoint

- [x] 7.1 Update ownership docs, OpenSpec context, build/symbol checks, and root audits for the selected shared server. Check that no duplicate dispatch, old dependency, temporary bridge, or fallback remains.
- [x] 7.2 Record final source/dependency/package deltas. Require a net production-source reduction and passing existing convergence limits, with no public contract or SQL schema change.
- [ ] 7.3 Run the explicit Android, iOS, and macOS ESP32 physical checkpoint for telemetry, saved recording, reconnect, and frontend/lifecycle recovery. Record each result and any user-authorized coverage deferral accurately.

## 8. Final verification

- [x] 8.1 Run focused boundary, cancellation, archive, state ordering, shutdown, and host integration checks after all deletions. Check that every added requirement has passing evidence.
- [x] 8.2 Run strict OpenSpec validation, formatting, dependency/source audits, and `git diff --check`. Resolve findings without removing required coverage.
- [x] 8.3 Run `just verify` with both hosts selecting the shared server. Commit the final verified implementation and matching task updates.

After all required gates pass, sync the delta specs and archive this change in one final commit, as required by the repository workflow.
