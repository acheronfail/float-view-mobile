## Context

The frontend currently renders connection, saved-board configuration, session browsing and detail, global settings, and diagnostics from one `+page.svelte`. Its local tab and selection state substitutes for navigation, while the page itself owns the only backend connection lifecycle. Styling is a hard-coded dark Tailwind palette, and `app.html` sets a legacy theme attribute and black background.

Production has a deliberate constraint: SvelteKit must emit exactly one inline `frontend/build/index.html`, and each native loopback backend serves that document only at `/`. Unknown non-API paths remain `404`. The hand-written settings contract is implemented independently by TypeScript, Swift, and Kotlin and is persisted as JSON inside the current settings row.

This change spans frontend architecture and all contract implementations. It adds no external dependency and preserves native ownership of persistence, Bluetooth, sessions, and lifecycle.

## Goals / Non-Goals

**Goals:**

- Make Home, Sessions, Boards, and Settings persistent, discoverable primary destinations.
- Give live-board, session-detail, and board-configuration state stable client routes.
- Keep one backend client alive across route navigation and recover active native state after WebView recreation.
- Separate application-wide settings from board-specific configuration.
- Persist a system/light/dark preference natively and render a stable effective theme.
- Establish a small ownership-first Svelte structure that keeps related UI, state, logic, and tests together.
- Remove the unused raw-packet setting from every contract implementation.

**Non-Goals:**

- Change native HTTP routing, add static asset serving, SSR, or clean-path fallback behavior.
- Change Bluetooth, telemetry, logging, Board Profile, session, diagnostics, or export semantics.
- Implement packet capture.
- Add a component framework, router, state manager, theme library, generated API, or platform-specific frontend.
- Migrate test-stage databases or preserve old settings JSON across incompatible builds.

## Decisions

### Use SvelteKit's hash router

Configure the installed SvelteKit router with `type: 'hash'`. The route tree is:

```text
#/                         Home / connect
#/live                     Connected-board live view
#/sessions                 Session list
#/sessions/[sessionId]     Session detail
#/boards                   Saved-board list
#/boards/[savedDeviceId]   Board configuration
#/settings                 Global application settings
```

Hash routing keeps the HTTP pathname at `/`, includes the client route manifest in the inline bundle, and works with the existing static adapter and native servers. Pathname routing was rejected because direct navigation or recovery would require native fallback routes and contradict the minimal-serving contract. A custom router was rejected because SvelteKit already supplies the required behavior.

The four primary destinations use a route-aware bottom tab bar:

| Tab label | Route |
| --- | --- |
| `connect` | `#/` |
| `sessions` | `#/sessions` |
| `boards` | `#/boards` |
| `settings` | `#/settings` |

Each tab renders a recognizable local or inline icon above its exact lowercase text label. The text supplies the accessible destination name, and the active top-level destination has a visible selected state. No icon dependency is added.

The tab bar renders only on those four exact top-level routes. It is hidden on `/live`, `/sessions/[sessionId]`, `/boards/[savedDeviceId]`, and any other subordinate route so focused workflows retain their own back navigation. The root shell continues to own backend lifetime regardless of tab-bar visibility. This navigation replaces and removes the Devices/Sessions segmented control.

Route identifiers are treated as bounded opaque values and are passed only to contract methods.

### Let the root layout own the application lifetime

Move `apiStore.connect()` and `apiStore.close()` from the current page into the root layout's application shell. Route navigation then changes only the rendered feature view, not the WebSocket or authoritative snapshot subscription.

Home initiates discovery and connection. Once `device.connect` is accepted, it navigates to Live so that connecting and subsequent states have a stable view. After initial `state.get`, an active connection replaces Home with Live. Live waits for authoritative absence before returning Home after disconnect; opening Live without an active connection returns Home.

Session and board route parameters drive their detail requests. A route waits for the shared backend connection rather than opening another one. `not_found` returns to the owning list with a short navigation-state message; other request failures remain visible without clearing unrelated authoritative state. Detail state is cleared when its owning route is left so a stale response is not shown for a later identifier.

### Organize frontend code by ownership

Add `frontend/src/lib/README.md` and reference it from `AGENTS.md`. Use this ownership map:

```text
$lib/app/                   application shell, navigation, effective theme
$lib/backend/               contract, bootstrap, transport, authoritative state
$lib/features/connection/   Home and Live
$lib/features/sessions/     session list, review, diagnostics, export
$lib/features/boards/       saved-board list and Board Profile editor
$lib/features/settings/     application-wide settings sections
$lib/ui/                    proven domain-independent shared presentation
```

Route files adapt route parameters and navigation to feature views; they do not become a second feature implementation. Feature directories remain flat until a complex component has meaningful private children or modules. Tests remain adjacent, rune-based state uses `*.svelte.ts`, imports are relative inside an owner and `$lib/...` across owners, and barrels or generic technical buckets are not created by default.

