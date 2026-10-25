# Ride Announcements Specification

## Purpose

Let riders hear sustained footpad contact loss and current board speed or load above a chosen threshold, with consistent timing and graceful audio failure across native hosts.

## Requirements

### Requirement: Global announcement configuration

Speed and load SHALL have independent enable switches, both off by default, and retain independent thresholds. Users SHALL set a shared whole-second repeat interval from 1 through 60 seconds, defaulting to 5. Speed thresholds SHALL be finite and greater than zero through 200 km/h, defaulting to 25 km/h; load thresholds SHALL be finite and greater than zero through 100 percent, defaulting to 65 percent.

#### Scenario: Disabled by default
- **WHEN** the application starts with default settings
- **THEN** no automatic announcement is spoken and the default thresholds and interval are available for editing

#### Scenario: Change units or enable flags
- **WHEN** users change speed units or enable/disable either rule
- **THEN** the physical speed threshold is preserved, its display converts to the selected units, and both independently saved thresholds remain available

### Requirement: Announcement volume

Users SHALL configure a shared whole-number announcement volume from 0 through 100 percent, defaulting to 100, relative to current device output volume. It SHALL apply to automatic and test speech without changing device volume or another app's volume setting. Zero SHALL mute speech without acquiring audio focus or ducking. Volume edits SHALL affect the next utterance; tests at zero SHALL finish without playback.

#### Scenario: Reduced speech volume
- **WHEN** users set announcement volume to 50 percent and test speech
- **THEN** the host uses half-scale speech gain relative to current output volume, without promising half the perceived loudness or raising muted device audio

#### Scenario: Muted announcements
- **WHEN** announcement volume is zero
- **THEN** automatic speech is skipped without taking audio resources, tests finish silently, and Settings clearly identifies the muted volume

### Requirement: Simultaneous speed and load rules

Both rules SHALL be enableable together. At each eligible attempt the service SHALL include the highest qualifying enabled load component, followed by qualifying enabled speed with its selected units, in one utterance. Both rules SHALL share one repeat cooldown and speech slot. No rule SHALL be delayed to another interval solely because the other qualifies. An unavailable or below-threshold rule SHALL not suppress the other.

#### Scenario: Speed and load qualify together
- **WHEN** both rules are enabled and motor is 78 percent, battery is 70 percent and speed is 32 km/h, each above its threshold, with duty below threshold
- **THEN** one utterance says “motor 78 percent, 32 kilometres per hour” and advances the shared cooldown once

#### Scenario: Second rule qualifies during cooldown
- **WHEN** speed exceeds its threshold during cooldown from a load announcement
- **THEN** it waits for the next shared eligible attempt, which includes the current qualifying load winner and/or speed without replaying the previous phrase

#### Scenario: Only one rule remains eligible
- **WHEN** enabled load readings become unavailable or drop below threshold while enabled speed remains eligible
- **THEN** automatic speech remains eligible and the next utterance includes speed only

### Requirement: Current board reading eligibility

Automatic announcements SHALL use only finite readings received less than two seconds ago from the ready active board and selected controller, in saved or guest mode. Speed SHALL use magnitude in the selected speed units. A reading SHALL qualify only when its unrounded value strictly exceeds the threshold. Standalone BMS, missing and out-of-range readings SHALL not qualify.

#### Scenario: Duty component exceeds load threshold
- **WHEN** fresh duty is 66 percent with a 65 percent threshold
- **THEN** the announcement says the current whole-number percentage and says “duty” before the percentage

#### Scenario: Equality and missing readings
- **WHEN** a component equals the threshold, is unavailable, or is two seconds old
- **THEN** it does not trigger an announcement

#### Scenario: Reverse speed
- **WHEN** a fresh reverse speed has a magnitude exceeding the speed threshold
- **THEN** it qualifies and speech identifies the positive speed and selected unit

### Requirement: Load component announcements

The enabled load rule SHALL use the live gauge component calculations and one shared percentage threshold. Each announcement SHALL name only the highest available component before its rounded percentage, provided it exceeds the threshold. Equal percentages SHALL use gauge tie order: duty, then motor, then battery. Missing components SHALL be omitted. Speech SHALL remain eligible while any enabled reading qualifies.

