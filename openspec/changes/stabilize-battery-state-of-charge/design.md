## Context

See proposal.md for motivation. `backend-service/src/battery_estimator.rs` currently smooths positive current with a two-second EMA, combines it with instantaneous voltage, and returns the curve inversion directly. `battery.rs` owns that filter in the active connection and publishes after accepted telemetry. The seven pinned community curves have no capacity metadata. The frontend and mock mirror the existing projection; `conformance/src/suite.ts` and `conformance/test/battery.browser.ts` cover static battery behavior.

## Goals / Non-Goals

**Goals:** Keep one deterministic, constant-space temporal estimator in the shared service; stabilize all configured boards; enable physical charge accounting when capacity is supplied; distinguish provisional initialization from settled tracking.

**Non-Goals:** See proposal.md. In particular, this change does not promise that community curves are laboratory OCV data, infer missing consumption, or convert firmware low-voltage warnings into SOC warnings.

## Decisions

### 1. Separate a voltage observation from tracked charge

Replace `CurrentFilter` with estimator state containing the accepted sequence/time, previous signed current, tracked charge fraction, voltage observation, and bounded settling statistics. Derive a load-compensated observation from voltage and current in the same accepted sample before temporal smoothing. Do not compensate raw voltage with delayed current. Retain the pinned curve interpolation and load clamp, without extrapolating beyond measured loads. For absent current or regeneration, the voltage observation uses the zero-load curve but is not considered a settled correction.

Maintain internal charge `q` on the original curve's [0,1] axis. Compute `qEmpty` once from the zero-load curve at the configured display-empty voltage and publish `clamp((q-qEmpty)/(1-qEmpty),0,1)`. This makes charge integration and the display reserve consistent; remapping each load curve separately would change the definition of capacity with load. Reject no currently valid empty voltage; handle tiny positive denominators safely and clamp finite results. Preserve curve knots/catalog provenance while documenting this deliberate remapping change.

### 2. Explicit optional capacity, no inferred cell ratings

Add nullable `batteryCapacityAh` to saved profile values, finite in [0.1,200]. It means full nominal pack charge capacity before display-empty remapping, not Ah per cell or Wh. A 4.5 Ah cell in 2P suggests 9 Ah; series count does not multiply Ah. The user can enter a known measured capacity. Blank means voltage smoothing only and does not make an otherwise configured board incomplete.

Do not add nominal ratings to the catalog in this change: several profiles are generic or ambiguous and their curves do not establish a measured capacity. Automatic defaults and learned capacity can be separate improvements. Use existing profile revision validation and archive storage; no migration or new persistence table is needed.

### 3. Integrate signed current on observed intervals

With capacity and consecutive finite current observations separated by at most five seconds, use trapezoidal integration:

`qNext = clamp(q - (Iprevious + Icurrent) * dtSeconds / (2 * 3600 * capacityAh), 0, 1)`.

Positive battery current consumes charge; negative current adds it, with unity charge efficiency as an explicit approximation. Do not use motor current, a smoothed current, or firmware accumulated-Ah counters with uncertain reset boundaries. Zero-duration, duplicate, and out-of-order samples consume no charge. Missing current breaks the integration interval. Never extrapolate through a gap or reconnect. While current is missing, retain temporal continuity using the smoothed voltage path and mark the result provisional; resume integration only after a new consecutive valid pair.

### 4. Gradual correction and explicit provisional state

Seed the first estimate from the current coherent voltage observation and mark it provisional. A seed taken under load is immediately useful but can be biased. With capacity/current available, track consumption during load and only correct toward voltage after sustained low current and stable voltage. Do not infer a full/empty charge anchor from a momentary voltage endpoint or a charging flag.

Initial implementation constants (engineering starting points, not accuracy claims):

- Settling requires 30 continuous seconds with absolute current at most 0.1 A per parallel cell, cell-voltage range at most 0.01 V over that interval, and no sample gap over five seconds. Maintain min/max and elapsed time for a qualifying window; reset the window when either criterion fails. Missing current and active charging do not qualify.
- After qualification, voltage correction uses a 120-second time constant and is capped at 0.1 displayed percentage points per second; apply the equivalent bound in internal charge coordinates. Continue checking stability; renewed load ends correction immediately. Clear provisional only after qualification and convergence within two displayed percentage points of the settled observation.
- Without capacity (or while current is unavailable), follow the coherent voltage observation with a 30-second EMA and the same 0.1 displayed percentage-point/second limit. This improves presentation, not absolute accuracy under sustained unsupported loads. Mark provisional during missing current, regeneration, load clamp, or single-curve discharge; require the settled convergence criterion to clear it.
- Real charge integration is not restricted by the voltage-correction rate limit. A known small pack can genuinely change charge quickly.

