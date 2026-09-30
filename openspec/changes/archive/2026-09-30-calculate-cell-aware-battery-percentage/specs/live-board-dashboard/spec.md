# Spec Delta

## ADDED Requirements

### Requirement: Shared estimated battery percentage in live views

The live power widget and detailed view SHALL display the authoritative estimated fraction rounded to the same nearest integer percentage. The widget fill, color thresholds, and accessible percentage SHALL use that estimate. The detailed view SHALL label it “Estimated battery level” and retain the raw value separately as “Controller-reported battery level”. Neither primary display SHALL substitute raw controller percentage when the estimate is unavailable. Both SHALL retain existing stale/dimmed connection behavior, charging indication, and raw pack voltage. Widget per-cell voltage SHALL use the same resolved series count as estimation and be unavailable when no count can be resolved.

#### Scenario: Controller percentage disagrees
- **WHEN** an active sample reports controller level 0.8 and the shared estimate is 0.5
- **THEN** both primary percentage displays and widget fill use 50 percent
- **AND** the detailed controller-reported field shows 80 percent

#### Scenario: Missing estimate
- **WHEN** the active board is not configured for estimation
- **THEN** both percentage displays show their unavailable placeholder, the widget has no fabricated charge fill, and pack voltage remains visible

#### Scenario: Connection becomes nonready
- **WHEN** the last sample and matching estimate are retained while reconnecting
- **THEN** both views retain the same percentage using their existing dimmed/stale presentation

### Requirement: Saved board battery estimation controls

The saved-board editor SHALL offer the bounded cell-profile choices and a display-empty voltage per cell alongside existing series/parallel inputs. It SHALL explain the 3.0 V default and that the setting changes the displayed estimate, not firmware protection thresholds. Controls SHALL use existing revisioned save behavior, be accessible to keyboard and primary-pointer input on desktop and touch, and preserve configuration across view recreation. Missing estimation configuration SHALL NOT introduce a profile-completeness warning or block unrelated board operations.

#### Scenario: Configure a saved board
- **WHEN** the user selects P45B, valid pack counts, and a valid empty voltage
- **THEN** the configuration is saved through the existing profile request and both live views use the resulting shared estimate

#### Scenario: Keyboard and touch configuration
- **WHEN** the user activates or changes an estimation control with keyboard or primary touch input
- **THEN** the intended interaction occurs once without duplicate saves caused by synthesized clicks
