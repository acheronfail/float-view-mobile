# Design

## Context

See proposal.md for motivation. The shared SvelteKit hash router already supports nested pages, guarded native links and bounded scroll/focus restoration. Checkpoints deliberately represent Settings as one global route.

## Goals / Non-Goals

Use the existing frontend navigation and authoritative settings owner. No backend, native, checkpoint or storage schema changes.

## Decisions

- Use one bounded category catalogue and a dynamic category route, with the existing licences route retained. Unknown category slugs show a clear unavailable message and a Settings return. Real routes provide consistent headers and native back integration; a home made entirely of accordions would recreate the long screen.
- Extract existing controls into focused views and remove the superseded flat layout. Use the existing SegmentedControl for Off / On choices, with a small shared labelled presentation.
- Preserve draft validation, serialization and native audio availability in Announcements; relocate its markup and disclose alert-specific fields without changing policy. Retain shared controls at the end.
- Map subpages to the existing Settings checkpoint, preserving session-lifetime route context through the existing bounded NavigationContext. No new persistence requirement.

- Recording and announcement detail pages use flat canvas sections, full-width sticky SectionTitle headings and simple dividers; surrounding inset cards would interrupt heading alignment. Numeric fields share the same elevated input style.

- Permission disclosures preserve manual expansion across unchanged snapshots and choose a fresh default after relevant permission facts change. Permission actions continue through the existing backend owner; presentation never requests access automatically. Backup/restore, CSV preferences and clearing remain under the existing transfer owner, with CSV content inserted between backup and clear sections.

## Risks / Trade-offs

- Moving controls invalidates flat-page browser selectors → update affected scenarios and run focused category, save, transfer, permissions and audio coverage in Chromium and WebKit.
- Hiding disabled alert fields makes those edits require enabling the rule → retain stored values and expose related controls immediately on enable.
- Seven rows may scroll on small screens or larger text → use compact readable rows, allow normal scrolling and inspect narrow/large-text screenshots rather than shrinking targets.

## Migration Plan

No data migration. Deploy or revert shared frontend changes as a unit. Validate Svelte, formatting, focused unit/browser tests and screenshots before syncing and archiving.
