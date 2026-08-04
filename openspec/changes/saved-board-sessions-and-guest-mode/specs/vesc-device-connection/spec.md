## MODIFIED Requirements

### Requirement: Saved identity precedes application readiness

After UART setup, the backend SHALL establish saved-device identity only for a connection explicitly chosen in saved mode, and only after a valid local VESC firmware response. In saved mode it SHALL derive or resolve the stable opaque `deviceHistoryId` from the meaningful board-level VESC identity when available, attach the local platform association to an existing matching imported board when possible, and load the Board Profile before choosing a controller route. Raw firmware identifiers SHALL remain native-private. It MUST NOT create a durable session until route, supported controller identity, Refloat compatibility, and telemetry setup reach application `ready`. A guest connection MAY reach `ready` without creating saved identity or loading a durable Board Profile, provided the same bounded identity and compatibility checks succeed.

#### Scenario: First valid saved VESC transport

- **WHEN** a first-time saved-mode BLE peer supplies a valid local VESC firmware response
- **THEN** the backend resolves or creates the saved board by stable opaque logical identity, attaches the local platform association, publishes `savedDeviceId`, loads the default or imported Board Profile, and continues identification without creating a session

#### Scenario: Saved peer matches imported board

- **WHEN** a saved-mode BLE peer produces a stable logical identity already present in an imported board entry
- **THEN** the backend reuses that board, attaches the local platform association, and does not create a duplicate board or split its session history

#### Scenario: Stable identity unavailable

- **WHEN** a valid saved-mode VESC response has no meaningful stable board identity
- **THEN** the backend creates an opaque random logical identity, does not claim an automatic cross-platform physical match, and continues with normal saved-board setup

#### Scenario: First valid guest VESC transport

- **WHEN** a first-time guest BLE peer supplies a valid local VESC firmware response
- **THEN** the backend continues identification without saving the peer as a board or creating a session

#### Scenario: Arbitrary UART peer

- **WHEN** a BLE peer exposes UART but never supplies a valid local VESC firmware response
- **THEN** the backend fails identification without saving it as a board or creating a session in either tracking mode

#### Scenario: Negotiation completes in saved mode

- **WHEN** controller and telemetry negotiation completes after saved identity is established
- **THEN** application state becomes `ready` and exactly one durable session is created or resumed according to lifecycle rules

#### Scenario: Negotiation completes in guest mode

- **WHEN** controller and telemetry negotiation completes for a guest connection
- **THEN** application state becomes `ready` with live telemetry available and no durable session or saved-board identity

### Requirement: Multiple-controller selection

When discovery finds more than one VESC motor controller, the backend SHALL pause before Refloat negotiation and publish at most 16 candidates sorted by CAN ID. Each candidate SHALL contain CAN ID, hardware/model name when available, firmware name when available, VESC version, and `vescSupported`. It MUST NOT expose firmware UUIDs, hide unsupported motor controllers, or silently select a candidate. In saved mode, an accepted choice SHALL update the Board Profile; in guest mode, it SHALL update only the active in-memory route.

#### Scenario: Ambiguous gateway

- **WHEN** discovery identifies two compatible VESC controllers
- **THEN** connection state becomes `selecting_controller`, both display-safe candidates are published, and no controller reaches telemetry polling until a selection is accepted

#### Scenario: Supported and unsupported controllers

- **WHEN** discovery identifies more than one motor controller and at least one VESC firmware version is unsupported
- **THEN** all controllers remain visible, unsupported candidates are marked and cannot be selected, and the backend does not silently choose a supported candidate

#### Scenario: Candidate overflow

- **WHEN** discovery finds a seventeenth VESC motor controller
- **THEN** the connection fails with `too_many_controllers`, publishes no truncated candidate list, and directs the user to configure a CAN ID when the connection is saved

#### Scenario: Valid saved selection

- **WHEN** the user selects a candidate from the current saved-mode discovery generation
- **THEN** the backend atomically merges its CAN ID into the latest Board Profile without overwriting other fields, advances the profile revision once, clears the candidates, resumes compatibility negotiation on that route, and can reach `ready`

#### Scenario: Valid guest selection

- **WHEN** the user selects a candidate from the current guest-mode discovery generation
- **THEN** the backend stores the route only for the active connection, clears the candidates, resumes compatibility negotiation, and writes no Board Profile or saved routing hint

#### Scenario: Stale or unknown selection

- **WHEN** a selection names an ID outside the current candidates or arrives after disconnect, reconnect, or another selection
- **THEN** the backend returns `invalid_state` without changing the active route or any persisted preference

#### Scenario: Malformed selection

- **WHEN** a selection contains a missing, fractional, negative, nonnumeric, or greater-than-253 CAN ID
- **THEN** the backend returns `invalid_request` without changing the active route or persisted preference

#### Scenario: Frontend recreation during selection

- **WHEN** the WebView is recreated while controller selection is required
- **THEN** `state.get` restores the same bounded candidates, tracking mode, and selection state while native discovery remains authoritative

### Requirement: Route-correct telemetry lifecycle

