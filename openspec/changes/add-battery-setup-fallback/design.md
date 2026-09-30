## Context

See proposal.md for motivation. `BatteryEstimate` already distinguishes `not_configured`, `series_mismatch`, `missing_voltage`, and `not_applicable`; `LiveSample.batteryLevel` retains the raw fraction. `battery.ts` currently rounds only the estimate. The Live route owns a `BottomSheet` containing `BoardEditor`, which opens a selected profile and serializes revisioned autosaves. The service remains the estimator and persistence owner.

## Goals / Non-Goals

**Goals:** Keep percentage source selection consistent across live views and reuse one battery editing implementation with safe modal ownership.

**Non-Goals:** No frontend estimation, contract additions, backend changes, new dependencies, migrations, or firmware writes. Guest and BMS-only connections retain their current behavior.

## Decisions

1. Extend the shared frontend battery presentation helper to return percentage, source, and setup-needed state. Prefer a non-null estimate. Only a saved board with `not_configured` can use a finite reported fraction in [0,1]; zero is valid. Missing/invalid reported data yields an unavailable percentage while retaining the setup warning. This is an intentional product fallback, not development-version compatibility. A fallback for every unavailable reason was rejected because it would hide mismatches and unrelated failures.
2. Use the selected percentage for text, fill, color, and accessibility in both live views. Label the primary detailed value and accessible widget value according to source; retain the separate raw controller detail. Derive cell voltage only from the authoritative resolved series count. Do not guess a count or tie its visibility to availability of reported percentage.
3. Extract battery fields and validation into a board-owned shared form section, used by the full editor and a battery-only editor mode. Include series count (1–36), parallel count (1–20), bounded profile choice, and optional display-empty voltage ([2.7,4.2), default 3.0 V). Reuse the current profile lifecycle, patches, and serialized autosave queue. A separate copied form would risk divergent validation and saving behavior.
4. Use one Live configuration modal mode (full or battery) and capture its saved-board identity on open. The warning opens the existing sheet with title “Battery configuration” and explanatory text that the shown percentage is reported by the board until configuration is complete. No unrelated name, motor, CAN, or identity controls appear. No second concurrent profile editor is mounted. Keep the modal open when the warning disappears after a successful save.
5. Follow existing blur/tick/flush close ordering, surface save/conflict failures, and avoid claiming failed saves succeeded. Never redirect pending patches to a newly active board. Invalidate the editor on board removal or identity change; reconnecting the same saved board can retain it. Restore focus to the warning on close, or a stable Live control if the warning disappeared. Use native button semantics, primary Pointer Events with cancellation, a guarded keyboard/assistive click path, and touch-action manipulation.

## Risks / Trade-offs

- Reported percentage may differ from the estimate → Explain the source in the warning modal and source-aware labels; switch only after the authoritative snapshot updates.
- Telemetry refresh can reset fields or disrupt taps → Initialize against the opened profile identity, retain drafts during unrelated snapshots, and test with ongoing mock telemetry.
- Queued saves can race closing or board switches → Retain one profile owner, flush on ordinary close, bind the editor and queued work to the opened board, and cover identity changes and failures.
- Extraction can regress the full editor → Exercise the same controls in both full and battery-only modes.

## Migration Plan

Commit planning artifacts before implementation. Deliver a frontend-only change using existing requests and storage. No migration or reset is necessary. A rollback reverts the frontend presentation and form refactor without modifying saved profiles.
