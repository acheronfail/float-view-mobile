# Proposal

## Why

Riders can miss the board's voltage buzzer in wind, while the app currently offers no battery charge announcement. Give riders advance notice near their board's configured voltage boundaries without assuming battery chemistry or adding multiple warning levels.

## What Changes

- Add one default-on Battery announcements switch beside the existing Ride announcements switches. It controls exactly two spoken phrases: “Battery low” and “Over voltage”.
- Resolve current Refloat low/high voltage thresholds using supported firmware semantics and the controller's series count. For resolved pack limits L and H, warn below L + 10% × (H − L) and above H − 5% × (H − L).
- Require fresh Refloat riding/running state for both alerts, with no speed check, and regen for advance high-voltage alerts. Add confirmation, proportional recovery hysteresis and fixed 60-second per-alert cooldowns independent of the existing configurable announcement interval.
- Reuse shared-service speech admission, native offline voices, volume and audio handling on Apple and Android; keep decisions independent of the WebView.
- Show independently qualified low/high warnings within the existing live battery widget using an amber label/voltage and slow full-track outline; high voltage also uses diagonal hatching. Use a 25% voltage-window low margin and immediate high entry above the configured profile’s full-charge voltage, including while stationary without regen. Preserve capped charge fill and support reduced motion independently of speech settings.
- Preserve supported settings and recordings. Missing battery-charge preference defaults to on; explicit off persists.

Non-goals: new banners, extra alert levels or phrases, cell chemistry defaults, user-adjustable margins, predicted seconds to a limit, BMS-cell-derived warnings, board configuration writes, and replacement of firmware protection or haptics.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `live-board-dashboard`: Add battery-widget voltage warning presentation without reusing firmware fault displays.
- `ride-announcements`: Add default-on battery charge settings, board-derived advance voltage alerts, qualification/recovery rules, independent cadence and priority within bounded shared speech admission; reconcile existing default and general eligibility/cadence requirements.

## Impact

- Frontend contract, defaults, client/mock and Settings announcement controls; shared product settings validation and backward-compatible decoding.
- Shared Rust announcement policy and existing connection/configuration/controller fact integration; loopback projection and existing C/JNI speech representations where the added closed battery phrase requires them.
- Apple and Android speech formatting/adapters, canonical conformance and focused frontend, persistence, shared-policy and native-boundary tests.
- No new dependency, database schema, storage marker or archive format is needed for the additive settings field. Keep frozen baseline fixtures unchanged. Reuse existing configuration readers and speech machinery; no superseded production path has been identified for removal and separate visual qualification remains in the existing shared owner without a new runtime or platform-specific frontend.
