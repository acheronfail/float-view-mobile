# Proposal

## Why

The live power widget and detailed view currently turn the controller's `batteryLevel` into a percentage. That value does not account for the board's selected cells and discharge curve. Community cell-specific voltage tables provide a concrete, inspectable basis for a better estimate, including approximate compensation for discharge current where measurements exist.

## What Changes

- Add a bounded, versioned catalog of six community cell profiles, plus an explicitly selected generic linear profile. Preserve the exact source tables and provenance with this proposal.
- Save the selected cell profile and display-empty cell voltage alongside the board's existing series and parallel counts.
- Calculate one shared-service estimate from pack voltage, pack current, pack configuration, and the selected profile. Interpolate voltage and measured load curves, filter discharge current, and map the configured empty voltage to zero percent.
- Publish the estimate in the authoritative connection snapshot and use it for the live power widget and detailed battery percentage, including consistent unavailable and stale states.
- Remove the two frontend calculations that use controller `batteryLevel` for the primary percentage. Retain that raw telemetry field and its historical/export semantics; label it separately in the detailed view.

Non-goals: changing firmware battery settings, coulomb counting, learning curves, temperature/age compensation, arbitrary curve editing, rewriting recorded rides, or estimating BMS-only connections. This is an approximate voltage-based display, not a remaining-range or battery-protection guarantee.

## Capabilities

### New Capabilities

- `battery-state-of-charge`: bounded cell profiles, saved configuration, deterministic estimation, and estimator lifecycle.

### Modified Capabilities

- `backend-contract`: profile validation and a shared battery estimate in complete connection snapshots.
- `live-board-dashboard`: cell-profile configuration and the same estimated percentage in compact and detailed live views.

## Impact

Implementation touches shared Rust product-store validation and application/service projection, `frontend/src/lib/backend/contract.ts`, frontend mock/client fixtures, BoardEditor, WidgetPower, DetailedView, and canonical conformance coverage. Both Apple and Android must expose the same shared projection without native formula implementations. Existing optional profile JSON can hold the configuration without database migrations or compatibility shims. Unconfigured boards and guests show an unavailable estimate rather than an inferred cell type. No new BLE requests or runtime dependencies are needed.
