## ADDED Requirements

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

The dashboard SHALL display the active connection's resolved board name and actual connection state. Values SHALL render normally only while the connection is `ready`; every other active connection state SHALL retain and dim the latest sample. The view SHALL NOT display per-sample age.

#### Scenario: Transient reconnect

- **WHEN** a ready connection enters `reconnecting`
- **THEN** the status changes to reconnecting and the latest readings remain visible with dimmed presentation

#### Scenario: Reconnect succeeds

- **WHEN** a reconnecting connection returns to `ready`
- **THEN** the status returns to connected and readings return to normal presentation as persisted updates resume

#### Scenario: Unsupported firmware

- **WHEN** firmware identity is known and marked unsupported
- **THEN** the live view presents a warning without hiding telemetry or active-session information

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

Configure board SHALL open a modal bottom sheet that covers almost the entire viewport and renders the same board configuration editor as the routed Boards flow. The sheet SHALL respect safe areas, contain focus, provide a close control, dismiss from Escape or backdrop activation, and restore focus to its trigger.

#### Scenario: Open live configuration

- **WHEN** the user selects Configure board for an identified active board
- **THEN** the sheet opens over the live dashboard and displays the board's resolved name, editable custom name, read-only platform Bluetooth identifier, and Board Profile fields

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
