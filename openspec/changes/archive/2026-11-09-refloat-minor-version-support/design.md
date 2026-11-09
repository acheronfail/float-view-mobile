# Design

## Context

See proposal.md for motivation. Both hosts already share the Rust compatibility and configuration owners. A separate unimplemented relax-refloat-schema-admission change explores unknown-minor read fallback and cosmetic schema profiles; neither is required here.

## Goals / Non-Goals

Use the supplied upstream stable-minor guarantee without inferring compatibility for future minors or prereleases. Keep packet validation, per-patch added capabilities, observed provenance and saved Tune namespaces.

## Decisions

- Retain the source-release catalog as evidence, but resolve stable profiles by supported minor. 1.3.0 lacks IMU-timeout handling; 1.3.1+ and exact known betas retain it. Canonical bounded parsing accepts supported stable patches, rejects aliases/leading zeroes/ranges and retains numeric patch suffix normalization.
- Resolve stable configuration through audited schemas in the same minor. Do not weaken fingerprint/signature validation; full schema changes remain feature-local failures. Own the observed package-version string rather than aliasing to the evidence entry. Legacy 1.0 still needs its historical fingerprint-selected source version because it lacks a patch in INFO.
- Publish stable minor ranges and exact beta exceptions through the existing supportedVersions field; no schema or native ABI change. Keep frontend eligibility aligned and correct the blanket prerelease rejection text. Keep exact saved Tune matching as a conservative independent product policy.
- Synthetic later-patch fixtures reuse pinned source bytes and explicitly test the policy, without claiming newly released firmware interoperability. Focused direct/CAN composition tests establish both host consumers through shared ownership.

## Risks / Trade-offs

An actual metadata edit within a patch can still fail fingerprint admission: cosmetic schema relaxation remains separate. The upstream statement guarantees configuration compatibility, so all existing bounded wire checks remain required and added optional capabilities retain their known patch thresholds.

## Migration Plan

No storage rewrite or migration. Reverting the app restores prior admission without modifying supported saved data. Reconcile the separate fallback plan with this policy before its future implementation.
