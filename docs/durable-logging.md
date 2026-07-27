# Durable native logging and background lifecycle

Status: core product architecture and R3-R8 acceptance specification.

Logging is Float View's primary capability. The UI is a disposable viewer and
controller for a native logging engine; it is never the owner, scheduler, buffer,
or durable store for a logging job.

This document defines the shared logging architecture, platform persistence,
background behavior, WebView teardown/recreation, data-access API, failure
semantics, and verification requirements.
Navigation, automatic session creation, adaptive idle/riding rates, and
user-facing review/diagnosis behavior are defined in
[`product-experience.md`](product-experience.md).
Per-board discovered/manual settings, corrections, derived values, and immutable
session snapshots are defined in [`board-profiles.md`](board-profiles.md).

## Three core log capabilities

Every storage, API, sampling, and performance decision must support all three
products below from one canonical log. Do not create separate incompatible
“live,” “review,” and “diagnostic” log formats.

### 1. Live use and alerting

While a board is in use, the system provides:

- low-latency device state, speed, duty cycle, currents, voltages, wattage,
  temperatures, orientation, firmware state, footpad sensors, and faults;
- continuously updating shared-frontend displays;
- configurable threshold, boolean-edge, duration/debounce, hysteresis, and
  cooldown alerts;
- native notification/audio/haptic presentation where platform policy allows;
- alert and state-change markers committed into the same ride log.

The first configurable alerts are duty cycle, high/low pack voltage,
motor/controller temperature, individual/combined footpad state, and connection
loss. Alerts are advisory. Acknowledgement clears an occurrence but does not
disable its rule.

The native alert evaluator receives every normalized sample before UI
coalescing. It cannot depend on JavaScript because alerts and logging must remain
functional while the WebView is absent. The shared frontend owns rule creation,
validation UX, display, and chart overlays; the backend implements only a small,
declarative, generated and cross-platform-conformant rule engine plus the
platform-native output surface.

Rules and their effective version are persisted with the ride so later diagnosis
can explain why an alert fired. Alerts are advisory and must not send motor,
configuration, or firmware commands.

### 2. Ride review

After a ride, the shared frontend provides fast overview and trend analysis for:

- battery voltage, consumption, sag, amp-hours, and watt-hours;
- speed, duty cycle, ERPM, distance, and elevation;
- motor/battery current and calculated wattage;
- motor, controller, and battery temperatures;
- footpad state, ride state, faults, alerts, and gaps;
- pitch, roll, setpoints, and supported firmware-specific values.

Opening a long ride must not deserialize every raw sample into the WebView. The
native query layer returns a bounded overview and resolution-appropriate
min/max/average/first/last buckets. The shared frontend owns chart types,
layout, units, interaction, synchronized cursor/zoom, map correlation, trend
interpretation, and ride summaries.

### 3. Issue diagnosis

Users can select a fault, alert, state transition, footpad change, drop, or
arbitrary timestamp and inspect a high-resolution window around it. Diagnosis
requires:

- lossless committed samples at the highest effective logging rate;
- aligned timestamps and sequences across every captured field;
- explicit VESC/Refloat fault and ride-state events;
- both footpad channels and derived footpad state;
- voltage, duty, currents, wattage, ERPM, speed, temperatures, pitch/roll, and
  other supported causal signals;
- connection, command timeout, malformed packet, dropped sample, suspension, and
  storage-gap events;
- firmware/hardware/profile identity and the alert rules active at that time;
- a raw packet capture capability for explicitly enabled diagnostic sessions,
  bounded separately and disabled by default.

Overview rollups are always derived. They never replace or destructively
downsample the canonical high-resolution samples. A deep query uses bounded
pages around a time/event and can progressively expand its window.

## One native sample pipeline

```text
CoreBluetooth / Android GATT
            |
            v
     VESC packet decoder
            |
            v
 normalized raw sample/event + profile revision + sequence/timestamps
            |
            +----> native alert evaluator ----> alert event + native signal
            |
            +----> durable write batches -----> canonical samples/events
            |                                      |
            |                                      +--> derived rollup buckets
            |
            +----> bounded/coalesced live fanout -> WebSocket -> shared UI
```

There is one decoder and one normalized sequence. Alert evaluation, persistence,
rollups, and UI delivery are consumers; none reparses Bluetooth independently.
Backpressure in the WebView path cannot delay alert evaluation or durable
writes. The log distinguishes:

- `observedSequence`: decoded by the native service;
- `committedSequence`: transactionally durable;
- `deliveredSequence`: last sent to a particular UI session.

This makes dropped UI frames harmless and makes live-versus-durable state visible
to users and tests.

