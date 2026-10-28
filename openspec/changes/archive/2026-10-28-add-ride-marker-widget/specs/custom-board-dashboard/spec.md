## ADDED Requirements

### Requirement: Ride marker action widget

The widget catalogue and configuration SHALL offer a Save ride marker button with supported dimensions. Configuration SHALL persist through the shared Apple and Android profile contract, preserving existing dashboards. A primary tap or keyboard/assistive activation SHALL request exactly one marker for the displayed active saved session. Pending or unavailable capture SHALL disable activation, and confirmed success SHALL provide accessible visual feedback.

#### Scenario: Save and confirm

- **WHEN** a rider activates the button during an active saved session
- **THEN** it displays saving feedback and, only after durable confirmation, an accessible Marker saved message with a visible check and brief highlight
- **AND** reduced motion retains static confirmation without animation

#### Scenario: Failure and lifecycle

- **WHEN** saving fails or its outcome is unknown
- **THEN** concise failure or unconfirmed feedback appears without success or automatic retry
- **WHEN** session identity changes or the widget is destroyed
- **THEN** old feedback clears, timers are disposed and late results do not appear in the new session

#### Scenario: Configure and preserve

- **WHEN** a rider adds or resizes the marker button, saves the dashboard and reloads
- **THEN** its order, identity and dimensions persist and configuration previews do not create markers

#### Scenario: Input suppression

- **WHEN** the rider holds the button to edit, scrolls, cancels a pointer or uses a non-primary pointer
- **THEN** no marker is created, including in previews and dashboard editing
- **WHEN** a normal activation produces a synthesized click
- **THEN** it does not create a duplicate marker
