## ADDED Requirements

### Requirement: Review Tune application before writing

Activating a compatible saved Tune SHALL open a confirmation before any application request. The confirmation SHALL show only changed tunable settings from a fresh current board read to the saved values, with readable labels, units and enum choices. Long differences SHALL scroll with keyboard and touch while the title and actions remain visible. Matching tunes SHALL show no differences and SHALL NOT cause a write.

#### Scenario: Confirm while idle

- **WHEN** the user activates a compatible Tune while the board is idle
- **THEN** the UI shows current and saved values and sends no application request until explicit confirmation

#### Scenario: Confirm while riding

- **WHEN** an eligible riding user opens the confirmation
- **THEN** the modal includes the momentary-cutout warning and retains authoritative live safety gating

#### Scenario: Cancel or matching tune

- **WHEN** the user cancels by button, backdrop, Escape or native Back, or inspects a matching Tune
- **THEN** no application occurs and dismissal restores focus to the activating control

### Requirement: Preserve reviewed Tune application state

Tune application SHALL retain the reviewed saved revision and refresh current board values before confirming. Changed tunable values SHALL refresh the comparison and require explicit review again. Read or apply failure SHALL retain the modal with actionable error feedback and no automatic retry. Connection or identity changes SHALL NOT authorize application using the old preview. Confirmation state SHALL remain transient across frontend recreation.

#### Scenario: Values change during review

- **WHEN** current tunable values change before confirmation
- **THEN** the updated comparison is displayed without an application request and the user must confirm again

#### Scenario: Failed operation

- **WHEN** the current read or application fails
- **THEN** the confirmation remains open with an error and allows dismissal or an explicit retry when eligible

#### Scenario: Connection changes

- **WHEN** the board disconnects or its exact configuration identity changes during review
- **THEN** confirmation cannot apply the old preview to the replacement connection or identity

#### Scenario: Frontend recreation

- **WHEN** the frontend is recreated with an open confirmation
- **THEN** it restores no pending confirmation or application request
