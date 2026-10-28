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
