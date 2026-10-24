# Design

## Context

See proposal.md for motivation. Recording-boundary events already identify the previous sample sequence and the first resumed sample. Pruning preserves both rows. Series generations also break for null fields and timing delays, so they cannot identify confirmed recording interruptions.

## Goals / Non-Goals

Keep one authoritative set of exact break endpoints per published snapshot and one reversible chart time mapping. Do not infer breaks from connection-context events or reduced series.

## Decisions

- Add required `recordingBreaks` to `SessionMetadata`, bounded to 1,000 sorted nonoverlapping `[startElapsedMs,endElapsedMs]` pairs (compact tuples keep the worst-case collection below the 64 KiB WebSocket message limit). Resolve existing boundary events against their retained sample rows under the published snapshot. Invalid or excessive boundary data fails the read; absence produces an empty array. This avoids a new data kind and per-field repeated queries.
- Implement the read and projection once in the shared Rust path used by Apple and Android. Native adapters need no new dispatch or OS behavior.
- Use a piecewise reversible elapsed-time mapping that subtracts only explicit recording intervals. Compose it with map-filter intervals. All requests, selections, labels and callbacks keep original elapsed milliseconds. Use the compact coordinate space for pan/pinch calculations as well as drawing and inspection.
- Draw a narrow contrasting vertical separator at every visible recording break. Include duration and original endpoints in accessible text and an SVG title. Keep ordinary timing/null gaps at their current width and never bridge traces.
- Keep the mapping independent of lazily loaded field groups and preserve offscreen curve projection during pan previews. Do not add dependencies or retain a competing coordinate path.

## Risks / Trade-offs

- Collapsing time changes gesture scale: test forward/inverse mapping and mouse/touch zoom and pan across a break.
- A viewport entirely inside a missing interval has no readings: return a stable empty plot, without fabricating points.
- Metadata has an additional bounded collection: enforce the limit and validate ordering and committed endpoints; never silently truncate.

## Migration Plan

No storage or archive migration. Derive endpoints from existing events, including imported and pruned recordings. Rollback changes presentation and metadata only.
