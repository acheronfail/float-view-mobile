## MODIFIED Requirements

### Requirement: Shared estimated battery percentage in live views

The live power widget and detailed view SHALL prefer the authoritative estimated fraction rounded to the same nearest integer percentage. For a saved board whose estimate is unavailable specifically because battery configuration is incomplete, both SHALL instead display the controller-reported fraction rounded identically when it is finite and in [0,1]. Text, widget fill, color thresholds, and accessible percentage SHALL use the selected source consistently. The primary detailed value and accessible widget value SHALL identify whether the percentage is estimated or controller-reported, and the detailed view SHALL retain the separate “Controller-reported battery level” raw value. Both SHALL retain existing stale/dimmed connection behavior, charging indication, and raw pack voltage. Widget per-cell voltage SHALL use the same resolved series count as estimation and be unavailable when no count can be resolved. Guests, BMS-only connections, and estimate failures other than incomplete configuration SHALL retain unavailable primary percentages rather than acquire this fallback.

#### Scenario: Controller percentage disagrees

- **WHEN** an active sample reports controller level 0.8 and the shared estimate is 0.5
- **THEN** both primary percentage displays and widget fill use 50 percent
- **AND** the detailed controller-reported field shows 80 percent

#### Scenario: Missing estimate

- **WHEN** a saved board lacks estimation configuration and reports battery level 0.8
- **THEN** both primary percentage displays and widget fill use 80 percent with controller-reported source semantics and a battery setup warning
- **AND** pack voltage remains visible and per-cell voltage is unavailable if no series count is resolved

#### Scenario: Reported zero or unavailable value

- **WHEN** a saved board lacks estimation configuration
- **THEN** a reported fraction of zero displays 0 percent
- **AND** a missing, non-finite, or out-of-range reported fraction displays the unavailable placeholder without fabricated fill
- **AND** the battery setup warning remains available in either case

#### Scenario: Other estimate failures

- **WHEN** estimation is unavailable because of a series mismatch, missing voltage, or a guest or BMS-only connection
- **THEN** the primary percentage stays unavailable and the missing-configuration fallback and setup warning are not shown

#### Scenario: Connection becomes nonready

- **WHEN** the last sample and matching estimate are retained while reconnecting
- **THEN** both views retain the same selected percentage and source using their existing dimmed/stale presentation

### Requirement: Saved board battery estimation controls

The saved-board editor SHALL provide a Basic/Advanced segmented control shared with the battery-only modal. Basic SHALL offer a manufacturer-grouped select of named battery variants and a concise summary of actual configured values. Advanced SHALL offer bounded cell-profile choices, series/parallel inputs, optional full-pack capacity, and display-empty voltage per cell, explaining the 3.0 V default and that it changes the displayed estimate, not firmware protection thresholds. Selecting a preset SHALL use its configured generic profile internally when a dedicated curve is absent; Basic SHALL NOT display a generic/approximation label, missing-curve warning, or extra confirmation for this case. Controls SHALL use existing revisioned save behavior, be accessible to keyboard and primary-pointer input on desktop and touch, and preserve configuration across view recreation. Missing estimation configuration SHALL NOT introduce a general profile-completeness warning or block unrelated board operations; the contextual live battery setup warning SHALL remain distinct.

#### Scenario: Configure a saved board

- **WHEN** the user selects P45B, valid pack counts, and a valid empty voltage
- **THEN** the configuration is saved through the existing profile request and both live views use the resulting shared estimate

#### Scenario: Keyboard and touch configuration

- **WHEN** the user activates or changes an estimation control with keyboard or primary touch input
- **THEN** the intended interaction occurs once without duplicate saves caused by synthesized clicks

## ADDED Requirements

### Requirement: Contextual battery configuration modal

Both live views SHALL provide a small warning icon button next to the primary battery percentage when the saved board is missing estimation configuration. Its accessible name SHALL communicate that battery setup is required. Activating it SHALL open a modal explaining the board-reported fallback and presenting the same Basic/Advanced battery form as the full board editor: named preset selection and summary in Basic; cell profile, series count, parallel count, optional pack capacity, and display-empty voltage in Advanced, with identical validation and save semantics. The modal SHALL expose a title, focus containment, keyboard dismissal, and focus restoration. Only one board configuration modal SHALL be active at a time.

#### Scenario: Open battery setup

