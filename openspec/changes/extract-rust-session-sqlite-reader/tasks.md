## 1. Consolidated package and immutable evidence

- [x] 1.1 Rename the unselected crate/package from `session-sqlite-writer` to `session-sqlite`, retain the complete `SessionWriter` API and tests, and remove every superseded writer-only workspace/recipe/path reference.
- [x] 1.2 Rename the writer fixtures to `backends/fixtures/session-sqlite` and add literal reader requests/expected projections for metadata, fields, stable/extension/null series, sample/event/location pages, gaps, published boundaries, and continuation keys.
- [x] 1.3 Replace the writer-only root test/audit recipes with consolidated session SQLite recipes that require the exact dependency graph and reject old paths, host selection, reader/writer bridges, generated bindings, arbitrary public SQL, copied native code, and production source/schema changes.

## 2. Read-only lifecycle, schema, and boundary ownership

- [x] 2.1 Add stable payload-free reader errors and bounded owned metadata, marker, field descriptor, published boundary, range, continuation key, page, event, location, and reduction values with redacted debug output.
- [x] 2.2 Implement independent read-only/no-mutex open, short busy handling, integrity and exact canonical table/index/singleton/cadence/marker/field validation, and close/drop behavior without mutation or filesystem discovery.
- [x] 2.3 Implement one deferred read transaction per query that validates equal history revision plus nonfuture sample/event/location boundaries inside the snapshot and releases on every exit.
- [x] 2.4 Expose a safe cloneable interrupter around SQLite cancellation, map interruption to the closed cancelled error, and prove the reader remains reusable after cancellation.

## 3. Bounded fields, samples, and series reduction

- [x] 3.1 Add the closed 36-field stable catalogue, admit bounded unique extension names only from durable field evidence, parameter-bind extension JSON paths, and reject unsupported/malformed/nonfinite values without caller-controlled SQL identifiers.
- [x] 3.2 Implement lossless 1–2,000-row sample pages ordered by `(elapsed_ms, sequence)`, capped by range/published sequence and optional continuation key, with exact nullable values and has-more key behavior.
- [x] 3.3 Implement streaming 1–16-field series scans capped at 200,000 source rows and feed exact ride phases, cadences, retained resolution, and gap policy directly into `session-history-kernel::SeriesAccumulator` for 1–1,024 buckets and bounded columns.
- [x] 3.4 Add exact and adversarial tests for every field/range/bucket/source/page/integer bound, stable/extension/null/tie/gap behavior, unsafe names/JSON, malformed values, boundary mismatches, continuation order, and no partial results.

## 4. Bounded event and location pages

- [x] 4.1 Implement 1–500-row supported transition-event pages ordered by `(elapsed_ms, sequence)`, capped by the published event boundary and optional continuation key, with exact details JSON and nullable source sequence.
- [x] 4.2 Implement 1–2,000-row location pages ordered by `(elapsed_ms, location_sequence)`, capped by the published boundary and optional prior sequence, with exact observations, greater-than-ten-second `noFix` gaps, boundary, and has-more sequence.
- [x] 4.3 Add exact/adversarial tests for all supported and excluded event kinds, malformed details/source facts, location boundaries/order/coordinates/accuracy, page edges, gap edges, stale/future boundaries, and transaction cleanup.

## 5. WAL concurrency and repository verification

- [x] 5.1 Add active-WAL tests proving an older published reader snapshot remains stable while `SessionWriter` commits newer samples/locations, writer progress is not obstructed, and a later read observes the newer marker.
- [x] 5.2 Exercise cancellation during a maximum bounded scan plus corrupt/incompatible/busy/locked databases and prove prompt resource release, reader reuse, payload-free diagnostics, and no file mutation/checkpoint.
- [x] 5.3 Run locked offline session SQLite tests/audits, existing writer parity, schema/dependency/ownership audits, formatting, and warnings-denied workspace linting.
- [x] 5.4 Run strict OpenSpec validation, `git diff --check`, and the complete `just verify` software gate.

## 6. Physical regression and completion

- [x] 6.1 Run the canonical ESP32 fixture matrix on the macOS host and record every non-location checkpoint.
- [x] 6.2 Run the canonical ESP32 fixture matrix on the connected Android device, including background BLE/session persistence; defer only bounded location acquisition.
- [x] 6.3 Run the canonical ESP32 fixture matrix on the connected iOS device, including background BLE/session persistence; defer only bounded location acquisition.
- [x] 6.4 Confirm the implementation commit contains no production Apple/Android/frontend/SQL source or package input, record exact verification evidence, and mark the change ready for canonical spec sync and archive.

## Verification Evidence

- 2026-08-30: `openspec validate extract-rust-session-sqlite-reader --strict`, `git diff --check`, the locked offline session SQLite tests/audit, warnings-denied workspace linting, and the complete `FLOAT_VIEW_ANDROID_EMULATOR=emulator-5554 just verify` gate passed with exit code 0. The root gate included frontend/conformance tests, complete Rust/Swift/Kotlin tests, Apple and Android packaging, macOS/iOS-simulator/Android execution, and repository audits.
- 2026-08-30: macOS passed all 16 canonical ESP32 scenarios against `/dev/cu.usbmodem111401` in one run, including identity, direct/legacy, direct/CAN configuration and control, shutdown, routing, every BMS topology, ride, reconnect, session persistence, and cleanup.
- 2026-08-30: Android device `1cfb7634120d7ece` passed all 17 ESP32 scenarios cumulatively across resumable runs. Three fixture `ADVERTISE_FAILED` setup errors occurred before application assertions; each run resumed at the interrupted scenario. Background BLE telemetry, native session accumulation, foreground recovery, persistence, and cleanup passed. Timed location acquisition alone was explicitly deferred and reported disabled with unchanged location rows/sequences.
- 2026-08-30: iOS device `00008101-00195021268B001E` passed all 17 ESP32 scenarios in one run. Background BLE telemetry continued through response 100, the native session remained available through foreground recovery, and cleanup passed. Timed location acquisition alone was explicitly deferred and reported disabled with unchanged location rows/sequences.
- 2026-08-30: `git diff --name-only HEAD -- backends/apple backends/android frontend backends/sql ':(exclude)backends/apple/ios/float-view.xcodeproj/project.pbxproj'` was empty. The excluded Xcode project file was an existing user-owned working-tree change and is not part of this implementation. No production host/frontend/SQL source, host package input, bridge crate, or generated binding changed.
