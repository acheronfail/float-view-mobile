# Ride Announcements Delta

## ADDED Requirements

### Requirement: Battery charge announcement control

Ride announcements SHALL show one Battery announcements switch alongside existing rule switches, controlling both battery phrases. It SHALL autosave with existing revision/conflict handling, default on, survive restart and support keyboard, assistive technology and guarded primary-pointer activation. No battery threshold or repeat controls SHALL be added.

#### Scenario: Toggle without duplicate activation

- **WHEN** a user activates Battery charge with primary touch, mouse, keyboard or assistive technology
- **THEN** it changes once and saves; cancelled or non-primary pointer input does not change it

#### Scenario: Disable and re-enable

- **WHEN** the user disables Battery charge during battery speech and later re-enables it
- **THEN** battery speech is cancelled, other rule preferences remain unchanged, visual qualification is unaffected, and re-enabling uses fresh eligible evidence with retained battery cooldown

#### Scenario: Revision conflict

- **WHEN** an autosave conflicts or fails while newer edits exist
- **THEN** the existing save/error/retry behavior preserves newer edits without silently presenting an unsaved value as persisted

### Requirement: Authoritative voltage threshold resolution

Battery charge SHALL use current Refloat low/high settings for the selected controller and connection, resolved to pack volts with supported package/firmware semantics. Per-cell conversion SHALL use the controller's series count. Missing, unsupported, non-finite, non-positive, reversed or unresolvable thresholds SHALL disable derived battery warnings, without guessed limits or disruption to other announcements.

#### Scenario: Pack and per-cell settings

- **WHEN** a supported board reports pack thresholds of 60 V and 86 V, or supported per-cell thresholds of 3.0 V and 4.3 V with controller series count 20
- **THEN** both resolve to the same 60 V and 86 V pack thresholds

#### Scenario: Missing conversion evidence

- **WHEN** per-cell conversion requires an unavailable controller series count, even though an app battery profile supplies one
- **THEN** derived battery warnings remain unavailable rather than using the app profile, schema defaults or inferred chemistry

#### Scenario: Failed or invalid configuration

- **WHEN** current configuration read fails, semantics are unknown, either limit is invalid, L is at least H, or a derived boundary is not finite and representable
- **THEN** battery warnings remain unavailable and existing speed/load/footpad rules continue independently

#### Scenario: Current values rather than editor state

- **WHEN** a user edits an unsaved threshold or views schema/default values
- **THEN** those values cannot change the battery announcement boundaries

### Requirement: Proportional advance voltage boundaries

For resolved pack limits L and H, battery charge SHALL use R = H − L, low entry L + 0.10R and high entry H − 0.05R. Low entry SHALL require voltage strictly below its boundary; high entry SHALL require voltage strictly above its boundary, using unrounded measurements independently of estimated battery percentage and display units.

#### Scenario: Agreed 20S example

- **WHEN** L is 60 V and H is 86 V
- **THEN** low entry is 62.6 V and high entry is 84.7 V; equality alone does not enter either advance warning

#### Scenario: Scale invariance

- **WHEN** both configured limits and measured voltage scale by the same positive factor
- **THEN** warning boundaries scale by that factor and eligibility is otherwise unchanged

#### Scenario: Percentage and units

- **WHEN** displayed battery percentage, battery profile or speed display units change without changing authoritative board thresholds or measurements
- **THEN** the voltage boundaries remain unchanged

### Requirement: Riding state battery eligibility

Battery attempts SHALL require fresh valid voltage and Refloat running state from the ready selected board, saved or guest, without a speed check. Advance high attempts below H SHALL additionally require fresh negative battery current indicating regen. At or above H, regen SHALL not be required. Standalone BMS and non-running, unknown, missing, stale or invalid required observations SHALL not qualify.

#### Scenario: Board is not riding

- **WHEN** voltage is in either warning range but Refloat is not running, or its package state is missing, unknown or stale, even if speed indicates motion
- **THEN** no battery attempt starts, including beyond a board threshold, and riding qualification resets without clearing cooldown

#### Scenario: Riding without a speed gate

- **WHEN** fresh Refloat running state and required electrical observations qualify while speed is zero, low, reverse or unavailable
- **THEN** battery eligibility is unchanged by speed or display units and all other battery confirmation/cooldown rules still apply

#### Scenario: Full pack under draw versus regen

- **WHEN** voltage while Refloat is running is above high entry but below H
- **THEN** high qualification requires negative battery current; zero, positive, missing or stale current suppresses it and resets high qualification

#### Scenario: Critical high boundary

- **WHEN** Refloat is running with fresh voltage at or above H and valid resolved thresholds
- **THEN** high eligibility does not require regen evidence

### Requirement: Battery confirmation and recovery hysteresis

Advance low entry SHALL require 2 continuous seconds of eligible evidence; advance high entry SHALL require 500 ms. At or below L or at or above H SHALL bypass entry delay. Active low/high warnings SHALL recover only after 2 continuous seconds at or beyond low-entry + 0.02R / high-entry − 0.02R respectively. Interrupted conditions SHALL reset their confirmation timers.

