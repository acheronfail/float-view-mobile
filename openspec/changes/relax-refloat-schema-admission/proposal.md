# Proposal

## Why

Refloat marketing releases are too coarse a compatibility boundary. Beta3 reused beta2's protocols and fixtures, yet adding it still required exact release admission; removing those registrations after stable would strand otherwise working boards. Cosmetic schema edits also invalidate configuration despite leaving its behavior unchanged. Reuse verified behavior, admit compatible public reads independently, and disable only features whose semantics are unproven.

## What Changes

- Keep existing audited release mappings, including beta2/beta3, with their current capabilities. A stable release does not revoke working beta support. Remove duplicate implementations and evidence when they add no value; retain mappings needed by supported boards.
- Add a bounded, read-only fallback for unknown Refloat versions on already supported VESC controllers. Require valid Refloat INFO and a successfully validated public command-33 core telemetry exchange before readiness or session activation. Preserve the observed version; do not guess compatibility from a version range or alias it to a known release.
- Let usable telemetry and app-side recording survive unavailable optional fields, alert history or configuration. Unproven status is unknown, never an inferred healthy/clear state. Unknown releases receive no device mutation authority, even if their responses and schema match an audited release.
- Replace the configuration service's non-legacy exact-version/full-fingerprint catalogue with shared schema profiles. After full bounded XML validation, ignore only parameter label/help contents for admission; retain actual full fingerprints for schema identity, drafts and exact-version Tunes. Unknown releases matching a known schema profile may read configuration; only audited operation semantics can authorize writes.
- Publish a small authoritative live compatibility summary and riding-Tune capability. Use existing configuration/control availability for individual features and remove frontend version eligibility duplication. Show limited compatibility without blocking connecting or recording.
- Extend existing independent fixtures and focused conformance for unknown versions, isolated feature failure, zero mutation traffic, exact identity and preserved beta access. Reuse fixtures rather than copying release-sized datasets.
- **Non-goals:** generic writes to unknown firmware; undocumented command probing; inventing upstream capability bits; supporting unknown VESC versions; cross-version Tune application; persistence/archive migrations; promising arbitrary future firmware interoperability.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vesc-device-connection`: Distinguish audited support, validated limited support and unusable protocols; retain actionable recovery and existing VESC admission.
- `refloat-telemetry-pipeline`: Validate a bounded public read fallback without treating optional or unknown status as healthy evidence.
- `refloat-configuration`: Admit shared cosmetic schema profiles independently of observed version while retaining exact identities and restricted writes.
- `refloat-live-controls`: Deny every device mutation in limited mode at the authoritative backend boundary.
- `backend-contract`: Publish minimal live support and feature availability; drive frontend eligibility from backend facts.
- `backend-conformance`: Prove future-identity fallback, feature isolation, preserved audited behavior and the schema boundary with independent evidence.

## Impact

Shared Rust discovery, compatibility, polling, configuration and mutation owners; the hand-written backend contract, client/mock, frontend connection/configuration presentation and both host integrations; existing fixture and conformance tooling. No new dependency, native ABI entry point, storage marker or archive format is planned. Actual schema/version identity becomes owned observed data rather than static catalogue references.

This plan supersedes the earlier exact-release-only scope and proposed beta retirement. [design.md](design.md) defines the trust boundary and [tasks.md](tasks.md) records implementation and validation work. Leave the change open for review; this planning request does not authorize implementation, commits or online changes.
