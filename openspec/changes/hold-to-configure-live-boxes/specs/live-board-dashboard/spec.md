## ADDED Requirements

### Requirement: Hold to configure live boxes

The five live boxes SHALL be configured by a primary-pointer hold, with no short-tap configuration. A completed hold SHALL NOT trigger a tap action. Movement, cancellation, lost capture, hiding and disposal SHALL cancel pending holds. Keyboard and assistive users SHALL have equivalent configuration access. New settings SHALL default to Empty, Wattage, Session timer, Odometer and centre Trip distance; Empty SHALL display Hold to set.

#### Scenario: Hold and cancel

- **WHEN** the user holds a box without moving
- **THEN** the shared picker opens once, while a short numeric tap, cancelled gesture, moved gesture or non-primary pointer opens nothing

#### Scenario: Keyboard configuration

- **WHEN** a keyboard or assistive user configures any box
- **THEN** the same picker and revisioned save flow are available with dismissal and focus restoration

### Requirement: Configurable runtime lighting boxes

The slot catalogue SHALL include Empty, Headlights and LEDs alongside numeric readings. Lighting boxes SHALL display confirmed runtime state and toggle on short tap through existing lighting requests. Unknown, unavailable, read-only, reconnecting and pending states SHALL block mutation while allowing configuration. A hold SHALL configure without changing lights. Accepted settings SHALL persist across frontend and host recreation.

#### Scenario: Tap and confirmation

- **WHEN** an eligible lighting box is tapped
- **THEN** exactly one request is sent, pending feedback retains the confirmed state, and only authoritative updates change the switch

#### Scenario: Hold lighting or failure

- **WHEN** the user holds a lighting box or its toggle request fails
- **THEN** holding opens configuration without toggling, and failure retains confirmed state with visible feedback

#### Scenario: Persist choices

- **WHEN** any combination of five valid numeric, empty or lighting choices is saved
- **THEN** both host integrations retain the exact choices and reject unknown fields or an invalid slot count

## MODIFIED Requirements

### Requirement: Extensible live action menu

The live header SHALL provide one accessible menu trigger whose menu contains the current view switch, stateful Refloat package and flywheel actions, Charts… for an active saved session, Clear fatal error only when a Refloat 1.2.x fatal latch remains after its active firmware fault has ended, Shut down controller when the VESC core profile supports the command, Configure board when saved identity exists, and Stop and disconnect. LED and headlight menu items SHALL be absent. Each remaining Refloat item SHALL display its last confirmed on/off state, SHALL be disabled unless its native preconditions are currently satisfied, and SHALL remain unavailable while its action is pending. Shutdown SHALL be unavailable unless authoritative controller-action state permits it. While authoritative state reports flywheel enabled, a non-dismissible full-application modal overlay SHALL replace access to normal application interaction and SHALL expose exactly one action, Disable, which requests the safety-gated flywheel-off transition. Disabling Refloat, enabling flywheel, and shutting down the controller SHALL require explicit contextual confirmation before the backend request is sent. Every menu, confirmation, and flywheel-overlay control SHALL retain native button semantics, use primary Pointer Events with cancellation and a guarded keyboard/assistive click path, avoid double activation, and manage focus within the modal surface.

#### Scenario: Open menu

- **WHEN** the user activates the live header menu trigger on a ready writable connection
- **THEN** a keyboard- and pointer-operable menu presents the applicable Refloat controls with confirmed state alongside the current view, configuration, and disconnect actions

#### Scenario: Saved identity pending

- **WHEN** an active connection has not yet produced a saved-device identity
- **THEN** Configure board is unavailable while applicable live-only Refloat controls, connection state, and disconnect remain usable

#### Scenario: Lighting unavailable

- **WHEN** Refloat INFO reports no lighting capability
- **THEN** configured lighting boxes show unavailable state and no lighting request is issued by the frontend

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

### Requirement: Generated Refloat configuration experience

