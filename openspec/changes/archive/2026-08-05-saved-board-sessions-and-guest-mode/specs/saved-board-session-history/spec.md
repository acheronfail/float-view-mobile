# Saved Board Session History Specification

## Purpose

Define the saved-board-first session experience, board history route, board summary, and explicit save-or-guest connection choice.

## ADDED Requirements

### Requirement: Sessions is the saved-board index

The Sessions route SHALL show only the user's saved boards, using bounded board identity and derived history summary data. It MUST NOT show a global `All sessions` list, a second session sub-tab, or an unscoped date-grouped session feed. An empty saved-board collection SHALL explain that saving a board is required for session history.

#### Scenario: Saved boards are listed

- **WHEN** the user opens Sessions
- **THEN** the route renders each saved board with its resolved name and bounded summary, and no standalone global session list

#### Scenario: No saved boards

- **WHEN** the user opens Sessions without saved boards
- **THEN** the route renders an empty state that explains guest rides are not retained and offers the normal connection flow

#### Scenario: Board summary remains bounded

- **WHEN** a saved board has history
- **THEN** its index summary contains session count, total riding time, total distance when available, and most recent session time, with no maximum-speed or active/in-progress count

### Requirement: Saved boards remain available without discovery

The Sessions saved-board index and each board-history route SHALL remain usable when a saved board has no currently discovered local Bluetooth association. The board may be renamed and configured from its history route, but only Home discovery SHALL initiate a connection. A board-history route SHALL NOT offer a connect action for an undiscovered board.

#### Scenario: Configure an undiscovered board

- **WHEN** a saved or imported board is not present in the current Home scan
- **THEN** it remains visible in Sessions and its Board Profile can be opened and edited without starting a connection

#### Scenario: Connection starts from Home

- **WHEN** the user wants to connect to a saved board
- **THEN** the user uses a discovered Home result and the backend receives the connection intent from the Home flow, rather than from the board-history route

#### Scenario: Empty imported board

- **WHEN** an imported board has no retained sessions and no local discovery association
- **THEN** it still appears in Sessions with an empty summary and remains configurable

### Requirement: Saved board history route

Selecting a saved board from Sessions SHALL open a subordinate board-history route. The route SHALL show the selected board's overall summary and its sessions filtered to that board, grouped by the user's local calendar date. The grouping SHALL be a presentation of the bounded board-scoped session pages and SHALL NOT create a second persisted grouping model.

#### Scenario: Open Board A history

- **WHEN** the user selects `Board A` in Sessions
- **THEN** the board route shows `Board A`'s overall summary followed by only `Board A` sessions grouped under local dates

#### Scenario: Continue board history

- **WHEN** more sessions exist for the selected board than fit in one page
- **THEN** the route requests the next bounded board-scoped page and appends it without duplicating sessions or changing date-group order

#### Scenario: Other boards are excluded

- **WHEN** a session belongs to another saved board
- **THEN** it is absent from the selected board's history route and summary

#### Scenario: Session detail return

- **WHEN** the user opens a session from a saved board's history and then returns
- **THEN** the detail view returns to that board's history route with the board selection and loaded page context intact when possible

### Requirement: Board history configuration menu

The board-history route SHALL provide one accessible menu item that opens the existing Board Profile configuration sheet for that saved board. The sheet SHALL reuse the same editor and revision-checked save behavior as the LiveView configuration sheet. Board configuration SHALL remain a subordinate action and SHALL NOT reintroduce a Boards primary tab.

#### Scenario: Configure Board A from history

- **WHEN** the user opens the board menu and selects configuration
- **THEN** the Board Profile sheet opens for `Board A` with its current authoritative values

#### Scenario: Configuration dismissal

- **WHEN** the user saves or dismisses the configuration sheet
- **THEN** the user returns to the same board-history route and any changed name or profile value is reflected in the board header and subsequent connection flow

#### Scenario: Missing board during configuration

- **WHEN** the board is deleted or becomes unavailable before the sheet loads
- **THEN** the sheet closes, the route reports the bounded not-found state, and the user returns to Sessions without reconnecting the backend

### Requirement: Forget board removes owned history

The board-history header menu SHALL provide a `Forget board` item for an inactive saved board. Selecting it SHALL show a destructive confirmation that says the board and all saved sessions for that board will be removed and the action cannot be undone. The backend SHALL remove the saved board, its Board Profile and private routing metadata, and every session owned by that board's history identity as one irreversible operation. The frontend SHALL return to Sessions after accepted deletion.

#### Scenario: Confirm Forget board

- **WHEN** the user confirms `Forget board` for `Board A`
- **THEN** the backend removes `Board A` and all of its saved sessions, the board disappears from Sessions, and no other board or session is changed

