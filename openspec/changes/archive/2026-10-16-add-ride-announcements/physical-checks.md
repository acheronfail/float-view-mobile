# Announcement physical procedure and historical scope

Core Android/iOS speaker, music, BLE and locked-screen checks passed within the
limits in the [Android summary](../../../../conformance/physical-checkpoints/2026-10-16-android-announcements.md)
and [iOS summary](../../../../conformance/physical-checkpoints/2026-10-16-ios-announcements.md).
Remaining manual checks were removed from the historical archive gate; they are
unverified, not failed or newly required. macOS audible behavior was not established.
Detailed device records and execution history are local.

## Full cross-platform procedure

Use one real iPhone and one real Android phone (prefer Android 15/API 35 or newer
for its foreground audio-focus restrictions), a connected board or approved
hardware fixture, a Bluetooth audio headset and a music player. Use safe low
thresholds with controlled telemetry; do not ride at dangerous speeds or loads
to trigger a test. For fixture runs, follow the existing safety and restore
procedure and retain its evidence. Restore announcement settings after testing.

| Check                              | Platforms                       | Steps and required result                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ---------------------------------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Installed offline voice and gain   | iOS, Android                    | Disable network access while retaining BLE. With both automatic rules off, run speed, load and combined tests at 100%, 50% and 0% announcement volume, waiting for cooldown. Hear the selected units and one load source followed by speed; reduced gain is audible and zero is silent without lowering music. Device media volume must not change or be raised from mute.                                                                                                                      |
| Real telemetry and repeat delivery | iOS, Android                    | Enable speed and load together, set safe thresholds and a five-second interval. Exercise below/equal/above readings, continued excess and a change in the winning load source. Compare speech with the gauge: only the highest of duty/motor/battery is named, followed by speed when it qualifies. Expect an immediate eligible attempt, current values on repeats, no overlap and no burst of delayed phrases. Battery means utilisation, not charge. Repeat once with the other speed unit.  |
| Background and locked screen       | iOS, Android                    | While fresh eligible BLE telemetry and recording continue, leave the app, lock the screen and listen for at least one minute. Announcements continue at the configured cadence while readings stay fresh. Unlock, return and replace/recreate the frontend: no duplicate speech or reset cooldown; recording and connection remain healthy. On Android, verify the existing foreground service remains active when non-muted announcements are enabled and after toggling volume/rules.         |
| Ducking and restoration            | iOS, Android                    | Play music through the speaker, then headset; exercise automatic and test speech. Music temporarily lowers (or the player pauses according to OS policy), then returns after every phrase. Change device volume during speech and confirm cleanup does not overwrite that choice. On iOS repeat with “Lower other audio while speaking” off: music mixes without requested lowering/pause. Change the switch during speech: the next phrase uses the new policy. Android must hide this switch. |
| Cancellation and stale readings    | iOS, Android                    | During automatic speech, disable both rules; separately disconnect the board or stop incoming telemetry for more than two seconds. Speech stops and music returns; no old readings are spoken. Reconnect: only fresh eligible telemetry can speak, with no backlog. Check a normal below-threshold transition also cancels an automatic phrase. BLE/recording errors must not result from audio cleanup.                                                                                        |
| Headset route changes              | iOS, Android                    | Disconnect/reconnect the Bluetooth audio headset during speech, then test again on the resulting OS-selected route. No stuck lowering, overlapping utterances or delayed old readings; subsequent fresh attempts recover. Confirm expected audible routing with the actual headset, including any switch away from its call route.                                                                                                                                                              |
| Audio interruption and calls       | iOS, Android                    | Interrupt a phrase using another app that takes audio focus, and probe one representative call. Speech may be skipped/interrupted during the call; successful call playback is optional. BLE and recording continue, audio resources are released and later fresh attempts recover after the interruption. No forceful takeover, stale replay or permanently lowered music. After an engine error or interruption, no audio resources may remain held beyond the ten-second admission deadline. |
| Unavailable voice and recovery     | Android; iOS where reproducible | Select an engine with no installed offline English voice or otherwise make speech unavailable. Test reports unavailable, automatic attempts remain interval-bounded, settings are retained and BLE/recording continue. Restore an offline voice and return to the app: readiness refreshes and a subsequent test/fresh automatic attempt works without restarting. Record cases the device cannot reproduce rather than claiming them passed.                                                   |
| Desktop synthesis and cleanup      | macOS                           | With installed offline English speech and music, run combined test speech at normal/reduced/zero volume; interrupt/cancel a test and test again. Check audible speech, native automatic mixing behavior, no retained audio resources and no lowering switch. Mobile locked-screen/BLE checks do not apply here.                                                                                                                                                                                 |

Ordinary protocol, load arithmetic, persisted-session interpretation and unrelated
permission/file flows do not need another physical regression for this feature.
Native fake-engine/focus tests are separate software coverage; physical checks
must not be used to claim those unimplemented tests exist.

## Android harness


Use the existing physical direct scenario first, then the interactive harness:

```sh
cd conformance
FLOAT_VIEW_PHYSICAL_SCENARIOS=direct FLOAT_VIEW_PHYSICAL_REFLOAT_VERSION=1.3.0 \
  npm run physical -- android <fixture-port> <adb-serial>
node --strip-types src/announcements-physical.ts <fixture-port> <adb-serial>
```

The harness announces its bounded commands, preserves the settings it changes,
uses the production BLE adapter, and resets the fixture on normal exit, signals
or its 20-minute session limit. `stop` ends the run. Human listening is required;
`finished` alone does not establish audible speech. If the device's app uses a
CI signing key, supply a matching signed APK through
`FLOAT_VIEW_ANDROID_PHYSICAL_APK=/absolute/path/to/test.apk`; do not uninstall an
existing product installation without authorization.
