# Shared-service acceptance evidence

All required software and physical acceptance checks pass. No coverage was deferred.

## Android selection and final bridge removal

Both hosts now use the process-owned Rust service. The native portable stores,
application policies, domain dispatch/projection and old runtime/transfer binding
entry points are deleted. The shared server has no native completion/publication
fallback. Android external imports stream through authenticated loopback HTTP;
exports retain a service-issued lease until the scoped destination copy closes
or the native picker is dismissed. Neither native host imports catalog SQL.

Android canonical software conformance passed at revision 691, including Activity
recreation. Linked JVM tests and instrumented boundary tests passed, including physical
coverage at the supported API-29 minimum.
Clean four-ABI Debug/Release packaging and the complete `just verify` gate
passed after the final fixes. Detailed environment/setup history is retained locally.

Two Android platform constraints were found by running production bindings:

- Rust 1.93 `File::try_lock` is unsupported on Android. Root ownership now uses
  an exclusive transaction in a dedicated SQLite lock file through the existing
  bundled dependency. Reader/export leases retain the lock connection. Same-
  process lease and independent-process exclusion tests pass.
- Android SELinux denies hard links in application-private storage. Session
  creation now exclusively reserves the destination and atomically renames a
  fully initialized temporary database over that reservation. Existing targets
  remain protected. Startup removes only precisely named interrupted temporary
  sessions and sidecars; a regression verifies unrelated files survive.

The old Android-framework `android_metadata` compatibility exception is removed.
The Rust producer uses the unchanged canonical SQL schema. No application release
or user-data migration is supported by this pre-release change.

## Physical checkpoints


Production BLE fixture acceptance passed on macOS, Android and iOS, including
guest telemetry, native export presentation/cancellation, reconnect and applicable
background persistence with new mobile location samples. Final publication
regressions passed on all hosts; no required physical coverage was deferred.
Detailed device, run and presentation reports are retained locally under
`.local/physical-testing/`.

The export result waits for correlated native acceptance before reporting
`presented`; rejection remains `staged`. Acceptance and presentation lifetimes
are bounded and stale/duplicate replies have software coverage.

## Ownership and invariant audit

The ownership audit passes with 28 workspace members, exactly two binding
families and zero temporary binding crates. The 105 external locked Rust
packages (name, version, source and checksum) are identical to the baseline.
`frontend/src/lib/backend/contract.ts`, `backends/sql/catalog.sql` and
`backends/sql/session.sql` retain their baseline hashes. The Apple package removes
its native CSQLite target; Android removes native SQL asset packaging. No new
external package or generated API was added.

Final measurements are in `final-measurements.json`: 42,916 production lines
(7,006 fewer than the 49,922-line baseline), with Debug/Release APK sizes of
52,427,719 / 28,539,593 bytes. Both APKs are smaller than the recorded baseline.
The measurement file records artifact hashes and the limits of the comparison.

Changed Markdown/TypeScript files pass Prettier; Swift formatting, Rust
formatting/warnings-denied Clippy, strict OpenSpec validation and
`git diff --check` pass. Repository-wide optional `just lint` still reports four
unchanged frontend ESLint errors (one `prefer-const`, three unkeyed Svelte each
blocks). Repository-wide Prettier also reports pre-existing unformatted files,
mostly historical specs. These are outside this backend change; all changed
files pass their applicable formatting checks.

The current software-gate log is `/tmp/float-view-shared-verify.log`.

## Final software gate

`just verify` exited successfully with canonical conformance on mock (revision
31), direct Rust (352), macOS (341), iOS simulator (359) and Android (368).
The final audit passes. Clean Apple packaging additionally passed after
an intermittent revision race was fixed: unchanged periodic Bluetooth/location
capabilities, storage capacity observations, stale timers and unchanged discovery
ticks no longer invent public revisions. Regression tests verify complete
snapshot stability and that actual scan expiry still publishes. Android and
macOS physical guest/recording/presentation reruns pass this final behavior;
the iPhone rerun also passes, including native-sheet cleanup and persisted background
location. Warnings-denied Rust lint passes on the final fix.
