# Android discovery location permissions — 2026-10-16

Physical checkpoint for the discovery permission fix in `fae1356` on a Samsung
Galaxy S9+ (SM-G965F), Android 10 / API 29. The ordinary debug app used the
production Android Bluetooth and permission adapters and embedded frontend.

## Setup and isolation

`assembleDebug --offline` passed with the installed Homebrew OpenJDK 21. The
installed app had a different signing certificate, so Android correctly rejected
an in-place update. The installed app was not uninstalled or cleared.

A temporary Gradle init script overrode only the application ID to
`fail.acheron.floatview.permissiontest`. This disposable copy allowed fresh OS
permission requests without resetting the original app's permissions, history,
or storage. Only the disposable copy was cleared between fresh-install tests.
The normal debug APK output was rebuilt without the override after testing.
Unrelated frontend working-tree edits were left untouched; the test APK used the
existing generated frontend assets containing the discovery fix.

The authenticated loopback connection supplied bounded permission and scan state;
ADB UI hierarchy inspection and native taps exercised the actual WebView and OS
permission/settings screens. No simulated permission responses or Bluetooth
adapter were used.

## Results

| Scenario                 | Physical observation                                                                                                                                                                                                                                                              |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| First use                | After accepting the safety screen, the real Android location prompt appeared automatically. The user independently confirmed the prompt appeared immediately. The captured prompt offered Allow only while using the app and Deny.                                                |
| Pending prompt           | The shared location row reported `pending: true`; scanning and route recording remained off.                                                                                                                                                                                      |
| Foreground grant         | Granting access started discovery and displayed a nearby board without a manual scan action. Background location was not requested; route recording remained off.                                                                                                                 |
| Denial                   | Selecting Deny produced `status: denied`, `action: requestPermission` and rationale guidance. The connect screen showed Location access is needed to find boards and Allow location, with scanning stopped. No repeated prompt appeared during the subsequent observation window. |
| Explicit retry           | Allow location opened the native rationale explaining Bluetooth board discovery and optional route recording. Continue reached the OS permission dialog again.                                                                                                                    |
| Permanent denial         | Selecting the real Deny & don't ask again option changed the action to `openSettings`. The connect screen exposed Open settings and retained the location explanation.                                                                                                            |
| Settings without a grant | Open settings reached Android App info. Returning without changes retained denied access, the recovery button and stopped scanning.                                                                                                                                               |
| Settings grant           | App info > Permissions > Location > Allow only while using the app granted access. Returning through the Settings task stack updated the location row to granted / precise / whileInUse and automatically resumed scanning. Route recording remained off.                         |
| Device Location off      | With the foreground grant retained, turning device Location off changed `servicesEnabled` to false, stopped scanning, hid discovered boards and displayed Location is off with Open settings.                                                                                     |
| Device Location recovery | Open settings reached the device-wide Location page, rather than App info. Turning on its native switch and returning removed the blocker and automatically resumed scanning. Route recording remained off.                                                                       |

Initial automation selectors were adjusted for Samsung's uppercase CONTINUE
button and its separate Deny & don't ask again button. Some retry interaction
was user-assisted. A delayed response reached the existing bounded presentation
timeout and displayed a service-unavailable error; the eventual OS denial still
updated the permission inventory, and the subsequent Settings recovery cleared
the error. These combined physical observations are not presented as one
uninterrupted automated test run.

Google Play services displayed a No location access informational dialog when
device Location was disabled. After dismissing that notice, the app's own
Location is off state and its settings destination were verified. The final
Location-switch recovery script passed all assertions and restored the original
Location mode in a `finally` block.

## Cleanup and limits

- Verified the original app's grants were unchanged: fine location granted,
  coarse location denied, and background location granted.
- Restored and verified the original device Location mode (`3`, enabled).
- Uninstalled the disposable test copy and verified its package was absent.
- Removed the device UI dump and test-created ADB forwarding.
- No board connection, ride recording, firmware/configuration write, or control
  command was requested. Discovery alone needed no programmable BLE fixture.

This phone cannot physically validate Android 12+ Nearby devices grouping,
approximate-versus-precise selection, or newer Android permission behavior.
Those paths retain software coverage; no newer-device physical result is claimed.