Move the current `$lib/api` implementation beside `contract.ts` under `$lib/backend`, rename the shared store to describe authoritative backend state, update imports, and delete `$lib/api`. Keeping separate `api` and `backend` directories was rejected because both are one frontend integration boundary and the repository explicitly prefers removing superseded paths.

### Make the settings contract exact

Replace `rawPacketCaptureEnabled` with:

```ts
type ThemePreference = "system" | "light" | "dark";
```

`Settings` continues to contain its revision and eight existing functional timing/linking fields, plus `themePreference`. Swift and Kotlin add the matching string-backed value, default it to `system`, validate only the three values, and include it in settings dictionaries, JSON payloads, snapshots, revision-checked updates, storage tests, mocks, and conformance. Unknown patch keys—including the removed raw-packet key—remain invalid.

The settings row already stores JSON, so no SQL column or schema migration is added. Existing incompatible test-stage settings may reset to current defaults when decoded by the new binary, consistent with the repository's disposable-schema policy. All frontend and native implementations ship from the same source revision, so no compatibility negotiation is introduced.

### Keep native settings authoritative while stabilizing first paint

An early inline script in `app.html` reads one bounded local presentation hint containing only `system`, `light`, or `dark`. It resolves `system` with `matchMedia('(prefers-color-scheme: dark)')`, sets the effective `data-theme` and `color-scheme` before ordinary content, and otherwise defaults to the current system preference.

After `state.get`, an application-owned theme controller:

1. Treats native `settings.themePreference` as authoritative.
2. Resolves the effective light or dark theme.
3. Updates the document attribute, `color-scheme`, and theme-color metadata.
4. Refreshes the local hint.
5. Subscribes to system preference changes only for effective-theme behavior while `system` is selected.

The local value is a disposable paint hint, not another persistence authority. A stale hint is overwritten as soon as native state arrives. Injecting settings into HTML was rejected because it would break the byte-identical artifact. Waiting for the backend without a hint was rejected because explicit themes would visibly flash on recreation.

### Use semantic color tokens

Replace hard-coded slate surface/text/border utilities with a small semantic palette defined for light and dark themes in `app.css`. Product components consume semantic canvas, surface, elevated surface, text, muted text, border, accent, success, warning, and danger colors. This keeps feature markup independent of palette values and avoids duplicating `dark:` variants across every component.

No general-purpose design system is created. A shared UI component is extracted only when at least two features need the same domain-independent behavior.

### Verify behavior at the owning boundary

Frontend tests cover route-to-feature behavior, persistent backend connection ownership, not-found recovery, theme resolution and media-query changes, stale local-hint reconciliation, settings patches, and the mock. Contract storage tests cover defaults, accepted theme values, invalid values/keys, revisions, and JSON round trips in Swift and Kotlin. Canonical conformance checks the same mandatory settings shape and removal of the packet field on every target.

The existing build assertion remains the production artifact gate. Focused frontend and backend checks run before the complete `just verify` gate.

## Risks / Trade-offs

- **[Hash URLs are less conventional]** → They are explicit and stable inside an embedded WebView, while preserving the stronger one-document and root-only-server constraints.
- **[Route effects can run before the backend is ready]** → Detail views wait on the one shared connection state and cancel or ignore stale identifier loads.
- **[A local theme hint can drift from native settings]** → Accept only three bounded values and always reconcile to the first authoritative snapshot.
- **[Theme conversion can leave isolated hard-coded colors]** → Define the semantic palette first and add light/dark component coverage for every route and state.
- **[The settings contract is temporarily incompatible with older binaries]** → Build frontend and native hosts from one revision, keep the contract mandatory, and reset incompatible test-stage settings rather than add negotiation or migration machinery.
- **[Domain folders can become ceremony in a small app]** → Keep them flat, prohibit speculative abstractions, and create component capsules or shared UI only after complexity or reuse exists.

## Migration Plan

1. Change and test the complete settings contract in TypeScript, the mock, Swift, Kotlin, storage, and conformance.
2. Consolidate frontend backend integration ownership and move connection lifetime into the shared shell.
3. Enable hash routing and add thin route adapters plus domain-owned feature views.
4. Add semantic tokens and the theme controller, then move global settings and board configuration to their owners.
5. Delete the monolithic paths, old `$lib/api` directory, hard-coded legacy theme attributes, and packet-setting remnants.
6. Run focused checks, production build assertions, and `just verify`.

Rollback is a source rollback and rebuild of all hosts. Because stored data is disposable and no migration is introduced, a rollback may reset incompatible settings JSON to that revision's defaults.

## Open Questions

None. The route hierarchy, hash-routing constraint, native settings ownership, packet-setting removal, and frontend ownership convention were resolved during exploration.
