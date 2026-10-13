# Frontend library structure

Organize code by ownership first. Keep related components, reactive controllers,
models, pure calculations, and tests together instead of separating them into
catch-all technical directories.

## Owners

- `app/` owns the persistent application shell, primary navigation, and effective theme.
- `backend/` owns bootstrap, transport, authoritative frontend state, and `contract.ts`.
- `features/connection/` owns Home discovery/connection and the live-board view.
- `features/sessions/` owns session lists, metadata-first progressive review, exact samples, event pages, grouping, and export.
- `features/boards/` owns saved-board lists and Board Profile configuration.
- `features/settings/` owns application-wide settings UI.
- `ui/` is reserved for domain-independent presentation reused by multiple features.

Route files are thin adapters: they read route parameters, coordinate navigation,
and render their owning feature. Product behavior belongs to the feature.

## Files and imports

- Keep feature directories flat by default.
- Give a complex component a PascalCase directory only when it has meaningful private children or modules.
- Keep component-private props and types in `Thing.svelte`.
- Use `thing.svelte.ts` for rune-based reactive state and controllers.
- Use specific names such as `chart.ts` or `formatSession.ts` for pure logic and models.
- Put `*.spec.ts` beside the file it tests.
- Prefer relative imports within one owner and `$lib/...` imports across ownership boundaries.
- A feature must not import another feature's private files.
- Do not add barrel files by default; add an entry point only when it enforces a useful public boundary.
- Avoid generic `components/`, `controllers/`, `effects/`, `stores/`, `types/`, `utils/`, and `helpers/` directories.
- Promote a component to `ui/` only after domain-independent reuse is demonstrated.

## Telemetry presentation

The shared frontend consumes the stable normalized contract without Apple or
Android branches. Live telemetry groups motion, raw electrical load,
temperatures, rider input, control contributions, accumulated values, charging,
source age, and semantic board states. Missing source values remain
unavailable, and actual, requested, directional, filtered, booster, and
field-weakening currents remain distinct ampere readings. Keep individual current readings in amperes; the drive-load gauge and configurable load box share the existing load calculation.

Session review loads metadata through the WebSocket control plane, then requests
only the selected stable or extension fields through the authenticated bounded
session-data endpoint. Overview series, exact retained samples, and chronological
event pages remain separate revisioned queries; `session.get` never embeds
history. Frequently updating controls use primary pointer release with
cancellation and a guarded keyboard or assistive click path.

The connected-board action overlay uses the same full-page modal pattern as the
live box picker, with scrollable two-column buttons grouped into View, Refloat,
Board and Connection. Each action has an icon; incomplete board setup retains
the warning triangle. Touch activation finishes the synthesized mouse events
before handing focus to the next dialog or route. Done and Escape dismiss it; keyboard dismissal restores
focus to the header button. Actions close the overlay before navigation or
confirmation, and active flywheel mode closes and disables it. Refloat
configuration, Tunes, BMS details, board/battery configuration and controller
selection share a full-screen overlay with the same safe-area layout, fixed header
and scrollable content. The header close action and Escape follow the existing
save/discard flow; these panels have no swipe or backdrop dismissal. Other header menus retain their
compact presentation.

The connected-board action overlay consumes only authoritative `RefloatControls`. It does
not infer package versions, protocol commands, or confirmed values. Lighting lives in configurable boxes and the Refloat menu group with stable labels and switches that retain confirmed state and show pending
feedback. Menu lighting toggles keep the overlay open. Unknown lighting state is displayed explicitly
without an off switch. Package and flywheel controls use action labels, with an
ellipsis when confirmation follows. These controls and conditional fatal clear remain disabled until
the native owner reports ready and writable state, and no frontend state is
optimistically changed or persisted. Package disable and flywheel enable use the
shared confirmation UI. When authoritative state detects active flywheel mode,
a non-dismissible application overlay replaces the menu escape path and exposes
only Disable; it remains until telemetry confirms normal mode. Pointer Events
and the guarded click path must not double-submit.

