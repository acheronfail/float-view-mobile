# Live Board Dashboard Spec Delta

## MODIFIED Requirements

### Requirement: Focused responsive live dashboard

The live-board screen SHALL remain subordinate without primary tabs. Live SHALL retain its fixed responsive widgets and five configurable boxes; Detailed SHALL group supported telemetry at stable readable positions; Custom SHALL use the saved board's configured widgets. All views SHALL keep unavailable placeholders and readable narrow layouts. Custom SHALL NOT be required to contain every telemetry field.

#### Scenario: First live sample

- **WHEN** a connected board publishes its first live sample
- **THEN** the selected view replaces applicable placeholders with available readings without changing configured widget order

#### Scenario: Before the first sample

- **WHEN** a connection is active but no sample is available
- **THEN** the selected view's structure remains visible with placeholders and without layout movement

#### Scenario: Narrow viewport

- **WHEN** the selected view is rendered on a narrow mobile viewport
- **THEN** its readings remain readable without horizontal page scrolling or unintended omission

#### Scenario: Future telemetry field

- **WHEN** the contract gains another supported live telemetry field
- **THEN** Detailed assigns it an intentional group rather than a catch-all field bucket, and existing custom layouts remain unchanged

## ADDED Requirements

### Requirement: Explicit live view selection

The live header SHALL expose directly selectable Live, Detailed and Custom choices for saved boards, indicate the selected view, and support hold-to-edit widgets in Custom. Guest connections SHALL retain Live and Detailed without Custom. Selecting a view SHALL NOT change connection, recording or telemetry policy.

#### Scenario: Select any eligible view

- **WHEN** a saved-board user opens the view choices
- **THEN** all three views are labelled and directly reachable without cycling through another view

#### Scenario: Guest connection

- **WHEN** the active connection is a guest
- **THEN** existing Live and Detailed views remain accessible and no durable dashboard is created

### Requirement: Parent-owned live warning overlay

Firmware-fault and Refloat-disabled warnings SHALL appear above Live, Detailed and Custom independently of widget selection and scrolling, with accessible text and existing action gates. Menus and dialogs SHALL NOT contain warning panels. Modal presentation SHALL conceal and disable the dashboard presenter without shifting its layout. Authoritative safety modals SHALL retain precedence.

#### Scenario: Dashboard without a gauge

- **WHEN** a firmware fault or Refloat-disabled state occurs in a custom dashboard with no gauge
- **THEN** the warning remains visible with its eligible action and is not duplicated in a widget

#### Scenario: Editing and safety precedence

- **WHEN** a warning appears while configuring a widget
- **THEN** warning panels are concealed and inert behind the modal, remain outside it and reappear on the dashboard after dismissal
- **WHEN** authoritative flywheel state requires its non-dismissible safety overlay
- **THEN** that overlay retains its existing exclusive interaction policy

### Requirement: Shared bounded live trend history

Custom charts and the live-chart overlay SHALL share one in-memory history for the active live screen, limited to 30 seconds and 1,500 points total. History SHALL consume existing delivered samples without additional telemetry requests, durable writes or background collection. Duplicate sample sequences SHALL NOT append duplicate points.

#### Scenario: Several chart widgets

- **WHEN** several custom charts and the live-chart overlay display the same connection
- **THEN** they read one retained sample history, and telemetry cadence, recording cadence and retained point capacity do not increase with chart count

#### Scenario: Switch views or edit

- **WHEN** the user changes live view, resizes, edits widgets or opens and closes the chart overlay
- **THEN** retained history remains available for the same connection within its time and point limits

### Requirement: Live trend continuity and disposal

Trends SHALL preserve missingness and gaps, reject stale arrivals as fresh points, and avoid joining samples across connection generations. Device changes, new generations and sequence resets SHALL clear incompatible history. Leaving the live screen or destroying the frontend SHALL release history and timers. Recreation SHALL begin with empty history and SHALL NOT replay collection or controls.

#### Scenario: Reconnect or sample sequence reset

- **WHEN** a connection generation changes or sample sequence resets
- **THEN** old points are not joined to new samples, while the board's dashboard configuration remains intact

#### Scenario: Missing interval

- **WHEN** no fresh sample arrives for an interval longer than the existing live-chart continuity threshold
- **THEN** trends show a gap or waiting state rather than a fabricated continuous trace

#### Scenario: Frontend replacement or screen exit

- **WHEN** the live screen is disposed or the frontend is recreated
- **THEN** its history and timers are released, and recreated charts begin from fresh delivered samples without reading saved sessions to refill the live buffer
