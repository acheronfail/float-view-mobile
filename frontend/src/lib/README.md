# Frontend library structure

Organize code by ownership first. Keep related components, reactive controllers,
models, pure calculations, and tests together instead of separating them into
catch-all technical directories.

## Custom board dashboards

`features/connection/CustomView.svelte` owns one draft editor for the active saved
board. `backend/contract.ts` defines optional `customDashboard` profile data;
`backend/custom-dashboard.ts` validates the closed shape and metric range domains,
mirrored in the shared Rust product store. Existing authenticated Board Profile
reads and revision-checked patches persist it. Dashboard reads do not replace an
open board editor. Conflicts retain the draft and require an explicit reload;
they never silently retry against a newer revision. Layouts are not browser data
or telemetry snapshot collections. Live's five slots remain independent.

`custom-dashboard.ts` derives initial widgets from nonempty Live slots and owns
identity-based ordering. `CustomWidget.svelte` composes the shared reading,
lighting, gauge, battery, temperature, footpad and plot renderers; it starts no
history collector or profile writes. `WidgetPicker.svelte` offers searchable values, popular Number/1×1 choices,
complete reading categories, and separate visual panels/controls from the closed
`widget-catalog.ts` catalogue. Cards use primary-pointer and guarded keyboard
activation, with inert presentation-only thumbnails. Selection stays outside the
draft until Add widget; Cancel adds nothing.
`WidgetConfiguration.svelte` configures one draft or staged widget. Compatible
Number/Bar/Gauge/Chart choices are visible buttons. Illustrated size buttons
replace Width/Height inputs and retain saved non-preset dimensions as a selected
button. Gauge and chart presets include 3×3, 3×4 and 4×4. The preview uses actual dashboard
cell geometry and the same footprint mapping as grid placement.
Oversized previews scale down uniformly to fit the editor. The modal’s Save
action updates the draft; widget × buttons are the only removal action. One typed switch and the existing reading catalogue define the
supported widgets; there is no registration/plugin framework.

The responsive grid saves order and bounded `{columns, rows}` intent, never pixel
coordinates or breakpoint layouts. Existing named compact/medium/large values
remain valid and unchanged; only explicit dimension changes replace them. Columns adapt to available container width,
spans clamp to fit, and ordered row placement fits widgets beside larger neighbours
without changing saved order or DOM/keyboard traversal order.
From 20rem of available container width, four compact widgets fit across; narrow
containers use two columns and the narrowest use one. Wider containers use eight
columns from 40rem and twelve from 72rem. Rows use 55% of column width, bounded
to 3.5–5rem. Existing compact widgets span 1×1 cells, medium 2×2 and large 4×2. Explicit dimensions independently support 1–4 columns and 1–4 rows; only
width clamps to available columns, so a 1×4 tile stays four rows tall.
Four compact widgets fit in the space beside a medium widget; they never stretch
to its height. One-row cards use 6px padding and bounded numeric text; shared battery and temperature renderers
condense their labels and scales while retaining the selected presentation.
Trends use the remaining cell height, compact numeric renderers tighten line
spacing, and four-temperature compact views use a two-column reading layout.
Narrow and short screens scroll vertically. Widget identity survives reflow and
size never changes the selected number/bar/gauge/trend presentation. Host viewport
and safe areas are respected; unreported physical hinges are not detected.

Editor controls float over the bottom of the dashboard with permanent scroll
clearance in both modes. Add/Cancel/Save appear without shifting widgets or the
scroll viewport, and hide during configuration, picking and discard confirmation.
Instructions are assistive text rather than varying in-flow paragraphs. The
existing motion primitives make outgoing controls inert during their fade.

Reading gauges use a cell-filling dial with the label, actual value and units
inside the arc, plus range endpoints and unavailable/out-of-range indications.
They retain the same reading catalogue and semantic colours as number/bar views.

Custom readings inherit one semantic colour through labels, values, ranges, bar
fills, gauge strokes and single-series traces. Load follows the authoritative
winning source. Shared Live number/chart defaults remain independent.

