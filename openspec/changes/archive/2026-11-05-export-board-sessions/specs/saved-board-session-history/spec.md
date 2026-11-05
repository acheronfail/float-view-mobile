## ADDED Requirements

### Requirement: Export all board sessions

The board-history menu SHALL offer Export all as Float View archives or CSV ZIPs. Each action SHALL deliver one ZIP containing individual exports of every non-active session belonging to that board at request time, independently of loaded history pages. The UI SHALL explain that active recordings are skipped, disable export for empty history and while busy, and report success or failure.

#### Scenario: Export beyond the visible page

- **WHEN** the user exports a board with more sessions than the loaded history page
- **THEN** the ZIP contains every eligible session for that board, excludes other boards and active recordings, and uses the chosen individual export format

#### Scenario: Recording or empty history

- **WHEN** the selected board has an active recording or no exportable sessions
- **THEN** recording continues unchanged and no partial or empty bundle is presented as success
