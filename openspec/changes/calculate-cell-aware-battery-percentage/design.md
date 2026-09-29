# Design

## Context

See `proposal.md` for motivation. `WidgetPower.svelte` and `DetailedView.svelte` currently multiply `LiveSample.batteryLevel` by 100 independently. Board profiles already store series/parallel counts in validated JSON. Raw samples are persisted before publication for saved rides. The shared Rust application/service owns estimation state; loopback projection exposes it, and both native hosts consume the same implementation.

## Goals / Non-Goals

**Goals:** deterministic, bounded estimation with an auditable data source; coherent sample/profile revisions; identical percentages across frontend surfaces and WebView recreation.

**Non-Goals:** interpreting voltage as measured remaining capacity, changing historical `batteryLevel`, guessing cell identity, or introducing another runtime owner. The proposal lists product exclusions.

## Decisions

### Pin the upstream measurements

Use the tables in `profile-data.json`, taken from [Floaty's BatteryDischargeCurvePicker](https://github.com/acheronfail/floaty/blob/c4c87fe26e91ef2345518c3e0a1adb941a211885/apps/app/components/BoardsPage/BatteryDischargeCurvePicker.tsx). The profile file last changed in commit `0b513b2d6e11d79e9cd257993983e3e749064d26` (2025-05-18, adding M35A); the inspected master commit is dated 2025-07-04. Repository push time alone is not evidence of newer tables. These are community cell-model profiles, not a comprehensive chemistry catalog or certified capacity measurements.

Supporting discussion: [profile data](https://discord.com/channels/1346864089170710739/1347601377815429151/1363086738440065125), [current compensation](https://discord.com/channels/1346864089170710739/1347601377815429151/1363089032397197432), and [minimum-voltage remapping](https://discord.com/channels/1346864089170710739/1347601377815429151/1363102665361457232). Float Control's [voltage/energy discussion](https://pev.dev/t/vesc-voltage-vs-battery-percentage/819) describes a different approach; integrating net energy is deferred.

During implementation move the canonical data into the shared Rust estimator with attribution and tests for all knots. Keep a catalog version for provenance in the projected estimate. Do not fetch profiles at runtime or duplicate numeric tables in the frontend. Expose the bounded ID/label choices from shared service state for the editor.

### Explicit saved configuration

Extend `BoardProfileValues` with optional `batteryCellProfileId` and `batteryMinimumCellVoltageV`. Profile IDs are exactly those in the data file. The minimum must be finite and in [2.7, 4.2); absent minimum means 3.0 V. No selected profile means unavailable. Existing S (1..36) and P (1..20) remain required for estimation, even for a zero-load-only curve, making configuration requirements uniform. Saved S is authoritative; if a known controller S disagrees, report `series_mismatch` until corrected. Do not guess from nominal pack voltage.

BoardEditor adds a cell selector and “Display empty voltage per cell”, explains the 3.0 V default and that this does not change controller protection settings, and retains revisioned autosave. Generic linear is an explicit selection, never a fallback. Optional new fields do not change existing profile-completeness policy. Product-store validation applies equally to direct saves and archive import; no schema migration is necessary.

### Curve calculation

All curves list cell volts at SOC fractions [1, .9, .8, .7, .6, .5, .4, .3, .2, .1, 0]. They must be strictly decreasing and finite. Let `V(q)` be piecewise-linear voltage at SOC q and `Q(v)` its inverse, clamped at endpoints.

For each load curve, remap to minimum m: `qMin = Q(m)`; its adjusted voltage knot at fraction q is `V(qMin + (1-qMin)*q)`. Force exact endpoints m and 4.2. This follows Floaty's range adjustment without intermediate four-decimal rounding; retain full precision until rendering.

Compute cell voltage as pack volts / S and discharge amps per cell as filtered positive pack amps / P. Between measured currents, interpolate each corresponding adjusted voltage knot linearly, then invert that resulting curve at the observed cell voltage. Clamp current above the highest measured load to that curve, never extrapolate. Single-curve profiles, missing current, and nonpositive current use the zero-load curve. Clamp the resulting fraction to [0,1]. Invalid/nonpositive voltage returns unavailable, never NaN or an old fresh-looking value. High current clamping and zero-load fallback are exposed as modes, not confidence claims.

Numerical oracles before filtering (minimum 2.7 V): P45B at 3.73 V/cell and 0 A/cell is 0.5; at 3.605 V/cell and 10 A/cell it is 0.5. Thus 20S2P, 72.1 V and an initialized 20 A pack-current filter yields 0.5. With minimum 3.23 V, qMin is 0.1 and the adjusted half-charge knot is the original curve at 0.55: 3.775 V, yielding 0.5. Remapping resamples eleven knots, so it is not generally identical to directly rescaling the original inverse at every voltage. Verify knot, midpoint, endpoint, and remap cases independently.

### Bounded current state and lifecycle

Use a time-based EMA on positive pack current, time constant 2 seconds, `alpha = 1-exp(-dt/2s)`, initialized to the first valid current. Use monotonic accepted sample times. Reset on board/controller change, reconnect, estimator configuration revision change, or a gap over 5 seconds. Missing/nonpositive current clears the filter and uses zero load immediately. Ignore duplicate/out-of-order samples. State is constant-size and driven by accepted telemetry, with no extra BLE polls or history scans. Publish saved-sample estimates only after the existing persistence gate succeeds.

A profile edit recomputes against the latest accepted sample, resetting current state, within the same authoritative snapshot update. WebView recreation reads the existing estimate without advancing the filter. Nonready connection snapshots retain the last sample and matching estimate with existing dimmed/stale semantics; the first accepted reconnect sample initializes anew. Percent may rise during regeneration or voltage recovery: do not enforce monotonic discharge or invent charging progress.

### Public projection and views

Add `batteryCellProfiles` (bounded ID/label choices) to the authenticated root snapshot, and mandatory `batteryEstimate` to board `ActiveConnection` projection. The estimate carries nullable `level` (fraction), `profileId`, `seriesCount`, `sampleSequence`, `profileRevision`, catalog version, and `mode` (`load_compensated`, `load_clamped`, `voltage_only`, `unavailable`). Add nullable `unavailableReason` with closed values `not_configured`, `series_mismatch`, `missing_voltage`, `not_applicable`; no arbitrary diagnostics. Metadata refers to the same current saved profile and accepted sample. Missing inputs yield null level; BMS-only and guest connections use `not_applicable`. Update TypeScript, mock fixtures, public Rust projection, and both host conformance paths together.

WidgetPower consumes only this estimate for fill, color, accessible label, and percentage; its per-cell voltage uses the estimate's resolved S to avoid differing pack interpretations. DetailedView shows the same rounded percentage as “Estimated battery level” and retains “Controller-reported battery level” for the raw field. Both round only for display to nearest integer percent and use unavailable placeholders for null, preserving charging indication and raw voltage. A shared frontend formatting helper prevents rounding divergence; it does not implement estimation. Remove the superseded primary percentage calculations. Recorded/exported samples retain their existing meaning.

## Risks / Trade-offs

- Community curves and unknown temperature/age cause error → label the value estimated, preserve source attribution and expose the selected profile; do not imply laboratory accuracy.
- Load above measured data and zero-load-only cells limit compensation → clamp and expose mode; never fabricate curves for other cells.
- Filtering can lag load changes → bounded time-based filter, explicit lifecycle resets, test step changes and regen.
- Missing configuration initially removes a familiar percentage → provide direct saved-board configuration controls and explicit unavailable state without a misleading controller fallback.
- Incorrect series count distorts voltage → reject known S disagreement and use one resolved count across the widget.

## Migration Plan

Implement contract, shared service, and frontend as coherent buildable slices; existing optional JSON storage remains valid without migrations or aliases. Preserve raw recordings. Run canonical conformance and `just verify` before implementation completion. A rollback reverts implementation and optional fields remain inert data; do not add compatibility readers for this development-only change.
