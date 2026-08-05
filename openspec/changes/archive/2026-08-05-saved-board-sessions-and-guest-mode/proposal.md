## Why

Float View currently turns every successful board connection into a saved board and a durable session. That makes one-off connections unexpectedly persistent, while the separate Boards tab keeps board identity, summaries, and ride history apart. This change makes saving an explicit choice for new boards and makes Sessions the single place for saved-board history.

## What Changes

- **BREAKING** Replace the four primary destinations with `connect`, `sessions`, and `settings`; remove the top-level Boards tab and global board list route.
- Show saved boards in Sessions. Selecting a board opens a board-history route with its overall ride summary and a date-grouped list of only that board's sessions.
- Keep board configuration available from a board-history menu as a configuration sheet, and retain the existing LiveView configuration sheet for connected saved boards.
- Add explicit saved-versus-guest connection intent to the frontend contract and native connection lifecycle.
- Connect known saved boards directly to LiveView without an additional choice.
- When an unknown board is selected, show a New board modal offering cancel, guest connection, or save-and-connect. Guest connections show live telemetry but create no saved board, session, ride history, location track, or durable tracking data.
- Save a new board only when the user chooses save-and-connect; continue to resolve its identity only after valid VESC identification.
- Allow guest connections to complete controller selection and reconnect without persisting Board Profile changes or learned routing hints.
- Use a stable opaque board identity derived from the meaningful local VESC firmware identity for cross-platform session export/import, while retaining platform Bluetooth identifiers only as local discovery associations and keeping raw firmware UUIDs private.
- Import sessions into the matching saved board by stable identity, or create a configurable saved-board entry when no matching board exists. Boards remain configurable even when they are not currently discovered; connection starts only from a Home scan result.
- Add `Forget board` to the board-history header menu with destructive confirmation; forgetting removes the board, its profile/routing data, and all saved sessions irreversibly, and is rejected during an active connection.
- Add bounded per-board summary data without maximum speed or active/in-progress counts and expose only the board-filtered native session-catalog operation needed for board-history pagination.
- Use the existing Refloat current-ride distance field for optional guest display when available; do not fetch, calculate, persist, or display lifetime odometer data in guest mode.
- Update the shared frontend, mock backend, Apple backend, Android backend, SQL projections, archive import/export, and canonical conformance suite together.
- Remove superseded global session-list and Boards-tab presentation paths while preserving standalone session review and board configuration behavior.

### Non-goals

- Guest sessions will not be retroactively converted into saved history.
- No account, cloud sync, cross-device board transfer, or multi-board simultaneous connection support.
- No new historical telemetry format or separate aggregate database; board summaries remain derived from retained session catalog rows.
- No global “All sessions” view or second Sessions sub-tab.
- No unfiltered/global backend session-catalog query; session listing is board-scoped.

## Capabilities

### New Capabilities

- `saved-board-session-history`: Saved-board index, per-board summaries, board-scoped date-grouped session history, and board-detail configuration access.

### Modified Capabilities

- `app-navigation`: Remove the Boards primary destination and make saved-board history subordinate to Sessions.
- `backend-contract`: Add saved/guest connection intent, guest-aware active connection state, discovered saved identity, board summaries, and board-scoped session listing.
- `vesc-device-connection`: Define explicit saved versus guest lifecycle, including non-persistent guest identification, controller selection, reconnect, and disconnect behavior.
- `live-board-dashboard`: Keep LiveView usable in guest mode, clearly identify the non-persistent mode, and restrict board configuration to saved connections.
- `platform-hosts`: Mirror guest lifecycle gates and saved-board/session persistence behavior on Apple and Android.
- `backend-conformance`: Add saved-board choice, guest isolation, board summaries, board-scoped pagination, and navigation-visible contract coverage.

## Impact

The change affects `frontend/src/lib/backend/contract.ts`, frontend navigation and Sessions/Boards/connection features, the mock backend, both native `ApplicationService` implementations, the loopback payloads, SQLite catalog projections, archive import/export, and conformance scenarios/targets. Guest mode changes the cross-platform connection and telemetry lifecycle: native services must publish bounded ephemeral live samples while skipping durable board, session, location, profile, and routing writes. Saved-board identity and local platform association become distinct so imported sessions can be merged across platforms. Existing disposable storage remains recreate-on-version-mismatch with no migration.