#### Scenario: Cancel Forget board

- **WHEN** the user cancels the destructive confirmation
- **THEN** the board and all of its saved sessions remain unchanged

#### Scenario: Active board cannot be forgotten

- **WHEN** `Board A` has an active connection
- **THEN** the Forget board menu item is unavailable and a direct or raced delete request fails with `invalid_state` without removing data

### Requirement: Session archive import preserves board ownership

Session archive export SHALL include the stable opaque board identity and the bounded board snapshot required for cross-platform import, but MUST NOT include raw VESC firmware identifiers, Apple or Android platform identifiers, or private controller-route UUIDs. Import SHALL associate every session with the board represented by that identity. When a matching saved board exists, imported sessions SHALL be added to that board's history; otherwise import SHALL create a saved-board entry with the imported board snapshot and no local platform association before adding the sessions.

#### Scenario: Import into an existing board

- **WHEN** an archive's stable board identity matches a saved board
- **THEN** imported sessions appear in that board's date-grouped history without creating a duplicate board

#### Scenario: Import creates a board entry

- **WHEN** an archive's stable board identity has no matching saved board
- **THEN** import creates a configurable board entry and places the imported sessions in its history even though the board is not locally discoverable

#### Scenario: Import without stable identity

- **WHEN** an archive has no valid stable board identity
- **THEN** import fails without guessing a board or partially adding sessions

#### Scenario: Import does not expose raw identity

- **WHEN** a session archive is exported or imported
- **THEN** the portable archive contains only the stable opaque board identity and no raw hardware or platform identifier

### Requirement: Known and unknown board connection choices

The connection flow SHALL distinguish an exact match to a saved board from an unknown board. A known saved board SHALL connect directly to LiveView using saved-board tracking. An unknown board SHALL first present a modal titled `New board` with exactly the choices Cancel, Connect as guest, and Save and connect, and SHALL explain that guest mode does not save or track rides.

#### Scenario: Known saved board

- **WHEN** the user selects a discovered board whose local association matches a saved board
- **THEN** the backend is asked to connect in saved mode and the frontend opens LiveView without a save prompt

#### Scenario: Unknown board prompt

- **WHEN** the user selects a discovered board with no saved-board match
- **THEN** the frontend opens the `New board` modal and does not start connection until the user chooses an action

#### Scenario: Cancel unknown board

- **WHEN** the user selects Cancel in the `New board` modal
- **THEN** the modal closes, no board is saved, no connection is started, and discovery remains usable

#### Scenario: Guest connection

- **WHEN** the user selects Connect as guest
- **THEN** the backend connects in guest mode, LiveView displays live telemetry including current-ride distance when Refloat supplies it, and no board, session, ride sample, location, controller preference, or learned routing hint is durably written

#### Scenario: Save and connect

- **WHEN** the user selects Save and connect
- **THEN** the backend creates the saved-board identity before application readiness, connects in saved mode, and enables the existing durable session and ride logging behavior

#### Scenario: Save binds an imported board

- **WHEN** a save-and-connect operation produces a stable logical identity that already belongs to an imported saved board
- **THEN** the backend attaches the local discovery association to that board and does not create a duplicate board or split its session history

### Requirement: Guest live flow is fully usable

Guest mode SHALL retain the normal bounded connection lifecycle, including firmware identification, direct-or-CAN routing, controller selection, telemetry polling, reconnect, and explicit disconnect. Guest mode SHALL clearly communicate that the live data is temporary and SHALL not expose Board Profile editing or saved-board history actions.

#### Scenario: Guest controller selection

- **WHEN** guest discovery finds multiple controllers
- **THEN** LiveView presents the same bounded controller choice and the selected route is held only for the active connection

#### Scenario: Guest reconnect

- **WHEN** a guest connection temporarily disconnects
- **THEN** the backend retries the guest generation without creating a saved board or session, and LiveView recovers the guest state

#### Scenario: Guest disconnect

- **WHEN** the user disconnects a guest connection
- **THEN** the active connection is removed and no guest history appears in Sessions

### Requirement: Board history uses one durable source

Board summaries and board-scoped session lists SHALL be derived from the existing saved-device/session catalog and SHALL use bounded cursor context. The implementation MUST NOT add a separate aggregate table, global history cache, or platform-specific frontend data source. Catalog or summary invalidation SHALL refresh the affected board index and route without requiring a full application reload.

#### Scenario: Board catalog update

- **WHEN** a saved session is completed, deleted, or pruned
- **THEN** the relevant board summary and board-history list update from authoritative catalog state

#### Scenario: Stale board page

- **WHEN** a board-history cursor no longer matches the catalog revision or board filter
- **THEN** the backend returns the standard bounded conflict and the frontend restarts that board's first page
