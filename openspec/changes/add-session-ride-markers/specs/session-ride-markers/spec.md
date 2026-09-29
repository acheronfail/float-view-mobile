## Purpose

Let riders save durable bookmarks during a session and revisit the same moments in telemetry charts and recorded locations after the ride.

## ADDED Requirements

### Requirement: Quick marker capture

The live view SHALL provide a centre target on its main large gauge that saves a marker after two deliberate primary-pointer taps. The target SHALL support equivalent single keyboard or assistive activation, retain native button semantics, and prevent duplicate activation from synthesized clicks. It SHALL be at least 64 CSS pixels in each dimension and expose an accessible save-marker name and a brief double-tap hint.

#### Scenario: Double tap
- **WHEN** two primary taps complete in the centre target within 350 ms and within 24 CSS pixels of each other, with each held at most 300 ms and moving at most 12 CSS pixels
- **THEN** exactly one marker save is requested for the displayed active saved session

#### Scenario: Cancelled or unrelated input
- **WHEN** a single tap, non-primary button, drag, simultaneous second pointer, cancellation, target exit, route destruction, or session change interrupts the gesture
- **THEN** no marker is requested from that incomplete gesture

#### Scenario: Triple tap and synthesized click
- **WHEN** a successful pair is followed by a third tap or its synthesized click
- **THEN** no second marker is created without another completed pair

#### Scenario: Accessible activation
- **WHEN** the eligible centre control receives keyboard or assistive activation
- **THEN** one marker is requested without a timed double-key gesture, and key repeat does not generate repeated markers

### Requirement: Durable and session-scoped marker creation

The authenticated marker command SHALL identify the intended session and a bounded operation ID. The session owner SHALL assign a stable marker identity and authoritative session-relative elapsed time, persist it with the logs, and acknowledge success only after durable commit. Creation SHALL require an active saved session with an attached writer and SHALL NOT depend on a recent telemetry sample or available GPS. Markers SHALL extend review duration when their time follows the last sample without making later publication regress.

#### Scenario: Saved session while idle
- **WHEN** an eligible session receives a marker command between telemetry samples or while stationary
- **THEN** the committed marker records the session clock time rather than the last sample time, and is visible within the published review duration

#### Scenario: Guest or ended session
- **WHEN** a marker command targets a guest connection, ended session, missing writer, or different session from the current one
- **THEN** it fails with a bounded error and does not create a session or attach the marker elsewhere

#### Scenario: Reconnect window
- **WHEN** a connection is recovering while the same saved session and writer remain active
- **THEN** a marker can be saved to that session with continuous elapsed time

#### Scenario: Duplicate request or lost response
- **WHEN** the same session and operation ID are submitted again after a successful commit, including after reconnect or response loss
- **THEN** the original committed marker is returned without another marker or another count increment

#### Scenario: Close race and recovery
- **WHEN** a save races with session closure or the process stops after commit
- **THEN** the serialized order either commits the marker to the intended session or rejects it, and recovery preserves every committed marker without silently assigning it to a later session

### Requirement: Optional recorded location

A marker SHALL contain either a recent accepted recorded location from the same session or no location. The selected fix SHALL already be committed, precede or equal the marker time, and be no more than 10 seconds old by both session elapsed time and observation timestamp. Location recording and permission SHALL permit capture at that time. Stored coordinates, accuracy, and source observation time SHALL remain stable after creation; the application SHALL NOT interpolate, substitute a future fix, or reuse another session's fix.

#### Scenario: Recent fix
- **WHEN** an eligible fix exists when a marker is saved
- **THEN** the marker retains that fix's coordinates, accuracy, and observation time for later map display

#### Scenario: No eligible fix
- **WHEN** GPS is disabled, permission is unavailable, no fix exists, or the last fix is too old
- **THEN** the time-only marker still saves, remains available in chart review, and is labelled “No location” without prompting for GPS access

#### Scenario: Later fix or pruning
- **WHEN** a new GPS fix arrives later or the original track is pruned
- **THEN** the marker's stored location does not move or disappear

### Requirement: Honest and brief capture feedback

A confirmed save for the currently displayed session SHALL produce an approximately 250 ms glow/pulse on the live gauge display and a small “Marker saved” toast lasting approximately two seconds. Feedback SHALL remain readable in light and dark themes, preserve gauge legibility, avoid blocking interaction, and announce the toast accessibly. Reduced-motion settings SHALL replace the pulse with a brief static highlight. At most one feedback toast and one pending save SHALL exist at a time.

