# Spec Delta

## Purpose

Let riders hear current board speed or duty above a chosen threshold, with consistent timing and graceful audio failure across native hosts.

## ADDED Requirements

### Requirement: Global announcement configuration

Announcements SHALL default to off. Users SHALL select exactly one mode: off, speed, or duty, retain independent speed and duty thresholds, and set a shared whole-second repeat interval from 1 through 60 seconds, defaulting to 5. Speed thresholds SHALL be finite and greater than zero through 200 km/h, defaulting to 25 km/h; duty thresholds SHALL be finite and greater than zero through 100 percent, defaulting to 65 percent.

#### Scenario: Disabled by default
- **WHEN** the application starts with default settings
- **THEN** no automatic announcement is spoken and the default thresholds and interval are available for editing

#### Scenario: Change units or mode
- **WHEN** users change speed units or switch between speed and duty
- **THEN** the physical speed threshold is preserved, its display converts to the selected units, and both independently saved thresholds remain available

### Requirement: Current board reading eligibility

Automatic announcements SHALL use only finite readings received less than two seconds ago from the ready active board and selected controller, in saved or guest mode. Speed SHALL use magnitude in the selected speed units; duty SHALL use magnitude as a percentage, not combined drive load. A reading SHALL qualify only when its unrounded value strictly exceeds the threshold. Standalone BMS, missing and out-of-range readings SHALL not qualify.

#### Scenario: Duty exceeds threshold
- **WHEN** fresh duty is 66 percent with a 65 percent threshold
- **THEN** the announcement says the current whole-number percentage and identifies duty

#### Scenario: Equality and missing readings
- **WHEN** duty equals the threshold, exceeds 100 percent, is missing, or is two seconds old
- **THEN** it does not trigger an announcement

#### Scenario: Reverse speed
- **WHEN** a fresh reverse speed has a magnitude exceeding the speed threshold
- **THEN** it qualifies and speech identifies the positive speed and selected unit

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

Announcement policy SHALL belong to the native service independently of the frontend and recording cadence. Disconnection, controller replacement, stale or ineligible telemetry, and configuration changes SHALL cancel pending automatic speech. Reconnection SHALL require fresh telemetry and retain process-local cooldown. Process restart SHALL retain settings but discard speech, readings and deadlines. Late callbacks SHALL not affect a replacement operation.

#### Scenario: WebView replaced
- **WHEN** the WebView is replaced or its listener restarts during a connection
- **THEN** announcement policy continues with the existing service without duplicate timers or reset cooldown

#### Scenario: Disconnect and reconnect
- **WHEN** a board disconnects above threshold and reconnects
- **THEN** its old reading cannot trigger speech, and only a fresh qualifying reading after cooldown can trigger another attempt

#### Scenario: Disable or drop below threshold
- **WHEN** users turn announcements off or telemetry stops qualifying
- **THEN** pending automatic speech is cancelled and no old reading is replayed

### Requirement: Native speech and graceful audio fallback

Hosts SHALL use native speech with an installed offline-capable voice and temporarily request appropriate audio mixing or ducking. Audio resources SHALL be released on completion, cancellation, interruption, error and timeout. Unavailable speech, denied focus and calls SHALL skip the attempt without changing system volume settings, retry loops, stale playback, or disruption to BLE, recording or other application work. Successful playback during calls SHALL not be required.

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
