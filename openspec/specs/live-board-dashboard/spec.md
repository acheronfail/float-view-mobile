# Live Board Dashboard Specification

## Purpose

Define the focused live-board experience, including responsive telemetry, connection and session state, board configuration, and safe disconnect controls.

## Requirements

### Requirement: Focused responsive live dashboard

The live-board view SHALL remain a focused subordinate screen without primary tabs and SHALL present every field in the current live telemetry contract at an intentional stable position in a responsive grid. The grid SHALL render three, two, or one usable columns as available width decreases, SHALL preserve stable cell placement while values update, and SHALL render unavailable values as placeholders.

#### Scenario: First live sample

- **WHEN** a connected board publishes its first persisted live sample
- **THEN** the dashboard replaces placeholders with the sample's speed, eRPM, voltage, duty, currents, temperatures, ride state, board state, footpad state, sequence, and elapsed time in their assigned cells

#### Scenario: Before the first sample

- **WHEN** a connection is active but no persisted sample is available
- **THEN** the complete dashboard structure remains visible with placeholders and without layout movement

#### Scenario: Narrow viewport

- **WHEN** the live view is rendered on a narrow mobile viewport
- **THEN** every reading remains readable without horizontal page scrolling or omission

#### Scenario: Future telemetry field

- **WHEN** the contract gains another live telemetry field
- **THEN** the frontend assigns it an intentional dashboard position rather than rendering it through a catch-all field bucket

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

### Requirement: Active-session live summary

The live view SHALL present the active session's sample count, riding elapsed time, total elapsed time, distance, maximum speed, maximum duty cycle, state, and integrity from the same authoritative snapshot as the latest sample.

#### Scenario: Persisted sample updates summary

- **WHEN** a sample is successfully persisted
- **THEN** the dashboard receives the resulting latest sample and current active-session aggregates in one newer authoritative snapshot

#### Scenario: Unavailable aggregate

- **WHEN** an optional active-session aggregate has not been established
- **THEN** its assigned cell displays a placeholder rather than a fabricated zero

### Requirement: Extensible live action menu

The live header SHALL provide one accessible menu trigger whose menu contains Configure board and Stop and disconnect actions and can accept additional actions later without changing the focused route model.

#### Scenario: Open menu

- **WHEN** the user activates the live header menu trigger
- **THEN** a keyboard- and pointer-operable menu presents the two current actions and identifies the trigger accessibly

#### Scenario: Saved identity pending

- **WHEN** an active connection has not yet produced a saved-device identity
- **THEN** Configure board is unavailable while connection state and disconnect remain usable

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

### Requirement: Conditional disconnect confirmation

Stop and disconnect SHALL proceed without confirmation only when the latest persisted ride state is exactly `idle`. A non-idle, unknown, or unavailable latest ride state SHALL require explicit confirmation before the disconnect request is sent.

#### Scenario: Idle disconnect

- **WHEN** the latest persisted ride state is `idle` and the user selects Stop and disconnect
- **THEN** the frontend requests disconnect immediately and returns Home after authoritative state removes the active connection

#### Scenario: Riding disconnect

- **WHEN** the latest persisted ride state is not `idle` and the user selects Stop and disconnect
- **THEN** the frontend requests confirmation and disconnects only after the user confirms

#### Scenario: No live sample

- **WHEN** no latest persisted sample is available and the user selects Stop and disconnect
- **THEN** the frontend requests confirmation before disconnecting

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

### Requirement: Detailed live board telemetry

The shared live dashboard SHALL present available detailed telemetry in coherent groups for motion, electrical load, temperatures, rider input, control contributions, and accumulated values. It SHALL clearly distinguish measured, requested, directional, filtered, booster, and field-weakening currents in amperes and duty as a ratio. It SHALL NOT derive or display torque or current utilization. Missing values SHALL display as unavailable rather than zero.

#### Scenario: Detailed dynamic session

- **WHEN** a Refloat 1.2.3 session supplies full dynamic telemetry
- **THEN** the dashboard can display its IMU, footpad, current, ATR, tilt, remote, and core motor values without platform branching

#### Scenario: Legacy unavailable value

- **WHEN** the selected legacy detail level does not provide a field shown by the dashboard
- **THEN** that field is hidden or marked unavailable and is not displayed as zero

### Requirement: Live semantic board states

The live experience SHALL surface package state and mode, footpad state, wheelslip, darkride, charging, stop condition, setpoint adjustment or pushback type, alert reason, and bounded firmware fault with accessible text and non-color-only distinctions.

#### Scenario: Duty pushback

- **WHEN** telemetry reports running state, duty pushback, and a duty alert reason
- **THEN** the live view identifies duty pushback distinctly from high-voltage, low-voltage, temperature, speed, and firmware-fault states

#### Scenario: Wheelslip and footpad

- **WHEN** wheelslip is active and only one footpad side is engaged
- **THEN** both states are conveyed accessibly and neither is inferred solely from current or speed

### Requirement: Detailed historical telemetry review

Session review SHALL expose metadata first, then the same supported detailed fields through bounded authenticated columnar series/exact-sample requests, plus observed semantic transitions through chronological event pages. It SHALL preserve nullability and units, allow inspection of fixture-generated scenes after disconnect, cap each data request to 16 fields/1,024 buckets/65,536 observations, cap sample pages at 2,000 rows and event pages at 500 rows, and disclose retained/pruned history metadata.

#### Scenario: Review field weakening

- **WHEN** a session includes field-weakening current and high duty
- **THEN** historical detail shows both raw series independently in their contract units

#### Scenario: Review stop event

- **WHEN** a session transitions through a stop condition
- **THEN** the event page makes the state transition inspectable alongside surrounding telemetry

### Requirement: Detailed telemetry interactions

Any interactive telemetry disclosure, group selector, chart control, or session annotation control SHALL use native semantics, appropriate `touch-action`, primary Pointer Events with cancellation, and a guarded keyboard and assistive-technology click path without double activation.

#### Scenario: Touch disclosure

- **WHEN** a primary touch pointer opens a detailed telemetry group in a frequently updating live view
- **THEN** it opens exactly once and remains operable while samples continue updating

#### Scenario: Non-primary activation

- **WHEN** a non-primary pointer or cancelled gesture targets a detailed telemetry control
- **THEN** no action is performed

#### Scenario: Prompt recovered

- **WHEN** the frontend is recreated while selection remains pending
- **THEN** the prompt is reconstructed from `state.get` without starting another connection or discovery request

#### Scenario: User disconnects instead

- **WHEN** the user chooses to disconnect while the prompt is visible
- **THEN** the frontend requests disconnection and closes the prompt when authoritative state removes the active connection
