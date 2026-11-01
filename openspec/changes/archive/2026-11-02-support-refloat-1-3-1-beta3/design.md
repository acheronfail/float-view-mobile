# Design

## Context

Beta3 (`e6388f75e63919bc42a3e10cecb68824ac413ae4`) changes frequency/latency tracking and identity; the source audit found no schema, wire layout, alert or control change from beta2. The initial implementation duplicated fixtures and repeated version branches. Its source, raw checks and ExpressT restoration evidence are preserved locally.

## Decisions

- Keep exact beta2/beta3 runtime entries mapped to existing selectable telemetry, stable controls and timeout handling. Configuration fingerprints and exact-version Tune namespaces remain unchanged. No API, storage or host-binding change is needed.
- Reuse configuration fixture files through their existing manifest `files` references. The generator has one explicit reuse mapping and verifies the generated compressed schema, values and normalized expectations against the referenced fixture before sharing files. Source tag/commit and release identity remain beta3. This is fixture deduplication, never runtime identity aliasing or schema admission.
- Let the existing simulator codec catalogue own telemetry command, control family and timeout capability. Derive its version type, configuration version type and control profiles there; consumers select behavior by those capabilities. Keep production policy independent of simulator metadata so fixtures remain an independent oracle.
- Keep existing exhaustive beta2 profile tests, with a focused beta3 identity/profile/configuration-sharing regression and canonical beta3 scenario. Profile equality establishes reuse of the tested safety decisions; avoid reindenting or duplicating every existing test for each release label.
- When stable 1.3.1 is audited, replace active beta release registrations with stable support, remove beta-only scenarios/vectors and keep only fixture bytes still useful to the supported profile. Reused files may be renamed to stable/neutral ownership then. Preserve beta-created sessions, events, archives, provenance and Tunes as stored data without retaining beta runtime admission or enabling cross-version Tune apply.

## Verification and limits

Rerun generator equality checks, fixture/type tests, focused Rust admission/configuration/Tune tests and canonical beta2/beta3 scenarios. Runtime behavior is unchanged by fixture consolidation, so retain the completed stationary ExpressT checkpoint rather than repeat hardware mutations. The linked Apple/Android checks and physical result from the initial implementation remain evidence for the unchanged production behavior. Any discovered runtime change requires reassessing that coverage.

## Follow-up

Record these boundaries and cleanup criteria in the unfinished schema-admission proposal. Its later design must distinguish audited release behavior, schema write compatibility, exact active metadata and reusable test evidence without introducing another catalogue framework.