#### Scenario: Successful save
- **WHEN** durable save confirmation arrives for the displayed session
- **THEN** the short glow/pulse and success toast appear once

#### Scenario: Failure or unknown outcome
- **WHEN** saving fails or the transport loses the result
- **THEN** the application shows a concise failure or unconfirmed-save message, does not show success feedback, and does not automatically retry with a new operation ID

#### Scenario: Session changes while pending
- **WHEN** the rider changes session or leaves the view before a response arrives
- **THEN** no response animates the new session and obsolete gesture/feedback timers are disposed

### Requirement: Bounded marker history and admission

The system SHALL admit at most 1,000 markers per session and at most one new marker per 500 ms. Duplicate operation IDs SHALL return their existing result before applying these limits. Marker reads SHALL be authenticated, filtered by session, committed published snapshot and elapsed range, cursor-paged at no more than 256 markers per page, and constrained by existing byte limits. Results SHALL be ordered by elapsed time and stable event sequence. Metadata SHALL include a marker count without embedding marker arrays. Reads and UI collections SHALL remain bounded independently of telemetry event volume.

#### Scenario: Full session or rapid submissions
- **WHEN** a new marker exceeds the session cap or rate limit
- **THEN** the command returns a bounded error without evicting older markers or displaying save success

#### Scenario: Paginated review
- **WHEN** the client follows valid marker cursors for one published snapshot
- **THEN** each committed in-range marker appears once in stable order, including equal-time markers, without requiring all telemetry events to be loaded

#### Scenario: Stale snapshot or malformed cursor
- **WHEN** a read names a superseded snapshot, an invalid cursor, or a cursor for another session/range
- **THEN** the established conflict/invalid-request behavior applies, and the client refreshes metadata before a bounded retry where appropriate

### Requirement: Synchronized marker review

Session charts SHALL show vertical orange marker bars at their exact elapsed times on each visible time-series plot, independent of telemetry downsampling. The map SHALL show high-contrast orange marker icons distinct from the rider icon for markers with coordinates. A compact marker navigator SHALL provide previous/next and individual marker selection with marker index and elapsed time. Selecting any marker SHALL select the same elapsed time across charts and map, reveal that time in the chart viewport, and centre the map on its recorded position when present. Selection SHALL support touch, mouse, keyboard, and assistive activation.

#### Scenario: Select a chart or map marker
- **WHEN** the user activates a marker bar or map icon
- **THEN** the navigator, chart detail selection, and map selection refer to that same marker and elapsed time

#### Scenario: Overlapping markers
- **WHEN** multiple marker times or coordinates overlap at the current zoom
- **THEN** the UI can group them visually with a count and every marker remains individually reachable through the navigator

#### Scenario: Marker without location
- **WHEN** a time-only marker is selected
- **THEN** its chart position and detail are selected without fabricating a map icon or reserving an empty map

#### Scenario: Only marker coordinates remain
- **WHEN** a session has stored marker coordinates but no retained base track
- **THEN** the map can display those markers without inventing a route between them

### Requirement: Marker retention and portability

Committed markers SHALL survive disconnect, WebView replacement, process restart, session recovery, telemetry/location pruning, and native archive export/import with identity, ordering, time, and optional location intact. A session containing a committed marker SHALL be exempt from automatic no-riding and minimum-distance discard. Explicit session or board deletion SHALL still remove the owned markers. Sessions without markers SHALL retain their existing retention policy. CSV SHALL remain telemetry-only. Imports SHALL validate marker payload types, ordering, bounds, and coordinates; incompatible development formats SHALL use explicit format handling without migrations or legacy readers.

#### Scenario: Short stationary marked session
- **WHEN** a session with a committed marker ends without riding or below the configured minimum distance
- **THEN** it remains available for review, including after recovery finalization

#### Scenario: Prune and archive round trip
- **WHEN** a marked session is pruned, exported in the native archive, and imported
- **THEN** all markers retain their identities, exact times, and optional stored locations

#### Scenario: Explicit deletion
- **WHEN** the user deletes the session or forgets its board
- **THEN** its markers are deleted with the rest of its logs

#### Scenario: Malformed archive or storage failure
- **WHEN** marker payloads are invalid or current-format storage is corrupt or unavailable
- **THEN** the operation fails safely without silently resetting current data or reporting an unsuccessful marker as saved
