# Design

## Context

Board profiles already own liveWidgetSlots and customDashboard and the Rust service allows disconnected profile operations. Frontend helpers and renderers currently derive their target and data from ActiveConnection. See proposal.md for motivation.

## Goals / Non-Goals

Reuse actual Live layout, value picker, Custom composition and BoardIcon. Keep example presentation local and explicitly separate from connection state. No host, protocol, contract or storage-format changes; no new dependencies.

## Decisions

- Use one full-screen board-scoped editor route from both menus rather than expanding the connection-owned Live route. It pins the saved-device identifier and displays its name and configured artwork. Keeping a separate route preserves it through disconnects; frontend recreation returns to board history without replaying a draft.
- Narrow the shared widget presentation input to the connection facts it consumes. Supply fixed local example facts and chart history in preview; never create an active backend connection or use another board's telemetry.
- Pass explicit preview slot choices and selection callbacks through Live renderers. In preview, taps configure and every runtime action is suppressed. Keep ordinary connected gestures unchanged.
- Reuse revisioned profile writes, remove active-connection guards from board-scoped dashboard saves, and retain lifetime/identity guards in editors. Live slot writes read the selected profile and retry a single conflict; whole Custom saves keep explicit conflict recovery.
- Keep existing Live immediate saves and Custom drafts. The dedicated editor enters Custom editing without a hold and protects dirty exits and view switches. Custom Save or accepted Cancel returns to the originating surface, falling back to board history after disconnect. Preview browsing never writes lastLiveView.

## Risks / Trade-offs

- Example values could be mistaken for measurements → persistent Preview · Example values label, static bounded data and no connected state claim.
- Shared profile revisions can conflict → retain drafts and explicit reload for Custom; merge a Live slot onto freshly read profile choices.
- Nested overlays and Back can bypass a draft guard → reuse requestLeave and common dismissal ownership, verify native/keyboard paths.
- Different saved-board types → only show the connected entry for saved boards, retain offline imported-board support, and use the same artwork default as the saved-board list.

## Migration Plan

No data migration. Existing fields and defaults are unchanged; removing the frontend feature restores prior access without altering saved layouts.
