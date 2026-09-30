## Context

See proposal.md for motivation. `BatteryEstimate` already distinguishes `not_configured`, `series_mismatch`, `missing_voltage`, and `not_applicable`; `LiveSample.batteryLevel` retains the raw fraction. The fallback and battery-only modal were implemented in `fae1403`, including the shared `BatteryFields.svelte`, serialized saves, and browser verification. The remaining work extends that form with presets; preserve the completed work and its evidence. The Live route owns a `BottomSheet` containing `BoardEditor`, which opens a selected profile and serializes revisioned autosaves. The service remains the estimator and persistence owner.

## Goals / Non-Goals

**Goals:** Keep percentage source selection consistent across live views, make initial setup a familiar battery-name selection, and reuse one form with safe modal ownership and optional expert controls.

**Non-Goals:** No frontend estimation, duplicate product catalog, new dependencies, migrations, or firmware writes. The companion SOC change owns the shared catalog and contract additions consumed here. Guest and BMS-only connections retain their current behavior.

## Decisions

1. Extend the shared frontend battery presentation helper to return percentage, source, and setup-needed state. Prefer a non-null estimate. Only a saved board with `not_configured` can use a finite reported fraction in [0,1]; zero is valid. Missing/invalid reported data yields an unavailable percentage while retaining the setup warning. This is an intentional product fallback, not development-version compatibility. A fallback for every unavailable reason was rejected because it would hide mismatches and unrelated failures.
2. Use the selected percentage for text, fill, color, and accessibility in both live views. Label the primary detailed value and accessible widget value according to source; retain the separate raw controller detail. Derive cell voltage only from the authoritative resolved series count. Do not guess a count or tie its visibility to availability of reported percentage.
3. Extend the existing board-owned shared battery form in both the full editor and battery-only mode. Advanced includes series count (1–36), parallel count (1–20), bounded profile choice, optional pack capacity (0.1–200 Ah), and optional display-empty voltage ([2.7,4.2), default 3.0 V). Reuse the profile lifecycle and serialized autosave queue. The shared service supplies preset metadata and resolves selections; the frontend does not duplicate pack policy or estimate charge.
4. Use one Live configuration modal mode (full or battery) and capture its saved-board identity on open. The warning opens the existing sheet with title “Battery configuration” and explanatory text that the shown percentage is reported by the board until configuration is complete. No unrelated name, motor, CAN, or identity controls appear. No second concurrent profile editor is mounted. Keep the modal open when the warning disappears after a successful save.
5. Follow existing blur/tick/flush close ordering, surface save/conflict failures, and avoid claiming failed saves succeeded. Never redirect pending patches to a newly active board. Invalidate the editor on board removal or identity change; reconnecting the same saved board can retain it. Restore focus to the warning on close, or a stable Live control if the warning disappeared. Use native button semantics, primary Pointer Events with cancellation, a guarded keyboard/assistive click path, and touch-action manipulation.


### Basic and Advanced interaction

Use two adjacent, labelled native buttons, “Basic” and “Advanced”, in a labelled group with exposed selected state. Retain primary-pointer/cancellation handling and guarded keyboard/assistive click activation, visible focus, and touch-action manipulation. Use a native select with manufacturer optgroups for the battery options. The full editor and Live sheet render the same implementation.

Open Basic for an entirely unconfigured battery or a saved named preset. Open Advanced for existing partial/manual configuration without a preset identity so opening setup never silently replaces it. Keep mode local to the editor; toggling changes presentation only and preserves all drafts, including invalid input. A field blur caused by switching modes must not implicitly save or discard that draft. Subsequent explicit edits and ordinary close retain the existing validation/flush behavior. Do not infer a branded preset from matching S/P/profile values: different products share those values.

Basic has a “Battery” select with an unselected “Choose your battery” placeholder and “Custom / not listed”. Selecting Custom opens Advanced while retaining values and making no save until an actual edit. Selecting a named preset deliberately replaces all battery defaults through one revisioned request: identity, S/P, resolved profile, nominal pack Ah, and the 3.0 V display-empty default. It does not touch unrelated profile values. A summary may show “20S2P · Samsung 50S · 10 Ah nominal”; names lead, electrical details are secondary, and there is no current-limit, firmware, or controller setup UI. Basic never shows “generic approximation”, the generic curve label, missing-curve warnings, or an extra confirmation when the shared catalog maps a pack to `generic-linear`. Generic remains available as a truthful technical curve choice in Advanced. Retain normal estimated-versus-reported percentage semantics without claiming measured accuracy.

The selected pack variant must be explicit where cells differ (for example X7 P28A, X7S P30B, and X7 Long Range 50S; GT P42A rather than every stock GT). Manufacturer groups and names come from the bounded shared catalog described in the companion design. Do not guess a pack from controller identity, Bluetooth name, or measured voltage. Keep Custom usable if the catalog has no matching option.

Treat the selected option as a draft until the save is accepted; show pending/error feedback and do not publish success from a failed or conflicting request. Telemetry cannot reset the draft. Serialize rapid selections, bind every queued selection to the opened board, and only display accepted estimates/revisions. Explicit preset selection replaces unsaved battery drafts; already queued manual edits finish before it and must never overwrite its defaults afterward. On reopening, use persisted preset identity rather than reverse-matching electrical values. Advanced S/P/profile edits that depart from the selected preset clear identity atomically in the service; capacity or display-empty overrides retain identity and appear in the Basic summary as actual values. Effective S/P changes clear the previous capacity unless that same manual patch provides an explicit replacement; curve-only changes preserve capacity. Draft values departing from the preset show Custom immediately rather than retaining a misleading brand label. Toggling back to Basic never reapplies defaults. For manual values without identity, show Custom rather than choosing a brand. A deliberate preset re-selection restores its defaults, including a previously cleared capacity. Provide “Reset to battery defaults” in Advanced for a named preset, using the same selection request, because choosing an already-selected native option may not fire a change event.

### Coordination and delivery

The preset catalog, selection semantics, nominal capacity, and stable preset identity are owned by `stabilize-battery-state-of-charge` (design decisions 2 and 2a, contract/capacity tasks). This UI extension depends on that slice, not on completed temporal tuning. The companion change owns method/provisional explanations; this change owns the segmented control and all battery form fields, including capacity. Keep the dashboard deltas complementary: this change modifies setup controls/modal; the companion adds capacity/temporal explanation requirements. Sync must retain both.

## Risks / Trade-offs

- Reported percentage may differ from the estimate → Explain the source in the warning modal and source-aware labels; switch only after the authoritative snapshot updates.
- Telemetry refresh can reset fields or disrupt taps → Initialize against the opened profile identity, retain drafts during unrelated snapshots, and test with ongoing mock telemetry.
- Queued saves can race closing or board switches → Retain one profile owner, flush on ordinary close, bind the editor and queued work to the opened board, and cover identity changes and failures.
- Preset UI can regress the full editor → Exercise the same controls in both full and battery-only modes.
- Battery names can hide cell variants → Use explicit variants and source-backed metadata, keep Advanced available, and never infer identity from a controller.
- Several names have identical electrical settings → Persist the selected identity in the shared profile; do not reverse-match names.
- A mode toggle can accidentally commit a blurred field → Suppress toggle-induced commits and preserve the draft without a request.

## Migration Plan

Commit revised planning artifacts before the remaining implementation. Preserve completed tasks 1–3 and their historical verification. First land the companion shared catalog/capacity/identity contract slice, then implement the unchecked preset UI tasks here. No migration or reset is necessary. Reverting the UI extension leaves accepted electrical settings and capacity intact; no alternate estimator or legacy reader is introduced.