Hold any widget to create a local draft with Save/Cancel; keep holding to drag
the lifted preview. There is no Edit button. Keyboard/assistive users have a
labelled Arrange action on each widget; empty dashboards offer Add. While editing,
tap any widget to configure;
hold and drag anywhere on touch to reorder, or drag with a mouse. Touch movement
before the hold scrolls the dashboard. While editing, each widget has a circular ×
button that removes it from the draft. Its independent activation cancels on
movement and never opens configuration or starts a drag. Focus moves to the next
widget (or Add when empty). Widget surfaces describe Alt+Left/Right shortcuts for
keyboard ordering; configuration has no Move earlier/later buttons. `dashboard-gesture.ts` owns this
bounded Pointer Events arbitration, cancellation and scroll-container autoscroll.
The frame intercepts pointerdown before ordinary controls register activation; a
short stationary release invokes the original button once, while holding never
activates it. Widget surfaces forward early vertical movement to the scroll owner
in both modes so the same pointer can continue dragging after a hold. An inert
pointer-following preview marks the lift and neighbouring widgets animate into
the proposed order. Cancelling restores the original draft ordering.
Entering edit mode requires a stationary 600ms hold; while already editing,
touch holds arm dragging after 300ms. Pending presses shrink/tint the widget,
then a raised preview gains an accent outline and shadow. Scrolling/cancellation
clear the press state. Editing intercepts all ordinary widget controls. Borders
wiggle with their contents; reduced motion suppresses scaling and animations. Only Save writes
the complete dashboard; leaving a dirty draft asks whether to discard it.

`live-history.svelte.ts` creates one route-scoped `LiveHistoryBuffer`, reused by
the existing Live charts dialog and custom trends through `LiveChartPlot.svelte`.
It retains at most 30 seconds/1,500 existing delivered samples total, deduplicates
sequences, captures per-point BMS/estimate values and clears on incompatible
connection boundaries. No chart requests extra telemetry, session backfill or
recording changes. Leaving the live route or recreating the WebView releases
history/timers. Hiding the frontend stops collection and its clock; foreground
charts start from fresh samples. View changes and resizing preserve the surviving buffer.

`LiveWarnings.svelte` belongs to the live route, above all three view compositions.
`warning-layer.ts` keeps its single presenter attached to the dashboard. Menus,
widget panels and confirmation dialogs conceal and disable it without reparenting
or changing the dashboard clearance. Dismissal restores the dashboard warnings;
ordinary editing still shows them. The authoritative flywheel overlay retains exclusive interaction.
The UI checkpoint restores Custom in viewing mode only; drafts, widget dialogs,
drag state and chart samples remain transient.

## Owners

- `app/` owns the persistent application shell, primary navigation, and effective theme.
- `backend/` owns bootstrap, transport, authoritative frontend state, and `contract.ts`.
- `features/connection/` owns Home discovery/connection and the live-board view.
- `features/faults/` owns shared fault labels, bounded session-event loading and the fault list.
- `features/sessions/` owns session lists, metadata-first progressive review, exact samples, event pages, grouping, and export.
- `features/boards/` owns saved-board lists and Board Profile configuration.
- `features/settings/` owns application-wide settings UI.
- `ui/` is reserved for domain-independent presentation reused by multiple features.

Route files are thin adapters: they read route parameters, coordinate navigation,
and render their owning feature. Product behavior belongs to the feature.

## Files and imports

Session metadata includes bounded `recordingBreaks` tuples of start, end and
reason (`pause`, `connectionLoss`, `unknown`) derived from explicit recording
boundaries. Missing historical reasons stay unknown without inference.
`features/sessions/recording-time.ts` compacts only deliberate `pause` intervals.
Chart paths, inspection, markers and gestures use that same
mapping while requests and map synchronization retain original elapsed times.
Connection losses, unknown interruptions and ordinary telemetry delays remain uncompressed. Both native hosts use the shared
Rust reader and loopback projection, including for imported and pruned rides.

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

## Board appearance

`features/boards/BoardAppearanceEditor.svelte` owns the touch parts picker;
`BoardEditor.svelte` opens it from a compact preview in a full-screen overlay
with a fixed Done header, native modal isolation and focus restoration.
`BoardIcon.svelte` is its public drawing, also used on Connect, saved-board and
session cards. `BoardProfileValues.appearance` stores the complete validated
selection through the existing profile revision/save flow. Rapid edits keep one
queued replacement. Saved-device snapshots project the current appearance, so
cards follow later edits rather than a session's historical configuration.
Missing or null appearance uses style 1 for every part, silver rails/fender,
graphite tyre, black rim and white hooks. Colours are bounded to the contract
palette; fender hooks require a split or full fender. Appearance is optional and
does not affect profile completeness. The existing profile JSON and archive
snapshot preserve it without a catalog schema or storage marker change.

## Telemetry presentation

Board actions includes Fault history. The read-only sheet loads persisted faults
for saved rides without changing the selected session, and native-owned recent
history for guests. Session review reuses the list, with timed danger markers on
charts and continuous GPS segments. Times are explicitly approximate; untimed
records stay in the list without markers. Both lists page their visible rows.
Event reads refresh only when the committed event boundary changes, and errors
remain visible with retry. The sheet participates in the existing UI checkpoint.

