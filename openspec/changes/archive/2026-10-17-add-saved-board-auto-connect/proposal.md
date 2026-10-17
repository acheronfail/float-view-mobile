# Proposal

## Why

Opening Float View should connect to the rider's preferred saved board when it is nearby, including when the app restores a screen other than Home. Explicit disconnect must remain effective until the next foreground visit, without requiring a native process restart.

## What Changes

- Allow one locally associated saved board to be selected through a “Connect automatically” toggle in board settings, backed by nullable, installation-local `Settings.autoConnectSavedDeviceId`.
- Give each foreground visit one 15-second discovery/auto-connect opportunity once Bluetooth, required permissions, and disclaimer acceptance are ready. Reuse normal bounded Bluetooth discovery, independently of the displayed route.
- Keep Home's ongoing discovery behavior; expiration of automatic eligibility must prevent later Home advertisements from connecting automatically.
- Cancel automatic eligibility on manual connection intent, explicit disconnect, backgrounding, or the first automatic connection attempt. Rearm only on a new foreground visit; active connection/reconnect work takes precedence.
- Preserve restored non-live destinations while reflecting the automatic connection through authoritative snapshots.
- Replace the Home-only connection restriction and clarify that UI restoration does not itself start discovery, while foreground lifecycle policy may do so independently.

Non-goals: multiple preferred boards, name/RSSI-based selection, indefinite automatic search, background discovery, new reconnect policy, new permission prompts, or a separate Bluetooth implementation.

## Capabilities

### New Capabilities

- `saved-board-auto-connect`: Single-board local preference, bounded foreground discovery, automatic admission, manual override, and lifecycle cancellation.

### Modified Capabilities

- `backend-contract`: Permit foreground automatic saved connections in addition to explicit Home discovery connections.
- `app-navigation`: Preserve restored non-live routes during automatic connection and allow lifecycle discovery independently of Home.
- `ui-state-restoration`: Distinguish route restoration from independently triggered foreground discovery.

## Impact

- Shared Rust product settings, board deletion, discovery/connection policy, timers, and loopback contract projection; existing Apple C and Android JNI service consumers.
- Frontend contract/client/mock, board settings control, and navigation integration; canonical conformance coverage for both host families.
- Persist one local nullable saved-board ID, defaulting to null; validate association and clear it atomically on selected-board deletion. Exclude the preference from portable board/session archives.
- No new dependencies or generated APIs. Reuse the scan and connection paths; remove the superseded Home-only policy wording. If settings encoding requires an incompatible storage change, use an explicit format/schema marker rather than migrations or legacy readers.
