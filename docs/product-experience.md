# Float View product experience

Status: product and navigation specification for the first complete experience.

Float View has two primary destinations: **Device** and **Sessions**. Native
shells host the same responsive Svelte experience on iOS, Android, and macOS.
Native UI remains limited to OS-required permission, notification, background,
sharing, and lifecycle surfaces.

## Navigation model

```text
Start
├── Device
│   ├── Saved devices
│   ├── Scan for a device
│   ├── Connecting
│   └── Live device view
└── Sessions
    ├── Filterable session list
    └── Session review
        └── Diagnostic zoom
```

The two tabs remain available as top-level navigation where screen size permits.
On compact mobile screens, entering Live, Review, or Diagnosis may use a
full-screen detail route while preserving the selected tab and back stack.

App foreground restoration returns to the most recent valid route. If the
WebView was destroyed, the new frontend reconstructs its route from the native
snapshot; it never assumes that its previous JavaScript state survived.

## Device tab

### Saved devices

The disconnected Device tab shows:

- previously saved boards, most recently connected first;
- user-defined name and latest known advertised name;
- last-seen and last-connected time;
- latest known firmware/hardware identity;
- availability when currently discovered;
- last connection result when useful;
- actions to connect, rename, forget, or scan.
- access to Board Settings for battery, motor, rails/appearance, CAN/BMS, and
  calibration.
- a visible incomplete-profile indicator when optional settings are not yet
  configured.

Saved identity is backend-owned and app-private. The API exposes an opaque
`savedDeviceId`; it does not expose an Android MAC address or assume an Apple
peripheral UUID has portable meaning. Each backend stores the platform binding
needed to reconnect. Forgetting a device removes that binding but never deletes
sessions already recorded from it.

The empty state explains what a VESC is and offers **Scan for a device**.

Board Settings and its discovery/override rules are defined in
[`board-profiles.md`](board-profiles.md). The saved-device display name and
visual descriptor make similar boards easy to recognize.

### Scan

Scanning:

- presents permission/radio recovery before or during the scan;
- shows supported discovered devices incrementally;
- deduplicates advertisements by the backend's opaque identity;
- lets the user select, connect, and optionally name/save a device;
- has explicit scanning, timed-out, cancelled, permission-denied, radio-off, and
  failed states.

A successfully connected new board is saved by default. Saved boards never
auto-connect at launch; the user explicitly selects Connect. A later setting may
allow one-time connections.

### Connection

After selection, the UI displays the contract-defined connection states rather
than an indefinite spinner. Once the device reaches `ready`:

1. the backend durably creates a user-facing recording session;
2. the native logging job starts automatically;
3. the Device tab enters Live;
4. initial firmware and state information populate as available.

There is no easy-to-forget “start logging” step. If durable storage cannot start,
the UI must show a blocking logging failure rather than silently entering a
normal live state. A diagnostic/test capability may allow a non-recording
connection, but ordinary product flow records by default.

## Live device view

The Live view is designed for quick, legible use while riding. It shows:

- connection, firmware, riding state, and background eligibility;
- recording session state, current sampling mode/rate, committed-through time,
  storage pressure, drops, and gaps;
- primary values such as speed, duty cycle, battery voltage, power, currents,
  temperatures, distance, and supported orientation/state data;
- footpad sensor state and firmware fault/ride state;
- active alerts with clear severity and acknowledgement behavior;
- controls for alert settings, device details, and safe disconnect.
- access to the connected board's settings and automatic/manual discovery state.

The shared frontend may offer configurable layouts later. The native backend
continues evaluating alerts and logging if the WebView disappears.

### Automatic adaptive logging

Initial defaults:

- `idle`: probe Refloat ride state at 2 Hz and persist 1 normalized sample per
  second while not riding;
- `riding`: 10 normalized samples per second;
- `postRide`: retain the riding rate for 15 seconds after leaving riding state,
  then return to idle.

All rates and the post-ride duration are configurable within backend-advertised
limits. The backend persists requested and effective values with every session.

Refloat's reported ride state is authoritative for the initial detector. The
detector runs natively from normalized state. A state transition, fault, footpad
edge, connection event, and alert is always persisted even when it occurs
between scheduled samples. The transition to riding raises the requested
sampling rate immediately. Debounce/hysteresis prevents rapid mode flapping.
The backend advertises effective idle detection latency and must not claim
immediate transition if the 2 Hz probe cannot provide it.

### Automatic session lifecycle

A recording session begins when a ready connection obtains durable logging
ownership. It contains:

- low-rate pre-ride context;
- one or more high-rate riding intervals;
- high-rate post-ride context;
- low-rate connected/non-riding intervals;
- alerts, state transitions, faults, gaps, lifecycle, and connection events.