#### Scenario: Multiple components exceed threshold
- **WHEN** the threshold is 65 percent and fresh duty, motor and battery loads are 68, 74 and 60 percent
- **THEN** one announcement says “motor 74 percent”, omitting duty and battery even though duty also exceeds the threshold

#### Scenario: Equal load components
- **WHEN** duty and motor tie for highest available load above threshold
- **THEN** the announcement names duty only; when motor and battery tie above duty, it names motor only, matching the gauge even for saturated 100 percent readings

#### Scenario: Winner changes
- **WHEN** motor replaces duty as the highest component during cooldown
- **THEN** the next eligible announcement names the current motor percentage without resetting cooldown or replaying the old duty value

#### Scenario: Battery utilisation
- **WHEN** battery current utilisation exceeds the threshold
- **THEN** speech says “battery” and that utilisation percentage, not battery state of charge

#### Scenario: Limits unavailable or regenerated
- **WHEN** current limits are missing or zero for a component, or a current changes sign
- **THEN** unavailable components are omitted, valid duty can still announce, and available current components use the same signed-limit and filtered-current rules as the gauge

#### Scenario: Gauge saturation
- **WHEN** a finite component's utilisation exceeds its limit
- **THEN** it saturates at 100 percent as on the gauge and remains eligible below a 100 percent threshold

#### Scenario: Component set changes
- **WHEN** duty drops below threshold but motor remains above it
- **THEN** repeat eligibility remains active and the next attempt names the current highest component only

### Requirement: Repeat cadence without backlog

The first qualifying reading SHALL trigger an immediate attempt when cooldown permits. Subsequent attempts SHALL be separated by at least the configured interval using monotonic time and SHALL use the latest eligible value. Below-threshold crossings, settings edits and failed attempts SHALL not bypass cooldown. Busy playback SHALL cause due announcements to be skipped, not queued. Missed intervals SHALL not produce catch-up speech.

#### Scenario: Continued excess
- **WHEN** duty stays above 65 percent with a five-second interval
- **THEN** speech attempts occur no more often than every five seconds and each speaks the current value, even when unchanged

#### Scenario: Threshold chatter
- **WHEN** readings repeatedly cross the threshold during cooldown
- **THEN** they do not produce extra attempts; once cooldown expires a fresh qualifying reading can trigger speech

#### Scenario: Slow speech or delayed worker
- **WHEN** an utterance lasts longer than the interval or execution resumes after missed intervals
- **THEN** utterances never overlap and old intervals are not replayed

### Requirement: Lifecycle and connection isolation

Policy SHALL run independently of the frontend and recording cadence. Disconnection, controller replacement, loss of fresh valid board context, and rule configuration changes SHALL cancel pending automatic speech. Reconnection SHALL require fresh telemetry and retain process-local cooldown. Process restart SHALL retain settings but discard speech, readings and deadlines. Late callbacks SHALL not affect a replacement operation.

#### Scenario: Volume-only edit

- **WHEN** volume changes during an utterance
- **THEN** that utterance retains its admitted gain and the next utterance uses the new volume

#### Scenario: WebView replaced

- **WHEN** the WebView is replaced or its listener restarts during a connection
- **THEN** announcement policy continues with the existing service without duplicate timers or reset cooldown

#### Scenario: Disconnect and reconnect

- **WHEN** a board disconnects above threshold and reconnects
- **THEN** its old reading cannot trigger speech, and only a fresh qualifying reading after cooldown can trigger another attempt

#### Scenario: Disable or drop below threshold

- **WHEN** users disable all automatic rules or no enabled reading qualifies
- **THEN** disabling rules cancels pending automatic speech; a threshold crossing with fresh valid readings lets admitted speech finish, prevents further attempts below threshold, and never replays an old reading

#### Scenario: Drop below threshold during playback

- **WHEN** fresh valid readings drop to or below every enabled threshold during automatic speech
- **THEN** the admitted utterance finishes without cancellation, no further speech starts while readings remain below threshold, and later qualifying readings obey the existing cooldown without replaying old values

