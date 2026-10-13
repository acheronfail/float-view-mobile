# Proposal

## Why

iOS deliberately destroys the WebView and stops its UI server when backgrounded. Returning recovers the native connection but loses the user's destination and open panels, interrupting session browsing and configuration. Preserve that place without retaining the frontend or adding background work.

## What Changes

- Retain one small, typed UI checkpoint in the existing process-owned shared service, updated through authenticated request/result methods when navigation or supported presentation choices change.
- Restore the route, session/board target, Live normal/detailed mode and one supported ordinary open surface after obtaining fresh backend state.
- Keep current background WebView/server teardown. Checkpoints live only in memory and disappear when the native process ends.
- Reopen configuration surfaces from current values; do not restore drafts or replay actions. Validate stale targets and fall back to the nearest valid screen.
- Make restoration ordering explicit in the application shell instead of always redirecting recovered startup through Home to Live. Remove the superseded Live-mode persistence TODO when implemented.

### Non-goals

No restoration after process termination, disk/browser storage, database changes, retained WebViews, native message bridges, shutdown handshakes, periodic checkpoint timers, chart/map positions, scroll/focus offsets, editor drafts, action confirmations, operation replay or general component-state serialization.

## Capabilities

### New Capabilities

- `ui-state-restoration`: Minimal process-lifetime checkpoint capture, supported presentation restoration, target validation, bounded resource use and debugging behavior.

### Modified Capabilities

- `app-navigation`: Restore an eligible checkpoint destination before applying the existing recovered-connection navigation fallback.
- `backend-contract`: Add mandatory authenticated, bounded checkpoint get/set operations with the same semantics on Apple, Android and the frontend mock.

## Impact

Affected owners are the frontend contract/client/mock and application shell, Live and board-history presentation state, the Rust `backend-service` process owner and `loopback-server` dispatch, and canonical conformance coverage. Existing Apple and Android bindings expose the shared implementation without platform-specific restoration logic. Verify iOS background/foreground behavior on a scoped physical checkpoint during implementation.

This extends the cross-platform public API but changes no durable schema, archive, recording, Bluetooth or native lifecycle policy. Add no dependencies, compatibility shims, migrations or new scripts; use existing transport and test tools. No infrastructure removal is expected beyond superseded startup navigation logic and the Live-mode TODO.
