---
name: openspec-sync-specs
description: Merge delta specs into canonical specs, leaving the change open for a sync-only request.
license: MIT
---

Merge delta intent without losing unrelated requirements. Follow the
[OpenSpec workflow](../../../docs/development.md#openspec).

- Use the named or clearly identified change. List changes and ask only if ambiguous.
- Run `OPENSPEC_TELEMETRY=0 openspec status --change "<name>" --json`. Respect
  `actionContext`, the store and reported paths. For the spec-driven schema, read
  `artifactPaths.specs.existingOutputPaths` and matching canonical files under
  `<planningHome.root>/openspec/specs/`.
- Read both sides. Apply ADDED, MODIFIED, REMOVED and RENAMED requirements while
  preserving existing scenarios not superseded by the delta. Partial modifications
  are intentional; never replace a whole requirement with a partial delta.
- Already-applied requirements, scenarios and renames are no-ops. Create new
  capabilities with meaningful Purpose and Requirements sections when needed.
  Reconcile intervening changes within scope or ask about material conflicts;
  never silently discard either side or blindly append duplicate scenarios.
- Validate the change and affected canonical specs under the repo's rules. Review
  the diff for unintended deletion or duplication and report capabilities updated.
- A sync-only request leaves the change active. Continue to archival when part of
  the requested implementation or closeout.

If there are no deltas, distinguish a spec-free change from an unfinished
behavioral draft. Do not describe the latter as successfully synced.
