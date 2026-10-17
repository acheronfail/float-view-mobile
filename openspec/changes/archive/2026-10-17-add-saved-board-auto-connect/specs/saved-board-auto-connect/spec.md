# Spec Delta

## Purpose

Connect to one explicitly preferred saved board during a bounded discovery opportunity on each foreground visit, while respecting manual connection intent and restored navigation.

## ADDED Requirements

### Requirement: Single installation-local preference

Settings SHALL expose nullable `autoConnectSavedDeviceId`, defaulting to null, through existing revision-checked atomic settings operations. A non-null value MUST identify an existing saved board with a local Bluetooth association. The preference SHALL persist locally and SHALL NOT be carried by portable board/session archives.

#### Scenario: Select replace and clear

- **WHEN** a current-revision settings patch selects an eligible board, replaces it with another eligible board, or clears the value
- **THEN** the returned settings and complete snapshots contain exactly that selection, including after process restart

#### Scenario: Invalid or failed update

- **WHEN** a patch selects an unknown board or one without a local association, uses a malformed value or stale revision, or fails persistence
- **THEN** it fails with the applicable existing structured error and leaves the whole patch unapplied
- **AND** unknown boards return `not_found`, missing local associations return `invalid_state`, malformed values return `invalid_request`, and stale revisions return `conflict`

#### Scenario: Selected board deletion

- **WHEN** deletion of the selected inactive board succeeds
- **THEN** its selection is atomically cleared and the settings revision advances in the published complete state
- **AND** rejected deletion or failure before board-removal commit does not clear the preference; cleanup failure after committed removal retains the cleared preference

#### Scenario: Import isolation

- **WHEN** a board/session archive is exported or imported
- **THEN** the archive neither carries nor changes the local auto-connect preference
- **AND** an imported board without a local association cannot be selected until local association is established

### Requirement: Saved-board auto-connect control

Saved-board settings SHALL provide a “Connect automatically” toggle reflecting the single authoritative preference. Enabling a board SHALL replace the previous selection; disabling the selected board SHALL clear it. Boards without local association SHALL show the control unavailable with an explanation. The control SHALL support primary-pointer and keyboard/assistive activation without double submission.

#### Scenario: Change preferred board

- **WHEN** the user enables board B while board A is selected
- **THEN** one revision-checked update selects B and subsequent board settings show A disabled and B enabled
- **AND** a failed save retains the prior authoritative selection and exposes the existing save error

#### Scenario: Pointer and keyboard activation

- **WHEN** a primary pointer or keyboard/assistive activation operates an eligible toggle
- **THEN** exactly one preference update is submitted
- **AND** non-primary pointer input does not change the preference

### Requirement: Bounded foreground discovery opportunity

Each foreground visit SHALL offer at most one 15-second automatic discovery window when a preferred board exists and no connection is active. It SHALL start after disclaimer acceptance, powered-on Bluetooth, and required scan/connect permissions are ready. It SHALL use normal bounded discovery regardless of the displayed route, without initiating permission prompts. Initial foreground startup SHALL count as one visit.

#### Scenario: Startup or return on another screen

- **WHEN** the app starts foregrounded or returns from background with a selected board, readiness satisfied, and no active connection
- **THEN** it begins the discovery opportunity even if Settings, Sessions, or board history is displayed
- **AND** repeated foreground observations do not create another window

#### Scenario: Readiness arrives later

- **WHEN** Bluetooth, permission, or disclaimer readiness is initially missing and becomes satisfied during the same foreground visit
- **THEN** the 15-second window starts once readiness is satisfied
- **AND** backgrounding first cancels that pending opportunity without deferred scans or prompts

#### Scenario: Window expires while Home keeps scanning

- **WHEN** 15 seconds elapse after window start without an automatic connection
- **THEN** automatic eligibility ends and discovery requested solely by that window stops
- **AND** Home may continue normal discovery, but finding the preferred board later cannot auto-connect during that visit

#### Scenario: Existing active connection

- **WHEN** the visit begins with an active connecting, identifying, selecting-controller, ready, reconnecting, or failed connection
- **THEN** that connection takes precedence and no automatic window is started or queued for later in that visit

### Requirement: Exact discovery-based automatic admission

During an eligible window, a fresh normal discovery result matching the preferred board's exact local Bluetooth association SHALL initiate the existing saved-mode connection flow and consume automatic eligibility before starting the attempt. Names, signal strength, and stale cached discoveries SHALL NOT select a board. At most one automatic attempt SHALL be admitted per foreground visit.

#### Scenario: Preferred board discovered

- **WHEN** a current-window discovery result matches the preferred board, including an advertisement without the UART service
- **THEN** the existing saved identification, controller-selection, session, and reconnect behavior applies
- **AND** duplicate advertisements cannot initiate another attempt

#### Scenario: Other or stale board

- **WHEN** another saved board, a same-name device, an unmatched device, or a result from an obsolete scan is observed
- **THEN** it does not trigger automatic connection
- **AND** existing normal discovery visibility rules still apply

#### Scenario: Automatic attempt fails

- **WHEN** the automatic attempt fails setup or identification, or subsequent ordinary reconnect policy terminates
- **THEN** repeated advertisements do not retry automatically during that visit
- **AND** existing manual retry and disconnect actions remain available

### Requirement: Foreground cancellation and manual precedence

Backgrounding, explicit disconnect, or a valid manual connection command SHALL cancel remaining automatic eligibility. A new background-to-foreground transition SHALL rearm it. WebView recreation, transport reconnection, readiness recovery, and preference edits SHALL NOT rearm a consumed or cancelled opportunity within the same visit. Existing established-connection background and reconnect policy SHALL remain authoritative.

#### Scenario: Disconnect then return

- **WHEN** the user explicitly disconnects, remains foregrounded, then backgrounds and foregrounds the app
- **THEN** no automatic connection occurs before backgrounding and a fresh opportunity is available on return without restarting the native process

#### Scenario: Manual intent wins ordering

- **WHEN** a valid manual connection command is processed before a matching discovery result
- **THEN** the command cancels automatic eligibility even if its connection later fails
- **AND** the later advertisement cannot replace the user's choice

#### Scenario: Background cancels only search

- **WHEN** the app backgrounds while automatic discovery is pending or searching
- **THEN** that opportunity is cancelled and its scan demand stops
- **AND** a connection already admitted continues under existing connection/background policy

#### Scenario: Foreground continuity and late callbacks

- **WHEN** the frontend or socket is recreated without a background-to-foreground transition, or an old timer/result arrives after cancellation or a newer visit
- **THEN** no new opportunity is armed and obsolete callbacks cannot trigger a connection or extend the current deadline

#### Scenario: Preference changes during a visit

- **WHEN** selection changes during an unconsumed searching window
- **THEN** subsequent current results use the new selected association without extending the deadline
- **AND** clearing selection cancels the opportunity; changing selection after consumption/cancellation waits until the next visit

#### Scenario: Readiness lost during search

- **WHEN** Bluetooth or required permission readiness is lost after the window begins
- **THEN** scanning stops while unavailable and the original deadline keeps running
- **AND** recovery before that deadline may resume ordinary discovery without extending the opportunity