Live telemetry exposes the fatal error latch separately from an active VESC
firmware fault. A parent-owned danger overlay above all live views names the active or most recently
recovered fault while the latch remains set, including an unknown raw fault code.
Clearing an active fault alone does not remove a latched warning. The retained
last beep reason alone never creates this overlay. Detailed telemetry shows the
latch and last recovered fault too. Apple and Android receive these fields through
the same shared Rust snapshot projection and existing C/JNI host bindings.

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

Session charts hide the redundant Ah counters. Energy shows consumed, recovered
and net Wh relative to the first available session readings. The frontend pages
exact retained counter pairs before reducing into bounded chart buckets, keeping
the same baseline during zoom and map filtering. Missing values break traces;
a decreasing counter makes its later totals unavailable rather than estimating
energy lost across a reset. Reads are lazy and bounded to one million retained
samples and one thousand pages; larger sessions show an actionable chart error.
Raw stored counters and exports remain unchanged.

When ride-location recording is off or unavailable, the live header shows a fixed-size
ride-location button beside Board actions,
with a single-line truncated board name. Location details and Enable/Allow/Settings
recovery open only on request in a compact modal, also available through Ride location…
in Board actions. The header button uses warning text, border and a tinted background
when recording is off or unavailable. Authorized recording hides both the header
button and the Ride location… menu action. The panel keeps
deliberately disabled recording neutral and marks blocked recording as a warning.
Neither status nor the modal changes the live widget layout. Guests have no
ride-location controls.
Guest connections tint the Board actions button as a warning and show the
unsaved-data notice at the top of that overlay, leaving the live widgets free
of the warning banner.

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

Supported Refloat 1.1.0+ boards can apply Tunes while riding in normal mode.
Refloat 1.0.x still requires idle, ready state with both footpads released.
Unknown board state, special modes, read-only connections and competing board
operations keep application unavailable. Tapping a saved preset opens a compact
current-to-saved comparison and requires confirmation before applying, with progress
and verified success feedback. When the board is not idle, confirmation also warns
that applying a tune can cause a momentary cutout. Pointer activation waits for release and cancels on scroll;
keyboard activation follows the same path. Preset details remain available for
renaming and deletion when switching is unavailable.

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
and exports. Session charts use the separately recorded `estimatedBatteryLevel`
fraction, captured from that same service estimate; unavailable estimates stay absent.
Chart min/max indicators summarize visible lines in the displayed range and map filter.
Load, speed and cumulative charts show only their useful maximum.
Session overviews default to Smooth charts, a session-view menu toggle. Enabled
charts show curved, time-weighted trends over roughly eight screen pixels,
with faint bands retaining the reduced series' per-window extrema. Disabling
smoothing restores all retained series readings without bands. Range labels
and inspection use the unsmoothed readings. Zoom reduces the averaging window;
windows of one second or less show retained detail. Nulls, continuity generations
and excluded map intervals break both trends and bands.
WebView recreation never resets or advances shared charge tracking.
The estimator integrates on the original curve axis before applying the zero-load
display reserve. Initial estimates and uncertain intervals are provisional until
stable low-load observations converge; details identify the method and provisional
status. Missing capacity or a dedicated curve never adds a setup warning.
The service pins community cell-model curves and exposes their catalog version and
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
remains the history review path. The Load chart overlays absolute duty, motor load
and battery load on a fixed 0–100% axis with the gauge colours. Motor load uses
filtered, directional, then raw current, and raw current selects the draw/regen
limit. Session recordings capture motorLoad and batteryLoad fractions through
bounded numeric extension storage using the limits at observation time. Missing
inputs remain unavailable; load fractions are saturated at 100%.

## Navigation

Live and standalone BMS use the full screen without primary tabs. Go to Sessions in
the header dots menu returns to Sessions without disconnecting. In Board actions,
it sits in a separate App section after the board controls; Done closes the menu
and returns to the ride view. Groups with three actions use three columns. The primary tabs
there and on Settings keep Board as the first tab while connected, reopening that
connection.
Subordinate history screens use a labelled Back link. Session entry URLs carry the
return context in the hash query: Live uses `from=live`, board history uses `board`,
and Sessions uses neither. Recording updates never choose the return destination.
The process-memory checkpoint retains this context across WebView restoration.

