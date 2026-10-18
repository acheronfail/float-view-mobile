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

Live SHALL offer Disconnect and End ride; End ride SHALL finalize and disconnect, with confirmation unless the latest ride state is idle. Home SHALL show a paused-ride card with board, remaining window and End ride; the paused session viewer SHALL show the same status/action. Disconnected completion SHALL be immediate and retain qualifying data. Controls SHALL support primary pointer and guarded keyboard activation.

#### Scenario: Finish a paused ride
- **WHEN** the user activates End ride on Home or the paused session viewer
- **THEN** the ride is finalized once without reconnecting or requesting confirmation

#### Scenario: Finish while riding
- **WHEN** the user selects End ride on Live while riding or ride state is unknown
- **THEN** completion and disconnection occur only after confirmation
