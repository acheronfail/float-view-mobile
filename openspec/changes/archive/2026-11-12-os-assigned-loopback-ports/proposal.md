# Proposal

## Why

Stable and beta installations can collide on a fixed native loopback port and fail startup. OS-assigned ports let independent process owners coexist; embedding saved appearance avoids losing early theme hints when the web origin changes.

## What Changes

- Bind native loopback listeners to an OS-assigned port and load the embedded frontend from the reported origin.
- Discover actual native ports in development and conformance tooling; remove the fixed native port registry and same-number forwarding assumptions.
- Embed validated saved theme/accent preferences into initial HTML before any frontend script runs.
- Preserve authentication, loopback-only binding, Rust ownership and persistent data formats.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `platform-hosts`: OS-assigned native endpoints, discovery seams and authoritative initial appearance.
- `backend-conformance`: discover native endpoints and retain scoped forwarding cleanup; verify concurrent listeners and first-script appearance.

## Impact

Shared Rust loopback server, Android and Apple hosts, frontend startup/theme controller, development runners and canonical target adapters/tests. No wire product API or persistence format changes; host start configuration changes. No new dependencies or native message bridge. Non-goals: changing mock/Vite/fixture ports, BLE policy or durable product behavior.
