## ADDED Requirements

### Requirement: Saved-board view customisation entry points

Saved-board history and connected saved-board menus SHALL offer Customise views. It SHALL open a focused editor showing the selected board's name and configured board artwork, with Live View and Custom View choices. It SHALL work without discovery or connection, including imported boards, and return to its originating surface through guarded dismissal.

#### Scenario: Open without a board connection

- **WHEN** the user selects Customise views from a saved board's history menu
- **THEN** its name and artwork appear with both editable views without scanning or connecting

#### Scenario: Open from Live

- **WHEN** the user selects Customise views for a connected saved board
- **THEN** the same editor opens on Custom when Custom was displayed, otherwise Live, without changing connection or recording

#### Scenario: Dismiss or lose the target

- **WHEN** the user closes the editor or its target board is removed
- **THEN** normal dismissal protects dirty edits, removal invalidates the target, and no late operation applies to another board
