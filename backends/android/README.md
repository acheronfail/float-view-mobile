# Android host

Android is one Kotlin Gradle module. `SharedService` creates the process-owned
Rust service and executes a closed, bounded OS fact/effect exchange through the
existing `application-runtime-android-jni` family. The binding retains no JVM
references or callbacks. Listener handles borrow this service; Activities and
WebViews are disposable.

Kotlin owns BLE/GATT, permissions, location acquisition, the foreground service,
scoped file access, theme and UI integration. All product CRUD, identity,
reconnect, recording, configuration, archives, public dispatch and projection
live in Rust. There is no native SQL catalog or domain-operation bridge.

| Package          | Responsibility                                                |
| ---------------- | ------------------------------------------------------------- |
| `application/`   | Process composition, bounded OS executor and JNI declarations |
| `bluetooth/`     | BLE/GATT, permission presentation and test adapter            |
| `files/`         | Scoped URI import stream and export destination picker/copy   |
| `lifecycle/`     | Application lifetime and foreground recording service         |
| `notifications/` | Ride notification presentation and notification formatting    |
| `location/`      | Location acquisition and permission broker                    |
| `protocol/vesc/` | Emulator conformance fixture only                             |
| `theme/`         | System bars and WebView appearance                            |
| `transport/`     | Listener lifetime and authenticated native URI upload         |
| `ui/`            | Disposable Activity and WebView                               |

The URI importer streams at most 128 MiB through 64-KiB buffers into the same
shared HTTP importer used by the frontend. Exports retain their service lease
until the scoped destination stream closes. Native file callbacks have bounded
admission; no entire archive is materialized in Kotlin memory.