- **WHEN** the warning is activated by primary mouse, primary touch, keyboard, or assistive technology
- **THEN** exactly one battery configuration modal opens for that saved board with existing values populated
- **AND** unrelated board configuration controls are absent

#### Scenario: Cancelled or non-primary activation

- **WHEN** a pointer gesture is cancelled or a non-primary pointer activates the warning
- **THEN** no modal opens

#### Scenario: Configuration becomes complete

- **WHEN** valid battery settings are saved and the authoritative state supplies an estimate
- **THEN** both views switch to the estimate and remove the warning without reconnecting
- **AND** an already open battery modal remains open until dismissed

#### Scenario: Validation or persistence failure

- **WHEN** entered values are invalid, a save fails, or a profile revision conflicts
- **THEN** the user receives validation or save feedback and the UI does not claim those changes were saved
- **AND** percentage and warning changes follow accepted authoritative state only

#### Scenario: Close and reopen

- **WHEN** the user closes the modal after a valid edit
- **THEN** the pending field edit participates in the existing serialized save flow before ordinary closure completes
- **AND** focus returns to the warning or a stable Live control if the warning no longer exists
- **AND** reopening or recreating the view reads persisted configuration

#### Scenario: Telemetry and board lifecycle

- **WHEN** telemetry refreshes or the same saved board reconnects while the modal is open
- **THEN** unrelated updates do not erase in-progress input or multiply open editors
- **AND** if the active board identity changes or the edited board is removed, the editor is invalidated without applying its queued edits to another board

### Requirement: Named presets and non-destructive setup modes

An unconfigured battery or saved named preset SHALL open in Basic; an existing partial or custom battery without preset identity SHALL open in Advanced. Changing modes SHALL preserve drafts and saved values without issuing a save or resetting defaults, including preventing a toggle-induced blur from committing a field. Custom selection SHALL open Advanced without changing stored settings. An explicit named preset selection SHALL apply all battery defaults as one revisioned operation, preserve unrelated profile values, and retain its selected name across reopening and view recreation. The system SHALL NOT infer a branded preset from matching electrical settings or controller identity. Advanced S/P/profile changes departing from the preset SHALL clear the preset identity atomically; capacity and display-empty overrides SHALL retain it. Effective manual S/P changes SHALL clear prior capacity unless the same patch explicitly replaces it; curve-only changes SHALL retain capacity. Advanced SHALL offer an explicit reset to a named battery's defaults without relying on selecting an unchanged option. Basic SHALL summarize actual saved/draft values, including blank or overridden capacity, rather than silently restore nominal values. Failed saves SHALL remain visible as unsaved drafts; authoritative percentage changes SHALL use accepted state only.

#### Scenario: One-step named setup

- **WHEN** an unconfigured saved board selects Floatwheel ADV2
- **THEN** one save applies its identity, 20S2P counts, Samsung 50S profile, 10 Ah nominal capacity, and 3.0 V display-empty default
- **AND** both editors reopen with that name and the accepted values

#### Scenario: Preset without a dedicated curve

- **WHEN** a rider selects Funwheel X7S with P30B cells
- **THEN** setup applies 20S2P, 6 Ah nominal capacity, and the generic profile internally
- **AND** Basic shows the battery name and summary without a generic/approximation label, warning, or additional confirmation
- **AND** valid telemetry can produce an estimate without a missing-configuration warning

#### Scenario: Switching modes with a draft

- **WHEN** a user switches modes after entering a valid or invalid Advanced draft
- **THEN** the draft remains available, no field or preset is saved because of the switch, and switching back restores the same input

#### Scenario: Custom battery and overrides

- **WHEN** a user chooses Custom, edits S/P/profile, or overrides capacity or empty voltage
- **THEN** choosing Custom alone saves nothing, departing S/P/profile clears identity with the edit, effective count changes clear capacity unless explicitly replaced, and capacity/empty overrides retain identity
- **AND** returning to Basic preserves all values and never automatically selects a brand or restores defaults

#### Scenario: Equivalent packs and recreation

- **WHEN** two presets share the same electrical configuration and one is selected successfully
- **THEN** reopening identifies the selected preset by its persisted identity rather than substituting the other name

#### Scenario: Pending selection and failure

- **WHEN** a preset save is pending, rejected, or conflicts while telemetry continues
- **THEN** the draft and save status remain visible, no partial defaults are published, and the live percentage remains tied to accepted configuration
- **AND** rapid selections are serialized and board invalidation cannot redirect pending work
