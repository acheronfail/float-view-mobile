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

## Follow-up — 2026-10-01 (Adelaide)

Reconnecting the fixture did not by itself resolve the problem. Identity, guest, direct, direct-legacy and control-direct passed; control-can again failed with the generic notification error. Cleanup completed. Log: /tmp/float-view-linux-physical-reconnected.log.

Added numeric NimBLE error diagnostics, built with ESP-IDF 5.4.1 / PlatformIO espressif32 6.11.0, and flashed the fixture. A subsequent ambiguous-controller run reported NOTIFY_FAILED_6: BLE_HS_ENOMEM. The pinned upstream source confirms that ble_gatts_notify_custom consumes the supplied buffer on either result and its notification callback reports an enqueue attempt, not remote delivery.

The fixture's configured FreeRTOS tick is 100 Hz. Its former pdMS_TO_TICKS(5) retry delay was zero, allowing all transient-buffer retries to run without waiting. Replaced that with at least one tick between rejected submissions and a one-second budget; removed the redundant enqueue-status semaphore. Accepted submissions are never replayed and retain 20 ms pacing. This is OS transport handling only; scenario logic stays in conformance.

The corrected firmware built and flashed successfully. Its binary SHA-256 is 8bfeb8d8c231215b21939fc06b942fd2bead0c8612f14c3b39056bb5c02e9ec8. The C parser checks and TypeScript fixture tests passed.

A first run with the fix encountered a guest disconnect without a notification error. The focused repeat then passed all nine selected scenarios: identity, control-can, guest, ambiguous, bms-direct-controller, bms-express, bms-standalone, ride, unexpected-disconnect. Report: /tmp/float-view-linux-physical-final-focused.json. Long configuration transfer and restore, recording/review/export, and reconnect passed. All 17 scenario names have now passed across the recorded runs, but this is not a claim that one complete 17-scenario invocation passed or that the separate radio/suspend checkpoints are complete.

Additional software/desktop evidence:

- Linux unit suite: ten passed, one explicit desktop test ignored by default; all-target/all-feature warnings-denied Clippy passed.
- Real desktop test additionally passed a cold restart with saved light appearance and an initially dark native toolkit.
- Duplicate presentation completion after cancellation is now ignored by the Linux UI adapter; the focused test passed.
- Linux shutdown contention test passed: the ten-second OS deadline is enforced, ownership is retained on timeout, and shutdown/reopen succeed after releasing the lock. Together with independent cleanup admission and the existing shared stale-lifetime/sequence/timer and correlated-completion tests, this completes task 2.3.
- Added an opt-in interactive native Save cancellation checkpoint; its result is recorded separately below.

The real native Save cancellation checkpoint passed after the user confirmed the
dialog appeared and was cancelled. The runner independently verified the Exports
directory was empty, then cleaned up the fixture and host. Report:
/tmp/float-view-linux-native-file-check.json. Native destination selection/copy
success and failure still require separate evidence.

The interactive minimize checkpoint passed: user-confirmed minimize/restore and
authoritative state showed the same active session with increasing samples.
Task 5.2 combines the earlier dev/proxy/HMR/occupied-port checks with the now
verified native presentation path shared by browser and embedded modes.

The first user-operated radio cycle exposed a Linux adapter defect: powered-on
state returned but the session stayed reconnecting. A read-only BlueZ query
confirmed its unpaired Device object had been removed. Linux now re-discovers
the exact saved address when that object is absent, within the existing setup
deadline. The repeated real radio off/on checkpoint passed: same session and
advancing samples. Logs: /tmp/float-view-linux-desktop-interruptions.log (failure)
and /tmp/float-view-linux-desktop-recovery.log (successful radio repeat).

The user could not suspend the machine now. The suspend step was explicitly
left unconfirmed; the runner reset the fixture, turned its LED off and stopped
the owned host. No suspend/resume pass is claimed, and tasks 4.3 and 6.3 remain
open. The radio pass preceding that deliberate stop is retained in its log.

The final real desktop regression additionally passed fresh system appearance
and cold light/dark starts with a persisted preference opposite to the toolkit
initial appearance. Task 2.5 is complete. Task 3.4 is supported by canonical
software loss/recovery checks plus physical saved-association restarts, guest
telemetry, unexpected disconnect and the real radio recovery checkpoint. Task
6.4 records the actual WebKit recovery, minimize, close/quit, appearance and
Save-dialog observations above; untested copy/error/suspend paths remain
explicit in their corresponding tasks.