The live-board action surface SHALL present `Configure Refloat` after configuration discovery begins and `Tunes…` when an exact supported package identity is available. Configure Refloat SHALL open a full-height responsive editor generated from bounded normalized schema pages, with a native selector that presents one logical schema subgroup at a time, search scoped to the selected subgroup, typed controls, on-demand sanitized help, source selection between current and defaults, dirty and validation state, a bounded write diff, explicit persistent-write confirmation, operation progress, and unsaved-change handling. The editor SHALL combine consecutive or non-consecutive transport pages with the same exact group and subgroup identity into one ordered logical subgroup without changing complete-editor draft, validation, or write behavior. Each field SHALL expose a `(?)` help button instead of permanently rendering its description; activation SHALL open a non-urgent modal dialog containing that field's label and help text, with bounded dismissal, focus containment, and focus restoration. Persistent schema fields such as LEDs On and Headlights On SHALL be labelled as startup defaults, while the live boxes SHALL label the dedicated lighting controls as current runtime state. In flywheel mode schema/default inspection may remain available, but reading current, editing or saving from current, persistent write, and Tune apply SHALL be visibly unavailable until normal mode and a fresh current read are established. It SHALL never render raw XML or infer protocol encodings. Every interactive control MUST retain native semantics, primary-pointer cancellation behavior, guarded keyboard/assistive click behavior, and focus restoration without double activation.

#### Scenario: Open generated editor

- **WHEN** a user opens Configure Refloat on a ready supported connection
- **THEN** the editor loads current values, selects the first available logical subgroup, and renders only that subgroup's schema-defined fields with their types, choices, units, bounds, and steps

#### Scenario: Navigate schema subgroups

- **WHEN** the user selects another subgroup
- **THEN** the editor replaces the visible fields with that logical subgroup while preserving all draft values, validation state, dirty state, and schema order across the complete configuration

#### Scenario: Chunked logical subgroup

- **WHEN** one schema group and subgroup is delivered across multiple bounded transport pages
- **THEN** the selector exposes one option for that exact pair and the selected logical subgroup contains all of its fields in received order

#### Scenario: Ambiguous subgroup labels

- **WHEN** more than one schema group contains the same subgroup name
- **THEN** the selector qualifies those otherwise ambiguous option labels with their group and retains each exact group/subgroup pair as a distinct destination

#### Scenario: Search selected subgroup

- **WHEN** the user searches while a subgroup is selected
- **THEN** only that subgroup's fields are filtered using its group, subgroup, field name, label, and help text, navigation does not change, and an empty match is visibly reported

#### Scenario: Show field help

- **WHEN** the user activates a field's `(?)` help button by primary pointer, keyboard, or assistive click
- **THEN** exactly one modal dialog shows the field's presented label and sanitized help text, contains one explicit dismissal action, supports Escape and backdrop dismissal, contains focus while open, and restores focus to the triggering button when dismissed

#### Scenario: Load package defaults

- **WHEN** the user selects Read default values
- **THEN** the draft changes to the complete device-reported defaults, is visibly marked as not yet written, current board state remains unchanged, and the selected subgroup is retained

#### Scenario: Confirm persistent write

- **WHEN** an eligible valid draft differs from the latest current values, including values changed outside the selected subgroup
- **THEN** the UI presents a bounded complete field diff and warning and sends exactly one complete write only after explicit confirmation

#### Scenario: Invalid or read-only draft

- **WHEN** any field is invalid including a field outside the selected subgroup, the connection is not writable, safety state is ineligible, or schema/current identity is stale
- **THEN** Write is visibly unavailable and no mutation request is sent

#### Scenario: Reconnect while editing

- **WHEN** the board reconnects while an editor draft is dirty
- **THEN** editing becomes non-writable and the draft is not applied until the active schema/current identity is re-established and explicitly reconciled

#### Scenario: Schema replacement changes navigation

- **WHEN** schema replacement removes the selected exact group/subgroup identity
- **THEN** the editor falls back to the first available logical subgroup rather than retaining a stale selection

#### Scenario: Flywheel while editing

- **WHEN** authoritative state enters flywheel while the editor is open
- **THEN** current values and any current-based draft become stale, all device-write actions except the separate eligible flywheel-disable control are unavailable, and a fresh current read is required after normal mode returns

#### Scenario: Runtime and persistent lighting

- **WHEN** runtime lighting differs from `leds.on` or `leds.headlights_on` in the configuration
- **THEN** the UI presents the values as current runtime override and persistent startup default respectively rather than implying they have diverged accidentally
