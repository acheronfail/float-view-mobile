## ADDED Requirements

### Requirement: Durable interruption provenance

The service SHALL attempt to persist each interruption before recording detach. Explicit user disconnect/native user stop SHALL record `pause`; automatic recovery SHALL record `connectionLoss`; other interruptions SHALL remain `unknown`. A saved reason SHALL survive reopen/restart and appear in the resumed boundary. Attempts SHALL be bounded and idempotent; storage failure SHALL be reported without blocking disconnect or erasing data.

#### Scenario: Pause before restart

- **WHEN** a user disconnects and the process restarts before the ride resumes
- **THEN** resumed history retains the deliberate pause reason

#### Scenario: Connection loss followed by deliberate pause

- **WHEN** automatic recovery begins and the user explicitly disconnects before new samples arrive
- **THEN** the resumed boundary records a deliberate pause
- **AND** repeated recovery notifications do not create unbounded interruption events

#### Scenario: Persistence fails before detach

- **WHEN** saving the interruption fails
- **THEN** Bluetooth disconnect and recording release still proceed, disconnected state is published and the cleanup failure is reported
- **AND** a reason which was not persisted does not become deliberate-pause provenance

### Requirement: Disconnect is independent of storage cleanup

Explicit disconnect and native user stop SHALL issue the reserved Bluetooth disconnect, cancel connection work/location, release recording ownership and publish disconnected state regardless of storage cleanup failures. Cleanup SHALL continue after an earlier error and report errors afterwards. Catalog/expiry preflight SHALL NOT block disconnect. Committed data SHALL remain recoverable.

#### Scenario: Detach or catalog failure

- **WHEN** reason persistence, writer detach/checkpoint/close, draft cleanup or catalog updates fail during disconnect
- **THEN** the native disconnect effect remains available, no recording writer or stale sample marker remains attached and the public connection is cleared
- **AND** cleanup failures are reported after disconnected state is published

#### Scenario: Storage fails before request dispatch

- **WHEN** catalog work for expired rides or connection drafts would fail when the user requests disconnect
- **THEN** disconnect bypasses fallible catalog preflight and still releases the connection

## MODIFIED Requirements

### Requirement: Compact confirmed recording breaks

Ride charts SHALL collapse only recording-break intervals whose persisted reason is `pause` into narrow vertical separators, with original elapsed endpoints and missing duration available in accessible text. Traces SHALL remain disconnected. All field charts, markers, fault markers, inspection, selections, zoom and pan SHALL use the same time mapping while preserving original elapsed timestamps in labels, requests and map synchronization. Map filtering SHALL compose with deliberate pauses. Automatic connection losses, unknown historical interruptions, ordinary telemetry delays, missing field values and recorded stationary time SHALL retain their existing missing-time width without pause markers. Intent SHALL NOT be inferred from duration or telemetry.

#### Scenario: Rest within a ride

- **WHEN** a ride includes a persisted deliberate pause
- **THEN** charts replace its empty width with a vertical break marker
- **AND** inspection on either side reports original ride times without connecting the traces

#### Scenario: Automatic or unknown interruption

- **WHEN** telemetry resumes after connection loss or a boundary without a recorded reason
- **THEN** its original missing-time width and disconnected traces remain without a pause marker

#### Scenario: Short unconfirmed delay

- **WHEN** telemetry samples have a timing delay without a recording-boundary event
- **THEN** the existing missing-time width and trace break remain

#### Scenario: Interactions and filters

- **WHEN** the user inspects, selects, zooms, pans or filters a ride containing deliberate pauses and other interruptions
- **THEN** charts and map selection remain synchronized to original ride times
- **AND** pauses outside the visible range do not produce separators
- **AND** a range containing only missing time shows no fabricated readings
