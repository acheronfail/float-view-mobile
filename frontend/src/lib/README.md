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
field-weakening currents remain distinct ampere readings. Do not calculate or
label any current or torque utilization.

Session review loads metadata through the WebSocket control plane, then requests
only the selected stable or extension fields through the authenticated bounded
session-data endpoint. Overview series, exact retained samples, and chronological
event pages remain separate revisioned queries; `session.get` never embeds
history. Frequently updating controls use primary pointer release with
cancellation and a guarded keyboard or assistive click path.

The connected-board menu consumes only authoritative `RefloatControls`. It does
not infer package versions, protocol commands, or confirmed values. The four
state-labelled boolean actions and conditional fatal clear remain disabled until
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
platform transport identity and serialized configuration bytes never cross the
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
