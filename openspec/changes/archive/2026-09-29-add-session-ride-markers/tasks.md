## 1. Durable marker storage

- [x] 1.1 Extend the sole session writer/runtime with typed marker events, durable operation-ID deduplication, bounded payloads, and event-only elapsed publication; verify transaction failure, duplicate retry, between-sample timing, later aggregate monotonicity, and restart recovery in Rust tests.
- [x] 1.2 Add bounded marker-only reads by published event boundary, elapsed range, and cursor, plus metadata counts; verify pagination, equal-time ordering, maximum bounds, invalid/stale/cross-session cursors, and isolation from large telemetry event streams.
- [x] 1.3 Preserve markers through pruning and native archive validation/export/import, and exempt committed marked sessions from automatic discard; verify short/idle/recovered retention, unchanged unmarked retention, explicit deletion, malformed archive rejection, coordinate preservation, and lossless round trips. If format invariants change, replace the explicit development format marker without migrations and verify corruption never triggers reset.

## 2. Shared command and contract

- [x] 2.1 Define marker types, limits, `session.marker.add`, metadata counts, and the typed `markers` data query in the frontend contract/client/mock and shared server/service, keeping both native binding paths aligned; verify type checks and native contract parity tests.
- [x] 2.2 Implement current-session validation, attached-writer eligibility, reconnect behavior, admission-time session clock, durable acknowledgement, rate/capacity limits, and duplicate-before-limit handling; verify guest, ended, stale-session, close-race, response-loss, and over-limit cases through service tests.
- [x] 2.3 Associate only an eligible same-session committed GPS fix with each marker, retaining its original time/accuracy; verify the 10-second boundary, disabled/revoked location, future/stale fixes, previous-session fixes, and successful time-only markers.
- [x] 2.4 Extend canonical conformance scenarios and fixtures for capture/read/restart/reconnect, archive/pruning/retention, bounded errors, and publication ordering; verify the same outcomes through direct, Apple, and Android shared service paths.

## 3. Live interaction and feedback

- [x] 3.1 Add the accessible large-gauge centre target and cancellable primary-pointer double-tap recognizer using the design's timing/movement limits; verify primary mouse/touch/pen, single/triple taps, non-primary input, drag/cancellation/multi-touch, synthesized clicks, keyboard/assistive activation, and repeated-key handling.
- [x] 3.2 Connect one pending save to the captured session/operation identity, clear gesture state on navigation/session changes, and disable ineligible capture; verify reconnect-window capture, guest behavior, late responses, and no automatic new-ID retries after ambiguous failure.
- [x] 3.3 Add the acknowledged-save 250 ms glow/pulse and brief accessible toast, reduced-motion highlight, and concise failure/unconfirmed messages; verify success follows commit, failures never pulse, feedback does not stack or obscure readings, and both themes remain legible.

## 4. Historical review

- [x] 4.1 Load typed marker pages into one bounded session-review owner with cancellation and snapshot refresh; verify all 1,000 markers fit the four-page bound without telemetry-event enumeration or stale-session leakage.
- [x] 4.2 Add orange vertical bars to each visible time-series plot and wire externally selected marker time into the chart viewport/detail selection; verify zoom, downsampling, sparse/no samples, overlapping times, and pointer/keyboard selection.
- [x] 4.3 Add high-contrast orange marker icons to the map from stored positions, including marker-only maps and overlapping-position groups; verify distinction from the rider, no invented track or no-fix icons, map centring, layer cleanup, and light/dark contrast.
- [x] 4.4 Add compact previous/next and individual marker navigation, selected index/time, and “No location” treatment; verify chart, map, navigator, and bounded detail reads share the exact selected elapsed time on touch and desktop.

## 5. Verification and completion

- [x] 5.1 Run focused frontend interaction/review, Rust persistence/service/reader, and conformance marker tests plus formatting/type checks; verify durable acknowledgement, save-without-GPS, marked short-session retention, and archive/prune review work end to end.
- [x] 5.2 Run `just verify` with the required SDK/simulator environment and resolve failures; record the full software result separately from any physical ride UX checks.

## Verification notes

- Focused Rust storage, service, and archive suites cover commit failure, deduplication and capacity, restart, pruning, marked idle-session retention, optional/future/stale fixes, permission revocation, and malformed marker rejection.
- Canonical direct/mock conformance covers capture, pagination, cursor binding, publication, ended-session rejection, and marker preservation through pruning.
- Frontend checks include pointer gesture/collision/validation tests and production Chromium/WebKit capture and review, including actual WebKit touch taps, keyboard focus, failed-save feedback, empty numeric charts, and a stored-marker map without a base track.
- Physical checkpoint: skipped for this change. It changes shared service persistence and frontend interaction, with no OS adapter, permission request, BLE, lifecycle, or firmware behavior changes. The software gate retains Apple/Android builds and simulator conformance. Real-ride ergonomics remain optional follow-up feedback.

- `ANDROID_HOME=/Users/cosmotherly/Library/Android/sdk just verify` passed: packaging, unit suites, browser/startup checks, direct/macOS/iOS-simulator/Android-emulator canonical conformance, repository ownership audit, and simulation exclusion. The stale mandatory-method count was corrected from 34 to 35. Final frontend checks passed with 196 tests, zero Svelte diagnostics, lint, and Chromium/WebKit review regressions.
- Owned development storage advances to version 13 and native archives to version 2; incompatible development data uses the existing explicit reset policy, with no migrations or old-format readers.
