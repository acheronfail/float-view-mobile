## ADDED Requirements

### Requirement: Compact confirmed recording breaks

Ride charts SHALL collapse only confirmed recording-break intervals into narrow vertical separators, with original elapsed endpoints and missing duration available in accessible text. Traces SHALL remain disconnected. All field charts, markers, fault markers, inspection, selections, zoom and pan SHALL use the same time mapping while preserving original elapsed timestamps in labels, requests and map synchronization. Map filtering SHALL compose with recording breaks. Ordinary telemetry delays, missing field values and recorded stationary time SHALL retain their existing presentation.

#### Scenario: Rest within a ride
- **WHEN** a ride includes a confirmed multi-minute recording interruption
- **THEN** charts replace its empty width with a vertical break marker
- **AND** inspection on either side reports original ride times without connecting the traces

#### Scenario: Short unconfirmed delay
- **WHEN** telemetry samples have a timing delay without a recording-boundary event
- **THEN** the existing missing-time width and trace break remain

#### Scenario: Interactions and filters
- **WHEN** the user inspects, selects, zooms, pans or filters a ride containing confirmed breaks
- **THEN** charts and map selection remain synchronized to original ride times
- **AND** breaks outside the visible range do not produce separators
- **AND** a range containing only missing time shows no fabricated readings
