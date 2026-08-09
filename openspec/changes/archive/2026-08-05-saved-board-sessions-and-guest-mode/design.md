## Context

Float View currently has four primary frontend destinations. Sessions is a global date-grouped session browser, while Boards is a separate saved-device configuration list. The native connection lifecycle currently creates a saved-device record while identifying a valid VESC and creates a provisional durable session when the connection reaches `ready`. The same lifecycle is mirrored by the mock, Apple, and Android hosts.

The requested product model has two deliberate connection outcomes:

```text
known saved board ───────────────▶ saved connection ─▶ live view + durable session
unknown board ─▶ user choice ────▶ guest connection ─▶ live view only
                         └───────▶ saved connection ─▶ save board + durable session
```

The frontend contract is hand-written and complete snapshots are authoritative. Native services own Bluetooth, session persistence, location capture, learned controller routing, archive import/export, and lifecycle. The existing SQLite catalog already stores `device_history_id` on each session and the native session query already supports that filter internally, but the wire-level `session.list` method currently exposes only the global query. A saved board therefore needs two identity layers: a stable opaque logical identity that can travel in a session archive, and an optional local platform association used to rediscover it on Apple or Android.

## Goals / Non-Goals

**Goals:**

- Make saving a newly discovered board an explicit user decision.
- Keep guest connections fully live, including controller selection and bounded reconnect behavior, without durable board or ride history.
- Give each saved board the canonical VESC firmware UUID as logical identity when meaningful for archive import/export, while allowing a board to exist without a local Bluetooth association.
- Make Sessions a saved-board index with board-specific overall summaries.
- Show each board's sessions in its own detail route, grouped by date, with no global “All sessions” list or sub-tab.
- Open the existing Board Profile editor from a board-detail menu as a configuration sheet, while retaining the LiveView configuration sheet for saved active boards.
- Keep the contract, mock, Apple backend, Android backend, SQL projections, frontend, and conformance suite aligned.
- Preserve the repository's bounded message, collection, cursor, persistence, and privacy rules.

**Non-Goals:**

- Guest history migration or a later “save this guest ride” operation.
- Accounts, cloud synchronization, board transfer, or multiple simultaneous boards.
- A second aggregate database, materialized history table, or schema migration.
- A global session browser, session grouping across boards, or a new session sub-tab.
- Changing the VESC protocol, telemetry fields, or historical session-data API beyond board selection and return navigation.
- Cloud synchronization or automatic physical-board matching when the board has no meaningful stable VESC identity; such boards retain their imported identity but require an explicit saved connection to bind a local association.

## Decisions

### 1. Carry an explicit tracking mode through the connection contract

Add a platform-neutral `saved`/`guest` tracking mode to the `device.connect` request and to `ActiveConnection`. The frontend supplies `saved` for a known saved board and after the user chooses “Save and connect”; it supplies `guest` after the user chooses “Connect as guest”. A retry uses the mode already attached to the failed active connection.

The backend treats this mode as a lifecycle policy, not a presentation hint. `saved` permits saved-device establishment, Board Profile use, learned routing hints, session creation, telemetry persistence, and location capture. `guest` forbids all of those durable operations while permitting identification, controller selection, telemetry polling, live state publication, reconnect, and disconnect.

Alternative considered: connect first and ask whether to save after firmware identification. Rejected because it creates a partially connected pre-choice state, complicates cancellation, and makes it easier to accidentally create a session before the user's decision. The modal is shown from discovery before `device.connect` is sent.

### 2. Separate stable board identity from local discovery association

Every saved board SHALL have one stable logical identity used by session archives and one optional local platform association. For a board with a meaningful local VESC endpoint firmware identity, native code uses the canonical 24-character lowercase UUID directly as that identity. If no meaningful firmware identity is available, the board receives an opaque random logical identity and cannot be automatically matched to the same physical board on another installation. The stable identity is not a secret or an authorization key.

The archive manifest carries the canonical VESC firmware UUID as stable logical board identity when meaningful, canonical controller/BMS UUID metadata where available, and the board snapshot. It does not carry an Apple CoreBluetooth UUID or Android address. Import uses the VESC identity to merge sessions into an existing saved board when present; otherwise it creates a saved-board entry with the imported name/profile and no local platform association. Such an imported board remains visible and configurable in Sessions even when it is not currently discoverable. Archives without a stable identity are rejected rather than guessed into a board.

