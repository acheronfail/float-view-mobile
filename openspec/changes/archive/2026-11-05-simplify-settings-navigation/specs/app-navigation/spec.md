## MODIFIED Requirements

### Requirement: Global settings destination

Settings SHALL contain application-wide preferences and app support tools grouped by purpose. Board-specific values MUST NOT be edited from Settings, and application-wide settings MUST NOT remain embedded in session browsing.

#### Scenario: Open global settings

- **WHEN** the user selects Settings
- **THEN** the frontend renders seven labelled destinations: Appearance & units, Ride announcements, Ride recording, Connection & permissions, Data & backups, Help & diagnostics and About

#### Scenario: Native-owned option update

- **WHEN** the user changes a valid global option
- **THEN** the frontend sends a revision-checked settings patch and renders the backend-returned settings

## ADDED Requirements

### Requirement: Focused settings pages

Settings home SHALL show navigation rows with short descriptions instead of preference controls or ride totals. Each destination SHALL have a titled page and a return to Settings, retaining in-memory landing-page position and keyboard focus. Licences SHALL return to About. Recreated frontends SHALL use the existing Settings checkpoint without extending the wire contract.

#### Scenario: Open and return

- **WHEN** a user opens a category with touch or keyboard and returns by header, native or browser back
- **THEN** Settings retains its position and keyboard users regain focus on the category link

#### Scenario: Frontend reconstruction

- **WHEN** a frontend is recreated from a saved Settings checkpoint
- **THEN** Settings home opens without replaying any preference edit or support action

### Requirement: Settings progressive disclosure

Ride recording SHALL keep minimum ride distance, technical sampling and chart-gap controls in a collapsed Advanced recording section. Announcements SHALL group each alert with its associated options, disclose those options when enabled, and retain their saved values when disabled. Shared audio and testing controls SHALL be grouped separately. Boolean preferences SHALL use labelled Off / On segmented controls with guarded pointer and keyboard activation.

#### Scenario: Enable an alert

- **WHEN** a user enables speed, load or footpad announcements
- **THEN** its saved threshold or delay becomes editable beside that alert without resetting the value

#### Scenario: Invalid alert value

- **WHEN** a user disables an alert while its edited threshold is invalid
- **THEN** the invalid field and error remain reachable until corrected and saved, rather than hiding the correction path

#### Scenario: Recording detail

- **WHEN** a user opens Ride recording and expands Advanced recording
- **THEN** technical recording and chart-gap controls become available without changing their saved values

### Requirement: Settings permission entry points

Connection & permissions SHALL indicate when a required permission needs attention. Ride location recording SHALL link to the permissions page. Diagnostic recovery links SHALL open Help & diagnostics directly.

#### Scenario: Required permission unavailable

- **WHEN** a required permission is denied, restricted or limited, or its required service is off
- **THEN** the Settings category indicates attention is needed and opens the existing permission recovery controls

### Requirement: Focused permission presentation

Connection & permissions SHALL lead with permission status and place Reconnecting afterward. Granted permissions without an active limitation SHALL start compact, with details available on demand. Permissions needing attention SHALL initially expose guidance and recovery. Unchanged snapshots SHALL preserve manual disclosure choices; changed facts SHALL re-evaluate the default without prompting.

#### Scenario: Inspect granted permission

- **WHEN** a user expands or collapses a granted permission and unchanged permission facts arrive in a new snapshot
- **THEN** its status remains visible, the chosen disclosure is preserved and no OS permission action occurs

#### Scenario: Permission needs attention

- **WHEN** authorization or relevant OS facts change to require attention
- **THEN** the current explanation and supported recovery action become visible without replaying a permission request
