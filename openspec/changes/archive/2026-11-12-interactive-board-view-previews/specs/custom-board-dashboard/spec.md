## MODIFIED Requirements

### Requirement: Theme toggle widget

Custom widget selection and configuration SHALL offer Light / dark mode with supported dimensions. Its identity, order and dimensions SHALL persist through the shared Apple and Android profile contract without changing existing dashboards. It SHALL use the same persisted appearance toggle as Live, independently of telemetry availability.

#### Scenario: Save and restore the widget

- **WHEN** a rider adds or resizes the theme widget, saves and reloads
- **THEN** the widget configuration is preserved and activation toggles appearance once

#### Scenario: Preview and edit suppression

- **WHEN** the widget is shown in a picker/configuration thumbnail, held to edit, dragged, scrolled, cancelled or activated by a non-primary pointer
- **THEN** appearance remains unchanged

#### Scenario: Interactive board preview

- **WHEN** the user normally activates the theme widget in a board's interactive preview
- **THEN** the app appearance toggles exactly once through the existing persisted preference

### Requirement: Board-scoped preview editing

Customise views SHALL use labelled example readings and shared renderers with normal tap/hold behavior. Live boxes SHALL open configuration on hold with immediate revisioned selection saves. Custom SHALL open in viewing mode and enter draft editing on hold, retaining existing add, configure, reorder, remove, Save and Cancel actions. Save or accepted Cancel SHALL return to viewing without leaving the preview.

#### Scenario: Edit and reopen offline

- **WHEN** the user changes a Live box or saves a Custom composition while disconnected and reopens the editor
- **THEN** the accepted choices remain for that board and the views retain independent configurations

#### Scenario: Preview controls

- **WHEN** the user taps, holds, cancels or uses non-primary input on preview lighting, theme or marker widgets
- **THEN** eligible short taps perform the documented preview interaction, holds enter editing without activation, cancelled/non-primary input does nothing, and no board command or ride marker is sent

#### Scenario: Default and unavailable profile

- **WHEN** a board has no Custom composition
- **THEN** its preview defaults incorporate that board's Live choices without saving merely on open
- **WHEN** a profile read fails
- **THEN** retry is offered without replacing the stored configuration with defaults

## ADDED Requirements

### Requirement: Interactive preview action isolation

Interactive previews SHALL toggle headlights and LEDs only in temporary frontend state shared by both views for the selected board. Theme widgets SHALL use the app appearance preference. Ride markers SHALL remain unavailable without a real session. Picker/configuration thumbnails SHALL remain inert. Preview interactions SHALL never send device commands or create ride markers, even while another board is connected.

#### Scenario: Example lighting across views

- **WHEN** the user taps an example light switch and changes between Live and Custom
- **THEN** both views and duplicate controls show the same example switch state without changing the saved layout or sending a device command
- **WHEN** the user leaves the board preview and returns
- **THEN** example lighting resets to its initial state

#### Scenario: Live distance example

- **WHEN** the user taps a Live distance box in preview
- **THEN** it switches locally between ride distance and odometer without changing the saved box choice

#### Scenario: Edit and gesture suppression

- **WHEN** the user holds, drags, scrolls, cancels or uses a non-primary pointer on an interactive widget
- **THEN** no tap action runs and no theme or example switch state changes
- **WHEN** the user saves or cancels Custom edits
- **THEN** normal widget interaction resumes in the preview and dirty exits retain their existing discard guard
