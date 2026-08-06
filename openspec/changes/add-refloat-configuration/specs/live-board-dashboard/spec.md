## ADDED Requirements

### Requirement: Generated Refloat configuration experience

The live-board action surface SHALL present `Configure Refloat` after configuration discovery begins and `Tunes…` when an exact supported package identity is available. Configure Refloat SHALL open a full-height responsive editor generated from bounded normalized schema pages, with subgroup navigation, search, typed controls, sanitized help, source selection between current and defaults, dirty and validation state, a bounded write diff, explicit persistent-write confirmation, operation progress, and unsaved-change handling. It SHALL never render raw XML or infer protocol encodings. Every interactive control MUST retain native semantics, primary-pointer cancellation behavior, guarded keyboard/assistive click behavior, and focus restoration without double activation.

#### Scenario: Open generated editor

- **WHEN** a user opens Configure Refloat on a ready supported connection
- **THEN** the editor loads current values and renders only the active release's grouped fields with schema-defined types, choices, units, bounds, steps, and help

#### Scenario: Load package defaults

- **WHEN** the user selects Read default values
- **THEN** the draft changes to the complete device-reported defaults, is visibly marked as not yet written, and current board state remains unchanged

#### Scenario: Confirm persistent write

- **WHEN** an eligible valid draft differs from the latest current values
- **THEN** the UI presents a bounded field diff and warning and sends exactly one complete write only after explicit confirmation

#### Scenario: Invalid or read-only draft

- **WHEN** a field is invalid, the connection is not writable, safety state is ineligible, or schema/current identity is stale
- **THEN** Write is visibly unavailable and no mutation request is sent

#### Scenario: Reconnect while editing

- **WHEN** the board reconnects while an editor draft is dirty
- **THEN** editing becomes non-writable and the draft is not applied until the active schema/current identity is re-established and explicitly reconciled

### Requirement: Exact-version Tunes experience

Tunes SHALL present bounded named full Refloat configurations applicable to the active exact package version and schema signature, support create/rename/describe/delete with revision-conflict recovery, show a bounded diff from current values, and apply only through the normal persistent-write confirmation and verification flow. UI copy SHALL identify these records as complete Refloat configurations and MUST NOT claim compatibility with upstream Refloat's curated QML Tune subset or migrations.

#### Scenario: Matching Tunes

- **WHEN** Tunes opens for a supported active package
- **THEN** it lists only applicable exact-version/signature records with bounded metadata

#### Scenario: Apply Tune

- **WHEN** the user confirms an eligible saved configuration whose values differ from current
- **THEN** the frontend sends one apply request and reports success only after native readback verification

#### Scenario: Version changes

- **WHEN** the connected Refloat exact version changes
- **THEN** records from the former version disappear from the applicable list and are not silently transformed