#### Scenario: Stale readings during playback

- **WHEN** the active board's readings become two seconds old or its controller context becomes invalid during automatic speech
- **THEN** pending automatic speech is cancelled

### Requirement: Native speech and graceful audio fallback

Hosts SHALL use native speech with an installed offline-capable voice and request mixing or ducking using the saved supported preference. Audio resources SHALL be released on completion, cancellation, interruption, error and timeout. Unavailable speech, denied focus and calls SHALL skip the attempt without system-volume changes, retry loops, stale playback or disruption to BLE and recording. Successful playback during calls SHALL not be required.

#### Scenario: Music playing
- **WHEN** an announcement plays while compatible music playback is active
- **THEN** the host requests temporary ducking and releases it after speech so the other playback can return to its normal level

#### Scenario: Focus denied or engine unavailable
- **WHEN** audio focus is denied, a voice is unavailable, or speech initialization fails
- **THEN** the attempt terminates gracefully, no reading is queued, and later automatic attempts remain interval-bounded

#### Scenario: Interruption or missing completion
- **WHEN** playback is interrupted or its completion callback never arrives
- **THEN** playback is stopped and resources are released within ten seconds of admission, and obsolete callbacks cannot release a later utterance's resources

### Requirement: Bounded speech admission and recovery

The service SHALL admit at most one active utterance across automatic and test speech, with no pending utterance queue and a ten-second terminal deadline. A busy automatic attempt SHALL be dropped. A busy test SHALL fail with invalid_state. Manual tests SHALL ignore the automatic repeat interval and MAY repeat immediately after the previous utterance finishes. Each admitted test SHALL advance the automatic cooldown. Audio readiness SHALL refresh at bounded native lifecycle opportunities, allowing recovery without app restart or continuous initialization retries. Normal logs SHALL omit spoken readings and private identity.

#### Scenario: Concurrent test and automatic trigger
- **WHEN** automatic speech is active and a test is requested
- **THEN** the test fails with invalid_state and cannot enqueue or interrupt that speech

#### Scenario: Voice becomes available
- **WHEN** users return from system settings after making an offline voice available
- **THEN** readiness is re-evaluated and a subsequent eligible attempt can speak without restarting the app

### Requirement: Optional lowering of other audio on iOS

On iOS, users SHALL be able to choose whether announcements request lowering other audio, enabled by default. When disabled, speech SHALL request mixing without ducking or interrupting spoken content. Android and macOS SHALL retain automatic audio handling without exposing this choice. Automatic and test speech SHALL use the same preference, captured at admission; edits SHALL affect the next utterance without changing device volume.

#### Scenario: Lowering enabled
- **WHEN** an iOS announcement plays with lowering enabled and compatible music is active
- **THEN** the host requests temporary ducking and releases it at the end; other spoken content may pause according to native policy

#### Scenario: Lowering disabled
- **WHEN** an iOS announcement plays with lowering disabled
- **THEN** it requests mixing without lowering or pausing other playback and still releases its session on all terminal paths

#### Scenario: Edit during speech
- **WHEN** the lowering preference changes while an utterance is active
- **THEN** the current utterance retains its admitted audio policy and the next automatic or test utterance uses the new preference

### Requirement: Concise speed and preferred native voice

Speech SHALL announce speed using its number and selected units without a
redundant speed prefix. On Android, a selected installed offline English engine
voice SHALL be preferred over other eligible voices; if unavailable, a bounded
deterministic installed offline English fallback MAY be used. Voices requiring
network or marked not installed SHALL not be selected.

#### Scenario: Concise combined phrase
- **WHEN** motor load is 78 percent and speed is 32 kilometres per hour
- **THEN** combined speech says “motor 78 percent, 32 kilometres per hour”

#### Scenario: Voice settings return
- **WHEN** a user selects an engine or eligible voice in system settings and returns
- **THEN** native readiness refreshes and subsequent speech uses the refreshed choice without restarting or disturbing current speech resources

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
