# Tasks

## 1. Confirmed break metadata

- [x] 1.1 Add bounded snapshot-aware SQLite break reads, shared metadata projection, frontend contract/mock updates and documentation; verify source references, ordering, pruning and limits with Rust tests and metadata consumers with type checks.
- [x] 1.2 Extend canonical reconnect conformance to assert exact committed break metadata and archive/pruning preservation; run the focused mock and direct scenarios.

## 2. Chart presentation and interactions

- [x] 2.1 Implement compact time mapping composed with map intervals, connected to paths, tooltips, markers and mouse/touch gestures; verify reversible coordinates, viewport boundaries and ordinary gaps with focused frontend tests.
- [x] 2.2 Render accessible vertical break markers with durations; verify browser inspection, zoom, pan, reset and map-filter behavior with rendering coverage.

## 3. Integration evidence

- [x] 3.1 Run relevant frontend checks/build, Rust consumer checks, formatting and strict OpenSpec validation; record actual results and coverage limits.
- [x] 3.2 Load the supplied ride archive, inspect the completed charts and deliver a screenshot without checking private ride data into the repository.

## Verification outcome

- Frontend: 40 focused Vitest tests; Svelte check; affected ESLint/Prettier checks; production build passed.
- Browser: all 12 Chromium/WebKit cases passed across recording breaks, mouse interaction, chart ranges and map filtering. The ignored middle-button branch uses explicit pointer events because Windows WebKit omitted a subsequent primary pointerdown in native automation; primary zoom and right-button pan still use native input.
- Rust: both new SQLite reader tests passed, including published boundaries, exact endpoints, pruning, malformed references and the 1,000-entry limit. Affected crates/consumers compiled and warnings-denied Clippy and formatting passed.
- Canonical reconnect scenario passed against the mock. Broader native runtime tests/direct conformance are unavailable on this Windows host: existing storage creation opens a directory for fsync and returns Io. This is outside this change's scope; no native import or OS integration validation is claimed.
- The production reader confirmed the supplied archive's six break endpoints after removing the archive wrapper table from a private copy, matching the importer’s existing preparation. The finished frontend rendered its recorded samples without page errors. Screenshot and private preview data remain in ignored conformance/generated/ride-review/.
