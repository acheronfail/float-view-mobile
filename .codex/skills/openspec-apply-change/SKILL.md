---
name: openspec-apply-change
description: Implement or continue an OpenSpec change through verification, spec sync and archival.
license: MIT
---

Implement through the repo's
[completion and validation rules](../../../docs/development.md#openspec).

- Use the named or clearly identified change. Otherwise run
  `OPENSPEC_TELEMETRY=0 openspec list --json`; select the only applicable change
  or ask if multiple scopes remain plausible. Announce the name.
- Run `OPENSPEC_TELEMETRY=0 openspec status --change "<name>" --json`, then
  `OPENSPEC_TELEMETRY=0 openspec instructions apply --change "<name>" --json`.
  Respect the returned schema, store, paths and `actionContext`. Read the returned
  `contextFiles` before editing implementation. Check the full required artifact
  dependency graph using the workflow's planning-readiness rule, even if apply
  instructions report `ready` or `all_done`.
- If required artifacts are missing, create them in dependency order with
  `OPENSPEC_TELEMETRY=0 openspec instructions <artifact-id> --change "<name>" --json`.
  Continue within scope; ask only about missing material decisions. Do not implement
  an explicitly unfinished draft without authorization to develop it further.
- Implement pending tasks, checking them off as work is verified. Reconcile
  artifacts with discoveries; routine corrections do not need a separate invocation.
- Run relevant checks for changed behavior and affected consumers. Fix failures
  caused by the change and rerun affected checks. Checked boxes and `all_done` do
  not prove verification passed; inspect current evidence and the diff.
- Once implementation and required verification are complete, use
  `openspec-sync-specs` and `openspec-archive-change` to finish locally unless asked
  to leave the change open. Do not ask the user to request closeout again.
- If required hardware, information or another external dependency is unavailable,
  keep the change open and record the specific outstanding check. Finish independent
  authorized work and report the actual blocker and next action.

Report the behavior changed, checks and limits, and archive location or specific
remaining work. No commit, push or online mutation is implied.
