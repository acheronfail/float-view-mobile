# Spec Delta

## MODIFIED Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement the WebSocket control methods `state.get`, `uiState.get`, `uiState.set`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `controller.shutdown`, `refloat.control.set`, `refloat.fatalError.clear`, `savedDevice.update`, `savedDevice.delete`, `settings.update`, `announcements.test`, `boardProfile.get`, `boardProfile.update`, `session.list`, `session.get`, `session.marker.add`, `session.export`, `session.delete`, `session.delete.cancel`, `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`, plus the authenticated session-data request kinds `series`, `samples`, `events`, `locations`, and `markers`. Session-group methods SHALL NOT remain; `session.getDiagnostics` SHALL NOT remain; exact retained historical reads use `samples`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory control method and session-data request kind is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method or session-data kind outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

## ADDED Requirements

### Requirement: Announcement settings and readiness

The contract SHALL expose complete announcement settings and bounded audio readiness in authoritative state. Settings SHALL use existing revision-checked atomic settings updates and durable ownership. Readiness SHALL be initializing, ready, or unavailable, with a nullable closed reason of no_offline_voice, engine_unavailable, or audio_unavailable. Readiness SHALL not promise that a future utterance is audible.

#### Scenario: Settings persist
- **WHEN** a valid announcement patch is accepted at the current settings revision
- **THEN** the returned settings and subsequent complete snapshots contain the accepted values, including after frontend or process recreation

#### Scenario: Invalid or conflicting save
- **WHEN** an announcement patch is out of bounds, non-finite, has an unknown mode, uses a stale revision, or fails persistence
- **THEN** it fails with the existing applicable structured error and leaves all settings unchanged

#### Scenario: Audio availability changes
- **WHEN** native audio readiness changes
- **THEN** a newer complete state snapshot reports it without changing saved announcement preferences or recording state

### Requirement: Authenticated announcement test

Every host SHALL implement announcements.test with a closed speed/load metric parameter and a terminal outcome of finished, interrupted, or unavailable within ten seconds of admission. The test SHALL use a fixed representative reading through the normal native audio path without requiring a board, changing settings or bypassing global cooldown. Arbitrary text, values and voice identifiers SHALL not be accepted. Invalid and busy requests SHALL use existing structured errors.

#### Scenario: Test with announcements disabled
- **WHEN** an authenticated client tests load while automatic announcements are off and the speech slot and cooldown permit
- **THEN** the host attempts “duty 66 percent, motor 72 percent, battery 80 percent” and returns its terminal outcome without enabling announcements or changing telemetry

#### Scenario: Test speed
- **WHEN** an authenticated client tests speed with the speech slot and cooldown available
- **THEN** it speaks the fixed physical speed of 25 km/h rounded in current display units, identifies those units, and returns a terminal outcome

#### Scenario: Blocked test
- **WHEN** a valid test is admitted but the voice, focus or audio route prevents playback
- **THEN** it returns unavailable without claiming audible success, queuing speech, or disturbing connection or recording

#### Scenario: Invalid request or cooldown
- **WHEN** a test contains an unknown metric or extra speech input, or an utterance or cooldown already occupies admission
- **THEN** the backend rejects invalid input with invalid_request or occupied admission with invalid_state without creating a waiter or speech queue

#### Scenario: Client disconnect
- **WHEN** a client disconnects with its test pending
- **THEN** its waiter is released and its speech is cancelled without cancelling an unrelated operation; the admission cooldown remains in effect