Settings keeps a compact open-source summary and links to `/settings/licences`
for complete offline project/artwork/dependency notices. That read-only page
uses the app header's Settings return link and scrolls independently of the
Settings content. Its process-restoration destination is the parent Settings
route, without adding a backend wire route kind.

`ui/back-dismissal.ts` owns Escape and native Back dismissal of the topmost visible
surface. Surface owners register their existing Done/Cancel handler, so save errors
and busy guards apply to every dismissal path. Android forwards system Back to
`floatview:back`; the shell follows the route's return link only after surfaces have been handled.
Live/BMS return to Sessions. Settings returns to Sessions while connected, or
Connect while disconnected. Back at Sessions while connected returns control to Android. Keyboard dismissal restores focus to the opener.

## UI restoration

The application shell saves one explicit version-1 UI checkpoint through
`uiState.get`/`uiState.set`. The shared process retains it only in memory;
WebView/server disposal preserves it, process termination clears it. Nothing is
written to browser storage, SQLite or archives, and no UI work runs in the
background. The typed contract bounds the encoded record to 4,096 UTF-8 bytes
and each target to 512 bytes.

The record contains the current route and session/board return context, Live
normal/detailed/custom mode and at most one allowlisted actions or feature surface.
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

The live dashboard reserves full-height number and battery rows so a short viewport
cannot squeeze their contents out of view. Short landscape screens place the gauge
beside the remaining widgets; compact dashboards scroll vertically with the header
remaining fixed. Compact gauges retain a 44 px centre-box target and a smaller
marker target, hiding the marker hint when it would overlap the centre box. The
repeatable mobile rendering matrix is documented in `conformance/README.md`.
Within each section, detailed telemetry rows form one, two or three readable
columns as width permits. Each label/value pair wraps within its cell; notes stay
with their section and long firmware identities remain readable in narrow columns.

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

Ride announcement controls belong to `features/settings/Announcements.svelte`.
They automatically submit complete revision-checked announcement preferences on
control changes or numeric-input blur, and authenticated
test requests. Saves are serialized, newer edits survive delayed responses, and
tests await successful saves. Invalid numeric fields use persistent danger-styled
inline hints with accessible descriptions, rather than browser validation popovers;
speech, telemetry decisions and scheduling stay native/shared.
The iOS lowering switch renders from mandatory `announcementAudio.loweringControl`,
without platform sniffing. Live load views only format `LiveSample.load`;
`features/connection/gauge.ts` contains presentation selectors, not utilisation
or winner calculations. Session views retain recorded raw duty and captured
motor/battery fractions.

The announcement Voice settings action renders only when mandatory
`announcementAudio.voiceSettingsAvailable` is true. It sends the authenticated
`announcements.openVoiceSettings` method, never an OS URI or platform guess;
native hosts own chooser availability, launch and return-time voice refresh.

## First-launch safety agreement

The app shell requires an explicit riding-safety agreement before mounting routes
or restoring a screen. Acceptance is saved in native/shared settings before access
is granted and survives WebView and process recreation. Escape, native Back and
the backdrop cannot dismiss the gate. Failed saves keep it locked and allow retry.
The only action is “I agree”; users who do not agree can close the app themselves.
The disclaimer covers riding risk, safe app use and the author's
responsibility for injury or damage, to the extent permitted by law.

The full saved-board editor exposes Connect automatically through the single
installation-local Settings.autoConnectSavedDeviceId. It uses revision-checked
settings updates, not Board Profile values. Foreground discovery and its
15-second automatic admission window belong to the shared service regardless of
route; Home keeps ordinary discovery. Existing connection navigation opens Live
from Home while preserving restored non-live destinations, with no replayed
connection request.

## Support diagnostics

Settings owns the Export diagnostics flow. The shared service owns bounded local
incident collection, immutable sanitized ZIP preparation and native presentation;
error surfaces link to Settings rather than building their own logger. Diagnostic
windows never enter AppState. Global frontend error reporting sends closed codes
only, with a bounded memory queue and recursion protection. Users manually write
GitHub issues and attach saved ZIPs; the app never prefills or submits issues.
See [diagnostic instructions](../../../docs/diagnostics.md).

## Recorded connection history

Session details show the recorded Refloat version, motor-controller VESC firmware
(including test version), and direct/CAN route. CAN entries distinguish the
Bluetooth peer's firmware from the motor's firmware, including Express gateways.
Each successful reconnect has its own elapsed-time entry, even when facts match.
`SessionConnectionContext.svelte` owns one page and a disposable review controller;
metadata refresh cancels obsolete reads without replacing the charts or map.
History sits below charts so its loading/pagination does not move the interactive
chart viewport. Native buttons use shared pointer and accessible activation.

