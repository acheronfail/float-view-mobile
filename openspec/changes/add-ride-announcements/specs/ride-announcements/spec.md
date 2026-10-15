# Spec Delta

## Purpose

Let riders hear current board speed or load above a chosen threshold, with consistent timing and graceful audio failure across native hosts.

## ADDED Requirements

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

Policy SHALL run independently of the frontend and recording cadence. Disconnection, controller replacement, loss of all eligible readings, and rule configuration changes SHALL cancel pending automatic speech. Reconnection SHALL require fresh telemetry and retain process-local cooldown. Process restart SHALL retain settings but discard speech, readings and deadlines. Late callbacks SHALL not affect a replacement operation.

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
- **WHEN** users disable both rules or no enabled reading qualifies
- **THEN** pending automatic speech is cancelled and no old reading is replayed

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

The service SHALL admit at most one active utterance across automatic and test speech, with no pending utterance queue and a ten-second terminal deadline. A busy automatic attempt SHALL be dropped. A busy test SHALL fail with invalid_state. Audio readiness SHALL refresh at bounded native lifecycle opportunities, allowing recovery without app restart or continuous initialization retries. Normal logs SHALL omit spoken readings and private identity.

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
