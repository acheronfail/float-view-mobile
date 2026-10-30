## ADDED Requirements

### Requirement: Net consumption readings

Live boxes and Custom widgets SHALL offer Net watt-hours (`wattHoursNetWh`, Wh net) and Net amp-hours (`ampHoursNetAh`, Ah net), computed as drawn minus charged from the current sample. Both SHALL retain signed values, use the same units regardless of unit preference, and persist through existing settings and board-profile contracts on Apple and Android without altering other selections or supported stored data.

#### Scenario: Select and reload net readings

- **WHEN** a user selects either net reading in a Live box or Custom number/chart widget and saves
- **THEN** the selection survives reload and shows the same derived value in both views

#### Scenario: Consumption and regeneration

- **WHEN** drawn and charged watt-hours are 120 and 20
- **THEN** net watt-hours displays 100.0 Wh with the draw colour
- **WHEN** charged exceeds drawn
- **THEN** the net reading remains negative with the charging colour rather than clamping to zero
- **AND** Wh uses one decimal and Ah uses two; zero uses a neutral colour

#### Scenario: Missing telemetry and reconnect

- **WHEN** either source counter is missing or nonfinite, including while telemetry is unavailable after reconnect
- **THEN** the net reading is unavailable rather than assuming a zero counter or retaining a derived stale value
- **WHEN** both counters become available again
- **THEN** the net reading reflects their current difference
