# Spec Delta

## MODIFIED Requirements

### Requirement: Ordered restoration and target validation

Restoration SHALL run once per frontend startup after authentication and fresh authoritative state. It SHALL precede recovered-connection fallback navigation, suppress capture of initialization defaults, and open surfaces only after required data loads. Missing targets SHALL fall back to the nearest valid parent; transient failures SHALL retain the checkpoint and expose retry without repeated automatic restore loops.

#### Scenario: Initial loading

- **WHEN** the initial checkpoint and authoritative state are still loading
- **THEN** the frontend presents neutral loading feedback without mounting Home to scan or publishing a default checkpoint over the saved one; independently triggered bounded foreground discovery and auto-connect may proceed without changing restoration order

#### Scenario: Session no longer exists

- **WHEN** the restored session has been removed or discarded
- **THEN** the frontend reports that it is unavailable and returns to its known existing board history or otherwise Sessions

#### Scenario: Board no longer exists

- **WHEN** a restored board history or Board Profile target has been removed
- **THEN** the frontend falls back to Sessions or closes the Live configuration surface while preserving a valid Live route

#### Scenario: Changed active device or unavailable feature

- **WHEN** the active device differs from the saved Live target or its panel is no longer eligible
- **THEN** the frontend keeps an appropriate current connection route but skips the old device-specific presentation

#### Scenario: Load fails temporarily

- **WHEN** checkpoint retrieval or required screen data fails for an operational reason
- **THEN** the frontend offers explicit retry, retains the accepted checkpoint and does not classify the target as missing

#### Scenario: Explicit incoming destination

- **WHEN** a new frontend starts with an explicit non-root destination different from its saved route
- **THEN** it honors that navigation intent rather than replacing it with the checkpoint; root startup and reloading the saved destination remain eligible for restoration

#### Scenario: Ordinary transport reconnect

- **WHEN** the existing frontend reconnects its transport without being recreated
- **THEN** its current UI remains in place and startup restoration is not rerun

#### Scenario: Lifecycle discovery during restoration

- **WHEN** a foreground auto-connect window starts while restoration is pending
- **THEN** discovery does not require Home or replay checkpoint actions, and a resulting connection does not supersede an eligible non-live restored destination
