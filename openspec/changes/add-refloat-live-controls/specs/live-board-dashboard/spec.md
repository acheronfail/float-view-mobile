## MODIFIED Requirements

### Requirement: Extensible live action menu

The live header SHALL provide one accessible menu trigger whose menu contains the current view switch, stateful Refloat package and flywheel actions, lighting actions only when lighting capability is present, Clear fatal error only when a Refloat 1.2.x fatal latch remains after its active firmware fault has ended, Shut down controller when the VESC core profile supports the command, Configure board when saved identity exists, and Stop and disconnect. Each Refloat item SHALL display its last confirmed on/off state, SHALL be disabled unless its native preconditions are currently satisfied, and SHALL remain unavailable while its action is pending. Shutdown SHALL be unavailable unless authoritative controller-action state permits it. While authoritative state reports flywheel enabled, a non-dismissible full-application modal overlay SHALL replace access to normal application interaction and SHALL expose exactly one action, Disable, which requests the safety-gated flywheel-off transition. Disabling Refloat, enabling flywheel, and shutting down the controller SHALL require explicit contextual confirmation before the backend request is sent. Every menu, confirmation, and flywheel-overlay control SHALL retain native button semantics, use primary Pointer Events with cancellation and a guarded keyboard/assistive click path, avoid double activation, and manage focus within the modal surface.

#### Scenario: Open menu

- **WHEN** the user activates the live header menu trigger on a ready writable connection
- **THEN** a keyboard- and pointer-operable menu presents the applicable Refloat controls with confirmed state alongside the current view, configuration, and disconnect actions

#### Scenario: Saved identity pending

- **WHEN** an active connection has not yet produced a saved-device identity
- **THEN** Configure board is unavailable while applicable live-only Refloat controls, connection state, and disconnect remain usable

#### Scenario: Lighting unavailable

- **WHEN** Refloat INFO reports no lighting capability
- **THEN** LED and headlight actions are absent and no lighting request is issued by the frontend

#### Scenario: Read-only or reconnecting connection

- **WHEN** firmware is read-only or the active connection is not ready
- **THEN** Refloat mutation actions are visibly unavailable while current retained state may remain displayed and disconnect remains usable

#### Scenario: Confirm package disablement

- **WHEN** the user chooses to disable an idle enabled Refloat package
- **THEN** an explicit warning explains that balancing will be disabled and sends the request only after confirmation

#### Scenario: Confirm flywheel engagement

- **WHEN** the user chooses to enable flywheel from an eligible idle normal state
- **THEN** an explicit warning requires the board to be disengaged, nose-up, and balanced before confirmation sends the request

#### Scenario: Flywheel mode actions

- **WHEN** authoritative state reports active flywheel mode
- **THEN** a non-dismissible modal overlay blocks the normal application, explains that the board must be disengaged, and presents a single **Disable** button that can issue only the flywheel-disable request

#### Scenario: Flywheel disable remains pending or fails

- **WHEN** Disable is activated while its mutation is pending or the exact command cannot be transmitted or confirmed
- **THEN** no other application action becomes available and the overlay remains until fresh authoritative telemetry reports flywheel disabled

#### Scenario: Clearable fatal latch

- **WHEN** authoritative state reports a latched Refloat fatal error and no active firmware fault
- **THEN** the menu presents Clear fatal error and removes it after the confirmed clear publishes newer state

#### Scenario: Primary and keyboard activation

- **WHEN** a Refloat action is activated by primary pointer or keyboard/assistive click
- **THEN** exactly one backend request is issued, while non-primary activation and the guarded synthetic click do not double activate it

#### Scenario: Confirm controller shutdown

- **WHEN** the user chooses Shut down controller on an eligible idle connection
- **THEN** a modal warns that not every controller supports software shutdown, that unsupported hardware may remain on, and that the board must be stationary before **Shut down** sends exactly one request while **Cancel** sends none

#### Scenario: Shutdown transmission is not power confirmation

- **WHEN** the backend accepts a shutdown command but the connection remains available
- **THEN** the frontend does not claim that power-off succeeded and continues to render the authoritative connection after pending intent clears

#### Scenario: Shutdown causes transport loss

- **WHEN** an accepted shutdown powers off the controller and its attached transport
- **THEN** the live view exits through the normal user-initiated disconnect presentation without showing an automatic-reconnect failure
