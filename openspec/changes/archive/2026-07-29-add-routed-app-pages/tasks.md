## 1. Complete Settings Contract

- [x] 1.1 Replace `rawPacketCaptureEnabled` with the exact `system | light | dark` `themePreference` type and default in the frontend contract, frontend state tests, and deterministic mock.
- [x] 1.2 Update Apple settings records, accepted keys, enum validation, JSON persistence and loopback payloads, and add storage/service tests for defaults, all accepted themes, invalid themes, removed keys, and revision conflicts.
- [x] 1.3 Update Android settings records, accepted keys, enum validation, JSON persistence and loopback payloads, and add storage/service tests for defaults, all accepted themes, invalid themes, removed keys, and revision conflicts.
- [x] 1.4 Update canonical conformance settings fixtures and assertions to require the new complete shape on every target and reject the removed packet key.

## 2. Establish Frontend Ownership

- [x] 2.1 Add `frontend/src/lib/README.md` with the ownership-first directory, naming, import, colocation, component-capsule, shared-UI, and no-default-barrel conventions, and link it from `AGENTS.md`.
- [x] 2.2 Move bootstrap, client, authoritative state, and their colocated tests from `$lib/api` into `$lib/backend`, update all imports, and delete the superseded `$lib/api` directory.
- [x] 2.3 Create flat `$lib/app` and `$lib/features/{connection,sessions,boards,settings}` owners as implementation needs them, without speculative `ui` abstractions or generic technical subdirectories.

## 3. Add the Routed Application Shell

- [x] 3.1 Configure SvelteKit hash routing while retaining the static adapter, inline bundle, root-only prerender entry, and strict one-file build assertion.
- [x] 3.2 Implement the application shell and four routed primary destinations.
- [x] 3.3 Move backend connect, reconnect, subscription, and close lifetime into the root layout so client navigation does not recreate the backend connection.
- [x] 3.4 Implement thin route adapters for Home, Live, Sessions, session detail, Boards, board configuration, and Settings with bounded opaque parameters.

## 4. Split Connection and Session Features

- [x] 4.1 Move Bluetooth readiness, discovery, scanning, and connection controls into the connection-owned Home view.
- [x] 4.2 Move active connection, firmware, state transitions, and disconnect controls into Live, including accepted-connect navigation, recovered-active navigation, and no-active/disconnect return to Home.
- [x] 4.3 Move recent-session summaries, linked-ride summaries, and group unlinking into the Sessions list route.
- [x] 4.4 Move review series, diagnostics, export, and confirmed deletion into the parameterized session view, clearing stale detail state and returning useful not-found or deleted results to Sessions.

## 5. Split Board and Global Settings Features

- [x] 5.1 Move saved-board summaries and completeness into the Boards list, with navigation from each bounded saved-board identifier.
- [x] 5.2 Move series cells, parallel cells, motor, revision, completeness, validation, and save behavior into the parameterized Board Profile editor, including not-found recovery to Boards.
- [x] 5.3 Move automatic session linking, link window, logging cadence, post-ride tail, and reconnect duration into purpose-grouped global Settings sections and remove them from session browsing.
- [x] 5.4 Add the system/light/dark control to global Settings using revision-checked native settings updates.

## 6. Implement Appearance Resolution

- [x] 6.1 Replace the legacy root theme and hard-coded black background with a bounded early appearance hint that resolves explicit or system preference before ordinary content.
- [x] 6.2 Implement and unit-test the application-owned theme controller for authoritative reconciliation, local hint refresh, media-query changes under `system`, explicit-theme stability, `color-scheme`, and theme-color metadata.
- [x] 6.3 Define semantic light and dark canvas, surface, elevated, text, muted, border, accent, success, warning, and danger tokens and convert every routed view and control away from palette-specific surface/text/border colors.
- [x] 6.4 Exercise all routes and connection/loading/error/empty/detail states under light, dark, and system resolution and correct any unreadable or stale-theme presentation.

## 7. Cleanup and Verification

- [x] 7.1 Delete the superseded monolithic tab/selection/settings code, removed packet-setting remnants, legacy theme attributes, stale imports, and empty directories; verify targeted searches find none.
- [x] 7.2 Run `just test frontend` and `just build`, confirming route/type tests pass and `frontend/build` contains only inline `index.html`.
- [x] 7.3 Run `just test apple`, `just test android`, and `just test conformance` to verify native persistence, contract parity, invalid settings, and mock behavior.
- [x] 7.4 Run `just verify` as the complete software gate.

## 8. Add Route-Aware Bottom Navigation

- [x] 8.1 Replace the current shell navigation and Devices/Sessions segmented control with a bottom tab bar whose four tabs render recognizable local or inline icons above the exact labels `connect`, `sessions`, `boards`, and `settings` and navigate to their mapped top-level routes.
- [x] 8.2 Render the tab bar only on the four exact top-level routes, hide it on Live, session detail, board configuration, and other subordinate routes, and retain focused return navigation from those subordinate views.
- [x] 8.3 Add frontend coverage for tab mapping, selected state, route-aware visibility, segmented-control removal, and backend connection persistence across tab navigation.
- [x] 8.4 Run `just test frontend`, `just build`, and `just verify`.
