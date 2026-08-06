## Why

An otherwise healthy board can occasionally drop or delay the first VESC identification exchange after BLE UART notifications are enabled, causing both Apple and Android backends to enter `failed` even though an immediate manual retry succeeds. The current logs expose only the outer connection state, so they do not identify which bounded negotiation stage or request outcome failed.

## What Changes

- Retry a transient initial controller-identification request within a bounded connection attempt before exposing failure to the user.
- Keep retry generations, request deadlines, and late responses isolated so retries cannot revive stale connections or create duplicate sessions.
- Add bounded, privacy-safe diagnostics for BLE setup and controller-identification stages, attempts, outcomes, and final failure codes on Apple and Android.
- Compile or runtime-gate diagnostic console logging to development/debuggable builds and remove unconditional production warnings for expected connection failures.
- Add equivalent Apple and Android regression tests plus conformance coverage for a dropped first identification response.
- Non-goals: changing the frontend/backend wire contract, extending total unbounded retry behavior, logging device addresses or private controller identity, or masking incompatible firmware and configuration failures.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vesc-device-connection`: Require one bounded recovery attempt for a transient initial identification response loss and privacy-safe development diagnostics.
- `backend-conformance`: Require both native implementations to prove the first-response-loss recovery and stale-response bounds.

## Impact

The change affects the shared-by-design application connection state machines and request schedulers in `backends/apple` and `backends/android`, their host diagnostic sinks, native unit tests, and the canonical connection conformance suite. It does not change the authenticated API contract, persistence schema, frontend behavior, dependencies, or portable data.