Pin these constants in deterministic fixtures and document any trace-driven tuning before landing. Do not expose tuning sliders. UI-only animation was rejected because readers and recreated views would disagree and it would not improve the underlying estimate.

### 5. Lifecycle and public projection

Keep state within the existing application owner. WebView recreation and snapshot reads cannot advance or reset it. A same-board/controller reconnect or gap retains the last tracked value as a provisional seed, clears the integration pair and settling window, and resumes without charging for unobserved time. A new board/controller, process restart, or estimation configuration change discards the seed and initializes from a fresh valid observation (or the latest accepted sample for a profile edit). A name-only revision does not reset charge. Retain no unbounded per-board cache or durable estimator state.

Invalid voltage keeps the existing `missing_voltage` unavailable result and breaks observation/integration continuity; recovering telemetry can reuse the same-board seed provisionally without accounting for the invalid interval. Nonready snapshots retain the last value as stale. Profile validation failure, persistence failure, repeated reads, and rejected samples must not advance state. Apply estimate/state changes transactionally with accepted publication.

Extend `BatteryEstimate` with `method: 'voltage_smoothed' | 'charge_integrated' | null` and `provisional: boolean`; null/false applies to unavailable results. Existing `mode` continues to describe the latest voltage observation's compensation, not the charge integration method. `charge_integrated` means the current update uses valid charge accounting, even if rest correction also applies; initial seeding and broken intervals use `voltage_smoothed`. Update the authoritative contract, shared projection, mock, Apple/Android consumer checks, and canonical assertions together. No native host estimator is introduced.

### 6. Configuration and display integration

Add “Pack capacity (Ah, optional)” to the existing battery section, including units, range feedback, and the 2P example. Reuse it in the battery-only modal if `add-battery-setup-fallback` is implemented. Add concise method/provisional explanation in battery help or detail, not a second smoothing layer. Keep both primary percentage views rounded from the shared fraction. Raw voltage, controller percentage, charging state, and firmware alerts bypass SOC smoothing.

The other proposal's controller-reported fallback remains active only for `not_configured`. Missing capacity is not `not_configured`; reported values do not enter this estimator. Use an added dashboard requirement to avoid replacing the other change's modified requirements during sync, and reconcile its enumerated battery-only fields to include optional capacity if necessary.

## Risks / Trade-offs

- Current offsets and wrong capacity cause drift → Explain inputs, rest-correct gradually, and test sign/units and measured traces. Do not claim a learned capacity or reference-instrument accuracy.
- Voltage-only operation can remain biased during prolonged loads → Clearly identify the method and offer capacity configuration; smoothing alone is not charge accounting.
- Disconnects or process restarts leave unknown usage → Never integrate missing time; use provisional reseeding and gradual correction.
- Slow correction can retain a poor startup estimate → Expose provisional status and quantify convergence in tests rather than silently snapping after a throttle release.
- Display-empty values close to full amplify percentage changes → Integrate on the original charge axis and apply correction bounds after remapping; test extreme valid settings.
- Recorded traces may be unavailable → Synthetic tests establish determinism and stability only. Report missing trace evidence and do not claim measured accuracy improvements without it.

## Migration Plan

Commit the plan, then implement coherent slices with task updates. Replace the old filter rather than retaining a version fallback. Extend existing optional profile storage; no reset or migration is needed. Ship the public contract consumers together. Run focused crate/consumer tests and canonical host integration coverage, then sync/archive after verification. Reverting the change removes optional capacity usage and restores the prior estimator; no destructive data operation is required.

## Validation Approach

Use independent charge-accounting oracles and fixed-timestamp traces: acceleration/release, prolonged discharge, regeneration, charging, rest recovery, unsupported loads, missing telemetry, and reconnect. For a 9 Ah pack at constant 60 A over one observed second, internal charge decreases by 1/540 (about 0.185 percentage points), before reserve remapping. In voltage-only mode, an 80%-to-30% observation step changes an initialized published level by at most 0.1 percentage points in one second. No rest correction occurs during a short release. Replay available captured traces alongside synthetic cases; a new physical BLE matrix is not required for unchanged transport behavior.
