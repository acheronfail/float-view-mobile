# Implementation evidence

## Foundation slice

Completed tasks: 1.1, 1.2, 1.3, 1.4, 2.1.

- baseline.md inventories routes, all 34 methods, four history kinds, native
  helpers/application calls, public limits, source/deletion targets and measured
  package artifacts with provenance.
- dependencies.md records exact package choices, added lockfile entries,
  licenses and all nine successful target builds.
- bounds.md assigns numeric resource ceilings and boundary coverage. Assigned
  tests that have not been implemented are explicitly not passing evidence.
- loopback-server is an unselected safe Rust workspace member. Neither native
  binding nor any application/domain crate depends on it yet.
- exchange.rs implements count-bounded queued/in-flight/completed operations,
  process-unique generations, exactly-once completion consumption, bounded
  condition-variable waits, client detachment and idempotent destruction.

Passing checks:

- `just apple-packaging` on the original production servers: macOS Release,
  iOS simulator Release and iOS device Debug; restored macOS Debug archive.
- Locked offline candidate/dependency builds on all nine targets listed in
  dependencies.md.
- `cargo test --locked --offline --workspace`, including seven exchange tests.
- `cargo clippy --locked --offline --workspace --all-targets -- -D warnings`.
- `cargo fmt --all` followed by formatting verification.
- `OPENSPEC_TELEMETRY=0 openspec validate share-rust-loopback-server --strict`.
- Existing convergence audit and `git diff --check`.

## Still incomplete

Task 2.2 remains unchecked: the exchange is a generic lifecycle primitive, not
the versioned typed native protocol. Payload byte validation and the closed
operation/completion/fact records still need implementation. Registry/buffer and
panic-boundary integration, transfer storage cleanup and worker integration are
not implemented. No public routes, serializers or state publisher have moved.
Both native production servers and their dependencies remain selected.

The final `just verify`, shared-server canonical conformance, linked production
packaging, final source/size reduction and physical checkpoint have not passed.
Do not archive or interpret the baseline packaging as migration verification.
