# Android announcement checkpoint — 2026-10-16

Device: Samsung Galaxy S9+ (SM-G965F), Android 10 / API 29. Production Android BLE
adapter and connected ESP32 byte bridge, direct VESC route with Refloat 1.3.0.
Audio: Samsung TTS (`com.samsung.SMT`), installed English voices reported as
ISO 639-2 `eng`, with `isNetworkConnectionRequired=false`.

Base implementation: `fae12fb`, followed by the English-language selector fix
and regression tests in this checkpoint's commit. The user approved replacing
the incompatible previously signed installation after its APK and private data
were backed up and verified locally. Those private backups are not committed.

## Verified

- Canonical physical `direct` scenario passes (6.5 seconds): production BLE,
  firmware/package/current-limit negotiation and telemetry. The fixture's final
  RESET/READY handshake and serial/loopback cleanup passed.
- Device probing found a real adapter defect: the English selector accepted `en`
  but rejected Samsung's `eng`. Both standard ISO language identifiers are now
  accepted. This is support for a current native engine, not an old-data reader.
- After the fix, all three Android build variants and lint checks pass. All 31
  JVM tests pass in each variant (93 executions, no failures). The four native
  instrumentation tests pass on this phone, including engine initialization and
  shutdown, JNI boundaries and Activity/WebView lifetime checks (32.922 seconds).
- Manual listening: user confirmed the 100% combined test clearly said
  “motor 72 percent, speed 25 kilometres per hour”; native outcome was `finished`.
- Fixed physical telemetry and manual listening: user confirmed duty 78% plus
  speed 32 km/h, then motor 78% only, then battery 82% only. Repeats were roughly
  every five seconds with no overlap, followed by silence at duty 64% / 24 km/h.
  Backend snapshots independently confirmed those winners, values and fresh
  sequence advancement. This is observed cadence, not instrumented audio timing.
- During the initial unavailable status, a non-muted test returned `unavailable`
  and fresh BLE telemetry/session samples continued (sequence/sample count 76 to
  281 and beyond). The initial status was caused by the selector defect, so this
  does not establish a genuine missing-voice case or voice-install recovery
  without app restart.

- Manual listening with music: user confirmed normal speech at 100%, quieter
  speech at 50%, silence at 0%, temporary music lowering during phrases, and
  normal music volume between phrases and while muted.
- Locked-screen checkpoint: app was backgrounded, keyguard reported
  `showing=true`, power state was Dozing, and the ride service remained
  foreground. Across more than one minute, session sample count/telemetry
  sequence grew from 961 to 1,192. User confirmed regular audible announcements,
  music restoration between phrases, and clean stopping when both rules were
  disabled. This does not establish deep idle after prolonged unplugged use.
- Offline test: Wi-Fi and mobile data both read `0`; the combined test returned
  `finished` and user confirmed clear, correct speech. The user was prompted to
  restore network access afterward. The final observation showed Wi-Fi on and
  mobile data off; no network switch was changed automatically by the harness.
- Injected physical BLE disconnect/readvertise: firmware/current-limit/Refloat
  negotiation repeated, fresh telemetry resumed (sequence 1,537 to 1,578), and
  user confirmed normal speech recovery without overlap or a burst of old
  speech, followed by clean stopping when disabled.
- Final cleanup passed: captured announcement settings, units, minimum-distance
  and location-recording settings restored and read back; device disconnected;
  ESP32 RESET/READY verified; serial handle closed; app stopped and loopback
  forwarding removed. No motor actuation or firmware/configuration write was
  needed for this announcement checkpoint.

## Reproduction

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

## Scope limits

Follow-up: after installing the refinement build, the user confirmed the new
Voice settings button works on this phone. Three native voice-selection tests
also pass, including standard `en` and Samsung `eng` locales and preferred
installed/offline filtering. Revised prefix-free speech and an actual change of
engine/voice have not received a separate listening confirmation.

No iOS/macOS checks, Bluetooth audio headset routing, call/focus-denial/loss
cleanup, a genuinely missing voice and recovery without restart, prolonged
missing-completion cleanup, a telemetry-only stall with BLE still connected,
imperial spoken units, manual device-volume changes during speech, or Android
15+ foreground/audio-focus restrictions are established by this Android 10 device. Native fake-engine/focus coverage remains
separate from these physical observations. The broader physical protocol/firmware
matrix is not required for this announcement feature.
