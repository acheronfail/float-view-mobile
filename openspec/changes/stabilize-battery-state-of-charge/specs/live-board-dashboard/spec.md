## ADDED Requirements

### Requirement: Pack capacity and temporal estimate explanation

Battery configuration SHALL offer optional full-pack capacity in Ah with validation from 0.1 through 200. Basic preset selection SHALL provide sourced nominal capacity when known and a summary of actual configured values; Basic SHALL NOT require capacity entry or expose a generic/approximation label or missing-curve warning. Advanced SHALL allow manual capacity entry, override, and clear, and explain that parallel cells multiply cell Ah while series cells do not. Advanced help or battery details SHALL explain that absent capacity uses a smoothed voltage estimate, while capacity enables charge tracking with gradual settled-voltage correction. Battery help or detailed presentation SHALL identify the current estimation method and provisional status without implying measured absolute accuracy. The full board editor and battery-only configuration surface SHALL use the same Basic/Advanced field semantics and revisioned persistence. Mode switches, reads, and reconnects SHALL NOT restore cleared or overridden capacity; explicit preset selection SHALL restore its defaults. Capacity SHALL NOT be inferred from a curve name or series count. Missing capacity SHALL NOT trigger the missing-configuration warning or controller-reported fallback. Both live views SHALL use the same authoritative temporal percentage, while raw voltage, raw controller percentage, charging state, and firmware alerts SHALL retain their existing responsiveness.

#### Scenario: Optional capacity entry

- **WHEN** a user enters 9 Ah for a 2P pack using 4.5 Ah cells
- **THEN** the saved capacity is 9 Ah regardless of series count and invalid entries receive validation feedback

#### Scenario: No capacity supplied

- **WHEN** an otherwise configured board has no capacity
- **THEN** its voltage-smoothed estimate remains available and no missing-configuration warning is added because of capacity

#### Scenario: Provisional estimate

- **WHEN** a connection starts under load or resumes after missing telemetry
- **THEN** battery help or details identifies the estimate as provisional until settled convergence

#### Scenario: Sudden voltage sag

- **WHEN** load sharply reduces measured voltage while the tracked percentage changes slowly
- **THEN** voltage and any firmware low-voltage alert update normally without waiting for percentage smoothing

#### Scenario: Shared controls and input

- **WHEN** capacity is edited from the full board editor or an available battery-only modal using primary pointer or keyboard
- **THEN** validation and saving behave identically, occur without duplicate activation, and accepted values survive view recreation

#### Scenario: Preset nominal capacity and override

- **WHEN** a rider selects an ADV2 preset
- **THEN** the saved pack capacity is 10 Ah and Basic summarizes nominal capacity without requiring manual cell calculations
- **WHEN** the rider changes capacity to 8.7 Ah or clears it in Advanced and returns to Basic
- **THEN** the entered value or absence remains through reconnect and view recreation, and no mode switch reapplies the nominal value

#### Scenario: Generic curve with capacity

- **WHEN** a preset with no dedicated curve has valid nominal capacity and observed current
- **THEN** the shared estimator can track charge with its generic voltage observation
- **AND** Basic shows its familiar battery name without generic/approximation wording or a missing-curve warning

#### Scenario: Deliberate preset reset

- **WHEN** a user explicitly re-selects a preset after overriding or clearing capacity
- **THEN** the preset's capacity and other battery defaults are restored in one accepted revision
