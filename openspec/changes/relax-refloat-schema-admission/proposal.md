# Proposal

## Why

Refloat configuration labels/help can change without changing serialized settings, but Float View still requires an exact full-schema fingerprint for every admitted release. The beta3 support rework separates exact audited release identity from reusable protocol behavior and fixture bytes; use that boundary to simplify configuration admission without weakening writes.

## What Changes

- Determine which validated schema properties must gate reads/writes and which descriptive metadata should identify the active schema and cache without blocking compatible behavior. Preserve legacy 1.0.0/1.0.1 patch resolution.
- Keep audited release admission separate from schema compatibility, active device identity and test-fixture ownership. Beta3 now reuses beta2 fixture files through an explicit generator mapping that verifies every generated artifact; this does not alias runtime package versions or relax fingerprints.
- Reuse existing protocol/control capabilities instead of adding release-name branches. Keep conformance evidence independent of production policy; do not generate both sides from one runtime catalogue merely to eliminate a few explicit registrations.
- Investigate removing redundant runtime fingerprint catalogues and frontend version eligibility lists where validated schema compatibility and authoritative backend availability can replace them. Introduce no additional catalogue, migration or general generation framework without a demonstrated need.
- Preserve exact active schema metadata, bounded parsing, signed values, cache validation, mutation safety and mandatory complete write readback. A matching wire signature alone must not establish write compatibility; encoding, order, constraints and semantic changes require review.
- Preserve supported configurations, drafts, Tunes, sessions and archives. Keep exact-version Tune isolation separate from fixture/schema reuse; cross-version application is outside this change.
- When stable 1.3.1 is audited, remove beta2/beta3 runtime admission, frontend/simulator registrations and beta-only scenarios/vectors. Retain only shared fixture evidence needed by the stable profile, promoting its ownership/name if necessary. Preserve beta-created histories, events, provenance and Tunes without keeping beta firmware admission or adding automatic Tune conversion. Historical-data regressions may retain beta identities; storage/archive readers must remain independent of the active firmware catalogue, with reopen/import coverage after beta registration removal.
- Aim for a future audited release with unchanged behavior to require a small release registration and source-evidence update, focused differential regression and any required scoped interoperability checkpoint, with no copied payloads or repeated decoder/safety tests per label.
- **Non-goals:** automatic admission of unreviewed releases; implementing this policy in beta3 support; cross-version Tune migration/application; relaxing storage compatibility or write safety.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-configuration`: Reconsider full-fingerprint admission for audited releases while preserving validated schema/write compatibility and exact active identity.

## Impact

Likely affects the shared Rust configuration codec/service, existing release capability owners, frontend eligibility presentation and conformance fixture generation. The beta3 implementation already centralizes simulator capabilities in its codec catalogue and verifies fixture reuse; build on those mechanisms rather than introducing parallel policy. The later design must determine whether authoritative frontend availability needs a contract change. No dependency or public-contract/persistence change is selected here.

This change remains intentionally unfinished. Its design, delta specs and tasks still require a separate schema-safety review. The completed beta3 rework informs that design; it does not authorize or implement schema-admission relaxation.
