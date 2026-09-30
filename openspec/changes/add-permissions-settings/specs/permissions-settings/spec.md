## Purpose

Give users one reliable place to inspect permissions used by Float View and recover access through the appropriate operating-system controls.

## ADDED Requirements

### Requirement: Applicable permission inventory

Settings SHALL show the runtime permissions used on the current platform, including unresolved permissions, their purpose, current access and affected features. The inventory SHALL remain visible without a board connection and with optional location recording disabled. It SHALL distinguish authorization, limited access, restricted access, unavailable observations, and disabled system services. It SHALL exclude permissions the app does not request and non-actionable install-time declarations.

#### Scenario: Optional recording disabled
- **WHEN** location recording is disabled and Settings is opened
- **THEN** actual location authorization remains visible, described as optional, without requesting permission or enabling capture

#### Scenario: Platform-specific inventory
- **WHEN** permissions are inspected on iOS, macOS, or Android
- **THEN** only applicable permission rows appear, grouped according to user-facing OS controls, with separate background location only where the app requests it

#### Scenario: Partial access or disabled services
- **WHEN** only approximate location or some Bluetooth grants are available, or system services are off
- **THEN** the UI explains the precise limitation and feature impact without representing a granted permission as denied or all access as ready

### Requirement: Individual platform-appropriate recovery

Each recoverable permission SHALL offer the next supported action for that permission: a permission request or clearly labelled system-settings navigation. The backend SHALL select actions from current OS facts. Recovery SHALL respect denial, explain rationale where required, and never request unrelated permissions or silently change recording preferences. Restricted or unsupported recovery SHALL provide an explanation without a misleading retry button.

#### Scenario: Prompt remains available
- **WHEN** the user selects Request access and a prompt is available for that permission
- **THEN** the app presents any required rationale and the native request for that permission or its required OS group

#### Scenario: Prompt blocked
- **WHEN** the user selects Open settings for permission that cannot be re-prompted
- **THEN** the app opens its platform settings destination or relevant permission pane and provides instructions for the required control

#### Scenario: Settings cannot open
- **WHEN** the OS rejects settings navigation or the specific pane is unavailable
- **THEN** the UI reports that navigation failed and provides manual steps without claiming access was granted

#### Scenario: Background access prerequisite
- **WHEN** background location is applicable but foreground location is missing
- **THEN** the UI explains the prerequisite and does not issue a combined foreground/background permission request

#### Scenario: Background settings required
- **WHEN** the platform requires a settings page to grant background location
- **THEN** the action explains the feature and required setting, allows the user to decline, and navigates only after user activation

#### Scenario: Existing access sufficient
- **WHEN** current authorization supports the existing ride-capture mode
- **THEN** the UI does not require broader authorization solely to mark that mode ready; iOS When In Use is foreground-only and Always remains required for rides that begin while locked

### Requirement: Authoritative lifecycle recovery

Permission status SHALL update after native authorization changes and app activation, including returning from Settings. Recreated or reconnected frontends SHALL recover complete current state. Opening the permissions section SHALL not prompt automatically. Permission denial SHALL leave unrelated application functions usable.

#### Scenario: Return without change
- **WHEN** the user returns from Settings without changing permissions
- **THEN** the app refreshes and continues to show actual access rather than a successful grant

#### Scenario: Revocation or expired temporary access
- **WHEN** access changes outside the app or expires while inactive
- **THEN** activation refreshes the inventory and affected feature readiness and normal revocation handling stops unauthorized operations

#### Scenario: Client recreation
- **WHEN** the WebView reconnects or is recreated after a permission change
- **THEN** a complete monotonic snapshot reconstructs current access without relying on frontend request history

### Requirement: Accessible bounded interaction

Actions SHALL work on desktop and touch with native button semantics and text status. One activation SHALL produce at most one permission presentation. Actions SHALL be disabled while disconnected or presentation is pending; failures SHALL allow a later explicit retry without an automatic prompt loop.

#### Scenario: Pointer and keyboard activation
- **WHEN** a primary pointer or keyboard/assistive action activates a recovery control
- **THEN** exactly one request occurs, while non-primary or cancelled pointer interactions cause none

#### Scenario: Concurrent or abandoned requests
- **WHEN** multiple clients request recovery concurrently or the OS never completes presentation
- **THEN** at most one presentation is admitted, resource use stays bounded, and timeout or failure releases the pending state without falsely granting access
