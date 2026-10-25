# Ride Announcements Delta

## ADDED Requirements

### Requirement: Footpad announcement settings

Footpad announcements SHALL have one independent enable switch, off by default, alongside speed and load in Ride announcements. A delay slider SHALL offer whole seconds from 1 through 5, defaulting to 1, show the selected value, and support touch, pointer and keyboard input. The switch and delay SHALL save automatically, survive restart and retain the delay while disabled. Invalid delay values SHALL be rejected without changing saved settings.

#### Scenario: Configure footpad alerts
- **WHEN** a user enables footpad announcements and selects a three-second delay
- **THEN** the settings save automatically and remain enabled with a three-second delay after restart

#### Scenario: Disable and re-enable
- **WHEN** a user disables and later re-enables footpad announcements
- **THEN** the selected delay is retained and qualification begins from fresh observations without reusing an earlier elapsed duration

#### Scenario: Accessible slider
- **WHEN** a user adjusts the delay with touch, a primary pointer or keyboard
- **THEN** the slider selects only 1, 2, 3, 4 or 5 seconds and exposes its label and current value accessibly

#### Scenario: Invalid delay
- **WHEN** an update supplies a fractional delay or a value outside 1 through 5
- **THEN** the update fails validation and leaves saved settings unchanged

### Requirement: Preserve existing announcement preferences

Existing supported stored settings missing the footpad fields SHALL open with footpad announcements disabled and a one-second delay, preserving every existing preference and supported recording. New saves SHALL persist both fields. Invalid present values SHALL follow existing validation failures rather than silently defaulting or resetting storage.

#### Scenario: Open supported settings predating footpad alerts
- **WHEN** supported stored settings contain non-default speed, load, volume and repeat preferences but no footpad fields
- **THEN** those preferences remain unchanged, footpad alerts default off with a one-second delay, and the expanded settings survive save and reopen

### Requirement: Sustained footpad release eligibility

Each side SHALL qualify independently after continuous reported release for at least the configured delay while finite speed magnitude is strictly above 5 km/h and within the existing valid speed range. Eligibility SHALL require fresh known footpad state and speed from the ready active board and selected controller, in saved or guest mode. The fixed speed gate SHALL remain 5 km/h regardless of display units. Unknown or missing state SHALL never mean released.

#### Scenario: Side mapping and delay boundary
- **WHEN** only the right side is reported pressed and fresh speed remains 10 km/h for a configured two-second delay
- **THEN** the left side qualifies at two seconds, never earlier, and the right side does not qualify

#### Scenario: Independent sides
- **WHEN** the left side has met its delay and the right side subsequently releases
- **THEN** only the left side qualifies until the right side independently meets the full delay

#### Scenario: Speed boundary and reverse
- **WHEN** speed magnitude is exactly 5 km/h, above 5 km/h in reverse, or displayed in mph
- **THEN** equality does not qualify, reverse uses magnitude, and changing units does not change the physical gate

#### Scenario: Unsupported or unavailable observations
- **WHEN** footpad state or speed is missing, unknown, stale or invalid, or the connection is a standalone BMS
- **THEN** no footpad alert qualifies from that observation

### Requirement: Footpad release timer isolation

Contact returning SHALL reset that side's release duration. Speed at or below 5 km/h, unavailable speed or footpad state, a telemetry gap of at least two seconds, disconnection, controller replacement, or a footpad enable/delay edit SHALL reset both durations. Requalification SHALL require a full new delay. Timers SHALL use monotonic time and remain independent of WebView and recording cadence; reset SHALL retain the shared speech cooldown.

#### Scenario: Brief release and recovered contact
- **WHEN** a side releases for less than the delay and then presses again
- **THEN** it does not qualify and a later release starts a new full delay for that side

#### Scenario: Speed and freshness resets
- **WHEN** speed reaches 5 km/h or below, known footpad evidence disappears, or no new telemetry arrives for two seconds
- **THEN** neither side retains elapsed release time across the interruption, even if a worker processes the next sample after the gap

#### Scenario: Reconnection and controller replacement
- **WHEN** a connection is replaced or restored after a release was qualifying
- **THEN** fresh observations must satisfy the full delay for the new context and cannot bypass the shared cooldown

#### Scenario: Settings and WebView lifecycle
- **WHEN** the delay is edited during release qualification or the WebView is replaced during a ride
- **THEN** a delay edit restarts both durations while WebView replacement preserves service-owned qualification without creating duplicate timers

### Requirement: Footpad speech wording and ordering

An utterance SHALL say “left footpad off” for only the left qualifying side, “right footpad off” for only the right qualifying side, and “both footpads off” when both qualify. Footpad wording SHALL precede the existing highest qualifying load component and qualifying speed in a single utterance. Missing or ineligible footpad evidence SHALL not suppress eligible speed or load speech. No recovery phrase SHALL be spoken.

#### Scenario: Both sides meet the delay
- **WHEN** both sides have independently met the release delay
- **THEN** the next eligible utterance says “both footpads off” once rather than naming each side separately

#### Scenario: Footpad and load and speed qualify
- **WHEN** the right side qualifies, motor load is 78 percent and speed is 32 km/h, with those announcements enabled and above their thresholds
- **THEN** one utterance says “right footpad off, motor 78 percent, 32 kilometres per hour”

#### Scenario: Only footpad alerts enabled
- **WHEN** speed and load announcements are disabled and the left side qualifies
- **THEN** the next eligible utterance says “left footpad off”

#### Scenario: Missing footpad evidence
- **WHEN** footpad state is unknown but speed or load qualifies
- **THEN** the eligible speed or load announcement proceeds without a footpad phrase

### Requirement: Shared footpad announcement cadence and playback

Footpad alerts SHALL share the existing repeat interval, monotonic cooldown, speech slot, volume and native audio preferences with speed, load and tests. Persistent releases SHALL repeat using current eligible sides. Delay completion SHALL not bypass cooldown. Busy or unavailable audio SHALL skip attempts without queueing or catch-up. Contact recovery or slowing with fresh valid board context SHALL prevent new footpad attempts but let admitted speech finish.

#### Scenario: Persistent release repeats
- **WHEN** a side remains qualifying with a five-second repeat interval
- **THEN** attempts repeat no more often than every five seconds and describe the sides currently qualifying

#### Scenario: Delay completes during cooldown
- **WHEN** a footpad delay completes during cooldown from speed, load or manual test speech
- **THEN** it waits for the next shared eligible attempt without preempting speech or bypassing cooldown

#### Scenario: Busy, muted or unavailable audio
- **WHEN** an attempt is due while speech is busy, volume is zero or native audio is unavailable
- **THEN** existing bounded admission and audio failure behavior applies, with no queued warning or catch-up playback

#### Scenario: Recovery during admitted speech
- **WHEN** contact returns or speed falls to 5 km/h or below during footpad speech while board context remains fresh and valid
- **THEN** the admitted phrase finishes and subsequent attempts omit sides that no longer qualify

#### Scenario: Invalid context during admitted speech
- **WHEN** the board disconnects, its controller changes or board context becomes stale during footpad speech
- **THEN** automatic speech is cancelled under the existing lifecycle rules and late callbacks cannot affect a replacement utterance

#### Scenario: Disable and process restart
- **WHEN** footpad announcements are disabled during playback or the process restarts
- **THEN** disabling cancels active automatic speech under existing rule-edit behavior, while restart retains settings and discards speech and transient release durations
