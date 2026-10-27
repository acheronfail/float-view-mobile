# ride-continuity Specification

## Purpose

Keep a saved ride continuous across deliberate disconnects, charging breaks and process replacement while preserving honest gaps in recorded data.

## Requirements

### Requirement: Bounded durable ride pause

Disconnect SHALL release Bluetooth and stop automatic retries while leaving one saved ride resumable. Resume eligibility SHALL survive process restart without initiating Bluetooth work. The window SHALL start at disconnection, default to 60 minutes, and accept whole minutes from 1 through 1,440 independently of Bluetooth retry duration. Retention SHALL run only at final closure.

#### Scenario: Charging break and restart
- **WHEN** the user disconnects, the process restarts and the same saved board and verified controller reconnect before expiry
- **THEN** recording resumes under the original session identity with increasing sample/event ordering and elapsed time

#### Scenario: Boundaries and expiry
- **WHEN** the window expires, another board/controller is selected, or End ride is accepted
- **THEN** the paused ride is finalized before any new recording, and stale timers cannot close a replacement ride

#### Scenario: Repeated interruption and failure
- **WHEN** a resumed ride disconnects again
- **THEN** a new window begins; persistence failures preserve committed data and never report a successful resume

### Requirement: Honest recording gaps

Pauses and reconnects SHALL preserve missing intervals in elapsed time without adding unobserved riding time or estimated distance. Telemetry charts and GPS paths SHALL break at interruptions. Existing supported recordings and archive imports SHALL remain readable.

#### Scenario: Ride resumes at speed
- **WHEN** riding samples occur on both sides of an offline interval
- **THEN** summaries do not integrate speed or riding time across that interval and views do not fabricate connecting data

### Requirement: Shared ride completion actions

Live SHALL place End ride after Disconnect. Every End ride action SHALL require confirmation explaining that reconnecting starts a new ride. Confirmed completion SHALL finalize the ride and disconnect when connected. Home SHALL show a paused-ride card with board, remaining window and End ride; the paused session viewer SHALL show the same status/action. Cancellation SHALL retain the ride; qualifying data SHALL remain saved. Controls SHALL support primary pointer and guarded keyboard activation.

#### Scenario: Finish a paused ride
- **WHEN** the user activates End ride on Home or the paused session viewer
- **THEN** a confirmation appears; cancellation preserves the ride and acceptance finalizes it once without reconnecting

#### Scenario: Finish while riding
- **WHEN** the user selects End ride on Live in any ride state
- **THEN** completion and disconnection occur only after confirmation

#### Scenario: Menu order
- **WHEN** the Live menu shows both Disconnect and End ride
- **THEN** End ride appears after Disconnect


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

### Requirement: Current ride maximum speed

The service SHALL publish the greatest absolute finite speed observed in the current ride, or unavailable before any valid speed. Reconnects and view recreation SHALL preserve it. Resuming a saved ride SHALL restore its committed peak. A new ride SHALL clear it. Tracking SHALL use bounded state and SHALL NOT estimate speeds in recording gaps or change supported recordings.

#### Scenario: Forward and reverse peaks

- **WHEN** a ride observes speeds of 20, -30 and 10 km/h
- **THEN** its maximum is 30 km/h and later missing speed does not erase it

#### Scenario: Reconnect and restart

- **WHEN** the same ride reconnects or resumes after process replacement
- **THEN** its retained or committed peak remains and increases only for a greater observed speed

#### Scenario: New ride and unavailable data

- **WHEN** a new ride starts without valid speed or the prior ride has no saved peak
- **THEN** its maximum is unavailable until a finite speed is observed and no previous ride peak leaks

#### Scenario: Persistence failure

- **WHEN** a ride cannot be restored because storage fails
- **THEN** the existing failure is reported without fabricating a successful restoration or deleting supported data

### Requirement: Shared current-ride live distance

Live distance widgets, Live Activity and Dynamic Island SHALL display accumulated current-ride distance using observed speed over time, restored from saved distance on resume. Raw board trip resets SHALL NOT reset this total. Reconnect and missing-speed intervals SHALL add no unobserved distance. New rides SHALL start at zero. Accumulation SHALL use bounded state and preserve raw historical trip telemetry and supported recordings.

#### Scenario: Board powers off during a break

- **WHEN** a saved ride resumes after the board restarts and reports zero trip distance
- **THEN** every live distance display retains the saved ride distance and adds subsequent observed travel

#### Scenario: Reconnect without board reset

- **WHEN** the same ride reconnects repeatedly while the board trip counter retains its value
- **THEN** accumulated distance is retained without adding the board's previous trip distance again or integrating offline time

#### Scenario: Process replacement

- **WHEN** the app restarts and resumes an eligible saved ride
- **THEN** the first live distance uses the restored saved total without integrating the interruption

#### Scenario: Missing speed or failed persistence

- **WHEN** speed is missing or a saved sample fails persistence
- **THEN** no travel is fabricated across the missing observation and no failed saved sample is published as committed

#### Scenario: New ride and temporary connection

- **WHEN** a new saved ride or temporary connection begins
- **THEN** live distance starts at zero and advances only from observed travel in that ride

### Requirement: Extend a paused ride

Home and the paused session viewer SHALL offer Give me more time beside End ride. Each accepted extension SHALL retain the ride identity and restart the configured resume window from backend time without shortening the existing deadline. Extensions SHALL survive process restart and replace expiry scheduling without initiating Bluetooth or recording work.

#### Scenario: Repeated extension and resume

- **WHEN** a rider extends an unexpired paused ride repeatedly and later reconnects before the renewed deadline
- **THEN** the countdown reflects the latest deadline and recording resumes the same ride with the existing honest recording gap

#### Scenario: Restart and stale expiry

- **WHEN** the process restarts after an extension or an old expiry callback arrives
- **THEN** the renewed deadline remains authoritative and the ride closes only at that deadline

#### Scenario: Interactive controls and failure

- **WHEN** the action is activated by a primary pointer or keyboard
- **THEN** one request is sent, duplicate activation is guarded while pending, and failures appear on the card without claiming success
- **AND** non-primary pointers do not activate the action

#### Scenario: Clear pause wording and feedback

- **WHEN** a paused ride is shown
- **THEN** the card explains the remaining reconnect time and that reconnecting after expiry starts a new ride
- **WHEN** Give me more time succeeds
- **THEN** the updated countdown is shown with a brief confirmation of the remaining reconnect time
- **AND** failed requests show an error without success feedback