Older recordings show Not recorded. If capture starts on a later resume, the UI
marks earlier context as missing rather than applying later firmware facts to the
whole ride. Native archives preserve this history through pruning and import;
CSV remains telemetry-only. See [session storage](../../../backends/sql/README.md#recorded-connection-context)
for payload and read bounds.

## Saved tune library

The Tunes sheet shows all saved tunes in one list, with compatible tunes first.
Tapping a compatible tune opens an Apply confirmation with a fresh current-board
comparison and current/saved values. Every application requires confirmation;
matching tunes show no differences and disable Apply. The riding warning remains
visible when applicable. A fresh read before confirmation refreshes changed
differences for another review; failed reads and applies keep the modal open for
explicit retry. Disconnect or route replacement invalidates an open preview.
Every card shows its Refloat version. Incompatible cards are muted; tapping one
explains why it cannot be applied. A separate full-height card menu opens rename
and delete actions without applying the tune. Compatible cards also offer
Overwrite: a confirmed fresh board read replaces the existing preset values while
preserving its ID, name, description and exact identity, using its saved revision.
The overwrite confirmation compares a fresh board read with the saved tune,
showing only changed settings with their labels, units and saved/current values.
Long comparisons scroll with keyboard or touch while the title and actions stay
visible. Configuration writes, tune application and overwrite share the compact `RefloatConfigurationDiff.svelte` renderer: one labelled old → new row per setting, with direction shown once above the list. If board values change before confirmation, the differences refresh and
require another review before saving.
Save tune is the only list action.

`refloat.tune.list` requires an identity object for exact matches or explicit
`null` for all versions. Both scopes return at most 50 entries per page and bind
cursors to the filter. The UI loads matching pages first, then library pages as the list scrolls,
retaining at most 1,000 unique rows.
Metadata-only updates preserve the stored identity and values even when the
connected board uses another schema; changed values still require validation.
Apple and Android use the same shared Rust dispatch and store implementation.

The default accent is teal; saved accent choices remain authoritative. In
`src/app.css`, accent swatches and `bg-accent-fill` use the approved icon palette
(Fresh teal and Bright gold in light appearance). Pair solid fills with
`text-on-accent`. Use `text-accent`, `stroke-accent`, `border-accent` and
`outline-accent` for contrast-safe ink on ordinary surfaces; bright fills are
not suitable for small text. Soft selected surfaces use `bg-accent-soft` with
`text-on-accent-soft`. The accent settings browser test checks text at 4.5:1 and
meaningful graphics/focus indicators at 3:1 across all presets and both themes.

Confirmed unsupported Refloat versions use `RefloatCompatibilityDialog.svelte`.
The optional `ConnectionIssue.refloatCompatibility` snapshot record contains only
bounded public reported version text (including the INFO suffix) and exact audited
source releases from the shared compatibility catalog. It does not grant firmware
capabilities. The dialog offers offline supported-version guidance, explicit Check
again and primary Back to boards. Escape/backdrop/native Back disconnect; transient
or unreadable INFO failures retain generic retry guidance. Both native hosts use
the shared projection, without adapter-specific interpretation.

The live route restores `BoardProfileValues.lastLiveView` per saved board (Live by default), with an eligible process checkpoint taking precedence. Explicit selection saves only that field before mounting the destination so its dashboard editor reads the current revision. Preference conflicts retry once from a fresh profile; dashboard draft saves retain their explicit conflict handling. Guests do not persist a selection.

## Refloat configuration write feedback

The shared backend publishes `RefloatConfigurationSummary.writeBlockReason` for
edited configuration writes, using the same mutation admission policy as actual
writes. `writable` remains a capability flag; supported riding Tune application
keeps its separate policy. The frontend maps closed reasons to actionable text
without inferring board authority from telemetry.

The editor keeps readiness guidance in its fixed footer and confirmation, so
scrolling fields cannot hide a blocked write. Expanded section/search controls
scroll within a bounded area on short screens, and status changes do not anchor
the field scroller to a different position. A failed-write message remains in
the footer after dismissal and reopening; only a successful write or explicit
read clears it. Reads deliberately replace the draft with their selected source;
status changes and error dismissal preserve edits and scroll and never retry a
mutation automatically.

Live slots and Custom reading widgets offer Max speed (`maxSpeedKph`), the shared
current-ride maximum absolute observed speed. Reconnect and view recreation retain
it; saved ride resume restores the committed session peak. A new ride clears it.
Both views format the same peak in km/h or mph; offline speeds are not inferred.
