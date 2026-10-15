# iOS announcement physical checkpoint — 2026-10-16

Closeout note: the user removed remaining physical checks from this change on
2026-10-16 and owns further manual testing. Unrun cases below remain unverified;
they are not archive gates. Native software failure-path coverage was completed
subsequently; see the [archived closeout tasks](../../openspec/changes/archive/2026-10-16-add-ride-announcements/tasks.md).

Device: iPhone 12, iOS 18.7.8, paired over USB with Developer Mode enabled.
Build: `fae1e0f`, Debug, signed with the user's configured development team.
Fixture: ESP32 on `/dev/cu.usbmodem1101`, direct Refloat 1.3.0 scenario,
production BLE adapter. Human listening confirmations are recorded below;
`finished` callbacks alone are not treated as evidence of audible speech.

## Confirmed results

- Combined test: “motor 72 percent, 25 kilometres per hour”, clearly audible,
  with no redundant speed prefix.
- Controlled telemetry: duty 78% with 32 kilometres per hour, then only motor
  78%, then only battery 82%. User confirmed correct highest-source speech,
  roughly five-second repeats without overlap, followed by silence. A below
  reading was followed immediately by disabling, so this run does not establish
  below-threshold cancellation independently of disabling.
- With music playing: 100% gain clearly audible, 50% quieter, 0% silent without
  lowering music. With lowering enabled, music restored after phrases. With
  lowering disabled, full-volume test speech mixed while music retained its
  normal level. The switch was changed between tests; this does not establish
  changing it during an active phrase or automatic mix-only speech.
- Automatic announcements with 20 km/h / 60% thresholds against controlled
  24 km/h / 64% duty readings continued while locked for more than one minute.
  User confirmed regular, non-overlapping speech and music restoration between
  phrases. Recorded samples grew from 1,411 to 1,809 across the checkpoint;
  fresh telemetry and the connection remained healthy after return.
- Imperial combined test: “motor 72 percent, 16 miles per hour”, confirmed.
- Combined metric test with Wi-Fi and mobile data disabled, Bluetooth retained:
  clear and correct. User was told to restore network access afterward.
- Automatic announcements stopped cleanly when disabled, confirmed by the user.

## Execution and cleanup

The iPhone app built with `xcodebuild -project
backends/apple/ios/float-view.xcodeproj -scheme float-view -configuration Debug
-destination platform=iOS,id=<device> -allowProvisioningUpdates
ENABLE_USER_SCRIPT_SANDBOXING=NO build`. Xcode account/provisioning and the
phone's developer-profile trust were configured by the user. Signature
verification passed and `devicectl` installed/launched the app without resetting
its data.

A temporary host controller derived from `src/announcements-physical.ts` used
`iosPhysicalTarget.endpoint()` and the existing fixture runner. The first
noninteractive controller was restarted with interactive input; subsequent
fixture RESET/READY and disconnect cleanup succeeded. The USB loopback test
connection timed out when another app was foregrounded/while locked. Fresh
connections after returning to Float View were used for subsequent control and
readback; autonomous BLE and speech continued. This is not evidence of a speech
failure, nor a claim that the USB connection stayed available in the background.

Captured announcement settings, units, minimum session distance and location
recording settings were restored and read back through a fresh authenticated
connection; the fixture was disconnected. The original controller's stale USB
socket timed out during its final disconnect/restore block, so that block did
not establish successful settings restoration. A fresh connection verified the
restored values again after it exited. The runner acknowledged final fixture
RESET/READY and closed its serial handle; the app and USB tunnel were then
stopped successfully. The controller's nonzero exit records this transport
cleanup failure, not an entirely successful automated run.
No motor actuation or firmware/configuration write was required.

## Remaining scope

Headset routing/disconnection, calls/focus interruptions, missing-voice recovery,
watchdog/missing-completion behavior, telemetry-only stalls, below-threshold
cancellation independently of disabling, live changes of lowering during speech,
automatic mix-only speech and manual system-volume changes remain unverified.
Native fake-engine/session fault tests and macOS audible playback remain separate
outstanding work. The change is not ready to archive solely from these results.
