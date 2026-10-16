# Design

## Context

See proposal.md for motivation. `HomeView.svelte` currently requests repeated normal scans while mounted. `backend-service/src/policy.rs` owns the one physical scan, bounded scan results, exact local saved-board associations, connection generations, timers, and native foreground facts. `connect` already starts the saved/guest identification path and stops discovery. The initial policy foreground value is currently true, so construction must not accidentally consume or duplicate the first real foreground opportunity.

Settings use revision-checked product-store updates. Portable archives can create boards with no local Bluetooth association. UI restoration deliberately precedes Home-to-Live fallback navigation.

## Goals / Non-Goals

**Goals:** Keep automatic admission and timing in the existing shared owner, retain one physical scanner, and make ordered lifecycle/manual/discovery input decide behavior consistently across hosts.

**Non-Goals:** A frontend-owned timer, another runtime, a new Bluetooth adapter API, a persisted suppression flag, or a second reconnect mechanism. Non-Home routes must not gain scan loops.

## Decisions

### 1. Persist one local reference through existing settings

Add required nullable `autoConnectSavedDeviceId` to the complete Settings shape, default null, patched through `settings.update`. Validate existence and local association inside the settings transaction, including mixed patches; preserve existing error semantics. Successful selected-board deletion must clear the reference and increment settings revision in the same product-store transaction that removes the board. Preserve the existing file cleanup ordering and failure behavior; do not clear the setting on rejected/failed deletion. Publish the resulting complete snapshot coherently.

The board editor presents a toggle derived from current Settings, not Board Profile completeness or a copied per-board boolean. Selecting B replaces A in one settings mutation. Use the existing native-button/pointer activation and revision/error patterns. Imported unassociated boards show an unavailable control with concise guidance to connect locally first. Exclude this local reference from portable archives.

Alternative: per-board flags require uniqueness enforcement across records and make installation-local policy look like portable board configuration. A single settings reference expresses the invariant directly.

### 2. One opportunity per native foreground visit

Represent the opportunity as constant-size service-owned state: visit identity, waiting/searching/closed state, and an optional monotonic deadline. Native lifecycle facts define visits, including initial startup; repeated foreground=true facts and WebView/socket recreation do not create visits. Audit both hosts' initial lifecycle delivery, including macOS application visibility semantics, so a service default cannot act as proof of actual foreground state. Hosts report OS facts only.

At visit start, an existing active connection (including a retained failed connection) closes the opportunity for that visit. With no active connection, wait for a configured target, accepted disclaimer, and existing radio/scan/connect permission readiness. Start one 15-second wall-clock window once ready. Selecting a target for the first time while still waiting may start that visit's opportunity. No selection means no lifecycle scan. Preference changes during search change only the target; they do not extend the deadline. Clearing selection closes an opportunity. Preference edits cannot reopen closed state.

Permission/radio loss during search removes scan demand but leaves the original deadline running; recovery may resume within it. Waiting does not poll or request permissions. Backgrounding closes waiting/searching state. The next actual foreground transition creates a new visit.

Alternative: frontend lifetime misses background/foreground cycles that retain the WebView. Process lifetime suppresses too long. Frontend mount notifications would rearm too often.

### 3. Share discovery demand and keep independent deadlines

Reuse normal scan effects, observations, filtering, expiry and published discovered devices. Track bounded normal-request and foreground-window demand separately over the existing single physical scan. Repeated normal scan requests remain idempotent within their existing bound. Finishing one demand must not stop the scanner while another remains, and must never extend automatic eligibility. Home keeps its current scan cadence; other routes rely only on the foreground demand.

On expiry/background, remove only foreground demand; stop the scanner if no valid ordinary demand remains. Normal per-request scan bounds still apply. An admitted connection stops all discovery through the existing connection path. Correlate scan observations and deadline callbacks with current scan/visit identity. Record the opportunity start boundary so a cached pre-window discovery cannot trigger automatic admission; joining a live scan requires a fresh observation during the window. At or beyond the deadline, reject automatic admission even if the expiry callback has not yet run.

Alternative: calling the current start/stop methods without demand accounting can either stop Home's scan early or let Home extend the automatic window. A separate scanner duplicates native work and callback races.

### 4. Consume before starting the existing connection flow

After accepting a current discovery observation, resolve the preferred board using its exact local association. Recheck target validity, foreground readiness, deadline, and absence of any active connection. Consume eligibility before invoking the ordinary saved connection flow. Preserve existing firmware/controller verification, candidate selection, recording preparation and established-connection reconnect behavior; do not treat a Bluetooth match as controller verification.

At public dispatch, a syntactically valid manual connection command or explicit disconnect closes automatic eligibility before connection processing, even if the operation subsequently fails. Malformed requests do not affect it. Internal auto-connect/reconnect calls must not masquerade as new manual intent. Ordered service input determines races: an already admitted automatic connection remains subject to the existing competing-connection rule, while a previously processed manual command prevents subsequent automatic admission. Setup/identification failure does not restore eligibility. User shutdown paths that explicitly suppress reconnection must also suppress this foreground opportunity.

Alternative: rearming on failure or each advertisement creates a retry loop; matching by name or RSSI can select another rider's board.

### 5. Preserve navigation and authoritative state

Automatic connections publish the existing complete connection snapshots; no frontend `device.connect` echo is needed. Home transitions to Live through its ordinary connection-state handling once restoration permits it. A displayed/restoring non-live route stays in place, including when the connection appears before checkpoint loading finishes. Keep restoration's neutral loading guard and authoritative controller/issue overlays. Update the mock to reproduce lifecycle windows and settings behavior deterministically rather than simulating automation in a view.

No public auto-connect status object is needed for this scope: existing Bluetooth scanning and connection state describe the visible work. The setting remains the only new public field.

## Risks / Trade-offs

- Native lifecycle signals differ across iOS/macOS/Android → verify one initial visit and genuine background/foreground transitions through each existing host adapter; do not rearm for duplicate observations or ordinary view replacement.
- Permission dialogs may themselves cause lifecycle changes → process only actual reported transitions and retain existing permission presentation policy; no new automatic prompts.
- Sharing scans can hide deadline extension bugs → deterministic tests must exercise both arrival orders of Home demand and foreground demand, expiry at the boundary, cancellation, and stale callbacks.
- A board powered on after 15 seconds will not auto-connect until another visit → intentional bounded behavior; Home remains available for manual connection.
- Board settings gain a global selection control → explain that enabling this board replaces the previous selection and derive every toggle from one authoritative setting.

## Migration Plan

Commit these planning artifacts before implementation. Add the setting and affected consumers in coherent buildable slices, then implement policy and UI with matching task updates. Do not add migrations or old-format readers. If strict persisted Settings decoding makes the new required field incompatible, advance the explicit owned-storage format/schema marker and reset only version-mismatched development data, retaining ordinary corruption/operational failure handling.

Validate focused store/service/loopback tests, frontend checks and interaction/navigation mock coverage, and canonical host conformance. Perform scoped physical foreground/permission/scanning checkpoints on affected Apple and Android hosts; record actual results and unavailable coverage. After implementation verification, sync deltas and archive in the final change commit. Reverting implementation removes the new preference and policy; it does not require retaining development-format compatibility machinery.