The backend SHALL serialize discovery and telemetry requests on the VESC stream, start polling only after the selected route and compatibility family are complete, and repeat route validation and layout negotiation after reconnect before resuming samples. Saved mode SHALL feed the durable session pipeline; guest mode SHALL feed only the bounded ephemeral live pipeline.

#### Scenario: Direct saved telemetry

- **WHEN** a supported saved-mode direct controller completes Refloat negotiation
- **THEN** direct polling produces persisted live samples and an automatic durable session

#### Scenario: Direct guest telemetry

- **WHEN** a supported guest-mode direct controller completes Refloat negotiation
- **THEN** direct polling produces live samples without a durable session, location track, or catalog update

#### Scenario: CAN saved telemetry

- **WHEN** a supported saved-mode CAN controller completes Refloat negotiation
- **THEN** polling remains forwarded to the selected CAN ID and samples are attributed to that saved board

#### Scenario: CAN guest telemetry

- **WHEN** a supported guest-mode CAN controller completes Refloat negotiation
- **THEN** polling remains forwarded to the selected CAN ID and samples remain ephemeral to the active guest connection

#### Scenario: Reconnect changes discovery generation

- **WHEN** BLE disconnects during identification or telemetry and later reconnects
- **THEN** stale responses from the old generation are ignored and the new generation validates its route before polling resumes with the retained tracking mode

#### Scenario: Reconnect reaches another controller

- **WHEN** reconnect cannot establish that the selected controller is the same controller that owns an active saved session
- **THEN** the backend closes that session as interrupted, fails with `controller_changed`, and appends no sample from the replacement controller; a guest reconnect uses the same bounded validation without creating history

### Requirement: Private learned controller identity

The backends SHALL parse and persist a meaningful VESC firmware UUID for reconnect validation only for saved connections. When public `preferredCanId` is null and exactly one supported controller is automatically discovered in saved mode, the backend SHALL persist its CAN ID and UUID as a private automatic hint. A public preference SHALL always take precedence. The UUID MUST NOT appear in frontend contract state, normal logs, exports, portable archives, controller-selection labels, or errors. A meaningful board-level firmware identity may feed the versioned one-way digest used for stable `deviceHistoryId`, but the raw value remains native-private and is never the archive identity. Guest connections SHALL never persist or update a learned hint or stable board identity.

#### Scenario: Remembered saved controller still matches

- **WHEN** a remembered CAN route reports the same meaningful controller UUID after reconnect in saved mode
- **THEN** the backend confirms that route without presenting a new ambiguity prompt

#### Scenario: Remembered controller changed

- **WHEN** a private automatic hint reports a different meaningful UUID and no active session requires continuity
- **THEN** the backend invalidates that hint and returns to bounded discovery rather than recording the replacement silently

#### Scenario: Same automatic controller moved

- **WHEN** an active saved-session reconnect cannot find the privately remembered CAN ID but bounded discovery finds the same meaningful UUID at another CAN ID
- **THEN** the backend updates the private hint, re-establishes telemetry on the new route, and resumes the same session without prompting

#### Scenario: Preferred controller changed

- **WHEN** a public preferred route reports a different meaningful UUID
- **THEN** the backend fails with `controller_changed` and does not fall back to discovery

#### Scenario: Public preference edited

- **WHEN** the user changes or clears `preferredCanId` through Board Profile
- **THEN** the backend invalidates the UUID associated with the old public route and stores a new meaningful UUID only after the replacement preferred route succeeds

#### Scenario: Guest controller route

- **WHEN** a guest connection discovers or selects a controller route
- **THEN** the route is held only for that connection generation and no UUID, CAN preference, or automatic hint is persisted

#### Scenario: UUID unavailable

- **WHEN** supported firmware omits a meaningful UUID
- **THEN** identification continues using route, hardware type, and supported firmware identity without fabricating or exposing a UUID

### Requirement: Recoverable connection failure

An asynchronous identification failure SHALL retain a failed active connection with a bounded stable `connectionIssue`, tracking mode, saved identity only when established in saved mode, and already verified route and firmware information. Polling and new session creation MUST remain stopped. `device.connect` for the same failed `deviceId` SHALL start a new generation using the latest Board Profile for saved mode or the retained guest intent for guest mode; a different active device SHALL remain invalid.

#### Scenario: Preferred CAN ID corrected

- **WHEN** a failed saved connection's Board Profile is updated and `device.connect` is requested again for the same device
- **THEN** the backend clears the old issue, starts a new bounded saved generation, and uses the updated preference

#### Scenario: Guest retry

- **WHEN** a failed guest connection is requested again for the same device
- **THEN** the backend clears the old issue, starts a new bounded guest generation, and does not create a saved board or read a persisted guest preference

#### Scenario: Different device while failed

- **WHEN** another device is requested while a failed active connection remains
- **THEN** the backend returns `invalid_state` and preserves the failed connection

#### Scenario: Disconnect from failure or selection

- **WHEN** the user disconnects while identification, controller selection, or failed state is active
- **THEN** the backend cancels that generation and removes the active connection through normal disconnect state changes without persisting guest state