## Product guarantees

A native logging job:

- is created durably before automatic Live readiness or an advanced
  `logging.start` returns success;
- continues when its WebSocket closes or its WebView is unloaded/destroyed;
- writes into app-private durable storage, never a cache or temporary directory;
- survives Activity/scene/window recreation, process restart, app updates, and
  device restart to the degree the operating system permits;
- bounds unwritten memory and reports the maximum durability window;
- records gaps, drops, errors, lifecycle changes, firmware identity, effective
  interval, and terminal state;
- is crash-recovered on the next native backend launch;
- stops and flushes deterministically on explicit stop or disconnect;
- never silently deletes completed user logs to reclaim space;
- can be queried through the local API while active and after completion.

“Continues in the background” is capability-qualified. Android, iOS, and macOS
have different execution guarantees. A platform must record an explicit gap or
terminal interruption when the OS prevents sampling; it must never synthesize
samples or leave a job appearing continuously active.

## Ownership and isolation

```text
Native host lifecycle
        |
        v
Application service -----> LoggingJob state machine
        |                         |
        |                         +----> Native sampler/protocol scheduler
        |                         +----> Durable LogStore
        |                         +----> Storage monitor
        |
        +----> bounded UI event sink ----> loopback WebSocket ----> disposable WebView
```

The backend process/service owns Bluetooth, the VESC protocol scheduler, logging
jobs, database handles, and lifecycle recovery. The WebView receives bounded
live previews and reads durable history through API queries. Destroying the
WebView must not close the database, Bluetooth connection, native timers, or
logging job.

Required injected ports:

```text
LogCatalog
  createJob(metadata)
  updateJob(job)
  listJobs(filter)
  recoverJobs()

LogWriter
  append(samples, events)
  flush(reason)
  checkpoint()
  close(terminalState)

LogReader
  querySamples(sessionId, range, fields, resolution, cursor)
  getSummary(sessionId)
  verifyIntegrity(sessionId)

StorageMonitor
  availableBytes()
  observePressure()

FrontendLifecycle
  webRuntimeDidStart()
  webRuntimeWillStop(reason)
```

All ports have deterministic fake implementations. No logging-layer type imports
WebKit, Android WebView, `Activity`, or UI state.

## Logging job state machine

```text
creating
  -> active_foreground
  -> active_background
  -> stopping
  -> completed

active_* -> suspended_by_os -> active_background | interrupted
active_* -> storage_pressure -> active_* | failed
active_* -> failed
creating  -> failed
```

The persisted job state contains:

- opaque `jobId`, `sessionId`, and `deviceId`;
- schema/profile version and firmware/hardware identity;
- requested and effective sample interval;
- UTC start/end timestamps plus monotonic elapsed duration;
- foreground/background/suspension intervals;
- last allocated sequence number and last committed sequence number;
- committed sample, declared-gap, dropped, and error counts;
- bytes written and last successful commit timestamp;
- durability policy and maximum uncommitted window;
- background capability and current eligibility;
- terminal reason and structured last error;
- database format and migration version.

On launch, `recoverJobs()` inspects all non-terminal jobs and their stores. It
uses committed sequence/checkpoint data to finalize, resume, or mark them
`interrupted`; it never leaves a stale job claiming to be active.

## Durable storage format

The baseline design uses native SQLite in write-ahead logging mode:

```text
FloatView Application Support/
  catalog.sqlite
  sessions/
    <opaque-session-id>.sqlite
  exports/
    <derived, user-requested files>
```

- `catalog.sqlite` contains saved devices, indexed session summaries, active job
  metadata, storage state, and the session-file index.
- Each session has its own SQLite database to isolate failure and make export,
  retention, and integrity checking explicit.
- Each session database contains immutable metadata, ordered samples,
  board-profile snapshots, raw observations, lifecycle/error
  events, alerts, gap records, checkpoints, derived rollup buckets, and a
  terminal record.
- WAL permits bounded batched writes while the UI reads committed history.
- Transactions provide atomic batches; incomplete transactions roll back during
  recovery.
- Stop/flush checkpoints WAL state, validates counts, writes the terminal record,
  and only then marks the catalog job terminal.
- Exports are derived and replaceable. Deleting an export never deletes its
  native source log.

The first schema stores normalized, explicitly unit-labelled telemetry columns
needed by the initial profile, plus a versioned extension payload for firmware
fields not yet promoted to columns. It indexes `(elapsedMs, sequence)` and event
time/type for fast range and incident lookup. Rollup tables are keyed by
resolution and time bucket and contain first/last/min/max/average/count values.
Optional raw packets require a separate capability and retention decision; they
are not enabled by default.