#### Scenario: Brief sag or spike

- **WHEN** eligible voltage crosses low entry for less than 2 seconds or high entry for less than 500 ms without reaching L or H
- **THEN** no battery warning qualifies and a later crossing starts a new full confirmation

#### Scenario: Confirmation boundary and critical entry

- **WHEN** eligible evidence persists for exactly the specified confirmation or fresh voltage while Refloat is running reaches L or H first
- **THEN** the matching warning qualifies at that point, still subject to its cooldown and shared speech slot

#### Scenario: Recovery example

- **WHEN** L is 60 V and H is 86 V and a battery warning is active
- **THEN** low clears after 2 seconds at or above 63.12 V, and high clears after 2 seconds at or below 84.18 V; brief recoveries reset recovery duration without clearing an otherwise eligible latch

#### Scenario: Hysteresis band

- **WHEN** voltage lies between an entry boundary and its recovery boundary
- **THEN** an otherwise eligible active warning remains active, while an inactive warning does not begin qualifying

### Requirement: Battery phrase cadence and priority

Battery attempts SHALL speak only “Battery low” or “Over voltage” as standalone phrases, with independent monotonic 60-second cooldowns. Due battery attempts SHALL take priority when the shared speech slot is free, without waiting for routine cooldown or preempting admitted speech. Over voltage SHALL win conflicting battery evidence. Ordinary attempts SHALL retain existing internal phrase ordering.

#### Scenario: One-minute low repeat

- **WHEN** low remains eligible after an attempt at time T
- **THEN** no further low attempt occurs before T + 60 seconds, and the next attempt uses fresh current eligibility

#### Scenario: Independent high cooldown

- **WHEN** high becomes eligible during a low or routine cooldown
- **THEN** it can attempt immediately when the slot is free and its own confirmation/cooldown permits; sustained high repeats no more often than once per minute

#### Scenario: Competing ordinary rules

- **WHEN** battery and footpad/load/speed speech are due with a free slot
- **THEN** battery speaks alone and advances the routine cooldown to avoid immediately following with routine speech; routine/manual speech never advances battery cooldowns

#### Scenario: Busy slot

- **WHEN** battery is due while automatic or manual speech is playing
- **THEN** playback is not interrupted and no phrase is queued; a subsequent free-slot opportunity re-evaluates fresh evidence without consuming a battery interval for the busy drop

### Requirement: Battery bounded audio failure behavior

Battery speech SHALL share existing volume, offline voice, audio preference, single-slot and ten-second deadline behavior. A free-slot battery attempt SHALL advance its cooldown even if muted, unavailable or denied audio. Failed attempts SHALL not retry tightly, queue or catch up. Ordinary playback SHALL remain available while battery speech is cooling down.

#### Scenario: Muted or unavailable audio

- **WHEN** a due free-slot battery attempt encounters volume zero, unavailable speech or denied focus
- **THEN** it consumes that kind's 60-second interval without queued playback; zero volume does not acquire audio resources or change device volume

#### Scenario: Delayed worker and missing completion

- **WHEN** execution resumes after missed battery intervals or native completion never arrives
- **THEN** missed intervals are not replayed, the existing ten-second deadline bounds admitted playback, and late callbacks cannot affect a replacement utterance

### Requirement: Battery lifecycle and configuration isolation

Battery policy SHALL survive WebView replacement within the existing service. Leaving running state, loss of required fresh observations, configuration invalidation, disconnect or controller replacement SHALL reset transient qualification. Reconnect SHALL require current settings and fresh evidence. Process-local battery cooldowns SHALL survive these resets; process restart SHALL retain settings and discard transient state.

#### Scenario: Telemetry gap

- **WHEN** battery inputs reach two seconds old, or a new sample arrives after a gap of at least two seconds
- **THEN** elapsed entry/recovery time cannot bridge the gap, new attempts require fresh qualification, and affected battery speech is cancelled on stale context

#### Scenario: Leave and resume riding

- **WHEN** a qualifying board leaves running state and returns to it before 60 seconds has passed since its last battery attempt
- **THEN** no battery speech starts while non-running, an admitted phrase can finish with otherwise fresh valid context, and returning to running requires qualification without bypassing cooldown

#### Scenario: Threshold replacement and reconnect

- **WHEN** configuration is invalidated for a read/write or the controller reconnects or changes
- **THEN** prior thresholds and qualification cannot trigger speech; affected battery speech is cancelled and confirmed current thresholds plus fresh measurements are required before new attempts

#### Scenario: Frontend and native lifecycle

- **WHEN** the WebView is recreated or the host backgrounds or locks while its existing execution/audio support remains available
- **THEN** the same service-owned battery policy continues without duplicate timers or cooldown reset, subject to fresh telemetry and existing native audio restrictions

#### Scenario: Process restart

- **WHEN** the application process restarts
- **THEN** the saved battery switch is retained and old readings, qualification, utterances and cooldown timestamps are discarded

## MODIFIED Requirements

### Requirement: Global announcement configuration

