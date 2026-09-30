## Context

See proposal.md for motivation. `backend-service/src/battery_estimator.rs` currently smooths positive current with a two-second EMA, combines it with instantaneous voltage, and returns the curve inversion directly. `battery.rs` owns that filter in the active connection and publishes after accepted telemetry. The seven pinned community curves have no capacity metadata. The frontend and mock mirror the existing projection; `conformance/src/suite.ts` and `conformance/test/battery.browser.ts` cover static battery behavior.

## Goals / Non-Goals

**Goals:** Keep one deterministic, constant-space temporal estimator in the shared service; stabilize all configured boards; enable physical charge accounting when capacity is supplied; distinguish provisional initialization from settled tracking.

**Non-Goals:** See proposal.md. In particular, this change does not promise that community curves are laboratory OCV data, infer missing consumption, or convert firmware low-voltage warnings into SOC warnings.

## Decisions

### 1. Separate a voltage observation from tracked charge

Replace `CurrentFilter` with estimator state containing the accepted sequence/time, previous signed current, tracked charge fraction, voltage observation, and bounded settling statistics. Derive a load-compensated observation from voltage and current in the same accepted sample before temporal smoothing. Do not compensate raw voltage with delayed current. Retain the pinned curve interpolation and load clamp, without extrapolating beyond measured loads. For absent current or regeneration, the voltage observation uses the zero-load curve but is not considered a settled correction.

Maintain internal charge `q` on the original curve's [0,1] axis. Compute `qEmpty` once from the zero-load curve at the configured display-empty voltage and publish `clamp((q-qEmpty)/(1-qEmpty),0,1)`. This makes charge integration and the display reserve consistent; remapping each load curve separately would change the definition of capacity with load. Reject no currently valid empty voltage; handle tiny positive denominators safely and clamp finite results. Preserve curve knots/catalog provenance while documenting this deliberate remapping change.

### 2. Optional capacity from explicit presets or manual entry

Add nullable `batteryCapacityAh` to saved profile values, finite in [0.1,200]. It means full nominal pack charge capacity before display-empty remapping, not Ah per cell or Wh. A 4.5 Ah cell in 2P suggests 9 Ah; series count does not multiply Ah. The user can enter a known measured capacity. Blank means voltage smoothing only and does not make an otherwise configured board incomplete.

Keep capacity metadata on a separate pack-preset catalog, not on the discharge-curve catalog: a curve does not establish measured capacity or uniquely identify a cell. An explicit preset selection supplies sourced nominal pack Ah, calculated as cell Ah × P where appropriate; mark the summary “nominal” until manually overridden. Never derive capacity from profile ID, S count, voltage, or controller identity. All subsequent estimates use the persisted numeric capacity, including with a generic curve. Blank or unknown preset capacity stays null. No capacity is learned automatically. Use existing revisioned profile and archive storage; no migration or new persistence table is needed.

A manual capacity override or clear survives telemetry, reconnect, reopening, and mode switches. Do not repopulate it from catalog metadata on reads. The UI can derive whether the current value equals nominal for display only; no persistent capacity-origin field is needed. Deliberately selecting/re-selecting a preset restores all its defaults. Effective S/P edits must clear capacity to null unless the same validated manual patch supplies an explicit replacement, avoiding stale capacity from the prior pack. A curve-only override clears identity but preserves explicit capacity.


### 2a. Shared battery presets and atomic selection

The shared Rust product catalog owns a checked-in, versioned list of at most 128 pack variants, projected by the existing shared server as `batteryPresets` and `batteryPresetCatalogVersion` in the authenticated snapshot. Each entry has `id`, `label`, `manufacturer`, `cellLabel`, `seriesCount`, `parallelCount`, `batteryCellProfileId`, and nullable `nominalCapacityAh`. Bound IDs/version to 64 characters and labels/manufacturer/cell labels to 128; reject duplicate IDs and invalid metadata in catalog tests. Counts remain S 1–36 and P 1–20. Existing curve IDs and curve data stay unchanged. Native hosts contain no separate preset or selection logic. The mock mirrors this contract with independently asserted examples.

