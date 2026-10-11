# Design

## Context

See proposal.md for motivation. Active sessions already expose a published durable snapshot through session.get. SessionHistoryChart and SessionLocationMap currently rely on snapshot-keyed remounting; metadata refresh must instead replace bounded readers while retaining interaction state.

## Goals / Non-Goals

Keep one shared review route and native recording owner. No contract, persistence, dependencies or guest history changes.

## Decisions

- Use Current ride buttons on Live and Sessions, routing to the existing opaque session ID. The active viewer supplies Back to Live and status from authoritative connection state.
- Refresh metadata every three seconds while visible, with one request in flight. Pause refresh while inspecting, resume on Jump to live, and perform a terminal refresh when the session ends. Keep errors local and retain the last successful snapshot. Scope responses to the open-session generation so route changes cannot publish stale data.
- Follow the whole growing ride by default. Chart pan, zoom, point selection and map navigation/filtering pause following. Jump to live clears inspection/filtering and fits the whole current ride. Field visibility is retained.
- Keep chart and map instances across snapshot updates. Dispose superseded readers and reject late results. Refresh markers without clearing selection. Keep the map camera unless the user explicitly changes the chart range or jumps live.
- Preserve existing bounded series, location, event and marker reads. No live telemetry duplication. A frozen review snapshot remains consistent while native logging proceeds.

## Risks / Trade-offs

- Frequent complete track reads can be costly: use a modest visible-page cadence and existing bounds; pause during inspection and cancel superseded reads.
- A provisional session can disappear at close: show a useful no-longer-available state and return navigation rather than clearing unrelated application state.
- Refresh errors: retain the last view, show failure feedback, and retry at the bounded cadence.

## Migration Plan

Frontend-only deployment; no storage migration. Revert the frontend change to roll back.