Gradle pins NDK `28.2.13676358` and minimum API 29. Builds package one
`libfloat_view_application_runtime_android_jni.so` for each of `arm64-v8a`,
`armeabi-v7a`, `x86`, and `x86_64`. `just test android` runs linked JVM checks;
`just android-rust-packaging` verifies clean Debug/Release packages and exact
JNI exports. Select tests and packaging checks by impact using the root
[verification policy](../../AGENTS.md#verification). Use a real Android device
only for affected BLE, background, Activity or file-presentation behavior that
software cannot establish; unrelated platforms need no regression run.

## Local SDK and verification

Install the [Android command-line tools](https://developer.android.com/studio#command-tools)
under `$ANDROID_HOME/cmdline-tools/latest` and select an installed JDK with
`JAVA_HOME` (Java 17 or 21). On macOS the conventional SDK directory is
`$HOME/Library/Android/sdk`. Install the pinned build dependencies:

```sh
"$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" --install \
  'platform-tools' 'platforms;android-36' 'build-tools;35.0.0' 'ndk;28.2.13676358'
rustup target add --toolchain 1.93.0 \
  aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

Keep `ANDROID_HOME` set when building, or put `sdk.dir=<absolute SDK path>` in
the ignored `backends/android/local.properties`. Build the frontend assets first
with `cd frontend && npm run build`. From `backends/android`, full software
compilation, all JVM test variants and lint run with:

```sh
./gradlew clean assembleDebug assembleRelease assembleConformance test \
  lintDebug lintRelease lintConformance assembleDebugAndroidTest
```

With an emulator running, `./gradlew connectedDebugAndroidTest` executes the
instrumentation tests. Add SDK `platform-tools` to `PATH`, then run
`cd conformance && npm run android` from the repository root for the canonical
Android host contract suite. These software checks do not verify audible speech,
ducking/restoration, real headset routes or locked-screen BLE delivery.

## GitHub Actions APK builds

The [app workflow](../../.github/workflows/build-apps.yml) builds a signed Debug APK
on Ubuntu for every push to `master` and manual run. It builds the shared
frontend and packages the Rust runtime for all four supported ABIs using Java 17,
API 36, NDK `28.2.13676358` and the repository-pinned Rust toolchain. Locked Cargo
dependencies are fetched before Gradle's offline Rust build. CI caches Gradle's
wrapper, dependencies, compiled build scripts and local task outputs, with
`--build-cache` enabled. Cargo downloads and compilation artifacts are cached
separately; CI disables Rust incremental compilation and excludes incremental
state from that cache to reduce transfer overhead. Local builds retain their
usual incremental settings. A new cache format or dependency set needs a cold
build before later runs benefit.

Successful pushes publish `FloatView-build-<ten-digit run number>.apk` alongside
the IPA and macOS app in the same numbered GitHub release after all three builds
succeed. Manual runs build all three platforms without publishing a release:
select **Actions → Build apps → Run workflow**, then download
`FloatView-apk-<run number>` from the run summary. Workflow artifacts are retained
for 14 days.

The APK can be sideloaded on Android 10/API 29 or newer. CI signs every published
APK with the same persistent key, allowing subsequent CI builds to update the app
without removing local data. These remain Debug builds. Locally built APKs use
the local debug key and cannot replace a CI installation unless signed with the
same CI key.

Repository Actions secrets `ANDROID_SIGNING_KEYSTORE_BASE64` (base64-encoded
PKCS12 keystore) and `ANDROID_SIGNING_PASSWORD` (both store and key password)
provide the signing identity, with key alias `float-view-ci`. The workflow fails
if either secret is missing, verifies the signed APK, and removes the temporary
keystore after signing. Keep a secure backup of the keystore and password;
GitHub secret values cannot be downloaded later. The local provisioning backup
is in the ignored `.local/android-signing/` directory. Do not rotate this key
casually: a different key prevents ordinary updates to existing installations.

## Ride notifications and Live Updates

The foreground service runs while a device connection or recording is active,
including guest connections. It reads the shared Rust `live_activity` projection
and updates one ongoing notification at most once per second. The notification
shows the primary gauge's winning load percentage, distance in the selected
units, battery charge and pack voltage to one decimal place. Unchanged visible readings do not trigger reposts;
after 15 seconds without telemetry, it shows “Waiting for telemetry”. Opening
the notification returns to the app, and “Stop and disconnect” uses the existing
shared-service stop request.

On Android 16/API 36 and newer, fresh board readings request Live Update
promotion and put the load percentage in the status-bar chip. Older versions,
disabled promotion and devices that decline promotion retain a standard ongoing
notification. Android owns the layout and styling; no custom notification view
is used. Swiping the notification away suppresses telemetry reposts for that connection
without stopping recording. Required foreground-service type changes, such as a
new location grant, may restore the ongoing notification without re-requesting
promotion. Disconnect removes the foreground notification.

Android 13+ requests notification permission once after a board connects while
the app is foregrounded. Denial does not prevent Bluetooth or recording; users
can manage access later under Settings → Permissions. The Notifications row
reports both app access and the ride-notification channel; a blocked channel
opens its own settings. Android 16+ also shows a separate Live Updates row for
promotion access. Both rows refresh from OS observations when returning to the
app. The automatic first-ride prompt and Settings actions share the same native
permission broker, dialog slot and request history. `POST_PROMOTED_NOTIFICATIONS`
is a separate, non-runtime permission, and the system/user controls promotion.

`notifications/` owns display formatting and native notification construction.
`permissions/` owns notification observations and OS permission presentation.
Native code does not calculate drive load or battery estimates. Unit tests cover projection decoding, units, stale/missing
readings, visible-update coalescing and dismissal across connection generations.

Physical checkpoint: compare readings with the app on a connected board, then
background/lock the phone and check BLE updates, notification permission denial
and later grant, Stop, dismissal and reconnect. Check both a standard notification
and Live Update promotion on a supporting Android 16+ device. Software checks do
not establish OEM presentation or background BLE behavior.

## Spoken announcements

Rust supplies the highest load source and optional speed in one bounded speech
request. `speech/SpeechAdapter` owns installed offline English TTS voices and
transient may-duck audio focus on the main looper. Voice selection accepts both
ISO 639-1 `en` and ISO 639-2 `eng` language identifiers, as emitted by Samsung
TTS. It never queues delayed focus. Completion, errors, interruption and admission expiry stop playback and release
focus. Speech gain is relative to current device volume. Audio lowering remains
automatic; only iOS exposes its lowering preference.

The existing ride foreground service includes mediaPlayback while automatic
announcements are enabled and non-muted, alongside connectedDevice and optional
location. It remains one service and one shared runtime. Background delivery,
headset routing and music restoration need real-device validation; calls can
gracefully prevent speech. Foreground return, connection and explicit tests
refresh speech readiness without continuous engine initialization retries.

Settings exposes the system TTS chooser when Android can resolve it. The adapter
honors the engine's preferred installed offline English voice and reinitializes
after returning from the chooser, deferring refresh until active speech ends.
This uses standard Android TTS APIs with the selected engine, including Google
and Samsung engines; it does not require Samsung hardware. Speed is spoken as
the value and units, without a redundant “speed” prefix.

The adapter delegates its main-looper lifecycle to one `SpeechExecutor`, using
closed engine/focus/clock ports for deterministic failure tests. Engine and focus
cleanup remain native effects; Rust still owns telemetry eligibility, cooldown
and admission. No second speech policy or playback queue is introduced.
