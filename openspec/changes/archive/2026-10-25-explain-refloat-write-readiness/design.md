# Design

## Context

See proposal.md for motivation. Configuration service summaries expose capability-level writable state; the mutation coordinator separately authorizes motion, package state/mode, and exclusive ownership. Apple and Android use one shared public projection. The editor already has a fixed action footer and a modal error region.

## Goals / Non-Goals

**Goals:** Report shared authoritative admission blockers and keep both readiness and attempted-write outcomes visible where users act.

**Non-Goals:** Change device-write or Tune permissions, persist UI errors, infer firmware behavior, retry mutations automatically, or introduce dependencies/storage changes.

## Decisions

- Extract mutation admission checks into one query also used by authorization. Compose that result with connection/schema/current state in the application owner and project a nullable closed `writeBlockReason` field. Keep existing capability-level writable semantics to avoid restricting supported riding Tune application.
- Prioritize connection/schema availability, then mutation safety/exclusivity, then current-value readiness. Unknown facts get an explicit unknown-state reason, not an invented safety diagnosis.
- Share one frontend mapping from reason to action-oriented text. Move the scrolling warning to the fixed footer and pass the same guidance into the confirmation. Local source, validation, incompatible-draft, and stale-base blockers also explain disabled writing without pretending to be backend facts.
- Retain attempted-write errors through modal dismissal/reopening and draft editing; clear them on a successful write or explicit read. Read buttons keep their existing deliberate replacement semantics. No automatic reads discard edits.
- Extend mock, client snapshot validation, and canonical conformance together. Both native hosts consume shared serialized facts through existing bindings; verify their shared binding consumers without adding host policy.

## Risks / Trade-offs

- A single reason hides simultaneous blockers → deterministic precedence, updated on complete snapshots as each clears.
- Footer feedback reduces field space on small screens → compact text, bounded scrolling feedback when needed, browser checks at short mobile sizes.
- Capability writable differs from edited-write eligibility → document field semantics and test riding Tune behavior separately.

## Migration Plan

Ship contract and producers/consumers together in the embedded app. No persistent schema, archive, or native ABI layout migration. Revert the additive field and UI together if needed.
