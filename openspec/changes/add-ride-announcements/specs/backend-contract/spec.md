# Spec Delta

## MODIFIED Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement the WebSocket control methods `state.get`, `uiState.get`, `uiState.set`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `controller.shutdown`, `refloat.control.set`, `refloat.fatalError.clear`, `savedDevice.update`, `savedDevice.delete`, `settings.update`, `announcements.test`, `announcements.openVoiceSettings`, `boardProfile.get`, `boardProfile.update`, `session.list`, `session.get`, `session.marker.add`, `session.export`, `session.delete`, `session.delete.cancel`, `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`, plus the authenticated session-data request kinds `series`, `samples`, `events`, `locations`, and `markers`. Session-group methods SHALL NOT remain; `session.getDiagnostics` SHALL NOT remain; exact retained historical reads use `samples`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities, except that the mandatory announcement audio state SHALL determine visibility of the iOS-only audio-lowering control.

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
- **WHEN** an announcement patch is out of bounds, non-finite, has invalid enable flags or volume, uses a stale revision, or fails persistence
- **THEN** it fails with the existing applicable structured error and leaves all settings unchanged

#### Scenario: Audio availability changes
- **WHEN** native audio readiness changes
- **THEN** a newer complete state snapshot reports it without changing saved announcement preferences or recording state

### Requirement: Authenticated announcement test

Every host SHALL implement announcements.test with a closed speed/load/combined test-kind parameter and a terminal outcome of finished, interrupted, or unavailable within ten seconds of admission. The test SHALL use a fixed representative reading through the normal native audio path without requiring a board, changing settings or bypassing global cooldown. Arbitrary text, values and voice identifiers SHALL not be accepted. Invalid and busy requests SHALL use existing structured errors.

#### Scenario: Test with announcements disabled
- **WHEN** an authenticated client tests load while automatic announcements are off and the speech slot and cooldown permit
- **THEN** the host attempts “motor 72 percent” and returns its terminal outcome without enabling announcements or changing telemetry

#### Scenario: Test speed
- **WHEN** an authenticated client tests speed with the speech slot and cooldown available
- **THEN** it speaks the fixed physical speed of 25 km/h rounded in current display units, identifies those units, and returns a terminal outcome

#### Scenario: Blocked test
- **WHEN** a valid test is admitted but the voice, focus or audio route prevents playback
- **THEN** it returns unavailable without claiming audible success, queuing speech, or disturbing connection or recording

#### Scenario: Invalid request or cooldown
- **WHEN** a test contains an unknown test kind or extra speech input, or an utterance or cooldown already occupies admission
- **THEN** the backend rejects invalid input with invalid_request or occupied admission with invalid_state without creating a waiter or speech queue

#### Scenario: Client disconnect
- **WHEN** a client disconnects with its test pending
- **THEN** its waiter is released and its speech is cancelled without cancelling an unrelated operation; the admission cooldown remains in effect

#### Scenario: Combined test and volume
- **WHEN** an authenticated client requests a combined test with admission available
- **THEN** it attempts the fixed winning-load test reading followed by speed at the saved announcement volume, independent of rule enable flags and thresholds; zero volume finishes without audio playback

### Requirement: Announcement audio lowering availability

Every host SHALL expose announcementAudio.loweringControl as adjustable on iOS or automatic on Android/macOS, and persist a boolean announcements.lowerOtherAudio defaulting to true. A false update on automatic-only hosts SHALL fail with unavailable and leave the entire patch unchanged. The frontend SHALL use this mandatory field to hide the unsupported control without platform detection or optional methods. Other announcement settings SHALL retain identical contract semantics.

#### Scenario: iOS preference save
- **WHEN** iOS receives a valid lowering preference at the current settings revision
- **THEN** it persists the preference atomically and subsequent snapshots and process recreation retain it

#### Scenario: Unsupported lowering request
- **WHEN** Android or macOS receives a current-revision patch setting lowerOtherAudio to false
- **THEN** the complete patch fails with unavailable without modifying any settings; other methods and audio playback remain available

### Requirement: Authoritative live load projection

Each live sample SHALL expose nullable duty, motor and battery load fractions plus a nullable winning source/fraction, calculated by one shared backend owner. All fractions SHALL be finite in [0,1]. Gauge, widgets, charts, native notifications/Live Activities and announcements SHALL consume this result or the same owner at their own cadence. Frontend and native views SHALL not calculate utilisation or choose the winning source.

#### Scenario: Equivalent sample across consumers
- **WHEN** the same sample and controller limits feed the gauge, load widget, live chart, notification, Dynamic Island and announcement
- **THEN** they use identical component values and winner before display rounding, independent of their refresh intervals

#### Scenario: Signed limits and motor preference
- **WHEN** motor or battery current changes sign
- **THEN** the owner selects the corresponding draw/regen limit using raw current sign, uses filtered then directional then raw motor magnitude, and caps absolute component fractions at 1

#### Scenario: Missing inputs and ties
- **WHEN** limits or required inputs are absent, zero or non-finite, or valid components tie
- **THEN** only affected components are unavailable, available duty remains usable, and equal highest fractions prefer duty, then motor, then battery

#### Scenario: Controller changes
- **WHEN** the active controller changes or a chart retains an older point
- **THEN** the point retains its own projected load; replacement-controller limits cannot reinterpret its values

### Requirement: Captured historical load values

Saved samples SHALL capture existing motorLoad and batteryLoad fractions using the shared load owner and limits applicable at capture. Session review SHALL retain its existing recorded dutyCycle and captured current-load fields. Historical readers, pruning and export/import SHALL preserve recorded values and missingness without using present-day limits. Load unification SHALL not introduce dutyLoad or replace raw duty with absolute saturated utilisation in session review.

#### Scenario: Session review after configuration change
- **WHEN** controller limits change after a ride and that ride is reviewed or exported
- **THEN** its load values remain those captured during the ride, with no frontend or reader formula using the new limits

#### Scenario: Raw duty differs from displayed load
- **WHEN** recorded raw duty is negative or exceeds the normalised load range
- **THEN** session review and export retain the recorded dutyCycle value; live utilisation consumers use its absolute saturated magnitude as a different displayed quantity

#### Scenario: Failed persistence or guest connection
- **WHEN** a saved sample append fails or the connection is a guest
- **THEN** saved public sample ordering is preserved; guest load remains ephemeral and creates no history

### Requirement: Native voice settings access

Every host SHALL expose mandatory boolean announcementAudio.voiceSettingsAvailable
and authenticated announcements.openVoiceSettings with empty parameters. The
frontend MAY use this closed OS fact to show the system voice settings action.
The operation SHALL return accepted true only after successful native launch,
return unavailable if unsupported, not foreground or launch fails, reject extra
parameters, and terminate within five seconds. It SHALL admit one settings launch
at a time and ignore obsolete completions.

#### Scenario: Open supported system chooser
- **WHEN** a foreground user activates Voice settings and the host can resolve the chooser
- **THEN** the native host opens it and completes with accepted true without changing saved announcement rules or acquiring audio focus

#### Scenario: Unsupported or missing chooser
- **WHEN** no supported chooser link is available
- **THEN** state reports false, the button is hidden, and the mandatory method returns unavailable