The session normally ends when the user disconnects or the connection reaches a
terminal state. Short OS restoration/reconnection gaps may continue the same
session when the persisted job identity is restored; the gap is recorded.
Unexpected disconnect uses bounded exponential reconnection for the active
session, with a five-minute default configurable from disabled through the
backend maximum. Explicit disconnect never reconnects. Multiple rides during
one connection remain distinct `RidingInterval` rows in that Session.

Force-quit ends recoverable ownership and the next launch marks the Session
`interrupted`. It never invents continuity for unobserved time.

## Sessions tab

The Sessions tab lists all native recordings, including active, completed,
interrupted, partial/recovered, and failed sessions where usable data exists.

Each row can show:

- saved-device name or the immutable device snapshot captured at recording time;
- start date/time;
- riding duration and total connected duration;
- distance;
- firmware identity;
- integrity/gap indicator;
- selected summary values such as maximum speed or duty cycle;
- active/interrupted/completed state.

Server-side filtering and sorting are required for:

- historical device identity, including devices that were later forgotten;
- start/end date;
- minimum/maximum riding or total duration;
- minimum/maximum distance;
- session state/integrity;
- ascending or descending date, duration, or distance.

Results are cursor-paginated. The frontend never loads the full session catalog
or every session's samples to render this list.

An active session appears in the list and opens the current Live view or a
read-only committed-history view depending on navigation.

### Linked sessions

Users may enable automatic linking of adjacent Sessions from the same
`deviceHistoryId` when their gap is below a configured duration. This is useful
for charging breaks. It is disabled by default; enabling it initially suggests
two hours and accepts 1 minute through 24 hours.

A linked group is reversible catalog metadata. It never joins, rewrites, or
deletes member databases. The Sessions list renders it as one expandable entry
with aggregate duration/distance and clear segment boundaries. Review combines
bounded member summaries/series in the shared frontend. Diagnosis always enters
one concrete member Session so timestamps, integrity, profile, and firmware
provenance remain unambiguous.

## Session review

Opening a completed session first returns a summary and bounded overview, then
renders:

- key ride statistics;
- riding and connected durations, distance, gaps, and integrity;
- battery use and voltage/sag trends;
- speed, duty cycle, ERPM, power, currents, and temperatures;
- footpad, fault, ride-state, and alert markers;
- pitch/roll and firmware-specific values where present;
- synchronized custom Svelte/SVG charts, timeline cursor, and event list.

GPS and route maps are post-release; the first release does not request location
permission.