Add optional nullable `batteryPresetId` to `BoardProfileValues`. A revisioned profile patch with a non-null preset ID is an explicit selection operation: resolve it in the service and atomically replace identity, S/P, resolved curve, capacity, and display-empty default (null, meaning 3.0 V). Preserve unrelated fields. Reject a selection patch that also supplies any battery-value field, so precedence is unambiguous. Re-selecting the same ID deliberately restores defaults. A null ID only clears identity; without other edits it preserves electrical values. Manual edits without an ID preserve identity if S/P/curve still match, otherwise clear it in the same transaction. Capacity/empty-voltage overrides alone preserve identity. Effective S/P changes clear old capacity unless an explicit replacement is included; curve-only edits do not clear capacity. Unknown IDs, invalid values, conflicts, or persistence failure leave both saved profile and estimator unchanged.

Imported full profiles are validated records, not selection commands: preserve valid capacity/empty overrides, and require any non-null identity to match stored S/P/curve. Reject mismatched or unknown identities rather than silently rewrite imported data. No preset identity is inferred for existing manual profiles. Persisting identity avoids choosing the wrong name among electrically identical packs. Reading snapshots/catalog updates never reapplies defaults. Use existing archive validation, not migrations, aliases, or old-format readers.

A known preset whose cell has no supported curve explicitly maps to `generic-linear`. This is normal product behavior, distinct from invalid user-supplied curve IDs (which still fail validation). Do not borrow a different cell's named curve or manufacture discharge data. Generic observation uses the existing single-curve path; charge integration still uses valid capacity/current. Basic hides the curve implementation and shows no approximation label or missing-curve warning. Advanced may show the actual catalog curve label. Continue truthful estimated battery semantics and method/provisional details; no measured-accuracy claim is added.

Preset identity changes alone must not reset the temporal estimator when effective S/P/curve/capacity/empty values are unchanged. A complete preset selection that changes effective values reseeds only once after the accepted transaction. Mode toggles issue no request and cannot reset state.

### Research and initial preset catalog

