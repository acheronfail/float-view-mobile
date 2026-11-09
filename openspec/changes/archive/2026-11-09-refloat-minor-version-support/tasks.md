# Tasks

## 1. Stable minor policy

- [x] 1.1 Implement bounded stable-minor compatibility and exact prerelease exceptions; verify all u8 patches, numeric suffixes, unknown minors, malformed identities and unchanged profile thresholds with focused Rust tests.
- [x] 1.2 Admit known schemas within stable minors while preserving observed versions and existing legacy/Tune identity; verify fresh/cache paths, writes and incompatible-schema rejection in configuration and composition tests.

## 2. Consumer behavior

- [x] 2.1 Publish minor support ranges, correct prerelease guidance and align riding/battery eligibility; verify projection, frontend/mock and browser coverage with exact reported identities.
- [x] 2.2 Exercise synthetic later-patch direct/CAN paths and existing prereleases through shared composition/conformance; run affected Rust/frontend checks, formatting and Clippy and record results.

## Workflow follow-up

- Sync canonical specs, reconcile the separate fallback proposal with stable-minor admission, validate and archive after required verification.

## Verification evidence

- Shared Rust behavior suites: 290 tests passed across compatibility, configuration, discovery, mutation, application coordination, backend service and Apple FFI/Android JNI boundaries. Follow-up current composition (18) and battery (10) tests passed after their final assertions were added. All selected doc tests passed on a serial rerun after concurrent build artifacts caused an initial missing-crate error.
- Focused compatibility/configuration suites passed, including all 1,024 stable minor/patch identities, numeric suffix normalization, fresh/cache schema paths, exact observed identity, unknown-schema rejection and guarded write/readback for synthetic 1.3.2.
- Warnings-denied Clippy for the changed Rust owners/consumers and all targets passed; cargo formatting and git diff whitespace checks passed.
- Frontend focused tests: 72 passed; Svelte check: zero errors/warnings; production build passed. Conformance TypeScript check and 11 focused configuration/Tune fixture tests passed.
- Direct conformance passed (revision 405); mock conformance passed (revision 86). Compatibility dialog recovery passed in Chromium and WebKit, including touch, non-primary pointer rejection, keyboard, reload and bounded viewport checks. Loopback/browser checks ran outside the sandbox.
- New stable-patch identities reuse pinned wire/schema evidence. These tests establish policy and software behavior, not interoperability with hypothetical future firmware. Schema fingerprints/signatures and exact saved-Tune namespaces remain enforced; cosmetic schema admission stays in the separate unimplemented proposal.
- Backend ownership guidance now links to canonical compatibility policy and no longer instructs beta retirement. The separate fallback plan now treats supported stable minors as fully supported.
- Closeout: canonical specs synced and change archived on 2026-11-09. `just spec-check` passed all 134 items. The two affected canonical specs retain exactly their prior 14 length warnings; new change validation has no issues. Final documentation formatting and whitespace checks passed.

## Maintenance review follow-up

- Removed catalog-order dependence and the unnecessary checked-patch panic from profile selection; the highest applicable source patch is selected explicitly. Exhaustive stable-patch coverage now independently checks the 1.3.1 IMU-timeout boundary.
- Removed an unnecessary owned-version clone from Tune write submission.
- Beta suggestions now require a beta-labelled reported version, rather than any suffix. Browser coverage confirms stable versions, numeric Makefile suffixes and release candidates hide beta suggestions, including transitions from beta failures.
- Revalidation passed: 30 compatibility/configuration Rust tests, 18 direct/CAN composition tests, 72 frontend tests, Svelte check, frontend build, conformance TypeScript check, Chromium/WebKit recovery tests, warnings-denied Clippy for changed Rust owners and consumers, formatting and whitespace checks.
- Full-schema fingerprint/signature admission remains an explicit limitation; harmless metadata changes within future patches can still disable configuration. The separate schema-admission proposal owns relaxation of that boundary.
