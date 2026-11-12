## ADDED Requirements

### Requirement: Board-scoped preview editing

Customise views SHALL use labelled example readings and shared dashboard renderers. Live SHALL allow direct selection of each of its five boxes with immediate revisioned saves. Custom SHALL enter draft editing directly with existing add, configure, reorder, remove, Save and Cancel actions. Preview actions SHALL never mutate runtime controls, appearance or ride markers.

#### Scenario: Edit and reopen offline

- **WHEN** the user changes a Live box or saves a Custom composition while disconnected and reopens the editor
- **THEN** the accepted choices remain for that board and the views retain independent configurations

#### Scenario: Preview controls

- **WHEN** the user taps, holds, cancels or uses non-primary input on preview lighting, theme or marker widgets
- **THEN** only eligible configuration gestures operate, without any runtime action or duplicate activation

#### Scenario: Default and unavailable profile

- **WHEN** a board has no Custom composition
- **THEN** its preview defaults incorporate that board's Live choices without saving merely on open
- **WHEN** a profile read fails
- **THEN** retry is offered without replacing the stored configuration with defaults

### Requirement: Isolated preview target and saves

The editor SHALL retain its selected saved-board target independently of connection changes and use no active-board readings. Preview view selection SHALL NOT change the next connected view. Leaving dirty Custom edits SHALL require a discard decision. Failed saves SHALL retain edits with feedback; stale revisions SHALL NOT silently overwrite newer Custom compositions.

#### Scenario: Another board or connection changes

- **WHEN** another board is active or connection state changes during editing
- **THEN** the editor retains its target and draft with fixed example readings, and any accepted save affects only that board

#### Scenario: Save failure or conflict

- **WHEN** a Live save fails or a Custom save fails or conflicts
- **THEN** actionable feedback appears, accepted data remains authoritative, and the user can retry or explicitly reload without silently losing Custom edits

#### Scenario: Switch or close a dirty editor

- **WHEN** the user switches away from dirty Custom edits or dismisses the editor
- **THEN** they can keep editing or explicitly discard, including through keyboard and native Back
