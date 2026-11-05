# Design

## Context

See proposal.md. Document scrolling is shared by ordinary routes; Live has internal scroll regions. SessionList currently owns all pagination state and restarts on mount or catalog revision changes. Its date-group index must retain reactive group references so later pages appended to an existing date render correctly. Header and native back use parent URLs rather than browser history.

## Goals / Non-Goals

Retain browsing context without changing the meaning of parent navigation or overlay dismissal. Preserve existing frontend-recreation checkpoint exclusions. No backend or durable cache.

## Decisions

- A shell-owned frontend context captures document offsets, visible list link anchors and focused content links before navigation. It restores after navigation and list readiness, using anchors ahead of raw offsets. Parent links keep their existing safe fallback URLs; restoration applies equally to fresh parent navigation and history traversal.
- Bound retained route positions to 32 entries and session pages to 8 boards / 4,000 summaries per board. Eviction uses normal initial presentation. Bounds live in source. Cache only successful pages and the intended restoration extent, keyed by board and catalog revision; stale catalogs reload from the first cursor through the retained extent before restoration. Cache data is never authoritative for mutations.
- SessionList exposes loading readiness to the shell through a DOM attribute. The shell observes content changes until ready, with a finite timeout and cancellation on navigation or user input. Retained destinations hide their content until restoration is applied; failures reveal normal retry controls.
- Capture occurs only after navigation guards accept the transition. Existing shared navigation activation and native back semantics stay intact. No history-back substitution that might leave the app on direct entry.

## Risks / Trade-offs

- Catalog mutations require refetching a previously loaded extent → bounded to retained summaries; preserve error and conflict handling.
- Removed anchors or changed geometry → retain several nearby anchors and fall back to a clamped offset.
- Delayed loading or user interaction → finite wait, cancel callbacks and prevent late scroll writes.
- Very large or evicted lists lose cached extent → ordinary presentation remains available; no unlimited memory retention.

## Migration Plan

Frontend-only rollout; reverting removes only ephemeral state. No data migration.
