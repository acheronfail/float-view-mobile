# Android host

Android is one Kotlin Gradle module. `SharedService` creates the process-owned
[Rust service](../core/README.md) through the existing JNI family. Activities,
WebViews and listeners borrow it. Kotlin executes OS effects; product CRUD,
recording, reconnect policy, archives and public projection remain in Rust.

Paths below are relative to `app/src/main/java/fail/acheron/floatview/`:

| Package                       | Responsibility                                       |
| ----------------------------- | ---------------------------------------------------- |
| `application/`                | Composition, OS executor and JNI declarations        |
| `bluetooth/`                  | BLE/GATT and test adapter                            |
| `files/`                      | Scoped import streams and export destination copies  |
| `lifecycle/`                  | Application lifetime and foreground service          |
| `notifications/`              | Native ride-notification formatting and construction |
| `permissions/`, `location/`   | Permission broker and location acquisition           |
| `protocol/vesc/`              | Emulator conformance fixture only                    |
| `speech/`                     | TTS engine and audio-focus lifecycle                 |
| `theme/`, `transport/`, `ui/` | Appearance, listener lifetime and disposable WebView |

Do not retain JVM references/callbacks in the binding or materialize whole archives
in Kotlin memory. Stream imports through the shared importer; retain export leases
until scoped copies close.

## Local SDK and verification

Install [Android command-line tools](https://developer.android.com/studio#command-tools)
under `$ANDROID_HOME/cmdline-tools/latest`. Select JDK 17 or 21 with `JAVA_HOME`;
Android Studio's bundled Java may be newer than Gradle supports. Install the
pinned SDK/NDK and Rust targets:

```sh
"$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" --install \
  'platform-tools' 'platforms;android-36' 'build-tools;35.0.0' 'ndk;28.2.13676358'
rustup target add --toolchain 1.93.0 \
  aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

Keep `ANDROID_HOME` set or put `sdk.dir=<absolute SDK path>` in ignored
`local.properties`. A Homebrew/macOS example:

```sh
export JAVA_HOME="$(brew --prefix openjdk@21)/libexec/openjdk.jdk/Contents/Home"
export ANDROID_HOME="$HOME/Library/Android/sdk"
export PATH="$ANDROID_HOME/platform-tools:$PATH"
```

Build frontend assets first with `cd frontend && npm run build`. Use
`just test android` for linked JVM checks and `just android-rust-packaging` for
clean packages/JNI exports, selecting checks by
[impact](../../AGENTS.md#verification). For comprehensive host compilation/tests,
run from `backends/android`:

```sh
./gradlew clean assembleDebug assembleRelease assembleConformance test \
  lintDebug lintRelease lintConformance assembleDebugAndroidTest
```

With an emulator running, use `./gradlew connectedDebugAndroidTest` for
instrumentation and `cd conformance && npm run android` from the repository root
for canonical host conformance.

## Signing and distribution

[Build apps](../../.github/workflows/build-apps.yml) owns APK build/signing inputs;
[releases](../../docs/releases.md) owns channels and numbering. Published APKs use
the persistent `float-view-ci` key from `ANDROID_SIGNING_KEYSTORE_BASE64` and
`ANDROID_SIGNING_PASSWORD`. Keep its secure backup under local custody: rotating
the app-signing identity prevents ordinary updates to existing installations.
Local Debug signing cannot replace a CI installation signed with another key.

For Play enrollment, upload-key custody and store replacement checks, use
[Android distribution](../../docs/android-distribution.md).

## OS integration constraints

- Receive MTU and outgoing write capacity are different. Older Express firmware
  advances fragments by `min(MTU, 255)` without subtracting ATT overhead; a small
  negotiated MTU can lose bytes. Preserve the source-derived receive workaround
  and pass confirmed write capacity to shared segmentation.
- The foreground service and notification consume the shared native presentation
  projection. OS permission, channel and Live Update promotion are separate facts;
  native code must not calculate drive load or battery estimates.
- `speech/SpeechAdapter` accepts both `en` and Samsung's `eng` language IDs for
  installed offline English voices. Main-looper engine/audio-focus cleanup is
  native; eligibility and cooldowns remain shared. Calls can prevent playback.
- Report foreground lifecycle facts rather than implementing a second auto-connect
  window in the Activity. Listener/WebView replacement must not create a new runtime.
- Startup retries preserve the process owner and data; recovery must not offer
  clearing storage or uninstalling as a routine fix.

For affected physical behavior, compare notifications with the app, then check
background/lock, Stop, dismissal/reconnect, permission denial/grant, and standard
versus promoted presentation on a supporting device. Speech checks need audible
headset routing and music restoration. Emulator/unit tests do not establish OEM
presentation, radio behavior or audibility. See
[physical checkpoints](../../conformance/physical-checkpoints/README.md).
