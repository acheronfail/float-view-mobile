## ADDED Requirements

### Requirement: Board-scoped session export

`session.export` and the authenticated session-export HTTP endpoint SHALL accept exactly one of `sessionId` or `savedDeviceId`, plus the existing format. A board selector SHALL produce one ZIP of existing individual exports, with collision-free portable entry names and unchanged archive/GPS semantics. Existing native presentation and browser download delivery SHALL apply without a new host interface.

#### Scenario: Invalid selector

- **WHEN** a request supplies both selectors, neither selector, or an unknown board
- **THEN** export fails without presenting a file or mutating history

#### Scenario: Bounded atomic bundle

- **WHEN** a board export exceeds an admission bound, encounters a failed session, or is cancelled by its owner disconnecting
- **THEN** export fails without delivering a partial bundle, releases temporary resources, and preserves source sessions

#### Scenario: Cross-platform delivery

- **WHEN** Apple, Android or web requests either board export format
- **THEN** the same shared policy selects sessions and produces one ZIP, delivered through native save/share or authenticated browser download
