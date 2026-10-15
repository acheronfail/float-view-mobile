# Physical announcement evidence and optional manual checks

Closeout decision (2026-10-16): the user removed all remaining physical checks
from this change and will perform further manual testing independently. The
procedure below is optional reference material, not an archive gate. Unchecked
items mean unverified, not failed or required by this change.

Status: core Android speaker/music/BLE checks passed on a Samsung Galaxy S9+
(Android 10) on 2026-10-16; see the [physical evidence](../../../../conformance/physical-checkpoints/2026-10-16-android-announcements.md).
Core iOS speaker/music/BLE checks also passed on an iPhone 12 (iOS 18.7.8);
see the [iOS physical evidence](../../../../conformance/physical-checkpoints/2026-10-16-ios-announcements.md).
macOS and the remaining cases below are pending. Emulator, simulator and JVM
results do not establish physical audio effects.

## Android results

- [x] Production BLE fixture negotiation and fresh telemetry.
- [x] Audible combined test and offline English speech with Wi-Fi/mobile data off.
- [x] Duty/motor/battery winning-source speech, combined speed and load, roughly
      five-second repeats without overlap, and below-threshold silence.
- [x] Speech volume 100% / 50% / 0%, music lowering/restoration between phrases
      and normal music while muted.
- [x] More than one minute locked, continued speech/BLE/session growth, and
      disabling with clean audio restoration.
- [x] Physical BLE disconnect/reconnect and fresh speech recovery without a burst.
- [x] Captured settings restored, ESP32 RESET/READY verified, app/tunnel cleanup.
- [ ] Headset routing/disconnection; genuine focus interruption/denial and call
      fallback; missing completion cleanup; genuine missing-voice recovery without
      restart; telemetry-only stalls; imperial spoken units; manual device-volume
      edits during speech; Android 15+ audio-focus/foreground restrictions.

## Full cross-platform procedure

iOS now has confirmed combined/winning-load speech, repeats, 100%/50%/0% gain,
test-speech lowering on/off with music restoration, a locked-screen minute with
continued BLE/recording, imperial wording, offline speech and clean disabling.
Headset/call/interruption cases, unavailable-voice recovery, independent
below-threshold/stale-telemetry cancellation, automatic mix-only playback and
changing lowering/system volume during speech remain pending. The USB controller
needed fresh connections after backgrounding; cleanup was independently verified
as recorded in the evidence.

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
