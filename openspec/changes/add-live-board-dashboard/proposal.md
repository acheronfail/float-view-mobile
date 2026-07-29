## Why

The focused live-board route currently exposes only connection identity and cannot show the telemetry already flowing through the native durable-session pipeline. Riders need a responsive live dashboard, immediate access to board configuration and disconnect controls, consistent board identity, and a global unit preference without leaving the active connection.

## What Changes

- Add a responsive, fixed-position live telemetry dashboard with instantaneous readings, active-session aggregates, empty placeholders, unsupported-firmware warnings, and dimmed retained values during reconnect.
- Publish the latest successfully persisted sample in complete monotonic `state.changed` snapshots at the configured persistence cadence, coalescing pending snapshots to the newest state.
- Replace the live view's standalone disconnect button with a header menu containing conditional disconnect and board-configuration actions.
- Render the same auto-saving board configuration editor from both the Boards route and a near-full-height modal bottom sheet in the live view.
- Resolve board names as custom display name, advertised Bluetooth name, then the platform Bluetooth identifier used by VESC Tool; show that identifier as read-only board metadata.
- Add a persisted global `metric` or `imperial` unit preference, defaulting to metric, and apply presentation conversions consistently to live and session views while retaining canonical stored and wire values.
- **BREAKING** Extend the mandatory frontend/native contract, settings payload, saved-device operations, active-connection state, mock, and conformance expectations together; no compatibility negotiation is added.
- Non-goals: Board Profile completeness warnings, navigation away from the focused live screen while connected, catch-all rendering for unknown future telemetry fields, swipe-to-dismiss gestures, and changes to telemetry acquisition or durable storage cadence.

## Capabilities

### New Capabilities

- `live-board-dashboard`: Focused live telemetry presentation, active-session statistics, connection-state treatment, action menu, configuration sheet, and conditional disconnect confirmation.

### Modified Capabilities

- `backend-contract`: Expose the latest persisted live sample, active saved-device identity, VESC Tool-compatible platform Bluetooth identifier, board renaming, complete settings shape, and cadence-aligned state events.
- `app-navigation`: Permit board configuration in a modal overlay while retaining the focused live route and shared routed configuration behavior.
- `app-appearance`: Persist a global metric or imperial presentation preference and apply it consistently without changing canonical backend values.

## Impact

The TypeScript contract, frontend backend state and deterministic mock, live/board/settings/session feature UI, Apple and Android application services and loopback serializers, current SQLite catalog and session schemas, canonical conformance suite, and focused tests will change together. The existing `display_name` and opaque platform identifier storage will be clarified into a nullable custom-name override plus independently retained advertised and platform identifiers; test-stage databases may be recreated under the current disposable-schema policy. The current route-specific board form will be replaced by one shared board-owned editor used by both surfaces. No new dependency, generated API layer, migration machinery, platform-specific frontend branch, or additional telemetry pipeline is introduced.
