# Design

## Context

The telemetry service already owns the resumable live ride clock. Saved session distance integrates absolute speed trapezoidally between continuous observations. Normalized samples also retain raw firmware trip distance. Shared backend projection supplies both WebView state and native Live Activity data.

## Goals / Non-Goals

Keep live distance independent of board boot lifetime and preserve honest recording gaps. Do not modify historical sample columns, summary algorithms, archives, storage markers, or native presentation layouts.

## Decisions

Track a separate current-ride distance in the existing telemetry owner, with one previous speed and the existing sample timestamp. Use the saved summary's absolute-speed trapezoidal integration. Restore its initial value alongside riding time through the current runtime/coordinator restore path. Clear the previous observation on reconnect; reset totals only for new rides. Missing speed breaks integration. Publish only finite totals.

Project this total through the existing live `distanceM` field and native `distanceMeters`; keep raw normalized `distance_m` for recording. The contract documents the live/historical distinction. Both native hosts already consume the shared projection, so no host-specific arithmetic is needed. Update mock behavior and conformance assertions.

Adding saved distance directly to the firmware counter would double-count retained counters and include unobserved movement. A counter-offset algorithm would add reboot detection and differ from saved summaries. Integrating observed speed keeps live and saved calculations consistent and supports firmware without trip telemetry. No dependency is needed.

## Risks / Trade-offs

- Wheel-derived speed retains existing calibration/slip limitations → use the same calculation as saved summaries.
- Saved totals are authoritative after process replacement → restore only committed distance and preserve persistence failure handling.
- Live versus historical distance has distinct meaning → document the existing live field explicitly and leave historical storage unchanged.

## Migration Plan

No data migration. Existing supported ride summaries seed resumed totals. Both hosts receive the shared implementation together; rollback changes only live presentation semantics.
