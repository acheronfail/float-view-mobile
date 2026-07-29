## Context

The native application service already decodes Refloat telemetry, applies phase-dependent persistence settings, appends selected samples to the durable session, and publishes internal sample events. The loopback servers currently ignore those sample events when constructing frontend state, while `/live` renders only connection and firmware identity. `AppState` snapshots are complete and monotonic by contract, so a separate lossy telemetry channel would create a second frontend state model and conflict with the repository's snapshot-only boundary.

Saved devices currently persist the literal `Board` as `display_name`, do not retain the advertised name on the session-opening path, and expose no rename operation. The routed Board editor combines route behavior, profile loading, validation, and form presentation, so it cannot be rendered unchanged in a live modal. Settings already use revision-checked native JSON persistence and are the natural owner for a global unit preference.

The change crosses the TypeScript contract, frontend owners, both native application services and serializers, disposable SQLite schemas, the deterministic mock, and canonical conformance. No external dependency is needed.

## Goals / Non-Goals

**Goals:**

- Derive live presentation from the same successfully persisted samples as durable session review.
- Deliver complete newest state at the configured idle or riding persistence cadence without an unbounded queue.
- Present every currently supported telemetry field in an intentional responsive layout, together with active-session aggregates.
- Keep the live route focused while making disconnect and board configuration available from one header menu.
- Reuse one auto-saving board editor on the routed and modal surfaces.
- Resolve and edit board identity consistently, with a VESC Tool-compatible platform identifier as the final fallback.
- Persist a global metric or imperial presentation preference while retaining canonical native values.

**Non-Goals:**

- A separate live telemetry request, event envelope, polling loop, or storage path.
- Catch-all UI for future telemetry fields; contract additions must receive an intentional position.
- Board Profile completeness warnings or blocking behavior.
- Leaving the focused live route for other application destinations while connected.
- Swipe gestures, dirty-state prompts, saved indicators, or optimistic native-state replacement.
- Database migrations or compatibility negotiation for test-stage data.

## Decisions

### Deliver the latest successfully persisted sample in `AppState`

The application service will retain a bounded `LiveSample` only after the persistence gate selects the sample and the append succeeds. The sample contains sequence, elapsed time, ride state, board state, footpad state, pack voltage, duty cycle, controller and motor temperature, motor and battery current, eRPM, and speed. `boardState`, which is decoded today but discarded before persistence, will become an explicit sample field and session column so the live and durable representations remain the same.

The active connection snapshot will contain its saved-device identity, current resolved display name, active-session summary, and latest sample. It will retain the last sample across transient reconnect states and clear it when the active connection is removed or replaced. A missing first sample is represented by `null`.

This was chosen over `session.get` polling because repeated bounded-series database queries add latency and duplicate lifecycle logic, and over a new `telemetry.changed` event because the frontend is intentionally restricted to complete monotonic snapshots.

### Publish at persistence cadence and conflate pending snapshots

Every successful persisted sample increments the state revision and invalidates the frontend snapshot. Idle, riding, and post-ride behavior therefore follows the already configured persistence gate. A capacity-one/conflated sender must retain the newest pending snapshot, not the oldest, and slow or suspended frontends may observe sequence gaps without affecting durable samples.

Snapshot construction remains bounded. Native implementations should avoid introducing repeated unbounded work when snapshots reach the configured maximum of twenty per second; coarse collections may be cached or queried through existing bounded indexes, while the active summary and sample are updated from the completed append. Sending every decoded probe was rejected because it would diverge from the durable record and ignore the user's persistence setting.

### Keep telemetry fields explicit

`LiveSample` uses named nullable fields rather than `Record<string, number>`. The dashboard assigns each known reading to Ride, Power, Temperatures, Board state, or Active session. Adding telemetry later requires a coordinated contract, both native implementations, persistence/conformance, and deliberate UI placement. This intentionally rejects an “Additional telemetry” bucket.

### Make native state authoritative during configuration auto-save

The boards owner will expose one shared `BoardConfigurationEditor` used by the routed Board view and the live modal sheet. Route navigation and modal mechanics remain in their respective wrappers. A colocated editor controller validates individual fields, serializes revision-checked mutations, and always starts the next mutation from the backend-returned revision. Text commits on blur; selects and numeric controls commit on their ordinary change event. Closing the sheet flushes an edit caused by the closing blur and awaits queued writes before dismissal, without a dirty or saved indicator.

