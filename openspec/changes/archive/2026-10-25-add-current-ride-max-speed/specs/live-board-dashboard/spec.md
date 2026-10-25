## ADDED Requirements

### Requirement: Maximum speed dashboard reading

Live slots and Custom widgets SHALL offer Max speed from the same authoritative current-ride peak. It SHALL use the selected speed units and one decimal place, show unavailable data consistently, and support the existing numeric widget presentations and revisioned settings/profile saves on both hosts.

#### Scenario: Shared metric and imperial display

- **WHEN** Max speed is selected in Live and Custom
- **THEN** both show the same current-ride peak in km/h or mph according to the unit preference

#### Scenario: Configuration and recreation

- **WHEN** a Max speed choice is saved and the view is recreated
- **THEN** the choice survives and displays the shared current-ride peak without starting another collector
