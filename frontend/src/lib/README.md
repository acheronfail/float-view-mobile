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

The connected-board menu consumes only authoritative `RefloatControls`. It does
not infer package versions, protocol commands, or confirmed values. Lighting uses stable labels and switches that retain confirmed state, show pending
feedback, and keep the menu open. Unknown lighting state is displayed explicitly
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
loopback boundary. Drafts are complete configurations tied to package version,
full-schema fingerprint, wire signature, value revision, and connection
generation. Reconnect or any identity change invalidates them.

Native code remains authoritative for schema audit, safety, serialization,
write acknowledgement, and mandatory readback. Flywheel entry immediately
invalidates current values and hides current/edit/write/apply actions; confirmed
normal-mode return requires a fresh current read. Runtime LED/headlight controls
remain live-only and distinct from similarly named persistent startup defaults
in the editor. Tunes are application-wide complete configurations filtered by
exact identity, not upstream QML curated or migrating presets.

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

The five configurable live number widgets (four above the gauge and one inside it) use the closed `liveWidgetFields` catalogue
in the wire contract. Each opens a full-page, scrollable picker with live previews;
missing fields remain selectable and show an unavailable value. Choices are an
ordered five-entry `Settings.liveWidgetSlots` tuple, shared across boards and
persisted by the Rust product store for both native hosts. Repeated fields are
allowed. Only accepted settings replace a slot; save failures leave the picker
open. Keyboard dismissal restores focus to the originating button; pointer
dismissal clears its focus so it does not retain an outline. Numeric presentation
lives in `features/connection/live-values.ts`, respects display units and source
nullability, and does not interpret firmware. Signed battery power is pack
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

Session map filtering is transient review navigation owned by the session view.
The map toggle selects chronological intervals from the complete continuous GPS
track; charts concatenate included duration with dashed separators and retain
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