Valid fields persist independently so an incomplete profile can be filled progressively. Invalid transient input is not sent and may be discarded on dismissal. Serial writes were chosen over parallel optimistic patches because Board Profile revisions would otherwise conflict and a closed sheet could appear to have saved data that the native owner rejected.

The boards feature owns this domain component and its controller as an explicit cross-feature surface; the connection feature renders it but does not import board-private implementation modules. No generic form or overlay abstraction is introduced.

### Separate custom, advertised, and platform identity

Saved-device state will expose a revision, nullable custom display name, latest advertised name, platform Bluetooth identifier, and resolved display name. Resolution is:

1. non-empty trimmed custom name;
2. non-empty advertised Bluetooth name;
3. platform Bluetooth identifier.

The platform identifier follows VESC Tool: CoreBluetooth UUID on Apple and Bluetooth address on Android. It is visible as selectable, read-only metadata in both board-configuration surfaces, but is not added to ordinary logs or exports. The session-opening path will persist or refresh the advertised name independently without overwriting a custom name.

A revision-checked `savedDevice.update` operation will set or clear the custom-name override. Reusing Board Profile JSON was rejected because identity and electrical/mechanical configuration have different ownership and revision lifecycles. The disposable catalog schema will replace the misleading mandatory `display_name` default with explicit custom-name and revision state rather than retain compatibility machinery.

### Keep measurement storage canonical and convert only presentation

Settings gain `unitPreference: "metric" | "imperial"` with `metric` as the default. Native telemetry, SQL, series values, exports, and wire field names remain in volts, amps, Celsius, metres, kilometres per hour, fractional duty, and eRPM. Shared pure frontend formatting converts speed and distance to mph/miles and temperature to Fahrenheit for imperial presentation; voltage, current, duty percentage, and eRPM do not change. The preference applies to live and session surfaces.

This avoids rewriting historical data, platform-specific conversion, and unit-dependent export semantics.

### Use standards-based focused overlays and conditional disconnect

The live header menu contains Configure board and Stop and disconnect. Configure board opens a modal bottom sheet covering approximately ninety percent of the viewport, respecting safe areas and responsive width. It provides modal focus containment, a close control, Escape handling, focus restoration, and backdrop dismissal. Standards-based browser primitives and CSS are sufficient; no gesture or popover dependency is added.

Disconnect is immediate only when the latest sample's ride state is exactly `idle`. Non-idle, unknown, or missing telemetry requires confirmation because disconnect closes the active durable session. During any active connection state other than `ready`, the retained dashboard values are dimmed and the visible connection label reflects the actual state. Returning to `ready` restores normal presentation. Unsupported firmware shows a warning without hiding telemetry; Board Profile completeness is not surfaced.

## Risks / Trade-offs

- [Complete snapshots at up to twenty updates per second increase serialization and database work] → Keep every collection and message bounded, reuse the completed append's active state, conflate pending delivery, and cover maximum-cadence behavior in native and conformance tests.
- [A slow frontend skips visual samples] → Treat the dashboard as a newest-state display; persistence remains authoritative and sequence gaps are expected.
- [Backdrop dismissal races a text-field blur] → Route all mutations through one flushable serial queue and await it before closing.
- [Apple and Android identifiers have different formats] → Expose one documented platform-identifier semantic and render it opaquely without frontend platform branching.
- [Immediate field saves can surface revision conflicts] → Serialize editor writes, apply backend-returned revisions, and preserve structured conflict handling.
- [Changing the disposable schema removes existing test data] → Bump the current storage version and recreate app-owned test-stage storage; add no migration.

## Migration Plan

1. Change the shared current SQL schemas and storage version; test-stage native stores recreate incompatible app-owned data.
2. Update the TypeScript contract, deterministic mock, Apple and Android records/serializers, and canonical conformance together so no mixed contract is supported.
3. Replace the route-specific form with the shared editor, then add the live dashboard, menu, and sheet.
4. Run focused frontend/native/conformance checks followed by `just verify`.

Rollback is a source rollback plus recreation of disposable app-owned storage; there is no data migration to reverse.

## Open Questions

None.
