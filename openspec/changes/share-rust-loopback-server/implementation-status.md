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

## Shared candidate (in progress)

Tasks 2.2 and 4.1 now have passing evidence. The closed versioned protocol,
process registry, listener, complete-state publisher, public dispatcher and
bounded binary archive exchange are implemented. Both existing binding families
now depend on the candidate. Native production still selects the original
servers; the new adapters are exercised only by focused tests.

Passing checks so far:

- 12 Rust exchange/transport tests, including listener restart/token continuity,
  failed bind, authentication before host admission, bounded transfers, malformed
  envelopes and large typed completion framing.
- C buffer overlap/extent/invalid handle/idempotent destruction tests and existing
  JNI null-environment/panic tests.
- Linked JVM publication, stale sequence, undersized buffer and private firmware
  identity tests (`ServerFactsTest`).
- Swift linked `SharedServerTests.canonicalCandidate` uses the existing SQLite,
  application service and software BLE fixture with canonical conformance.
- Candidate conformance also exercises chunked HTTP bounds, exact/+1 fragmented
  WebSocket bounds, authentication failures and subscriber-independent revisions.
- Warnings-denied focused Rust lint and unchanged mock conformance.

Pre-selection packaging passed after adding the shared read-only worker path:
macOS Release, iOS simulator Release/device Debug, Android four-ABI Debug/Release,
exact C/JNI symbols and linked JVM tests. Intel macOS/iOS target checks also pass.
Canonical candidate coverage includes a 15,000-sample archive larger than the
host envelope, identity-preserving import, abandoned uploads followed by successful
transfer, and continued saved recording across listener stop/restart under two seconds.
SQLite cancellation is sticky and isolated from another reader. Decoded response
boundary and gzip/read-admission recovery tests pass.

Native production still selects its original servers at this checkpoint.
Remaining gates include native selection/deletions, final source/package measurements,
physical ESP32 checkpoints and `just verify`. Android is currently unauthorized in
ADB; iOS and the ESP32 were discovered successfully.

## Apple selection

Apple production now selects the shared server through its process-owned thin
adapter. The complete Swift route/dispatch implementation, FlyingFox package
references and obsolete native history projection/read machinery were deleted.
Native catalog, archive validation, file presentation and pruning integration
remain. Shared connection phase encoding has a focused native regression test.

Passing evidence: all Swift tests, clean macOS Release/iOS simulator Release/iOS
device Debug packaging and canonical macOS/iOS conformance. Canonical native
runs now include transport authentication, chunking, fragmentation, multi-client
revision and archive checks. SQLite/catalog/prune host tests remain; historical
reader and cancellation checks exercise the selected Rust implementation.

## Android selection and lifecycle fixes

Android production selects the Rust server. Ktor, Kotlin route/projection code,
native historical reducer/readers and their obsolete private tests were removed.
The selected read-only Rust path owns historical cancellation; native catalogue,
archive and file UI integration remain. The original two binding libraries are
retained with exact-symbol audits. Clean four-ABI Debug/Release packaging, linked
JVM tests and instrumentation compilation passed. Canonical Android conformance
passes, including Activity replacement during saved recording and the same
session ID with advancing samples.

Regression fixes and coverage include Android's telemetry extension-field mapping,
redacted publication failure containment, two bounded readers, exact decoded
response limits, rejected history cursors before catalogue lookup and terminal
correlated errors for oversized host results. Responsive idle WebSockets survive
via heartbeats; peers ignoring pings are closed within the inactivity deadline.

iOS and macOS physical checkpoints passed in full; see physical-results.md.
Android physical coverage still requires USB-debugging authorization on the
connected phone. The final root software gate is running; no final gate or
physical Android pass is claimed yet.