Raw observed voltage/speed/distance inputs are canonical. Samples identify the
Board Profile revision/model needed to reproduce frontend corrections and
estimates. A mid-session profile edit appends a profile-change event and
snapshot; it never rewrites earlier samples. Native alerts may transiently use
the same versioned correction but never replace the raw evidence.

Use a full durability setting for committed transactions unless platform tests
demonstrate an unacceptable resource cost and a documented alternative is
accepted. Batch commits are bounded by both sample count and elapsed time. The
initial target is at most one second of uncommitted samples; the backend reports
the actual `durabilityWindowMs` so tests and users can distinguish committed
from recently observed samples.

Every file has a format version and migration path. Migrations run against a
copy in destructive tests and never replace the only user log until validation
succeeds.

## Shared SQL and minimum backend surface

Keep schema migrations, integrity queries, rollup SQL, and range-query templates
as shared versioned assets under a planned `logging/sql/` directory. Swift and
Kotlin provide only narrow bindings, transaction scheduling, row mapping, and
platform filesystem/lifecycle integration.

Use OS-provided SQLite APIs directly for the foundation:

- Apple uses system `SQLite3`;
- Android uses platform `android.database.sqlite`;
- no ORM is required for the first slice.

This supersedes the earlier Room assumption. The schema is intentionally narrow,
and identical SQL plus conformance fixtures reduce duplicated implementation
more than two platform-specific ORM models would.

Native backends implement only:

- Bluetooth and VESC/Refloat protocol I/O;
- timestamping, sequencing, and bounded native sampling;
- durable SQLite transactions, recovery, integrity, and generic range/rollup
  queries;
- the small declarative alert evaluator required while the frontend is absent;
- platform lifecycle/background ownership and native alert output;
- the authenticated local transport and generated API model mapping.

The shared frontend implements:

- live dashboards and device presentation;
- alert authoring, editing, validation messages, and visual overlays;
- ride list/review, custom Svelte/SVG charts, units, zoom, cursor synchronization, and trend
  calculations over bounded query results;
- diagnostic workflows, signal selection, incident comparison, and explanation
  UI;
- export/import orchestration and future sharing UX.

Any proposed backend feature must explain which native API, background,
durability, security, or data-locality constraint prevents it from living in the
shared frontend.

## Performance budgets

R3 establishes reproducible small, medium, and long synthetic rides and an
older-supported mobile reference device. Initial budgets, adjusted only through
an accepted measured decision, are:

| Path                                | Initial budget                                                              |
| ----------------------------------- | --------------------------------------------------------------------------- |
| Decode to native alert evaluation   | p95 under 20 ms after a complete BLE response                               |
| Decode to foreground UI delivery    | p95 under 100 ms at the effective UI rate                                   |
| Live UI event rate                  | configurable and capped; initial maximum 20 Hz with coalescing              |
| Durable commit window               | no more than 1 second or the configured sample batch, whichever comes first |
| Open ride and return first overview | under 500 ms for an 8-hour reference log                                    |
| Overview query result               | at most approximately 2 points per horizontal pixel per field               |
| Pan/zoom review query               | p95 under 150 ms after warm open                                            |
| Deep diagnostic query               | p95 under 250 ms for a 30-second full-resolution window                     |
| Frontend memory                     | bounded by visible/query windows, not total ride duration                   |

Queries support cancellation and stale-result suppression. Benchmarks report
database size, sample rate, field count, query point count, process RSS, and
device/OS. Correct lossless data and alerts take priority over UI frame rate;
the UI path coalesces first.

## Storage policy

- Native logs live in application-support/internal private storage, not cache.
- Data survives process death, ordinary reboot, and app updates, but normal
  sandbox semantics mean uninstalling the app may remove it.
- The backend checks free space before start and during logging.
- Warn below 1 GiB free, escalate visibly below 500 MiB, and stop cleanly below
  250 MiB. Thresholds remain configurable within backend-advertised safe limits.
- At the stop threshold, the logger commits what it can and terminates with
  `storage_unavailable`; it does not continue pretending to record.
- Completed logs are never automatically deleted. Explicit confirmed,
  idempotent Session deletion is included in the first release.
- The first release exports a lossless native Float View archive and CSV.
  Imports, cloud sync, and cross-device retention are later features.
- Exclude session databases from OS/cloud backup initially. “Durable” means
  local crash-consistent app storage; uninstall may remove it.
- Release diagnostics do not print full telemetry, file contents, BLE addresses,
  or location by default.

## Platform storage and background methods

### iOS

Storage:

- store the catalog and session databases under the app container's Application
  Support directory;
