---
name: openspec-archive-change
description: Verify completion, sync outstanding delta specs and archive an implemented OpenSpec change.
license: MIT
---

Complete local closeout under the repo's
[completion and validation rules](../../../docs/development.md#openspec).

- Use the named or clearly identified change. List changes and ask only if ambiguous.
- Run `OPENSPEC_TELEMETRY=0 openspec status --change "<name>" --json`. Read artifacts,
  tasks and verification evidence; respect the selected store and `actionContext`.
  Planning readiness is not implementation completion.
- Confirm implementation and required checks are complete. Finish authorized work
  or report the specific blocker and leave the change open. Cancellation is a
  separate disposition, not completed implementation; do not archive unfinished
  work merely to close it.
- Compare deltas and canonical specs. Use `openspec-sync-specs` to apply outstanding
  changes directly, without delegation or further confirmation. Already-synced
  content is a no-op. Canonical specs must match implementation before archival.
- Validate the change and affected canonical specs. Then run
  `OPENSPEC_TELEMETRY=0 openspec archive "<name>" --yes --skip-specs` to move the
  already-synced change without applying deltas twice. The flag skips the CLI merge,
  not the sync or validation above. Never use `--no-validate`. Retain `--store <id>`
  when a store is selected.
- Inspect the archive destination and tasks. Never overwrite a dated archive.
  Report its location and verification limits accurately.

No commit or publication is implied. Do not prompt again for requested local
implementation or sync-and-archive work.
