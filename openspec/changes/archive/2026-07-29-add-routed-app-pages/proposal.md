## Why

Float View's device, board, session, and application settings workflows currently share one large page with local tab and selection state, making navigation and ownership difficult to discover. The frontend also hard-codes a dark palette and exposes no persisted appearance preference even though it runs across hosts with different system themes.

## What Changes

- Introduce four primary application destinations—Home, Sessions, Boards, and Settings—with routed live-board, session-detail, and board-configuration views.
- Replace the Devices/Sessions segmented control with a bottom tab bar on the four top-level routes. Each tab SHALL present an icon above its exact text label: `connect`, `sessions`, `boards`, or `settings`; subordinate live, session-detail, and board-configuration routes SHALL hide the tab bar.
- Use SvelteKit hash routing so every route remains inside the one inline `index.html` served by native hosts only at `/`.
- Move the backend connection lifecycle into a persistent application shell while allowing route depth to control whether primary navigation is visible.
- Move saved-board configuration into Boards and move all app-wide options into Settings.
- Add a persisted `system | light | dark` appearance preference that defaults to `system`, follows system-theme changes when selected, and applies semantic light/dark color tokens.
- Encode an ownership-first Svelte library convention in `frontend/src/lib/README.md`, organize product code by domain, colocate tests, and point repository guidance to the convention.
- Consolidate frontend backend integration code under `$lib/backend` and remove the superseded `$lib/api` path.
- **BREAKING**: replace the unused `rawPacketCaptureEnabled` settings field with `themePreference` in the hand-written frontend/backend contract, native settings models and persistence payloads, mock, and conformance coverage.

Non-goals:

- Implementing raw packet capture or changing telemetry, Bluetooth, session recording, export, or board-profile semantics.
- Adding clean-path server fallbacks, a general native static-file server, SSR, or platform-specific frontend branches.
- Creating a broad component library or introducing a new UI, routing, state-management, or theming dependency.
- Adding database migrations while application data remains disposable test-stage data.

## Capabilities

### New Capabilities

- `app-navigation`: Four primary routed destinations and their live-board, session-detail, and board-configuration flows within the single-file frontend.
- `app-appearance`: Persisted system, light, and dark appearance selection with deterministic effective-theme behavior.

### Modified Capabilities

- `shared-frontend-artifact`: Preserve the one inline production artifact while adding client-side routes and a documented ownership-first frontend structure.
- `backend-contract`: Change the complete settings shape across all implementations by adding the appearance preference and removing the unused raw-packet flag.

## Impact

The frontend route tree, root layout, current monolithic page, styles, app template, backend client/store organization, mock, and tests will change. The hand-written TypeScript contract and matching Swift and Kotlin settings records, validators, payload serializers, storage tests, and canonical conformance suite will change together. Settings remain native-owned and JSON-persisted; no SQL schema column, migration, native route, or external dependency is added.