- use SQLite directly through a narrow Swift persistence adapter;
- apply a data-protection class that permits the active log to remain writable
  while the device locks and permits restoration after first unlock;
- test locked-device writes and restoration on physical devices;
- exclude large logs from automatic backup by default until the backup/export
  policy is explicitly accepted.

Background execution:

- declare the `bluetooth-central` background mode;
- create `CBCentralManager` with a stable restoration identifier;
- restore peripherals, subscriptions, delegates, active job metadata, and
  database handles through CoreBluetooth state restoration;
- process characteristic callbacks and commit native batches without starting a
  WebView;
- on iOS 26 and later, start a visible logging Live Activity before backgrounding
  when physical testing confirms it supplies the CoreBluetooth privileges needed
  by the active protocol;
- use a short background task only to finish an in-progress commit/close, never
  as an indefinite logging timer;
- tear down the WKWebView promptly after entering background once its local API
  session has closed;
- recreate the WebView and bootstrap token on foreground, then recover state
  through `system.hello`, snapshot, and log queries.

iOS does not guarantee arbitrary periodic timers forever in the background.
CoreBluetooth can wake an opted-in app for Bluetooth events and preserve pending
connections/subscriptions, but the process may still be suspended or terminated.
If the VESC requires host polling rather than sending autonomous notifications,
uninterrupted background sample frequency must be proven on each supported iOS
and firmware profile. Otherwise the backend advertises best-effort background
logging, records suspension gaps, and resumes honestly.

Force-quit and restoration eligibility are OS-version-dependent. Capability and
job state must explain when restoration is unavailable rather than promising
process immortality.

iOS 17, iOS 18, and iOS 26 are qualified separately. A Live Activity is not
credited with improved logging on an OS until a physical-device test proves it.
GPS/location is post-release and background location permission must never be
used only as a process keepalive.

### Android

Storage:

- use platform SQLite APIs for the shared catalog/per-session schema, stored in
  internal app-private storage;
- enable WAL and use explicit transaction batches and migration tests;
- keep active logs out of cache/external storage, requiring no broad storage
  permission;
- run database integration and migration tests on real Android SQLite as well as
  fast JVM-level service tests.

Background execution:

- a foreground service owns Bluetooth, the application service, protocol
  scheduling, and active log writers;
- start it while the app is foregrounded and before the UI backgrounds;
- declare the `connectedDevice` foreground-service type and required Bluetooth
  and foreground-service permissions for the target SDK;
- display an ongoing notification with logging state and a safe stop action;
- destroy/release the Activity WebView without stopping the foreground service;
- bind/reconnect a recreated Activity to the existing service and rebuild UI
  state through the local API snapshot;
- use WorkManager for deferred recovery, validation, cleanup, or export work,
  never as the continuous BLE polling owner.

If the process/service is killed despite foreground-service ownership, startup
recovery validates the active log, marks the gap/interruption, and follows the
advertised reconnect policy. It does not insert samples for the missing period.

### macOS

Storage:

- store catalog/session databases under Application Support in the sandbox
  container
  where applicable;
- use the same Swift persistence adapter and logical schema as iOS;
- keep the database writer alive independently of window/WKWebView lifetime.

Background execution:

- closing or hiding the product window destroys/releases the WKWebView while the
  native backend continues;
- active logging remains visible through a menu-bar or Dock status/control so
  the user can stop it;
- an ordinary explicit app quit stops, flushes, and closes the log by default;
- persistent work after app quit, login, or process termination requires a
  separately accepted `SMAppService` helper/agent design and clear user control;
- App Nap, sleep/wake, Bluetooth loss, and disabled background activity are
  explicit integration scenarios.

The first milestone guarantees continuation with no window while the app process
is running. It does not silently install a login item or daemon.

## Frontend teardown and reconstruction

The native host owns a small foreground/background state machine:

1. On mobile background, begin a 30-second grace period; under memory pressure,
   or on macOS window close, stop accepting new UI-only work immediately.
2. Close the WebSocket and invalidate its per-launch token.
3. Release subscriptions and bounded preview buffers owned by that UI session.
4. Destroy or unload the WebView no later than the grace deadline and clear
   nonessential web caches/resources.
5. Keep native Bluetooth, protocol, log writer, storage monitor, and platform
   background owner alive.
6. Flush a native log checkpoint within the OS-granted transition window.
7. On foreground/window reopen, construct a fresh WebView and token.
8. Run `system.hello`, fetch the authoritative snapshot, query the active log
   since its last committed sequence, then create a new bounded live
   subscription.

No correctness state may exist solely in JavaScript storage. Losing the
frontend is a normal lifecycle event, not a logging failure.

