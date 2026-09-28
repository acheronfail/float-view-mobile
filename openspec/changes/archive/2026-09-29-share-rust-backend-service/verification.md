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
recreation. Linked JVM tests and both instrumented boundary tests passed on the
connected Samsung SM-G965F running Android 10 (the supported API-29 minimum).
Clean four-ABI Debug/Release packaging and the complete `just verify` gate
passed after the final fixes. The dedicated Android emulator needed its empty
test partition recreated at 8 GiB after APK installation ran out of space.

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

- macOS: the original 16-scenario matrix passed using production CoreBluetooth
  and the ESP32. This covers identity, direct and supported older firmware wire
  formats, direct/CAN configuration and controls, shutdown/no-op, CAN selection,
  ambiguity, BMS, recording and reconnect. Added guest and ride/presentation
  checkpoints also passed on the final presentation protocol. The native CSV
  Save panel was inspected and cancelled through computer use; the harness
  verified that its staged file lease was then removed. Evidence:
  `/tmp/float-view-shared-macos-physical.json` and
  `/tmp/float-view-shared-macos-presentation.json`, with matching logs.
- Android: all 18 scenarios passed on the connected Android 10 Samsung. The
  background checkpoint recorded location rows and committed sequence 0 → 1
  with authorized/available background capture. The scoped save picker appeared
  and dismissed in both ride and background scenarios. After adding the explicit
  presentation acceptance reply, the affected ride/background scenarios passed
  again. Evidence: `/tmp/float-view-shared-android-physical.json` and
  `/tmp/float-view-shared-android-presentation.json`, with matching logs.
- iOS: all 18 scenarios passed after renewing provisioning and trusting the
  developer profile. Native file presentation and cancellation passed in ride
  and background scenarios. Authorized background capture persisted location
  rows and committed sequence 0 → 2. Evidence: `evidence/ios-physical.json`;
  the matching diagnostic log is `/tmp/float-view-shared-ios-physical.log`.

Physical conformance explicitly checks guest telemetry without catalog/session
creation, native file presentation and dismissal, and a persisted location
observation during background dwell. A zero-observation run no longer silently
claims background-location acceptance. No physical coverage has been deferred by the user. The final no-op publication
fix also passed guest/ride/background reruns on both phones and guest/ride
on macOS; see `evidence/*-publication.json`.

The shared export result now waits for a correlated native acceptance reply
before reporting `presented`; rejection reports `staged`. Pending acceptance is
bounded to 30 seconds, accepted presentation to 24 hours. Duplicate, foreign,
expired and late replies are covered. Private file-UI completion does not advance
public snapshot revisions. Swift host conformance includes duplicate native
completion, and the publication regression covers cleanup between state reads.

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
the iPhone rerun also passes, including native-sheet cleanup and background
location sequence 0 → 2. Warnings-denied Rust lint passes on the final fix.
