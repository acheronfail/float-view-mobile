# Design

## Context

See [proposal.md](proposal.md) for motivation and scope. iOS `AppBackend+Lifecycle.swift` removes the frontend endpoint and stops the listener on backgrounding; `ContentView.swift` consequently removes the WebView. Foreground starts another listener/frontend generation over the same `SharedService` and Rust process owner.

`AppShell.svelte` authenticates, obtains `state.get` and applies `connectionRedirect`. Live presentation is local state in `routes/live/+page.svelte`, including a normal/detailed mode persistence TODO and separate overlay flags. Board history owns its actions and Board Profile overlay. Session routes already identify their data; restoring their destination does not require keeping metadata, chart data or map instances alive.

## Goals / Non-Goals

**Goals:** Keep one inspectable record and a short, explicit capture/restore path. Preserve process ownership and authenticated transport. Keep feature-specific eligibility and rendering with existing frontend owners. Add no native-host domain policy or new bindings.

**Non-Goals:** See the proposal. In particular, opening an editor is restorable but its unfinished work is not. Treat a checkpoint as presentation intent, never as authority for board actions or data.

## Decisions

### 1. One typed record in the shared process owner

Add a nullable checkpoint field to `backend-service`'s existing process-owned state, not to listener state or the product store. The service retains structurally validated records; frontend owners decide how current state affects presentation eligibility. `loopback-server` dispatches `uiState.get` and `uiState.set` through the existing service. Apple, Android and macOS share this path; the mock implements equivalent semantics.

Use a version-1 discriminated TypeScript contract and hand-written Rust parsing/projection. Define only explicit fields:

- `route`: `home`, `sessions`, `settings`, `live`, `bms`, `boardHistory` with `savedDeviceId`, or `session` with `sessionId` and nullable `savedDeviceId` return context.
- `live`: null outside Live; otherwise `deviceId` and `mode` (`normal` or `detailed`). A replacement device does not inherit the previous device's presentation.
- `surface`: null or one discriminated descriptor. Live supports `liveActions`, `liveCharts`, `bmsDetails`, `refloatConfiguration`, `refloatTunes` and `boardConfiguration` with saved-board ID and `batteryOnly`. Board history supports `boardActions` and `boardConfiguration` for its board. Other routes have no restorable surface in version 1.

Enforce nonempty identifiers up to 512 UTF-8 bytes and total encoded checkpoint size up to 4,096 bytes, including escaping. Reject unknown fields/versions, extra targets and incompatible combinations atomically. `get({})` returns the record or null; `set({checkpoint})` returns `Accepted`, with null clearing it. Keep checkpoint operations out of snapshots and revision/event publication. Structurally valid records can become stale; do not query product storage on every checkpoint save.

Alternatives: browser storage has a different lifetime and would put retained UI data outside the chosen process owner; host message bridges duplicate transport and require platform wiring; generic component serialization obscures what is restored and captures unrelated state. All are unnecessary here. Add no dependencies.

### 2. Explicit capture from presentation choices

The app owner assembles the current route and delegates Live/board-history presentation fields to their existing feature owners. Maintain one complete current record, not per-route caches or a registry of arbitrary component state. At most one ordinary surface is checkpointed, so opening a child replaces the actions-menu descriptor and closing it produces null.

Submit only when this small record changes. Use one request in flight and one latest pending replacement, compare records to deduplicate and submit the next replacement after the previous request finishes. Use existing request timeouts; no periodic retry loop or checkpoint timers. Scope callbacks to the current frontend lifetime and reuse existing connection cancellation so old completions cannot mutate a replacement frontend.

Capture only after startup restoration resolves, and before an acknowledged UI choice can be superseded by a later save. User interaction remains immediate; it does not wait for checkpoint acknowledgement. If teardown interrupts the latest request, the last accepted record is the recovery point. There is deliberately no guarantee for a tap immediately followed by teardown, and no shutdown handshake to obtain that guarantee.