Discovered `Device` values gain a nullable saved-device reference or equivalent local-association marker. The native loopback payload derives it from the local platform association; the frontend never compares Apple UUIDs or Android addresses itself. Home scanning is the only surface that starts a connection. A saved board with no currently discovered association remains configurable but has no connection action from its history route. An unknown advertisement remains an ordinary Home discovery result and is not written to the catalog merely because it was scanned. When a saved connection produces a valid stable logical identity, native code may attach the local association to the matching imported board instead of creating a duplicate.

This preserves the privacy boundary around platform identifiers while allowing known locally associated boards to bypass the New board modal. A saved board's display name remains resolved by the existing custom-name, advertised-name, and stable fallback order; the absence of a local platform identifier is not an error.

### 3. Gate every durable side effect from one native tracking policy

Application services retain one connection-wide tracking mode for the active generation. The policy is checked at each existing durable boundary:

- during firmware identification, only `saved` mode calls `ensureSavedDevice` and loads its Board Profile or private routing hint;
- during controller selection, `saved` mode persists the preferred CAN choice and learned UUID hint, while `guest` mode keeps the selected route only in memory;
- at `ready`, only `saved` mode calls `logStore.create` and starts a durable session;
- during telemetry ingestion, `saved` mode appends selected samples and publishes the resulting durable session projection, while `guest` mode publishes a bounded ephemeral latest sample at the same live cadence without opening a session database;
- location capture requires both the existing opt-in/authorization conditions and a saved active session;
- disconnect and reconnect close, recover, or retain sessions only when a saved session exists.

The polling pipeline remains one shared telemetry owner. Guest mode does not introduce a second protocol or logging implementation; it changes the persistence decision around the existing decoded sample.

### 4. Keep guest connections capable of the complete live connection flow

Guest mode must not be treated as a reduced transport. It may reach identifying, selecting-controller, ready, reconnecting, failed, and disconnected states. An ambiguous controller selection is resolved for the current connection without a Board Profile write. A reconnect retains the guest policy and never silently turns into a saved session.

The live dashboard identifies guest mode and renders live samples normally. Session-only aggregates render as unavailable, board configuration is unavailable, and no location-recording action starts native capture for a guest connection. When Refloat supplies its current-ride `distanceM` value, guest LiveView may show it as transient telemetry; the lifetime `odometerM` is not used for summaries or tracking, and no distance is persisted in guest mode.

### 5. Use the existing session catalog for board summaries and board-scoped history

Expose a bounded summary projection with each saved board, derived from retained `session_catalog` rows. The first version includes session count, total riding time, total distance when available, and the most recent session time. It deliberately excludes maximum speed and active/in-progress counts. Empty boards use zero counts and null unavailable metrics rather than fabricated telemetry values.

Make the wire-level `session.list` request require a saved-board identifier. The native service resolves it to the board's `device_history_id` and reuses the existing keyset pagination. Cursor context includes the board filter, query shape, catalog revision, and last `(startedAt, sessionId)` key so a cursor cannot be reused for another board. Remove the unused unfiltered/global session-list query and its backend callers; session detail, export/import, deletion, and pruning remain available through their existing board/session ownership.

Alternative considered: load every session globally in the frontend and calculate board summaries there. Rejected because summaries would be incomplete until pagination finished and large histories would make the Sessions index expensive. Alternative considered: add a separate board-summary database table. Rejected because the catalog is already the source of truth and the repository intentionally recreates disposable schemas instead of migrating them.

### 6. Make Sessions a board index and keep session review subordinate to its board

The primary Sessions route renders saved boards only. A board row/card presents its resolved name and bounded overall summary and navigates to a subordinate board-history route. That route renders the summary, a menu item that opens the existing Board Profile editor in a bottom sheet, and a session list filtered to that board and grouped by local calendar date.

Session detail opened from a board-history route returns to that board after review, deletion, or a missing-session result. Imported or otherwise boardless sessions retain a Sessions fallback. No global session list is rendered.

The top-level Boards route and tab are removed. The reusable Board Profile editor remains owned by `features/boards`; route and sheet wrappers own only navigation and overlay mechanics. The history header menu also offers `Forget board`. Its destructive confirmation explains that all saved sessions for that board will be removed and the action cannot be undone. The action is hidden while that board has an active connection and the backend rejects it with `invalid_state` if a race still reaches the service.

Session archive import assigns each imported session to the board identified by the archive's stable logical identity. If that board is already saved, the session appears in its existing history; otherwise import creates the board entry first, including zero-session board configuration support. A board created by import without a local platform association can be renamed and configured but can only become connectable after Home discovers and explicitly saves a matching board.

### 7. Use a focused three-choice New board modal

Home opens an accessible modal only when the selected discovered device has no saved-device reference. The modal names the board using its bounded advertised display name and explains that guest mode does not save or track the ride. It provides:

