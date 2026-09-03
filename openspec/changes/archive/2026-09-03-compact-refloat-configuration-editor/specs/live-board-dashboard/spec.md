## MODIFIED Requirements

### Requirement: Generated Refloat configuration experience

The live-board action surface SHALL present `Configure Refloat` after configuration discovery begins and `Tunes…` when an exact supported package identity is available. Configure Refloat SHALL open a full-height responsive editor generated from bounded normalized schema pages, with a native selector that presents one logical schema subgroup at a time, search scoped to the selected subgroup, typed controls, on-demand sanitized help, source selection between current and defaults, dirty and validation state, a bounded write diff, explicit persistent-write confirmation, operation progress, and unsaved-change handling. The editor SHALL combine consecutive or non-consecutive transport pages with the same exact group and subgroup identity into one ordered logical subgroup without changing complete-editor draft, validation, or write behavior. Each field SHALL expose a `(?)` help button instead of permanently rendering its description; activation SHALL open a non-urgent modal dialog containing that field's label and help text, with bounded dismissal, focus containment, and focus restoration. Persistent schema fields such as LEDs On and Headlights On SHALL be labelled as startup defaults, while the live menu SHALL label the dedicated lighting controls as current runtime state. In flywheel mode schema/default inspection may remain available, but reading current, editing or saving from current, persistent write, and Tune apply SHALL be visibly unavailable until normal mode and a fresh current read are established. It SHALL never render raw XML or infer protocol encodings. Every interactive control MUST retain native semantics, primary-pointer cancellation behavior, guarded keyboard/assistive click behavior, and focus restoration without double activation.

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
