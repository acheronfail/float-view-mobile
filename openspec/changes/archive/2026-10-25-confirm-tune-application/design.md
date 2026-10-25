# Design

## Context

See proposal.md for motivation. Tunes currently has one conditional non-idle confirmation and a separate overwrite diff. The backend already accepts saved and current revisions and owns safety and verified writes.

## Goals / Non-Goals

Keep review state local to the feature and backend authority unchanged. Share the existing compact configuration-write comparison markup between configuration writes, apply and overwrite without adding a generic diff framework or dependency.

## Decisions

- Separate opening a preview from confirming it. Retain the fetched Tune and fresh board snapshot; do not refetch and silently substitute a different saved revision during confirmation.
- Read current values again before applying and compare tunable values, not revision numbers, because fresh reads can advance revisions. If values differ, refresh the preview and ask for review again. Pass the fresh revision to the existing apply method.
- Capture the device and controller route when opening; observe readiness and current-value invalidation to block a preview after reconnect or replacement. The public snapshot does not expose an internal generation token. Keep safety reasons derived from current authoritative state and backend revision checks.
- Extract the existing configuration confirmation’s compact label: before → after list into `RefloatConfigurationDiff.svelte`, used by configuration writes, apply and overwrite. Show direction once above the list, retain schema-aware units/choices and no-differences text, and remove the superseded tune-only renderer. Reuse ConfirmDialog's fixed actions and labelled keyboard-scrollable body.
- Keep errors in the modal for explicit retries. Matching tunes disable Apply and do not send redundant writes.

## Risks / Trade-offs

- Additional current reads add latency → show loading state and serialize requests.
- Board or saved values may change externally → compare fresh board values and preserve the saved revision for backend conflict rejection.
- WebKit host dependencies are unavailable → verify Chromium and report the missing WebKit coverage.

## Migration Plan

Ship as a frontend-only change with no data migration. Reverting restores the previous confirmation UI without storage changes.
