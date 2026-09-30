## Why

The current estimator combines instantaneous voltage with delayed current compensation and publishes each calculated percentage directly. Load steps can therefore look like large changes in pack charge; the displayed percentage should instead track charge over time and correct gradually from trustworthy voltage observations.

## What Changes

- Replace current-only smoothing with coherent voltage/current observations and a stateful shared-service estimate that suppresses load-step and recovery jumps.
- Add optional pack capacity in Ah, supplied by an explicitly selected named battery preset or entered manually. With capacity and valid signed battery current, integrate charge consumption and regeneration; use settled low-load voltage to correct drift gradually.
- Publish a bounded, service-owned catalog of named Onewheel/VESC battery variants with S/P counts, curve mapping, and sourced nominal pack capacity. Persist preset identity and apply complete selections atomically through the revisioned profile API.
- Resolve presets without dedicated curves to `generic-linear` internally; Basic setup presents familiar battery names without approximation labels, warnings, or additional confirmation.
- Without capacity, retain a bounded, smoothed voltage estimate rather than requiring another setup field or guessing capacity from a profile name.
- Define initialization, telemetry gaps, reconnect, configuration edits, missing current, and process lifecycle behavior, including provisional estimates after uncertain intervals.
- Extend the shared contract with estimation method and provisional status; expose capacity and concise estimation help in battery configuration.
- Remove the superseded current-only filter and instantaneous-publication policy. Keep raw telemetry, live voltage, charging indicators, and firmware alerts responsive.

Non-goals: automatic capacity learning, per-cell imbalance or aging models, temperature compensation, range prediction, BMS SOC selection, firmware writes, new BLE polling, durable estimator checkpoints, and guaranteed absolute SOC accuracy.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `battery-state-of-charge`: Coherent observations, bounded temporal estimation, charge integration, rest correction, and lifecycle rules.
- `backend-contract`: Shared preset catalog, atomic preset selection, identity/override validation, optional capacity, and explicit estimate method/provisional metadata across all consumers.
- `live-board-dashboard`: Capacity controls and explanation of the temporal estimate without delaying voltage or alerts.
- `backend-conformance`: Deterministic dynamic-load, charge-accounting, lifecycle, and host-parity coverage.

## Impact

Shared Rust backend-service estimator and product-store validation, loopback projection, frontend contract/client/mock and battery controls, Apple/Android contract consumers, canonical conformance, and documentation. Capacity and preset identity persist through the existing revisioned profile and archive mechanism; temporal estimator state stays bounded and process-owned. No new dependencies, database migrations, compatibility readers, or firmware protocol changes.

`add-battery-setup-fallback` already implemented reported-percentage fallback and a shared battery modal. Its new Basic/Advanced preset UI depends on this change's catalog/capacity/identity contract slice (tasks 1.1–1.3), which can land before temporal tuning. That change owns the setup form and capacity input; this change owns catalog policy, persistence, estimation, and method/provisional explanation. Preserve both complementary dashboard deltas during sync.
