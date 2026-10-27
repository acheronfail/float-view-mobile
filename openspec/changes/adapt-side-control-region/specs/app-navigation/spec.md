## MODIFIED Requirements

### Requirement: Route-aware bottom tab bar

The frontend SHALL render primary tabs on exactly Home, Sessions, and Settings. Tabs SHALL map `connect` to Home, `sessions` to Sessions, and `settings` to Settings, with recognizable icons, lowercase labels and a visible selected state. Tabs SHALL use a verified usable side control region when available, otherwise the bottom edge. A Boards tab and Devices/Sessions segmented control MUST NOT remain.

#### Scenario: Top-level tab bar

- **WHEN** the user views Home, Sessions, or Settings without a usable side control region
- **THEN** the bottom tab bar presents the three icon-and-text destinations and identifies the current destination

#### Scenario: Select a tab

- **WHEN** the user selects `connect`, `sessions`, or `settings`
- **THEN** the frontend navigates to the mapped top-level route without reloading the document or reconnecting the backend client

#### Scenario: Subordinate route

- **WHEN** the user views Live, saved-board history, session detail, or board configuration
- **THEN** primary tabs are absent and the focused view supplies its own return navigation

#### Scenario: Superseded Boards navigation

- **WHEN** the routed application shell is rendered
- **THEN** no Boards tab, Boards top-level route, or Devices/Sessions segmented control is presented

#### Scenario: Side primary navigation

- **WHEN** the host provides a usable side control region
- **THEN** primary tabs occupy that region without overlapping system UI, and the content no longer reserves bottom-tab space

## ADDED Requirements

### Requirement: Adaptive side controls

When a verified usable side region exists, the shared frontend SHALL place header actions and routed back controls there and release their former space for content. Foreground content and controls MUST avoid system-reserved regions. Control placement SHALL update with geometry changes and SHALL fall back to ordinary placement when safe geometry is unavailable or insufficient.

#### Scenario: Live controls

- **WHEN** Live is displayed with a usable side region
- **THEN** its header action buttons occupy that region while retaining their state, labels and actions

#### Scenario: Routed return navigation

- **WHEN** a subordinate page with a back control is displayed with a usable side region
- **THEN** its existing back destination is reachable from that region without a duplicate header control

#### Scenario: Resize or changed system obstruction

- **WHEN** the available region changes or disappears
- **THEN** controls update without a document reload, backend reconnection, lost focus or overlap with system UI

#### Scenario: Insufficient region

- **WHEN** the region cannot safely contain the controls
- **THEN** the application retains accessible ordinary controls rather than clipping or hiding navigation

#### Scenario: Input methods and overlays

- **WHEN** controls are activated by primary pointer, non-primary pointer, keyboard or assistive click, or a modal surface opens
- **THEN** existing activation guards, focus containment and dismissal behavior remain effective without duplicate activation or interactive controls behind the modal
