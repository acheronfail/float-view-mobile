## MODIFIED Requirements

### Requirement: Live connection-state presentation
The dashboard SHALL display the active connection's resolved board name, actual connection state, detected controller route, VESC version, Refloat package version, and VESC access mode when known. It SHALL label a direct route as direct and a CAN route with its CAN ID. A test-version controller SHALL be labeled read-only beside the VESC and Refloat versions. Values SHALL render normally only while the connection is `ready`; every other active connection state SHALL retain and dim the latest sample. The view SHALL NOT display per-sample age or private controller identifiers.

#### Scenario: Direct connection
- **WHEN** authoritative state reports a ready direct controller
- **THEN** the live firmware information identifies the controller connection as direct

#### Scenario: CAN connection
- **WHEN** authoritative state reports a ready controller on CAN ID `42`
- **THEN** the live firmware information identifies the connection as CAN ID `42`

#### Scenario: Test firmware access label
- **WHEN** authoritative firmware information reports an audited nonzero test-version marker
- **THEN** the live firmware information shows `Read-only (test firmware)` beside the VESC and Refloat versions

#### Scenario: Transient reconnect
- **WHEN** a ready connection enters `reconnecting`
- **THEN** the status changes to reconnecting and the latest readings remain visible with dimmed presentation

#### Scenario: Reconnect succeeds
- **WHEN** a reconnecting connection returns to `ready`
- **THEN** the status returns to connected and readings return to normal presentation as persisted updates resume

#### Scenario: Unsupported firmware
- **WHEN** firmware identity is known and marked unsupported
- **THEN** the live view presents the stable unsupported-controller or unsupported-Refloat issue, starts no polling or new session, and retains any previously persisted sample in dimmed presentation

#### Scenario: Recoverable connection failure
- **WHEN** authoritative state contains a failed active connection and connection issue
- **THEN** the live view explains that bounded issue, keeps board configuration and disconnect available, and offers retry for the same device

#### Scenario: Retry after configuration
- **WHEN** the user corrects or clears the preferred CAN ID and activates retry
- **THEN** the frontend sends `device.connect` for the same failed `deviceId` once and follows the new authoritative generation

#### Scenario: Incomplete Board Profile
- **WHEN** the connected board's profile is incomplete
- **THEN** profile completeness does not block or add a warning to the live dashboard

### Requirement: Live board-configuration sheet
Configure board SHALL open a modal bottom sheet that covers almost the entire viewport and renders the same board configuration editor as the routed Boards flow, including the optional preferred CAN ID. The sheet SHALL respect safe areas, contain focus, provide a close control, dismiss from Escape or backdrop activation, and restore focus to its trigger.

#### Scenario: Open live configuration
- **WHEN** the user selects Configure board for an identified active board
- **THEN** the sheet opens over the live dashboard and displays the board's resolved name, editable custom name, read-only platform Bluetooth identifier, optional preferred CAN ID, and remaining Board Profile fields

#### Scenario: Backdrop dismissal after text edit
- **WHEN** backdrop activation blurs a valid edited text field
- **THEN** the resulting save completes before the sheet dismisses and the updated authoritative value is visible after dismissal

#### Scenario: Reconnecting while configuring
- **WHEN** the active connection begins reconnecting while its configuration sheet is open
- **THEN** local board configuration remains usable and the underlying live status reflects reconnecting

## ADDED Requirements

### Requirement: Ambiguous controller prompt
The shared live view SHALL present an accessible modal controller-selection prompt whenever authoritative state is `selecting_controller`. It SHALL state how many VESC controllers were found, list every CAN-ID-sorted candidate's CAN ID and available display-safe identifying information, identify unsupported VESC versions, and require an explicit supported choice before sending `device.selectController`. Disconnect MUST remain available.

#### Scenario: Two controllers found
- **WHEN** state contains two current controller candidates
- **THEN** the prompt explains that two VESC controllers were found and renders both CAN IDs with their available hardware/model, firmware name, and VESC version

#### Scenario: Unsupported candidate
- **WHEN** a candidate has `vescSupported` false
- **THEN** the prompt keeps it visible with an unsupported explanation and disables its selection control

#### Scenario: Candidate activated
- **WHEN** the user activates one candidate with a primary pointer or keyboard
- **THEN** the frontend sends one selection request, prevents duplicate activation while pending, and follows subsequent authoritative state

#### Scenario: Prompt recovered
- **WHEN** the frontend is recreated while selection remains pending
- **THEN** the prompt is reconstructed from `state.get` without starting another connection or discovery request

#### Scenario: User disconnects instead
- **WHEN** the user chooses to disconnect while the prompt is visible
- **THEN** the frontend requests disconnection and closes the prompt when authoritative state removes the active connection
