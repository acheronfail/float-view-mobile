# Design

## Context

See proposal.md for scope. Stable tag `v1.3.1` resolves to `a2b55d96f508653b0436b21410100aaf91f14b04` (exact pin recorded in verification). The diff from beta3 contains only `version` and GitHub issue templates; all application source and configuration bytes are identical.

## Decisions

Add an exact stable catalogue entry using the existing selectable command-33, command-35 alert/IMU-timeout and stable control profiles. Require the same audited full schema fingerprint and signature. Do not infer compatibility from a version range. The unchanged upstream Makefile emits numeric suffix `1` for stable builds; the existing bounded patch-suffix rule normalizes it to `1.3.1`, and tests cover both numeric and empty forms.

The generator independently regenerates stable artifacts and checks equality before sharing beta2 fixture paths. Preserve stable source tag/commit in the manifest and observed identity in sessions, caches and configuration drafts. Only Tune identity maps exact beta2, beta3 and stable to `1.3.1`; preserve fingerprint/signature checks. New Tunes store the stable family. Existing beta-labelled rows remain intact and are normalized on read and matched by bounded SQL aliases, including pagination; no data migration or format change is needed. Apply validates the Tune family against the active exact schema and submits the active board version to the existing configuration writer. Frontend and mock use the same closed family semantics without relaxing draft/editor identity. Unknown beta labels and other releases remain separate.

Retain beta2 and beta3 runtime entries and tests, following the newer active compatibility design rather than the historical beta3 retirement note. Add stable to the existing focused canonical release scenario instead of duplicating its assertions.

## Risks and verification

Identity admission can accidentally omit frontend eligibility or configuration policy; focused Rust, frontend, fixture and direct/CAN canonical checks cover these consumers. Source equality permits reuse of existing protocol vectors and the beta3 physical checkpoint for unchanged wire behavior. The identical beta3/stable source and prior beta3 physical checkpoint establish unchanged firmware interoperability; no repeated physical checkpoint is required. Tune-family matching changes application policy, established by software tests; it does not change firmware writes or host behavior.

## Migration

No persistence or native boundary changes. Stored beta Tunes remain readable without rewriting their IDs, revisions, values or dates. Exported/imported supported archives retain their bytes and receive the same Tune-family matching on read.
