---
name: openspec-propose
description: Create an OpenSpec proposal and the artifacts needed to implement a described change.
license: MIT
---

Create concise, coherent artifacts for the requested outcome. Follow the
[OpenSpec workflow](../../../docs/development.md#openspec), including telemetry,
validation, draft and completion rules.

- Use the named or clearly identified change; check for existing work before
  creating a duplicate. Ask only if the intended scope is ambiguous.
- Create a new change with `OPENSPEC_TELEMETRY=0 openspec new change "<name>"`.
- Resolve schema and paths with
  `OPENSPEC_TELEMETRY=0 openspec status --change "<name>" --json`.
  Respect the selected store, `actionContext` and reported paths.
- Create missing artifacts in dependency order using
  `OPENSPEC_TELEMETRY=0 openspec instructions <artifact-id> --change "<name>" --json`.
  Read dependencies and follow its template, context and rules. For glob outputs,
  write concrete files under the reported directory, never a literal glob path.
  Reconcile existing decisions rather than overwriting them.
- Verify the full required artifact dependency graph using the workflow's
  planning-readiness rule before declaring the plan ready. Keep scope and non-goals
  explicit; do not add abstractions or adjacent capabilities just to fill a template.
- For an explicitly unfinished draft, create only the requested artifacts and
  label the proposal accordingly. Do not invent requirements or set `skip_specs`
  to silence validation of a behavioral draft.
- Validate an implementation-ready change. Report its location and readiness.
  Planning alone does not authorize implementation. If implementation was also
  requested, continue with `openspec-apply-change` without another confirmation.
