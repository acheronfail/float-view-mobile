## MODIFIED Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement the WebSocket control methods `state.get`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `controller.shutdown`, `refloat.control.set`, `refloat.fatalError.clear`, `savedDevice.update`, `savedDevice.delete`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.list`, `session.get`, `session.export`, `session.delete`, `session.delete.cancel`, `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`, plus the authenticated session-data request kinds `series`, `samples`, and `events`. Session-group methods SHALL NOT remain; `session.getDiagnostics` SHALL NOT remain; exact retained historical reads use `samples`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory control method and session-data request kind is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method or session-data kind outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

### Requirement: Authoritative application state

`state.get` SHALL return a bounded `AppState` containing a monotonic revision, a separate monotonic `sessionCatalogRevision`, Bluetooth state, discovered devices with an explicit saved-board match when known, saved devices with canonical VESC UUID identity when meaningful, resolved identity, and optional local platform association, active connection with `trackingMode` (`saved` or `guest`) and nullable saved-device identity, connection state, detected direct-or-CAN controller route, bounded controller candidates with canonical UUIDs when selection is required, bounded connection issue when failed, firmware UUID when meaningful, complete live-only Refloat control state when protocol discovery has begun, complete bounded controller-action state, saved-mode active-session summary when present, the latest successfully persisted sample for saved mode or bounded ephemeral latest sample for guest mode when known, and settings. Recent session summaries and session groups SHALL NOT be included; standalone or board-filtered session summaries SHALL be retrieved through `session.list`. Historical series and diagnostic windows MUST NOT be included.

#### Scenario: Initial state

- **WHEN** the frontend requests `state.get` after connecting
- **THEN** it receives all bounded state needed to render the main application, including saved-board matches, tracking mode, and current Refloat control availability when active, with no session-summary or session-group collection

#### Scenario: Guest state

- **WHEN** a guest connection publishes a live sample
- **THEN** the authoritative snapshot may expose that bounded ephemeral sample and live-only Refloat controls for LiveView while active, but exposes no saved-device identity, active durable session, or catalog entry for it

#### Scenario: Catalog revision is independent

- **WHEN** a persisted saved-mode telemetry sample or confirmed Refloat control publishes a newer complete state snapshot without changing session catalog membership or summary visibility
- **THEN** the global state revision advances while `sessionCatalogRevision` remains unchanged

#### Scenario: Recreated frontend during reconnect

- **WHEN** a disposable frontend reconnects while a saved or guest board is reconnecting
- **THEN** initial authoritative state contains the current tracking mode, connection state, detected route when known, pending candidates or connection issue when applicable, the latest permitted sample when one exists, Refloat controls in a non-actionable loading state when discovery had begun, and the current session catalog revision

## ADDED Requirements

### Requirement: Bounded Refloat control contract

`ActiveConnection.refloatControls` SHALL be null before Refloat discovery begins and otherwise SHALL contain `status` as `loading`, `ready`, or `unavailable`; `writable`; nullable `packageEnabled` and `flywheelEnabled`; `lightingAvailable`; nullable `ledsEnabled` and `headlightsEnabled`; `fatalErrorClearAvailable`; nullable `fatalErrorLatched`, `fatalErrorActive`, and `fatalErrorClearable`; and nullable `pendingAction` from `package`, `flywheel`, `leds`, `headlights`, or `clearFatalError`. `refloat.control.set` SHALL accept exactly one `control` from `package`, `flywheel`, `leds`, or `headlights` and one boolean `enabled`. `refloat.fatalError.clear` SHALL accept empty parameters. A successful method SHALL return the complete confirmed `RefloatControls`; unsafe, read-only, stale, unavailable, or competing requests SHALL use the existing stable error vocabulary and MUST NOT return optimistic partial state.

#### Scenario: Idempotent requested value

- **WHEN** `refloat.control.set` requests a value already confirmed by current authoritative state
- **THEN** the backend returns the unchanged complete controls without sending a board mutation

#### Scenario: Confirmed control result

- **WHEN** an accepted control is confirmed by its protocol response or later telemetry
- **THEN** the request returns complete controls matching the newer `state.changed` snapshot and the pending action is null

#### Scenario: Invalid control name

- **WHEN** `refloat.control.set` supplies an unknown control or malformed parameter
- **THEN** the backend returns `invalid_request` without sending a board command

#### Scenario: Control unavailable

- **WHEN** a control is requested before controls are ready, during reconnect, for absent lighting or fatal capability, on read-only firmware, or while flywheel mode prohibits that mutation
- **THEN** the backend returns `invalid_state` without changing board or authoritative control state

#### Scenario: Flywheel disable remains actionable

- **WHEN** authoritative state reports active flywheel mode and the user requests flywheel disabled, including while its package run state is normalized as riding
- **THEN** the backend may perform only the exact source-defined flywheel-disable operation while every other mutating control remains unavailable

#### Scenario: Confirmation timeout

- **WHEN** an accepted action cannot obtain its required bounded response or confirmation
- **THEN** the backend returns `unavailable`, clears `pendingAction`, and retains the last confirmed values

### Requirement: Bounded controller shutdown contract

`ActiveConnection.controllerActions` SHALL contain boolean `shutdownAvailable` and nullable `pendingAction`, whose only non-null value is `shutdown`. `controller.shutdown` SHALL accept empty parameters and return the existing `Accepted` result after the exact one-way request is accepted for transmission. `shutdownAvailable` SHALL be true only while the current selected route is ready, writable, on an audited stable VESC shutdown profile, authoritatively idle, outside flywheel mode, and free of another pending mutation. The result and state MUST NOT claim that controller power-off was confirmed.

#### Scenario: Shutdown accepted for transmission

- **WHEN** an eligible idle connection requests `controller.shutdown`
- **THEN** the backend publishes shutdown-pending state, submits exactly one non-forced non-restart request, and returns `Accepted` without fabricating a device acknowledgement

#### Scenario: Unsupported controller remains connected

- **WHEN** the accepted shutdown request is ignored by controller hardware and the selected route remains available through the bounded intent window
- **THEN** the backend clears shutdown-pending state, continues the active connection, and does not report confirmed power-off

#### Scenario: Expected loss after shutdown

- **WHEN** the selected controller or BLE transport disappears while current shutdown intent is pending
- **THEN** the backend ends the active connection as user initiated and suppresses automatic reconnection

#### Scenario: Shutdown unavailable

- **WHEN** shutdown is requested on VESC 6.02, an unaudited or read-only firmware build, a non-ready or stale connection, a non-idle board, flywheel mode, or while another mutation is pending
- **THEN** the backend returns `invalid_state`, sends no command, and preserves authoritative state apart from any bounded error
