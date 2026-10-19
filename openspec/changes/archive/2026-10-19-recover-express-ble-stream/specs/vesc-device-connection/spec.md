## ADDED Requirements

### Requirement: Bounded framed-stream recovery

After malformed framed traffic, the shared receiver SHALL reacquire complete CRC-valid frames without trusting corrupted length fields or losing valid packets surrounding corruption. Recovery SHALL retain existing payload and buffer limits and bounded work per input. Normal fragmented frames SHALL retain their boundaries, including frame-like payload contents. Starting a new connection SHALL reset recovery state.

#### Scenario: Corruption between valid responses

- **WHEN** an input contains valid responses before and after a malformed frame
- **THEN** valid responses are delivered in order and malformed payloads are never dispatched

#### Scenario: False incomplete header after corruption

- **WHEN** corrupted traffic leaves an incomplete false candidate ahead of a complete CRC-valid response
- **THEN** recovery can accept the valid response without waiting for the false candidate's declared length

#### Scenario: Legitimate nested frame bytes

- **WHEN** an undamaged fragmented frame contains a complete frame-shaped byte sequence inside its payload
- **THEN** normal assembly preserves the outer frame and does not publish the embedded sequence separately

#### Scenario: Resource bounds and reconnect

- **WHEN** noise or malformed candidates approach resource limits or a replacement connection starts
- **THEN** work and retained bytes remain bounded and previous recovery state cannot affect the new connection
