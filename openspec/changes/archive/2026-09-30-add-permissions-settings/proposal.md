## Why

Users cannot currently inspect or recover all relevant OS permissions in one place. Existing Bluetooth and location summaries mix feature readiness with authorization, leaving denied access and platform settings recovery difficult to diagnose.

## What Changes

- Add a Permissions section in Settings with applicable permission rows, current access, purpose, feature impact, and individual Request access or Open settings actions.
- Show authorization even when optional location recording is off; distinguish restricted access, approximate location, disabled services, and unavailable hardware.
- Choose recovery from current native OS facts, including Android request history/rationale and separate background-location steps, iOS app Settings, and macOS Privacy & Security guidance.
- Refresh on permission callbacks and app activation, including returning from Settings and recreating the WebView.
- Extend the mandatory cross-platform contract with bounded permission snapshots and targeted recovery commands. Reuse existing permission entry points through the same policy and remove superseded denial/request routing.
- Non-goals: new permission categories, new background capture behavior, notification or battery-optimization permissions, OS permission revocation controls, onboarding redesign, or permission-driven changes to recording preferences.

## Capabilities

### New Capabilities

- `permissions-settings`: Inspect applicable permissions and recover access through platform-appropriate actions.

### Modified Capabilities

- `backend-contract`: Add authoritative permissions and targeted recovery to the mandatory shared contract.

## Impact

Touches the shared Settings UI, contract/client/state/mock, Rust service projection/dispatch and loopback validation, Apple and Android permission adapters and lifecycle integration, existing C/JNI facts/effects, and canonical conformance tests. No new dependency or database schema is required. Permission grants remain OS-owned; native request history may be retained only to distinguish first request from blocked re-prompting. Remove coarse denial-only routing where replaced; do not add compatibility aliases or a second runtime.