Speed and load SHALL have independent enable switches, both off by default, and retain independent thresholds. Battery charge SHALL have one independent switch, on by default. Users SHALL set a shared speed/load/footpad whole-second repeat interval from 1 through 60 seconds, defaulting to 5. Speed thresholds SHALL be finite and greater than zero through 200 km/h, defaulting to 25 km/h; load thresholds SHALL be finite and greater than zero through 100 percent, defaulting to 65 percent.

#### Scenario: Disabled by default

- **WHEN** the application starts with default settings
- **THEN** speed, load and footpad announcements are disabled, battery charge announcements are enabled subject to their eligibility rules, and the default thresholds and interval are available for editing

#### Scenario: Change units or enable flags

- **WHEN** users change speed units or enable/disable either rule
- **THEN** the physical speed threshold is preserved, its display converts to the selected units, and both independently saved thresholds remain available

### Requirement: Current board reading eligibility

Automatic announcements SHALL use only finite readings received less than two seconds ago from the ready active board and selected controller, in saved or guest mode. Speed SHALL use magnitude in the selected speed units. Speed and load SHALL qualify only when their unrounded values strictly exceed their thresholds; battery charge SHALL use its specified voltage comparisons and qualification rules. Standalone BMS, missing and out-of-range readings SHALL not qualify.

#### Scenario: Duty component exceeds load threshold

- **WHEN** fresh duty is 66 percent with a 65 percent threshold
- **THEN** the announcement says the current whole-number percentage and says “duty” before the percentage

#### Scenario: Equality and missing readings

- **WHEN** a component equals the threshold, is unavailable, or is two seconds old
- **THEN** it does not trigger an announcement

#### Scenario: Reverse speed

- **WHEN** a fresh reverse speed has a magnitude exceeding the speed threshold
- **THEN** it qualifies and speech identifies the positive speed and selected unit

### Requirement: Repeat cadence without backlog

For speed, load and footpad rules, the first qualifying reading SHALL trigger an immediate attempt when cooldown permits. Subsequent speed/load/footpad attempts SHALL be separated by at least the configured interval using monotonic time and SHALL use the latest eligible value. Below-threshold crossings, settings edits and failed attempts SHALL not bypass cooldown. Busy playback SHALL cause due announcements to be skipped, not queued. Missed intervals SHALL not produce catch-up speech. Battery charge SHALL use its separate fixed cadence and shared-slot priority rules.

#### Scenario: Continued excess

- **WHEN** duty stays above 65 percent with a five-second interval
- **THEN** speech attempts occur no more often than every five seconds and each speaks the current value, even when unchanged

#### Scenario: Threshold chatter

- **WHEN** readings repeatedly cross the threshold during cooldown
- **THEN** they do not produce extra attempts; once cooldown expires a fresh qualifying reading can trigger speech

#### Scenario: Slow speech or delayed worker

- **WHEN** an utterance lasts longer than the interval or execution resumes after missed intervals
- **THEN** utterances never overlap and old intervals are not replayed

### Requirement: Bounded speech admission and recovery

The service SHALL admit at most one active utterance across automatic and test speech, with no pending utterance queue and a ten-second terminal deadline. A busy automatic attempt SHALL be dropped. A busy test SHALL fail with invalid_state. Manual tests SHALL ignore the automatic repeat interval and MAY repeat immediately after the previous utterance finishes. Each admitted test SHALL advance the speed/load/footpad automatic cooldown without advancing battery charge cooldowns. Audio readiness SHALL refresh at bounded native lifecycle opportunities, allowing recovery without app restart or continuous initialization retries. Normal logs SHALL omit spoken readings and private identity.

#### Scenario: Concurrent test and automatic trigger

- **WHEN** automatic speech is active and a test is requested
- **THEN** the test fails with invalid_state and cannot enqueue or interrupt that speech

#### Scenario: Voice becomes available

- **WHEN** users return from system settings after making an offline voice available
- **THEN** readiness is re-evaluated and a subsequent eligible attempt can speak without restarting the app

### Requirement: Preserve existing announcement preferences

Existing supported stored settings missing the footpad fields SHALL open with footpad announcements disabled and a one-second delay, preserving every existing preference and supported recording. Missing batteryChargeEnabled SHALL default to true; explicit false SHALL remain false. New saves SHALL persist the footpad fields and batteryChargeEnabled. Invalid present values SHALL follow existing validation failures rather than silently defaulting or resetting storage.

#### Scenario: Open supported settings predating footpad alerts

- **WHEN** supported stored settings contain non-default speed, load, volume and repeat preferences but no footpad fields
- **THEN** those preferences remain unchanged, footpad alerts default off with a one-second delay, and the expanded settings survive save and reopen

#### Scenario: Open supported settings predating battery charge alerts

- **WHEN** supported stored settings omit batteryChargeEnabled and contain customized existing preferences
- **THEN** battery charge announcements default on, all existing preferences and supported recordings remain intact, and saving persists the new field

#### Scenario: Explicit off and invalid present value

- **WHEN** batteryChargeEnabled is explicitly false or has an invalid present type
- **THEN** false survives save and restart, while an invalid present value fails validation without defaulting or resetting storage