Controller shutdown follows the same authoritative pattern. The live menu uses
only `controllerActions.shutdownAvailable` and `pendingAction`; it does not infer
support from a hardware name. Its destructive confirmation warns that some
controllers safely ignore software shutdown. The result means only that the
one-way request was accepted for transmission, not that controller power-off
was observed.

## Refloat configuration and Tunes

`features/connection/` owns the generated editor and its connection-scoped
controller. The frontend receives only normalized, paged schema fields and
typed values through `backend/contract.ts`; raw XML, VESC commands, routes,
private firmware identity, and serialized configuration bytes never cross the
loopback boundary.

Editor drafts persist in the shared product store per physical controller, with
exact package/schema/wire identity, complete form values, unfinished
numeric input and section/search/control/scroll position. Saves use a bounded
single in-flight request and one replacement, never a firmware write. Editor opens
and WebView restoration reuse the service's valid connection-scoped values and
revision, preserving the complete form without repeated Bluetooth reads. Missing
or invalidated values use the existing read path. Read current/defaults always
fetch the board and replace the form; board changes never introduce conflict prompts.
Foreground reconnect/flywheel invalidates write eligibility without losing edits.
Background link loss and any terminal disconnect clear that controller's saved
editor state and connection-specific presentation checkpoint. A new process has
no live connection and clears old editor sessions; listener/WebView restarts keep
the existing owner and its drafts. Incompatible drafts remain inspectable until
Read current or Read defaults successfully replaces them. Done keeps the draft; there is no separate
discard action.

Native code remains authoritative for schema audit, safety, serialization,
write acknowledgement, and mandatory readback. Flywheel entry immediately
invalidates current values and hides current/edit/write/apply actions; confirmed
normal-mode return requires a fresh current read. Runtime LED/headlight controls
remain live-only and distinct from similarly named persistent startup defaults
in the editor. Tunes are independent of editor drafts: saving a preset performs a
fresh current-board read, renaming changes only preset metadata, and applying a
preset merges its portable Refloat Tune fields into the board's current complete
configuration and writes it with mandatory readback. Tune fields follow Refloat's
QML exclusions for hardware, haptics, safety thresholds and package state. Presets
are application-wide and filtered by exact package/schema identity, not tied to a
saved board.

The live battery percentage is the shared service's temporal estimate. Optional
full-pack capacity (Ah) enables signed charge tracking; otherwise it follows
bounded, smoothed voltage observations. Saved cell profile, series/parallel counts
and display-empty voltage (default 3.0 V per cell) define the observation/reserve. These settings do not change firmware protection
thresholds. Unconfigured boards, guests and BMS-only connections have no estimated
percentage. For saved boards with missing estimation configuration, both live
views show a valid controller-reported percentage with a battery setup warning.
The warning opens the battery-only mode of the board editor; it shares cell
profile, series/parallel counts and display-empty voltage controls with the full
editor. Other unavailable reasons do not use the fallback. Both live views round
the same selected fraction; controller battery level remains a separately labelled raw detail and unchanged in recordings
and exports. WebView recreation never resets or advances shared charge tracking.
The estimator integrates on the original curve axis before applying the zero-load
display reserve. Initial estimates and uncertain intervals are provisional until
stable low-load observations converge; details identify the method and provisional
status. Missing capacity or a dedicated curve never adds a setup warning.
The service pins Floaty's community curves and exposes their catalog version and
compensation mode; temperature, age and remaining range are not modeled.

Battery setup uses one Basic/Advanced form in both board editors. The service supplies
the named preset catalog; the browser only renders it and sends serialized selection
commands. Basic shows manufacturer-grouped variants and actual draft capacity;
Advanced exposes counts, the actual curve, optional full-pack Ah and empty voltage.
Mode changes preserve drafts without saving. Explicit selection/reset replaces
battery defaults atomically; capacity/empty overrides retain the selected name,
while departing counts/curve become Custom. Accepted snapshots alone change live
estimates. Pending/failed saves remain visible, scoped to the opened board.