- `Cancel`: close without sending a connection request;
- `Connect as guest`: send `device.connect` with guest mode;
- `Save and connect`: send `device.connect` with saved mode.

The modal uses existing focus, Escape, backdrop, Pointer Events, keyboard, and assistive-technology activation patterns. A saved discovery bypasses the modal and sends a saved connection request directly.

### 8. Derive board history presentation from authoritative snapshots and bounded requests

The frontend keeps saved boards and their summaries in authoritative `AppState`. Board-history session pages are loaded through the existing `BackendState` client and are reset when `sessionCatalogRevision` changes. Every board page owns its own cursor and date grouping; no cross-board session cache or grouping state is added.

The board-detail configuration sheet uses the same revision-checked `Board Profile` editor as LiveView. Closing the sheet flushes pending edits before dismissal. A guest connection has no saved-device ID and therefore cannot open either configuration surface.

### 9. Test the choice and isolation boundaries in the canonical suite

Conformance adds scenarios for cancel-before-connect, unknown-board guest connection, unknown-board save-and-connect, saved-board direct connection, guest controller selection, guest reconnect/retry, guest disconnect, and guest isolation from saved-device/session/location/catalog state. It also covers stable archive identity, import merge/create behavior, configurable boards without discovery, Home-only connection initiation, board summary values, empty boards, board-scoped pagination, cursor binding, Forget board cleanup and active-connection rejection, session deletion updates, and board-detail return navigation in the mock and native targets.

The fixture remains a pure VESC byte bridge. Scenario logic, connection intent, session assertions, and summary expectations stay in `conformance/` and the shared mock.

## Risks / Trade-offs

- [A guest sample path could accidentally be treated as persisted history] → Keep durable append, session creation, location capture, and learned-hint writes behind one explicit native tracking policy; assert no catalog or storage change in conformance.
- [A frontend could send the wrong mode on a retry] → Store the mode on the active connection and require same-device retries to use that authoritative mode.
- [The New board modal could race discovery expiry] → Keep the bounded discovered device in the native generation until connection/cancellation completes and reject stale device IDs with the existing stable error.
- [Derived summaries add catalog work to state construction] → Use one bounded grouped query for the saved-device projection, cap saved boards, and do not include session rows in complete state.
- [Board-filtered cursors could be replayed for another board] → Bind cursor payloads to the filter, query version, catalog revision, and last key; reject mismatched or stale cursors.
- [Archive import could create duplicate boards or expose a raw hardware fingerprint] → Carry only the versioned opaque logical identity, merge by that identity, keep local platform association nullable, and make missing-identity imports explicit rather than guessing.
- [Forget board could remove the wrong data or race an active connection] → Resolve deletion by the saved board's history identity, delete its profile/routing metadata and all owned sessions in one native operation, reject active-board deletion, and update catalog state only after cleanup.
- [Guest mode could expose settings or location prompts that imply persistence] → Hide board configuration and suppress location capture/action availability when the active connection is guest.
- [Removing the global session list may make imported sessions less discoverable] → Keep imported sessions attached to their stable board identity and make those board entries visible even with zero sessions or no local discovery association.
- [The active BMS telemetry change touches the same contract and live-dashboard areas] → Keep this change's requirements additive and reconcile its contract/live behavior with `support-vesc-bms-telemetry` before implementation or archive.

## Migration Plan

1. Commit the proposal, design, delta specs, and tasks before implementation, as required by the repository workflow.
2. Extend the TypeScript contract, mock, conformance client, and frontend state/navigation seams with tracking mode, stable archive identity, nullable local association, summaries, required board-filtered session requests, import association, and Forget board semantics.
3. Implement the mirrored Apple and Android tracking policy, stable identity derivation, loopback payloads, guest ephemeral sample path, saved-only persistence gates, catalog summary projection, archive import merge/create behavior, and one-operation board deletion.
4. Replace the four-tab/session UI with the three-tab saved-board index, board-history route, board menu sheet, New board modal, and destructive Forget board confirmation. Preserve and adapt the existing Board Profile editor and session detail return behavior.
5. Remove the unused global session-list backend query and callers, then add conformance and focused frontend/native tests before running `just verify` and the explicit physical BLE checkpoint where applicable.

No data migration is planned. Existing local `deviceHistoryId` values remain valid opaque identities, while newly created boards use the versioned digest when a meaningful VESC identity exists. If the catalog or session schema changes, bump the existing disposable storage version so incompatible test data is recreated. Rollback is a source rollback plus disposable storage recreation.

## Open Questions

None. The saved/guest choice, board-index Sessions layout, per-board date grouping, board configuration menu, and persistence boundary are agreed.
