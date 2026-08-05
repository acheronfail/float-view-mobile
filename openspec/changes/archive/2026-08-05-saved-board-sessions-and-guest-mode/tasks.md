## 1. Contract and shared test seams

- [x] 1.1 Reconcile the new tracking-mode and board-history contract fields with the active `support-vesc-bms-telemetry` change before editing shared contract types.
- [x] 1.2 Add explicit saved/guest connection intent, stable opaque `deviceHistoryId`, nullable local platform association, saved-board match information, and bounded guest live-state representation to `frontend/src/lib/backend/contract.ts`.
- [x] 1.3 Extend `device.connect` validation, required board-scoped `session.list`, `savedDevice.delete`, and archive ownership seams with the board summary and cursor-binding rules.
- [x] 1.4 Update the frontend backend client, mock handler, mock fixtures, and contract-focused tests for saved and guest snapshots, stable archive identity, board-filtered pages, missing-filter conflicts, empty boards, import merge/create, and Forget board cleanup.

## 2. Native saved/guest lifecycle

- [x] 2.1 Add one explicit tracking-mode owner to the Apple connection/application state and preserve it across retry, reconnect, and WebView recreation.
- [x] 2.2 Mirror the tracking-mode owner and lifecycle retention in the Android connection/application state.
- [x] 2.3 Gate Apple saved-device creation, Board Profile writes, controller selection/hint writes, session logging, location recording, catalog revision updates, and ride metadata behind saved mode; provide bounded ephemeral guest samples for LiveView.
- [x] 2.4 Mirror the same durable-side-effect gates and ephemeral guest sample path in Android, including guest controller selection and reconnect behavior.
- [x] 2.5 Update Apple and Android firmware identification so save-and-connect derives or resolves stable saved identity before readiness, binds imported boards when possible, while guest connections can reach readiness without a board or session.
- [x] 2.6 Update Apple and Android controller routing so saved selections update Board Profile state while guest selections remain connection-local and never persist learned hints.
- [x] 2.7 Ensure saved-mode session retention and location behavior remain unchanged, while guest disconnect, failure, and reconnect clear in-memory state without creating history.

## 3. Board history and catalog projection

- [x] 3.1 Add required saved-device-filtered session queries, cursor validation, `savedDevice.delete` cleanup, and archive merge/create handling to the Apple SQLite/session-store and loopback implementation; remove the unused unfiltered session-list path.
- [x] 3.2 Mirror required board-filtered queries, cursor validation, board deletion cleanup, archive merge/create handling, and bounded board summary derivation in Android SQLite/session-store and loopback implementation.
- [x] 3.3 Keep board summaries derived from existing catalog/session data, ensure guest activity cannot change catalog revisions or board projections, and ensure imported boards remain configurable without discovery.
- [x] 3.4 Add or update disposable SQL fixtures for stable board identity, multiple saved boards, date boundaries, deleted/pruned sessions, Forget board cleanup, empty histories, archive import, and guest isolation without introducing a second aggregate schema.

## 4. Frontend navigation and interaction

- [x] 4.1 Remove the Boards primary tab and route while retaining board configuration as a subordinate surface; update route headers and bottom navigation to exactly connect, sessions, and settings.
- [x] 4.2 Replace the global Sessions list with the saved-board index and bounded board summary cards, including the empty state for users with no saved boards.
- [x] 4.3 Add the saved-board history route with overall summary, board-filtered pagination, local-date grouping, loading/error/empty states, and stable board-owned session navigation.
- [x] 4.4 Add a board-history menu item that opens the existing Board Profile editor in a configuration sheet and preserves the route on save, dismiss, not-found, and revision-conflict paths, including boards without local discovery.
- [x] 4.5 Add the `Forget board` header menu item, destructive confirmation copy, active-connection guard, and return-to-Sessions behavior after accepted cleanup.
- [x] 4.6 Update session detail, deletion, missing-session, archive-import, and board-forget navigation to return to the owning board history when board identity is known, with Sessions as the boardless fallback.
- [x] 4.7 Add the unknown-board `New board` modal with Cancel, Connect as guest, and Save and connect actions, accessible focus handling, explicit no-save/no-tracking copy, and pointer/keyboard activation behavior.
- [x] 4.8 Connect known saved-board discovery directly to saved-mode LiveView and route unknown-board choices to the matching backend intent without duplicate connection requests; keep connection initiation out of board history.
- [x] 4.9 Update LiveView to render guest telemetry and a temporary/no-history notice, hide saved-board configuration/history actions for guests, and retain controller selection, retry, reconnect, and disconnect behavior.
- [x] 4.10 Reuse the Board Profile editor from board history and LiveView, preserving existing revision-checked saves, safe-area sheet behavior, and focus restoration.

## 5. Canonical conformance and frontend tests

- [x] 5.1 Add canonical saved-mode scenarios for known-board direct connect, save-and-connect, stable identity matching, durable session creation, location/session retention, board summaries, and board-filtered cursor continuation.
- [x] 5.2 Add canonical guest scenarios for unknown-board connect, live telemetry, controller selection, reconnect, WebView recreation, disconnect, and the absence of every durable side effect.
- [x] 5.3 Add canonical archive import merge/create, Forget board, active-connection rejection, Home-only connection, cancellation, stale cursor/filter, catalog revision, malformed intent, competing connection, and bounded request/error scenarios.
- [x] 5.4 Update frontend navigation, board history, undiscovered-board configuration, session detail return, archive import, new-board modal, Forget board confirmation, guest LiveView, pointer activation, and accessibility tests.
- [x] 5.5 Run focused frontend, mock, SQL, Apple, Android, and conformance checks for each completed slice and keep task progress synchronized.

## 6. Verification and handoff

- [x] 6.1 Review the change for guest data leakage, accidental Board Profile writes, location prompts/records, retry-mode changes, and global-session UI remnants.
- [x] 6.2 Run `just verify` and resolve all contract, native parity, frontend, SQL, and conformance failures.
- [x] 6.3 Mark completed tasks, sync the delta specs to the main specs, and archive the change only after the verified implementation is complete.
