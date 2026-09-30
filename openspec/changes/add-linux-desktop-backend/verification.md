# Linux implementation verification — 2026-09-30

The implementation is available, but acceptance is incomplete. Do not sync or archive this change yet. Checked tasks have evidence below; unchecked tasks include unverified behavior even where implementation exists.

## Environment

- x86_64 TUXEDO OS 24.04; kernel 7.0.12-111030-tuxedo-dcn21fix1.
- Rust 1.93.0, Node 24.15.0, GTK 4.14.5, WebKitGTK 2.52.6, BlueZ 5.72.
- Bluetooth USB vendor/product 8087:0029; ESP32 fixture at /dev/ttyACM0.
- Fixture firmware was already installed. Its exact flashed revision was not established; repository firmware was inspected without reflashing it.

## Software results

Passed:

- just build-linux, just run linux (isolated startup/shutdown), just test linux (eight tests; desktop test ignored by default), Linux locked/offline all-target/all-feature Clippy with warnings denied, and Rust formatting.
- Standalone execution of the frontend embed build script without an HTML artifact failed with its actionable diagnostic.
- Shared cargo tests for backend-service, loopback-server, application-runtime-host, application-runtime-apple-ffi and application-runtime-android-jni: 63 tests including shared transport conformance and ABI checks. Matching warnings-denied Clippy passed.
- npm run direct: canonical conformance, revision 413.
- npm run linux: canonical software conformance, latest revision 431, with isolated storage and the explicit software feature/argument.
- Frontend bootstrap/export tests: ten passed. Frontend build and Svelte checks passed with the existing BoardEditor non-reactive-update warning.
- Conformance TypeScript check and dev/fixture tests: 16 passed, including Linux routing and refusal to attach to an occupied backend port.
- Ownership audit: 28 core members, two host binding crates, no temporary protocol bridge; Linux production source below its 2,000-line cap.
- Strict OpenSpec validation and git diff --check.

Apple/Android native packages were not built on this Linux host. Changed shared boundaries were exercised through their host, FFI/JNI and ABI tests; native host code was not changed.

## Real desktop observations

The explicit ignored GTK/WebKit test ran in this desktop session and passed: shared frontend rendering; dark/light settings RPCs and matching native canvas; reload; real WebKit process termination/recovery; blocked remote navigation; minimize/restore retaining the same service lifetime; window close and shutdown. Light/dark PNGs were visually inspected.

The development launcher rendered the frontend through Vite, returned Linux bootstrap metadata through the proxy, survived browser reload, and applied a temporary CSS change via HMR. The CSS was restored. Testing exposed an orphaned Vite child through npm; Linux now executes Vite directly and uses the native shutdown deadline.

These observations do not establish recording continuity while minimized, forced-exit recording recovery, native export/import dialog success/cancel/error paths, quit during an active copy, saved appearance opposite to system at cold startup, or actual suspend/resume. Those remain acceptance work.

## Real Bluetooth fixture results and blocker

The production build uses BlueZ and ignores fixture environment variables.

- Identity, guest and direct scenarios passed together. Initial report: /tmp/float-view-linux-physical-initial.json.
- An isolated identity repeat passed.
- Direct legacy and direct control scenarios passed in a subsequent run.
- A later CAN scenario exposed a target orchestration bug: isolated storage was deleted between scenarios although CAN requires the identity scenario's saved board. The target now retains its isolated root for the physical suite and removes it at suite completion.
- Full-matrix attempts intermittently failed in identity while ESP32 sent a fragmented configuration response: ERROR NOTIFY_FAILED. Latest log: /tmp/float-view-linux-physical-complete.log. Firmware emits this generic error for notification failure, including its 100 ms completion timeout; these logs cannot establish the cause. No timeout or firmware behavior was changed to turn failure into a pass.
- Failure cleanup reset the fixture, turned its diagnostic LED off and stopped the owned Linux process. No unrelated Bluetooth device was modified.

The user was asked to reconnect/reposition the fixture before another physical run. Remaining coverage includes the selected fixture matrix, adapter loss/recovery, available write-mode behavior, recording/reconnect and actual suspend/resume. No full physical acceptance is claimed.

## Remaining coverage

Unchecked adapter/runtime tasks still need their requested targeted fault coverage: multiple adapters, daemon/authorization changes, setup and write failures, saturation, stale operations, and timer/shutdown interruptions. Shared policy tests cover several invariants but do not establish all Linux OS adapter behavior. Native file-dialog and interruption checkpoints are also outstanding. Preserve unchecked tasks until their evidence exists.
