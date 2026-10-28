# Verification

Verified on 2026-10-28 against this change's working tree.

- `cargo test --locked --offline -p float-view-backend-service -p float-view-loopback-server`: passed, 151 tests, including six new flywheel policy tests. The first sandboxed run could not bind localhost listeners; the complete rerun outside the sandbox passed.
- `cargo clippy --locked --offline -p float-view-backend-service -p float-view-loopback-server --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `OPENSPEC_TELEMETRY=0 node <installed-openspec>/bin/openspec.js validate suppress-flywheel-ride-announcements --strict`: passed using the existing cached OpenSpec installation.
- `git diff --check`: passed.

Shared policy coverage verifies cancellation of each automatic phrase type,
sustained exclusion across sample/timer processing, absent/unknown mode handling,
fresh exit qualification, cooldown preservation, stale audio callbacks, connection
invalidation, manual previews and profile-derived visual over-voltage evidence.
No public wire contract or native host adapter changed.

## Session footpad overlay extension

Verified on 2026-10-28 against the follow-up working tree:

- `cd frontend && npm test -- src/lib/features/sessions/footpad-faults.spec.ts`: 10 tests passed, including six added flywheel cases covering each pad state, mode ordering, sparse retained samples and pagination.
- `cd frontend && npm run check`: no errors or warnings.
- ESLint for the changed detector and unit-test files, and Prettier checks for those files and `conformance/test/footpad-faults.browser.ts`: passed.
- `cd frontend && npm run build`: passed.
- `cd conformance && npx tsc --noEmit`: passed.
- `cd conformance && node --strip-types --test test/footpad-faults.browser.ts`: Chromium and WebKit both passed outside the sandbox, including flywheel exclusion for both overlays, normal marker selection and persisted toggles.
- Strict OpenSpec validation and `git diff --check`: passed.

This extension changes only shared frontend review derivation and its tests/docs;
it does not change recorded history, public contracts, native adapters or the
already verified shared Rust announcement policy.