The five configurable live boxes (four above the gauge and one inside it) use the closed `liveWidgetFields` catalogue
in the wire contract. A 600 ms primary-pointer hold opens a full-page, scrollable picker with live previews;
missing fields remain selectable and show an unavailable value. Choices are an
ordered five-entry `BoardProfileValues.liveWidgetSlots` tuple, persisted separately
for each saved board by the Rust product store for both native hosts. The complete
active snapshot projects the effective layout; unconfigured boards use defaults.
`Settings.liveWidgetSlots` is the separate guest layout. Repeated fields are
allowed. Defaults are Empty (Hold to set), Wattage, Ride time, Odometer,
and centre Trip distance. The timer displays the native-owned `LiveSample.ridingElapsedMs`
through the `elapsedMs` widget slot. Only consecutive riding observations advance it; idle,
unknown and disconnected intervals pause it. Reconnect preserves the accumulated time,
a new session resets it, and WebView recreation cannot affect it. The sample `elapsedMs`
remains the full session timeline for charts and events. Empty, Headlights and LEDs are selectable alongside
readings. Short taps on lighting request runtime changes through the existing
native control path; a hold only configures. Movement over 10 px, cancellation,
lost capture, hidden pages and disposal cancel the gesture. Keyboard activation
configures numeric/empty boxes; lighting uses Enter/Space to toggle and
Shift+Enter or the context-menu key to configure, with an alternate configure
button exposed to assistive technology. Unknown/unavailable lighting stays
configurable without showing a false off switch. Only accepted snapshots replace a slot; save failures leave the picker
open. Keyboard dismissal restores focus to the originating button; pointer
dismissal clears its focus so it does not retain an outline. Numeric presentation
lives in `features/connection/live-values.ts`, respects display units and source
nullability, and does not interpret firmware. BMS boxes show unavailable values without a fresh
attached BMS (five-second expiry). Highest temperature compares controller, motor,
the reported cell maximum, BMS circuitry and humidity-sensor temperatures, names the winning sensor,
and falls back to available sensors when the BMS is missing or stale. The temperature widget shows VESC, Motor, BMS and Cells bars when a BMS is present;
Cells uses the reported cell maximum, and BMS uses the shared service’s circuitry
maximum. The rolling temperature chart adds the same BMS and Cells readings while the BMS is present. The Cells bar reaches red at 45°C; other temperature bars reach red at
60°C. Four bars retain their 12 px height and grow the widget vertically.
Fresh BMS circuitry and highest-cell temperatures are also recorded with board session samples and appear in the
session temperature chart when those fields are present in the archive.
Signed battery power is pack
voltage times battery current; its displayed sign controls draw/regen labels and
theme-aware colours. Interpolation respects reduced motion.

The box inside the gauge defaults to trip distance. Drive load is available in every box’s picker. Load reuses the primary gauge calculation, source label and colour with the authoritative controller current limits.

Session map overlays use the typed `Settings.sessionDisplayOptions` object, persisted
by the shared product store for Apple and Android. Add future session display
preferences there and in its closed validation/projection, rather than a separate
WebView store. Both footpad overlays default off; the session menu reuses the
lighting switches and saves confirmed settings without closing the menu. Exact
retained speed samples and footpad transitions identify episode starts above
2 km/h (absolute speed); unknown states never imply a fault. Markers show each
pad's recorded state and interpolate only within continuous GPS segments in the
visible time range. The optional read is cancelled on unmount/disable and bounded
to 100,000 events, 1,000,000 samples and 2,000 fault episodes, with a visible error
instead of silently truncated fault results. Pruned sessions reflect retained
samples only.

Session… in the Live menu and Current ride in Sessions open the same session review route
for saved connections. Visible active review refreshes persisted metadata every
three seconds with one request in flight. Chart inspection and map navigation
pause following; Jump to live clears selection and filtering and resumes the full
ride view. Chart field visibility and the map instance survive snapshot updates.
Readers are replaced per snapshot and late metadata responses are scoped to the
open session and visible page. Recording remains native-owned; completion gets a
final refresh even while inspection is paused, and refresh failures retain the
last loaded data with visible feedback. Guest connections have no current history.

