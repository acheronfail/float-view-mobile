# Spec Delta

## MODIFIED Requirements

### Requirement: Global settings destination

Settings SHALL contain only application-wide appearance, session-retention, session-linking, logging cadence, post-ride, reconnect, and ride-announcement options. Board-specific values MUST NOT be edited from Settings, and application-wide settings MUST NOT remain embedded in session browsing.

#### Scenario: Open global settings

- **WHEN** the user selects Settings
- **THEN** the frontend renders the current authoritative global options grouped by purpose

#### Scenario: Native-owned option update

- **WHEN** the user changes a valid global option
- **THEN** the frontend sends a revision-checked settings patch and renders the backend-returned settings

## ADDED Requirements

### Requirement: Announcement settings controls

Settings SHALL provide independent speed/load switches and thresholds with explicit units, a shared repeat interval, announcement volume, audio readiness, and a test action. The same frontend SHALL support pointer and keyboard activation without duplicate actions. Validation and save/test failures SHALL preserve edits and remain visible. A test SHALL identify its selected speed, load or combined content even when both rules are off. Unavailable audio SHALL not silently disable the saved rule.

#### Scenario: Configure announcements
- **WHEN** users enable load and set 65 percent with a five-second repeat interval
- **THEN** Settings saves a revision-checked patch and presents authoritative accepted values

#### Scenario: Test feedback
- **WHEN** users activate a test by primary pointer or keyboard
- **THEN** exactly one test is sent, repeat activation is prevented while pending, and the terminal outcome is displayed without claiming that a finished utterance was necessarily heard

#### Scenario: Audio unavailable
- **WHEN** audio readiness is unavailable or a test cannot play
- **THEN** Settings explains the bounded reason or outcome and retains the user's announcement configuration and other working controls

### Requirement: Conditional audio lowering control

Settings SHALL show “Lower other audio while speaking” only when authoritative announcement audio state says lowering is adjustable. Helper text SHALL explain that music and other audio become quieter temporarily and that some apps may pause. Android and macOS SHALL hide this control while retaining announcement volume and other common settings. The frontend SHALL derive visibility from the mandatory state field, not detect the platform.

#### Scenario: iOS audio controls
- **WHEN** Settings receives adjustable lowering state
- **THEN** it shows the saved lowering switch and its explanation alongside announcement volume

#### Scenario: Android or macOS audio controls
- **WHEN** Settings receives automatic lowering state
- **THEN** it hides the lowering switch without hiding speech volume, thresholds, enable switches or testing