Research on 2026-09-30 inspected Float Hub 1.5.0 (package `com.thefloatlife.FloatHub`, version code 48), downloaded from [APKPure](https://apkpure.net/float-hub/com.thefloatlife.FloatHub/download/1.5.0). XAPK SHA-256: `4cc1e0e4c279b7e371038eaba84a57c701af42a96be8f872b738dfe7e4723db8`. Static Hermes v96 inspection found 48 preset records, 15 named cell records, and Custom. Its preset selection populates S/P/cell, capacity is cell Ah × P, and manual S/P changes select Custom. The app was not executed and publisher signing identity was not independently authenticated. Use the factual configuration evidence; do not vendor the APK, disassembly, or app implementation.

[Developer documentation](https://pev.dev/t/float-hub-documentation/2406) corroborates the preset approach but predates some APK additions. Manufacturer cross-checks include [X7 Long Range](https://fungineers.us/products/funwheel-x7-long-range), [X7S](https://fungineers.crisp.help/en/article/everything-you-need-to-know-about-the-x7s-1mzbf5l/), [Floatwheel Atom](https://floatwheel.co/index.php?product_id=9935&route=product%2Fproduct), [Z Battery Torque Pack](https://zbattery.solutions/products/torque-pack-18s2p-p42a), [Indy 18S2P](https://indyspeedcontrol.com/general-batteries/p/182pvesc), and [Chi 32.1](https://chibatterysystems.com/collections/onewheel-batteries/products/chivexrgt32-1). The latter two explicitly offer cell variants: an APK default is not proof of an installed pack's cell. The Atom manufacturer confirms 22S1P, resolving an inconsistent 2P description in the developer's controller table.

Seed the following bounded, variant-specific selection catalog. Each row's capacity is nominal, not measured. Include the manufacturer in ambiguous names and the cell variant where a product has alternatives; split the XR/Pint X shared research row into separate friendly labels with identical settings. Existing curve IDs are used below; `generic-linear` is internal metadata, never Basic copy. M35A identifies the M35A variant only; a Sanyo GA variant uses generic instead. Do not generalize the GT P42A entry to all GT production batteries. Retain supported installed older packs even when no longer sold; these are current hardware choices, not legacy storage compatibility.

| Manufacturer / battery variant | S/P | Cells | Nominal pack Ah | Internal curve |
| --- | --- | --- | --- | --- |
| Floatwheel — ADV | 20S2P | LR2170LA / DG40 | 8 | `dg-01-dg-40` |
| Floatwheel — ADV Pro | 20S2P | Molicel P42A | 8.4 | `molicel-p42a` |
| Floatwheel — ADV2 | 20S2P | Samsung 50S | 10 | `samsung-50s` |
| Floatwheel — Atom | 22S1P | Reliance RS50 | 5 | `generic-linear` |
| Fungineers — Funwheel X7 | 20S2P | Molicel P28A | 5.6 | `molicel-p28a` |
| Fungineers — Funwheel X7S | 20S2P | Molicel P30B | 6 | `generic-linear` |
| Fungineers — Funwheel X7LR | 20S2P | Samsung 50S | 10 | `samsung-50s` |
| Fungineers — Funwheel X7HV | 32S1P | Molicel P45B | 4.5 | `molicel-p45b` |
| Onewheel — GT S-Series | 27S1P | Molicel P45B | 4.5 | `molicel-p45b` |
| Onewheel — GT | 18S2P | Molicel P42A | 8.4 | `molicel-p42a` |
| Onewheel — XR Classic | 18S2P | Molicel P30B | 6 | `generic-linear` |
| Onewheel — XR / PintX | 15S2P | VTC6 / 30Q | 6 | `generic-linear` |
| Onewheel — Pint | 15S1P | Murata VTC5D | 2.8 | `generic-linear` |
| Indy Speed Control — 32s1p | 32S1P | Molicel P45B | 4.5 | `molicel-p45b` |
| Indy Speed Control — 30s1p | 30S1P | Molicel P45B | 4.5 | `molicel-p45b` |
| Indy Speed Control — 20s2p GT Pack | 20S2P | Molicel P45B | 9 | `molicel-p45b` |
| Indy Speed Control — 20s2p XR Split Pack | 20S2P | Molicel P45B | 9 | `molicel-p45b` |
| Indy Speed Control — 20s2p Battery Kit | 20S2P | Molicel P30B | 6 | `generic-linear` |
| Indy Speed Control — XRV PnP Kit | 18S2P | Molicel P45B | 9 | `molicel-p45b` |
| Indy Speed Control — 18s2p | 18S2P | Molicel P45B | 9 | `molicel-p45b` |
| Indy Speed Control — Smart Link 32s2p | 32S2P | Reliance RS50 | 10 | `generic-linear` |
| Indy Speed Control — Smart Link 30s2p | 30S2P | Reliance RS50 | 10 | `generic-linear` |
| The Board Garage — FAS Pack | 30S1P | Molicel P50B | 5 | `generic-linear` |
| The Board Garage — '777' VOW GT | 24S2P | Molicel P45B | 9 | `molicel-p45b` |
| The Board Garage — GT ME4T Pack | 20S2P | Samsung 50S | 10 | `samsung-50s` |
| The Board Garage — Extra Regular Pack | 20S2P | Molicel P30B | 6 | `generic-linear` |
| The Board Garage — Regular Pack | 20S2P | Molicel P28A | 5.6 | `molicel-p28a` |
| The Board Garage — 19s2p | 19S2P | Molicel P45B | 9 | `molicel-p45b` |
| The Board Garage — XR ME4T Pack | 18S2P | Samsung 50S | 10 | `samsung-50s` |
| The Board Garage — Power Pack | 18S2P | Molicel P45B | 9 | `molicel-p45b` |
| Z Battery Solutions — Speed Pack | 20S2P | Molicel P28A | 5.6 | `molicel-p28a` |
| Z Battery Solutions — Torque Pack [P42A] | 18S2P | Molicel P42A | 8.4 | `molicel-p42a` |
| Z Battery Solutions — Torque Pack [P45B] | 18S2P | Molicel P45B | 9 | `molicel-p45b` |
| Chi — CHI-VE XR/GT 32.1 | 32S1P | Molicel P50B | 5 | `generic-linear` |
| Chi — CHI-VE GT 20.2 | 20S2P | Molicel P50B | 10 | `generic-linear` |
| Chi — CHI-VE XR 20.2 | 20S2P | Molicel P30B | 6 | `generic-linear` |
| Chi — CHI-VE XR 18.2 | 18S2P | Molicel P50B | 10 | `generic-linear` |
| Chi — CHI-VE PintX/XR 24.1 | 24S1P | Molicel P50B | 5 | `generic-linear` |
| Chi — CHI-VE Pint X 20.1 | 20S1P | Molicel P50B | 5 | `generic-linear` |
| Chi — CHI-VE Pint 20.1 | 20S1P | Molicel P30B | 3 | `generic-linear` |
| Chi — CBCSO | 20S2P | VTC6 / 30Q | 6 | `generic-linear` |
| Chi — CBXR - Original | 15S3P | Molicel M35A | 10.5 | `molicel-m35a` |
| Chi — CBXR - VESC Power | 15S3P | Molicel P28A | 8.4 | `molicel-p28a` |
| Chi — CBXR Mini | 15S2P | Sanyo 20700B | 8 | `generic-linear` |
| Chi — CBSP | 15S1P | Molicel P42A | 4.2 | `molicel-p42a` |
| Chi — Quart - Standard | 15S1P | Molicel P42A | 4.2 | `molicel-p42a` |
| Chi — Quart - S Edition | 15S1P | Samsung 50S | 5 | `samsung-50s` |

Add cell-specific alternatives confirmed by the manufacturer pages: Indy 18S2P P50B (10 Ah, generic), Chi 32.1 P45B (4.5 Ah, `molicel-p45b`) and Samsung 50S (5 Ah, `samsung-50s`). Add a CBXR Original Sanyo GA variant (15S3P, 10.5 Ah, generic), keeping it separate from M35A. These are explicit choices, not automatic cell detection. Unlisted variants use Custom. Exclude the researched 40S1P entry because it exceeds the existing 36S contract; this change does not broaden pack limits or support early LiFePO4 Onewheel chemistries with Li-ion curves. Keep source URL, retrieval date, and nominal-capacity derivation beside the implementation catalog as development provenance, not Basic UI copy. Test every row's bounds, unique identity, profile resolution, and nominal-capacity arithmetic.

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

Keep state within the existing application owner. WebView recreation and snapshot reads cannot advance or reset it. A same-board/controller reconnect or gap retains the last tracked value as a provisional seed, clears the integration pair and settling window, and resumes without charging for unobserved time. A new board/controller, process restart, or estimation configuration change discards the seed and initializes from a fresh valid observation (or the latest accepted sample for a profile edit). A name-only or preset-identity-only revision with unchanged effective estimation values does not reset charge. Retain no unbounded per-board cache or durable estimator state.

Invalid voltage keeps the existing `missing_voltage` unavailable result and breaks observation/integration continuity; recovering telemetry can reuse the same-board seed provisionally without accounting for the invalid interval. Nonready snapshots retain the last value as stale. Profile validation failure, persistence failure, repeated reads, and rejected samples must not advance state. Apply estimate/state changes transactionally with accepted publication.

Extend `BatteryEstimate` with `method: 'voltage_smoothed' | 'charge_integrated' | null` and `provisional: boolean`; null/false applies to unavailable results. Existing `mode` continues to describe the latest voltage observation's compensation, not the charge integration method. `charge_integrated` means the current update uses valid charge accounting, even if rest correction also applies; initial seeding and broken intervals use `voltage_smoothed`. Update the authoritative contract, shared projection, mock, Apple/Android consumer checks, and canonical assertions together. No native host estimator is introduced.

### 6. Configuration and display integration

The companion `add-battery-setup-fallback` extension owns the shared Basic/Advanced form: Basic selects a named pack and summarizes its current capacity, while Advanced includes “Pack capacity (Ah, optional)”, range feedback, and the 2P example. This change supplies the catalog/capacity/identity contract first, plus method/provisional help in battery details or Advanced help. Basic does not require the rider to understand curves, estimation methods, or provisional-state terminology. Add concise method/provisional explanation in battery help or detail, not a second smoothing layer. Keep both primary percentage views rounded from the shared fraction. Raw voltage, controller percentage, charging state, and firmware alerts bypass SOC smoothing.

The other proposal's controller-reported fallback remains active only for `not_configured`. Missing capacity is not `not_configured`; reported values do not enter this estimator. Use an added dashboard requirement to avoid replacing the other change's modified setup requirements during sync. A preset mapped to generic is configured; absence of a dedicated curve never re-enables the missing-configuration warning. Invalid voltage and series mismatch retain their existing distinct handling.

## Risks / Trade-offs

- Current offsets and wrong capacity cause drift → Distinguish nominal from manually entered capacity in summaries, allow Advanced overrides/clear, rest-correct gradually, and test sign/units and measured traces. Do not claim learned capacity or reference-instrument accuracy.
- Product names span cell revisions → Label exact variants and retain sources; never infer installed hardware from an APK default.
- Generic curves lack cell-specific load data → Retain bounded single-curve semantics and shared temporal corrections, without Basic warnings or invented curve data.
- Catalog updates can accidentally overwrite measured capacity → Apply defaults only on explicit selection, never reads or mode switches.
- Voltage-only operation can remain biased during prolonged loads → Clearly identify the method and offer capacity configuration; smoothing alone is not charge accounting.
- Disconnects or process restarts leave unknown usage → Never integrate missing time; use provisional reseeding and gradual correction.
- Slow correction can retain a poor startup estimate → Expose provisional status and quantify convergence in tests rather than silently snapping after a throttle release.
- Display-empty values close to full amplify percentage changes → Integrate on the original charge axis and apply correction bounds after remapping; test extreme valid settings.
- Recorded traces may be unavailable → Synthetic tests establish determinism and stability only. Report missing trace evidence and do not claim measured accuracy improvements without it.

## Migration Plan

Commit the plan, then implement coherent slices with task updates. Replace the old filter rather than retaining a version fallback. Extend existing optional profile storage; no reset or migration is needed. Ship the public contract consumers together. Land tasks 1.1–1.3 as the prerequisite for the companion Basic/Advanced UI, without waiting for the remainder of temporal-estimator tuning. Existing fallback/modal implementation and its historical tests remain intact; new integration tasks need fresh evidence. Run focused crate/consumer tests and canonical host integration coverage, then sync/archive after verification. Reverting the change removes optional capacity usage and restores the prior estimator; no destructive data operation is required.

## Validation Approach

Use independent charge-accounting oracles and fixed-timestamp traces: acceleration/release, prolonged discharge, regeneration, charging, rest recovery, unsupported loads, missing telemetry, and reconnect. For a 9 Ah pack at constant 60 A over one observed second, internal charge decreases by 1/540 (about 0.185 percentage points), before reserve remapping. In voltage-only mode, an 80%-to-30% observation step changes an initialized published level by at most 0.1 percentage points in one second. No rest correction occurs during a short release. Replay available captured traces alongside synthetic cases; a new physical BLE matrix is not required for unchanged transport behavior.
