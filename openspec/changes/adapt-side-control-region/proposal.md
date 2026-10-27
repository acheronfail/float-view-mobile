## Why

On displays with a system control region at the side, the application's fixed bottom tabs and header buttons waste vertical content space and do not follow the system's reachable navigation layout. Capture real native/WebView geometry before changing their placement.

## What Changes

- Pin iOS distribution builds to the Duo-capable SDK so the layout is enabled in future distributed apps.
- Add an opt-in, bounded CI simulator capture for native bounds, safe areas, web viewport geometry and screenshots, independent of release workflows.
- Position primary tabs, Live header actions and routed back controls in a usable side region when the host reports one; release their former content space.
- Preserve the shared frontend, route ownership, accessibility and pointer/keyboard behavior. Recompute layout when the available region changes and fall back safely when it is unavailable or too small.
- Non-goals: native tab navigation, BLE changes, persistence changes, release publishing, always-running CI jobs or device-name/viewport-size detection.
- Replace fixed control placement only where measured host geometry supports it; remove temporary capture-only paths if a reusable diagnostic supersedes them.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-navigation`: adaptive placement of existing primary navigation, header actions and return controls around system-reserved areas.

## Impact

Shared shell, header, navigation CSS and frontend interaction/rendering tests; thin Apple UI geometry integration if required by captured WebKit behavior; conformance capture tooling, a dedicated CI workflow, and the iOS-only distribution toolchain selection. No backend wire-contract or storage change is planned. Geometry is an OS UI fact, not product state. Native and browser coverage will verify geometry and fallback behavior without BLE hardware.