Checkpoint save failure does not disrupt navigation, clear the accepted service record or imply the choice was saved. Report a fixed debug failure outcome; the next meaningful UI change can submit the newest complete record.

### 3. One startup restoration pass before normal redirects

After authentication, obtain fresh `state.get` and `uiState.get` through the existing request/result client. The shell holds a neutral loading presentation while resolving them; Home discovery and recovered-connection redirects do not run during this gate. Do not save default root-route/component state over the stored checkpoint.

An explicit incoming non-root route different from the saved destination takes precedence as new navigation intent; native foreground recreation loads the root URL and uses the checkpoint. Reloading the same destination can restore its presentation. With no applicable checkpoint, release the existing navigation fallback. With a checkpoint, resolve the route against current state, load ordinary detail/profile/configuration data, then let its owner restore the eligible surface. Session destination restoration uses its existing detail load and return-context handling; inspection state initializes normally. Refloat configuration uses normal open/read behavior and current values; it does not restore stale value maps. Live charts start their existing empty bounded buffer.

Explicitly check eligibility using the same rules as the corresponding menu action. A changed device skips old Live mode/surfaces. Absent saved boards return board history to Sessions or close only a Live board editor. A missing session returns to its existing board history, otherwise Sessions, with useful feedback. A missing connection uses existing Home fallback; a classified BMS connection uses its correct route. Authoritative flywheel and other required connection overlays take precedence, with incompatible ordinary surfaces dropped rather than deferred behind them.

Do not restore confirmations, discard prompts or operation flags. Do not invoke menu callbacks that submit mutations. Opening a surface invokes only its normal loading/presentation path. New user navigation cancels pending restoration, and late loads are scoped to the selected route/target. Once resolution finishes, capture the effective restored/fallback UI so skipped surfaces do not keep returning.

An operational load failure retains the native record and offers explicit retry; it is not evidence the target disappeared. Keep capture suppressed while that restore attempt is unresolved. Explicit navigation abandons the attempt and permits capture of the user's new choice. Transport reconnection within an existing frontend does not rerun startup restoration.

### 4. Debugging and verification remain bounded

Emit debug-only capture/restore outcomes using fixed route/surface kinds and reason codes such as `missing_target`, `device_changed`, `surface_unavailable` and `request_failed`. Never log IDs, addresses, tokens or record contents. The typed schema and tests are the debugging tools; add no debug endpoint, event stream or metrics subsystem.

Test contract validation, lifetime and isolation in the affected Rust crates and canonical conformance. Frontend tests cover redirect ordering, supported overlays, defaults, pending-save ordering, failure handling and late-load cancellation. Exercise the shared method boundary through Apple and Android integration coverage without broadening to unrelated packaging or devices. A focused physical iOS background/foreground check confirms actual teardown/return behavior for a session screen and ordinary Live panel; it does not require firmware interoperability or an all-platform BLE matrix.

## Risks / Trade-offs

- Last interaction can be lost before acknowledgement → recover the last accepted checkpoint; retain prompt teardown and document this boundary.
- Editors reopen without unsaved inputs → explicitly retain this non-goal and load current values without autosaving during restore.
- Startup defaults or late loads overwrite restored UI → gate capture/redirects and scope async restoration to the current route and frontend lifetime.
- Background work changes target eligibility → validate fresh state and use narrow fallbacks rather than replaying old actions.
- Future UI changes outgrow version 1 → replace the schema explicitly; add no old-format readers or migration paths for process-memory data.

## Migration Plan

Commit planning artifacts before implementation, then land coherent verified implementation slices with task updates. Implement contract, mock, shared service and dispatch together before wiring frontend capture/restore. No storage migration or reset is needed. Rollback removes the checkpoint methods, field and restoration code together, returning to current default startup behavior. After scoped checks pass, sync specs and archive in one final commit.
