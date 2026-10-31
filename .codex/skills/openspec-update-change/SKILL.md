---
name: openspec-update-change
description: Revise an existing OpenSpec plan and reconcile its artifacts without editing implementation code.
license: MIT
---

Apply the requested revision and keep the plan coherent. Follow the
[OpenSpec workflow](../../../docs/development.md#openspec).

- Use the named or clearly identified change. If ambiguous, run
  `OPENSPEC_TELEMETRY=0 openspec list --json` and ask which is intended.
- Run `OPENSPEC_TELEMETRY=0 openspec status --change "<name>" --json`. Read existing
  artifacts from `artifactPaths.*.existingOutputPaths`; respect the schema, store,
  paths and `actionContext`.
- Reconcile scope, requirements, design and tasks in both directions. The request
  authorizes necessary local revisions; do not seek confirmation per artifact.
  Ask only about unresolved material decisions.
- Create missing artifacts needed for the revision with
  `OPENSPEC_TELEMETRY=0 openspec instructions <artifact-id> --change "<name>" --json`
  in dependency order. Never write a literal glob path. Preserve an explicitly
  unfinished draft rather than completing it speculatively.
- Validate ready artifacts under the repo's rules. Identify specific new or changed
  work rather than indiscriminately resetting completed tasks. Report revisions,
  missing decisions and any implementation delta.

This skill edits planning only. If implementation was also requested, continue
with `openspec-apply-change` after reconciling the plan; otherwise do not edit code.
