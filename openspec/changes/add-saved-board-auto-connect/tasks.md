# Tasks

## 1. Local preference and contract

- [ ] 1.1 Add required nullable `autoConnectSavedDeviceId` with null default across the frontend contract/client/mock, Rust settings store, public projection, and affected Apple/Android consumers; verify focused settings serialization/projection tests and frontend type checking.
- [ ] 1.2 Validate target existence and local association transactionally through revision-checked settings patches, including mixed-patch rollback; add store tests for select/replace/clear, restart persistence, missing/unassociated boards, stale revisions and operational failure, and verify portable archives neither carry nor overwrite the preference.
- [ ] 1.3 Clear selection and advance settings revision atomically when selected-board deletion succeeds, preserving existing deletion cleanup/error semantics; verify selected/unselected deletion, active-board rejection, failure, and complete-state projection tests.
- [ ] 1.4 If required by strict settings decoding, advance the explicit owned-storage format/schema marker without migrations or old readers; verify version-mismatch reset separately from ordinary corruption/operational failures and update affected storage documentation.

## 2. Shared discovery and foreground policy

- [ ] 2.1 Add service-owned visit/waiting/searching/closed state and a 15-second monotonic deadline, gated by disclaimer, permission and radio readiness; verify deterministic startup, delayed readiness, duplicate foreground, no-target, first selection while waiting, and existing active/failed/reconnecting connection tests.
- [ ] 2.2 Share one physical scanner between bounded normal requests and the foreground window, preserving normal request idempotence and Home cadence; verify demand arrival in both orders, independent expiration, background removal, readiness loss/recovery without deadline extension, and no scan after all demands end.
- [ ] 2.3 Admit one saved connection from a fresh exact association match during the live window and consume eligibility before the attempt; verify absent-UART saved advertisements, other/same-name boards, cached discoveries, duplicate/obsolete results, deadline-boundary ordering and stale timer callbacks.
- [ ] 2.4 Cancel eligibility on syntactically valid manual connection intent, explicit disconnect, applicable user shutdown, backgrounding and preference clearing; verify failed manual commands still suppress, malformed requests do not suppress, mid-window replacement retains deadline, and foreground return rearms without process restart.
- [ ] 2.5 Reuse identification, controller selection, durable session preparation and established-connection reconnect behavior; verify automatic setup/identification failure never loops on advertisements and existing connection/reconnect tests still pass. Update shared policy documentation to describe foreground-owned automatic eligibility and scan sharing.

## 3. Native lifecycle integration

- [ ] 3.1 Audit and, where necessary, correct Apple initial and transition foreground observations over the existing service adapter, including macOS semantics; verify focused host/bridge tests prove one initial visit, real background/foreground rearming and no rearm from WebView/listener replacement. Update affected host lifecycle documentation.
- [ ] 3.2 Audit and, where necessary, correct Android initial and transition foreground observations over the existing service adapter without duplicating product policy; verify focused host/bridge tests for activity recreation, retained backend, initial foreground and repeated callbacks. Update affected host lifecycle documentation.

## 4. Frontend control and navigation

- [ ] 4.1 Add the board settings toggle backed by authoritative Settings and existing revision/error handling, explaining replacement of the previous selection and unavailability without local association; verify primary, non-primary, keyboard/assistive, double-activation and failed-save coverage plus frontend type/lint checks.
- [ ] 4.2 Handle automatic connection snapshots from Home without echoing a connection command, while preserving displayed/restoring non-live routes; verify mock/browser cases for connection before and after checkpoint resolution, Home-to-Live, controller-selection/failed states, and explicit disconnect returning to Home without immediate auto-connect.
- [ ] 4.3 Extend deterministic frontend mock lifecycle/settings behavior for bounded foreground auto-connect and retain Home's current scan cadence; verify background/foreground rearm, WebView/socket recreation suppression, window expiration while Home scans, and update the frontend ownership documentation.

## 5. Integration verification

- [ ] 5.1 Add and run canonical conformance scenarios for preference persistence/validation, exact automatic saved connection, manual suppression, foreground rearm, independent scan bounds and non-Home behavior against affected Apple/Android integrations; verify complete monotonic snapshots and existing saved/guest connection semantics.
- [ ] 5.2 Run affected Rust crate/consumer tests, relevant frontend type/lint/build and mock/browser checks, plus native integration checks justified by adapter changes; record commands/results and any unavailable coverage without requiring an unrelated full regression.
- [ ] 5.3 Perform focused physical checkpoints on affected iOS/macOS/Android hosts for startup scan readiness, permission recovery, foreground return after explicit disconnect and cancellation on background, preserving fixture safety/restore checks where used; record actual platform/scenario results and any unavailable required hardware.

## Workflow follow-up

- Keep implementation slices buildable and commit their corresponding task updates.
- After relevant verification, sync delta specs and archive this change together in the final commit; preserve actual verification evidence and outstanding physical coverage.
