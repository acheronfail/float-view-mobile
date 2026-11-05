## ADDED Requirements

### Requirement: Correlated bulk export progress and cancellation

Board export SHALL accept an optional bounded operationId. Authenticated `session.export.status` and `session.export.cancel` SHALL address that ID across the existing control and HTTP transports. The process owner SHALL retain at most one ephemeral progress record with phase and completed/total counts, without paths or session collections. Progress SHALL NOT change product revisions. Accepted cancellation SHALL stop encoding and prevent delivery.

#### Scenario: Unknown or stale operation

- **WHEN** progress or cancellation names an unknown or replaced operation
- **THEN** it returns not_found without affecting another export

#### Scenario: Cancellation races completion

- **WHEN** cancellation arrives before worker completion is accepted
- **THEN** cancellation is idempotent, temporary resources remain owned until the worker exits, and no partial artifact is presented
- **AND** cancellation after preparation completes returns ready without claiming that the export was cancelled

#### Scenario: Shared host policy

- **WHEN** web, Apple or Android starts a correlated board export
- **THEN** the same shared worker reports bounded monotonic completed counts and honours cancellation without new host interfaces

### Requirement: Bulk export disk admission

Bulk export SHALL support ZIPs larger than the individual-session bound using a separate bounded disk budget. It SHALL check estimated free storage before encoding and between sessions, preserve the existing individual archive compatibility bounds, and report actionable storage or size failures. A failed export SHALL release temporary files and preserve all source sessions.

#### Scenario: Insufficient storage

- **WHEN** estimated free storage is insufficient or storage is exhausted during encoding
- **THEN** export fails without presenting a partial ZIP and explains that storage must be freed before retrying

#### Scenario: Large board backup

- **WHEN** eligible individual sessions fit their supported bounds and together exceed the individual-session byte limit
- **THEN** the board exports successfully within the separate bulk ZIP and disk budgets