The prior
[`acheronfail/float-view`](https://github.com/acheronfail/float-view) viewer is a
UX reference for synchronized chart/map selection, points of interest, footpad
analysis, and responsive desktop/mobile behavior.

The backend returns resolution-appropriate buckets. Zooming requests a narrower,
higher-resolution range. The shared frontend owns visualization, units, layout,
derived explanations, and interaction.

## Diagnostic zoom

From Review, the user can:

- select a fault, alert, footpad transition, gap, ride-state transition, or chart
  point;
- choose **Inspect** to open Diagnosis centered on that event/time;
- progressively widen or narrow the time window;
- choose and overlay relevant signals;
- step sample-by-sample at full committed resolution;
- compare observed device events, derived alerts, connection/storage gaps, and
  active rule configuration;
- return to the same cursor/zoom in Review.

Diagnosis initially provides evidence rather than asserting a single cause. It
must make questions such as these answerable:

- Did the firmware fault or change ride state?
- Did one or both footpad sensors become inactive?
- Was there battery voltage sag under current?
- Was duty cycle near saturation?
- Did motor/battery current, temperature, or power show a limit?
- Was the apparent event actually a connection, suspension, command, or storage
  gap?

Automated causal analysis and incident reports can be added later in the shared
frontend using the same bounded diagnostic APIs.

## User-facing data model

The API distinguishes:

- `SavedDevice`: a persistent platform-local reconnect identity and user label;
- `deviceHistoryId`: a non-connectable grouping identity copied into sessions so
  forgotten-device history remains filterable without retaining BLE credentials;
- `BoardProfile`: versioned local battery, motor, rails/visual, CAN/BMS, and
  calibration settings for a saved board;
- `DeviceSnapshot`: immutable device/firmware/name information copied into a
  session so history survives rename/forget;
- `Session`: the user-visible recording and summary/filter object;
- `SessionGroup`: reversible catalog metadata linking ordered same-device
  Sessions without merging their canonical stores;
- `RidingInterval`: a time range in which the board was in riding state;
- `LoggingJob`: the current native execution owner for an active session;
- `Sample`: a lossless normalized time/sequence row;
- `SessionEvent`: fault, footpad, ride-state, alert, lifecycle, gap, connection,
  command, or storage event;
- `RollupBucket`: derived review data at a declared resolution;
- `AlertRuleSnapshot`: the effective rules/configuration for that session;
- `BoardProfileSnapshot`: immutable discovered/configured/effective settings used
  for session display and derived values.

Use `sessionId` in user-facing history/query APIs. `jobId` exists only for active
logging control and references one `sessionId`. The earlier ambiguous `logId`
name is retired from the v1 planning contract.

## Catalog schema

The app-private catalog contains at least:

```text
saved_devices
  saved_device_id
  device_history_id
  user_name
  platform_binding
  advertised_name
  first_seen_at
  last_seen_at
  last_connected_at
  latest_firmware
  created_at
  updated_at

board_profiles
  board_profile_id
  saved_device_id
  profile_revision
  display_name
  profile_json
  created_at
  updated_at

sessions
  session_id
  device_history_id
  saved_device_id nullable
  device_snapshot
  started_at
  ended_at nullable
  total_elapsed_ms
  riding_elapsed_ms
  distance_m
  state
  integrity
  sample_count
  event_count
  gap_count
  max_speed_mps nullable
  max_duty_ratio nullable
  database_format_version
  session_database_path

session_groups
  session_group_id
  device_history_id
  created_at
  updated_at
  link_origin
  aggregate_summary

session_group_members
  session_group_id
  session_id
  position

logging_jobs
  job_id
  session_id
  persisted state/counters/background ownership
```

Indexes support saved-device/date, duration, distance, state/integrity, and the
selected sort orders. A session keeps its immutable `device_snapshot` even if
the saved-device row is forgotten. Forgetting deletes or invalidates the
platform binding; it does not remove `device_history_id` from sessions.

Each per-session database contains:

```text
metadata
board_profile_snapshots
riding_intervals
samples
events
alerts
alert_rule_snapshots
rollup_buckets
checkpoints
terminal_record
optional_raw_packets
```

## Product API additions

Saved devices:

- `savedDevices.list`
- `savedDevices.rename`
- `savedDevices.forget`
- `savedDevices.get`

Sessions:

- `sessions.list({ filters, sort, cursor, limit })`
- `sessions.getFilterOptions`
- `sessions.get({ sessionId })`
- `sessions.getSummary({ sessionId })`
- `sessions.querySeries({ sessionId, range, fields, resolution, cursor? })`
- `sessions.listEvents({ sessionId, range, types?, cursor? })`
- `sessions.getDiagnosticWindow({ sessionId, eventId?, centerElapsedMs?, beforeMs, afterMs, fields?, cursor? })`
- `sessions.export({ sessionId, format })`
- `sessions.delete({ sessionId })`

Session groups:

- `sessionGroups.list`
- `sessionGroups.get`
- `sessionGroups.unlink`

Settings:

- `settings.get`
- `settings.update`
- settings include requested idle/riding/tail rates, reconnect duration,
  automatic-link enabled/window, alert output choices, and diagnostic raw-packet
  capture limits;
- backends clamp values to advertised limits and return requested/effective
  values.

Active logging:

- `logging.get({ jobId })`
- `logging.getActive`
- `logging.stop({ jobId })`
- automatic start is backend-owned after device readiness;
- manual `logging.start` remains test/advanced capability rather than ordinary
  product flow.

Events:

- `sessions.created`
- `sessions.updated`
- `logging.jobChanged`
- `logging.samplingModeChanged`
- `logging.samplesCommitted`
- `alerts.triggered`

## UX acceptance scenarios

1. First launch with no saved devices scans, connects, saves, enters Live, and
   starts a durable session without an extra logging action.
2. Returning launch shows the saved board and reconnects through an explicit
   user action.
3. Idle logging runs at the effective low rate, riding transition raises it to
   10 Hz, and post-ride tail returns it to idle without losing transition events.
4. Backgrounding destroys the WebView while native logging/alerts continue;
   foreground reconstruction returns to Live with committed catch-up.
5. Disconnect flushes the session, which immediately appears in Sessions.
6. Forgetting a saved device leaves its historical sessions readable and
   filterable through their captured snapshot.
7. A catalog containing thousands of sessions remains paginated and filterable.
8. An eight-hour ride opens from summary/rollups without loading raw samples.
9. Zooming progressively increases resolution while preserving cursor/event
   alignment.
10. Selecting a fault or footpad event opens a lossless diagnostic window with
    voltage, current, duty, state, footpads, alerts, and gaps aligned.
11. A recovered/partial session is visible with an integrity warning and
    readable salvage rather than disappearing.
12. A board can be named and configured as `20S2P` with a chemistry, motor,
    rails/visual descriptor, voltage correction, manual/automatic CAN ID, and
    smart-BMS mode; its effective profile is snapshotted into the next session.
13. Optional profile fields never block logging; missing configuration is clearly
    indicated in Device and Review.
14. Two same-device Sessions separated by a qualifying charge break appear in a
    reversible linked group without either database changing.
15. Deletion requires confirmation, is idempotent, and never affects another
    member Session; lossless archive and CSV exports are generated from canonical
    data.
