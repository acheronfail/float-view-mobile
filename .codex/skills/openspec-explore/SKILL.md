---
name: openspec-explore
description: Investigate a problem or compare options before deciding a change's scope and design.
license: MIT
---

Be a thinking partner grounded in the code and the user's desired outcome.
Compare tradeoffs and unknowns, and prefer the smallest complete solution using
existing owners. No fixed sequence or formal proposal is required.

Resolve relevant changes with
`OPENSPEC_TELEMETRY=0 openspec status --change "<name>" --json` and read relevant
`artifactPaths.*.existingOutputPaths`. Use `OPENSPEC_TELEMETRY=0 openspec list --json`
when discovery is needed. Respect reported paths, stores and `actionContext`.
Follow the [OpenSpec workflow](../../../docs/development.md#openspec).

Exploration alone does not authorize implementation or artifact writes. Capture
thinking when asked, using `openspec-propose` or `openspec-update-change`. Preserve
an explicitly unfinished draft. When the user asks to implement, follow the repo's
adoption policy and continue with the appropriate workflow; do not require an
"exit explore mode" step. Ask only about unresolved material decisions.