## API required for sessions, live logs, and future charts

Active logging requires:

- `logging.stop`
- `logging.get`
- `logging.getActive`
- `logging.getStorageState`
- `alerts.listRules`
- `alerts.replaceRules`

Automatic logging starts when a device reaches ready. A manual `logging.start`
is reserved for tests/advanced capabilities.

Recorded-session access requires:

- `sessions.list`
- `sessions.get`
- `sessions.getSummary`
- `sessions.querySeries`
- `sessions.listEvents`
- `sessions.getDiagnosticWindow`
- `sessions.export`
- `sessions.delete`

Required events:

- `sessions.created`
- `sessions.updated`
- `logging.jobChanged`
- `logging.samplingModeChanged`
- `logging.storageChanged`
- `logging.samplesCommitted`
- `alerts.triggered`

`sessions.querySeries` is paginated and bounded. It accepts:

- `sessionId`;
- UTC or monotonic elapsed range;
- requested fields;
- maximum points/resolution;
- aggregation/downsampling mode;
- opaque cursor.

The backend performs generic range filtering and bucket aggregation so charts
never load an entire long ride into the WebView. `sessions.listEvents` provides
indexed faults, alerts, state transitions, footpad changes, and gaps.
`sessions.getDiagnosticWindow` returns bounded lossless samples around an event
or timestamp.

Live charts combine committed history with a bounded telemetry preview and
reconcile by sequence number. Graphing, chart design, ride analysis, and
diagnostic explanation UX are later milestones, but these contracts prevent
those features from requiring a storage rewrite.

The UI displays whether values are observed but not yet committed, committed,
dropped, or separated by a declared gap. It must not equate a WebSocket sequence
with durable database commitment.

## Failure and recovery semantics

Required failure cases:

- process termination during a transaction;
- missing or corrupt WAL sidecar;
- corrupt final database page or terminal record;
- disk-full before start and during append;
- file protection/data unavailable while locked;
- schema migration failure;
- duplicate/reordered BLE response;
- WebView destroyed during append and during stop;
- Bluetooth/radio loss;
- OS suspension and restoration;
- Android foreground service stopped or killed;
- macOS sleep/wake and explicit quit;
- duplicate `logging.stop`, disconnect, and late callbacks.

Recovery produces a stable state and never overwrites the original corrupt file.
If readable data can be salvaged, expose it as a partial log with an integrity
warning and immutable recovery metadata.

## Verification gates

R3:

- finalize logging API shapes, SQLite logical schema, migration/version rules,
  and mock query/downsampling behavior.

R5:

- implement catalog/per-session stores, transaction batching, crash recovery,
  storage pressure, fake lifecycle, and WebView-loss service tests in Swift and
  Kotlin.

R6:

- stream real device samples directly into the native store on iOS, Android, and
  macOS; compare live/committed sequences and query while active.

R7:

- pass platform background, lock, restoration, foreground-service,
  window-closure, sleep/wake, process-kill, migration, disk-full, and integrity
  scenarios;
- run a minimum 60-second screen-off/background hardware log on every mobile
  platform and a window-closed run on macOS;
- run an extended ride-duration soak chosen before R7 completion.

R8:

- include background/foreground transitions and WebView recreation in the
  twenty-cycle hardware acceptance;
- verify every log's integrity, declared gaps, counts, and cleanup.

Detailed commands and evidence format remain in
[`verification-plan.md`](verification-plan.md).

## Platform documentation references

- Apple:
  [current Core Bluetooth overview and iOS 26 Live Activity behavior](https://developer.apple.com/documentation/corebluetooth),
  [Core Bluetooth background processing](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html),
  [Bluetooth state-restoration relaunch rules](https://developer.apple.com/documentation/technotes/tn3115-bluetooth-state-restoration-app-relaunch-rules),
  [central-manager restoration options](https://developer.apple.com/documentation/corebluetooth/central-manager-state-restoration-options),
  [Application Support directory](https://developer.apple.com/documentation/foundation/url/applicationsupportdirectory),
  [file data protection](https://developer.apple.com/documentation/uikit/encrypting-your-app-s-files), and
  [macOS ongoing background processes](https://developer.apple.com/documentation/appkit/managing-ongoing-background-processes-in-your-mac).
- Android:
  [BLE background communication](https://developer.android.com/develop/connectivity/bluetooth/ble/background),
  [connected-device foreground service](https://developer.android.com/develop/background-work/services/fgs/service-types#connected-device),
  [SQLite storage](https://developer.android.com/training/data-storage/sqlite),
  [app-specific storage](https://developer.android.com/training/data-storage/app-specific).
