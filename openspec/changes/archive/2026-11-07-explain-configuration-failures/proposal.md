# Proposal

## Why

A real configuration failure report retained only `invalid_state`, hiding whether readiness, mutation safety, identity or revision validation rejected the request. Support needs actionable context at the failure itself, including failures coalesced after the capture window.

## What Changes

- Preserve closed runtime failure categories and configuration admission facts before public error mapping.
- Record configuration/control request admission and completion, pending work, revision and identity match flags, write blockers and generation correlation.
- Retain failure details through incident coalescing and test sanitized ZIP export.
- Keep existing bounds and privacy exclusions. No raw errors, values, fingerprints or device identifiers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `support-diagnostics`: Explain configuration and mutation failures with bounded causal context.

## Impact

Shared Rust service diagnostic vocabulary, runtime/dispatch instrumentation and focused tests. Both native hosts consume the same service; no public API, native ABI, dependency or product persistence change. Existing generic error mapping remains for callers; its diagnostic information loss is removed. Non-goals: changing mutation authorization, fixing an unproven device failure, frontend changes or implementing the separate schema-admission proposal.