Session map filtering is transient review navigation owned by the session view.
The map toggle selects chronological intervals from the complete continuous GPS
track, holding the last known location during GPS gaps through the session end
for chart selection only. Time before the first fix remains excluded; map route
geometry remains gap-aware. Charts concatenate included duration with dashed separators and retain
original ride timestamps for inspection. Chart zoom/pan is disabled while the
map controls the range. Turning filtering off or hiding the map restores the
previous ordinary chart viewport. Session totals and event pages are not spatially
filtered. No persisted preference or native contract is involved.

Track review exhausts pagination within 100,000 points, 100,000 gaps, 16 MiB and
128 pages; incomplete tracks cannot drive filtering. Spatial review supports up
to 128 intervals, two concurrent series requests, 1,024 buckets per field group
and 16 MiB of decoded chart data. Limit/failure states are explicit; no truncated
prefix is presented as a complete result. Superseding map selections cancel
previous requests, and chart updates never automatically fit the filtered map.

Live charts… in the connected-board menu opens a full-screen, live-only telemetry
page for saved and guest connections. Six full-width charts follow a rolling
30-second window without inspection, zoom or pan. The disposable frontend buffer
is bounded to 1,500 samples and resets for device or sequence changes; missing
values and source gaps break the lines. Stale readings show unavailable values
while existing points scroll out. Display units match the live widgets. Session
remains the history review path.

## UI restoration

The application shell saves one explicit version-1 UI checkpoint through
`uiState.get`/`uiState.set`. The shared process retains it only in memory;
WebView/server disposal preserves it, process termination clears it. Nothing is
written to browser storage, SQLite or archives, and no UI work runs in the
background. The typed contract bounds the encoded record to 4,096 UTF-8 bytes
and each target to 512 bytes.

The record contains the current route and session/board return context, Live
normal/detailed mode and at most one allowlisted actions or feature surface.
Live and board-history owners report their presentation explicitly. One save
can be in flight with one latest queued replacement; identical records are
ignored. A change interrupted before acknowledgement can be lost.

An explicit incoming route different from the saved destination takes precedence
over restoration. Native foreground recreation loads the root URL.

Startup reads current native state and the checkpoint before mounting the
route. It validates targets, loads normal data and restores eligible surfaces;
missing targets fall back to their parent while operational failures offer
retry. Ordinary socket reconnection does not repeat restoration. Debug outcomes
contain only route kinds and fixed reasons, never target IDs or payloads.

Editors reopen from current values. Drafts, confirmations, operation flags,
chart/map inspection, scroll/focus position and live-chart samples are not
restored; presentation restoration never replays a mutation.

The detailed view distinguishes motor controller firmware from the Bluetooth
transport on CAN connections. Its Bluetooth gateway section shows the retained
transport identity's hardware, firmware name, VESC version and stable/test release.
Direct Bluetooth connections show the firmware once. These are live snapshot
metadata, with no extra discovery requests or private identifiers.

## Motion

`ui/motion.ts` and the CSS motion tokens define short, consistent transitions:
120 ms feedback and fades, 180 ms dialogs/disclosures, and 220 ms full-screen
panels. Motion respects the system reduced-motion preference and is disabled
while the initial route and presentation are restored. Route fades animate the
existing wrapper without remounting charts, maps, or feature state.

Closing Svelte surfaces become inert and hidden from assistive technology before
their visual fade; rapid reopening restores their semantics. Native dialogs close
immediately, including action-to-panel handoffs, and animate only on opening.
Animation never postpones a command or changes authoritative state. Discovery
rows animate only on insertion; telemetry and session refreshes do not replay
surface transitions. Explicit disclosures may animate height; controls, maps,
charts and refreshed lists otherwise retain their position. Unavailable readings
and first/recovered telemetry values change immediately; ordinary continuous
readings use short interpolation.
