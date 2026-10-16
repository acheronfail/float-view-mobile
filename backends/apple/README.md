# Apple host

No personal development team is committed in the Xcode project. For signed
device builds, supply `DEVELOPMENT_TEAM=<team-id>` as an `xcodebuild` build-setting
override. Keep local signing/account configuration out of version control.
Unsigned packaging checks do not need a development team.

Swift owns Apple OS integration for the shared Rust backend. One `SharedService`
creates the process owner in a private application-support root, protected until
first unlock on iOS. Its bounded executor maps Bluetooth/location callbacks,
executes timers and permission requests, and supplies native theme and file UI.
It contains no product storage, configuration policy, reconnect policy, domain
dispatch or snapshot reconstruction.

`LoopbackServer` borrows that owner through the existing C binding. Native hosts
supply HTML and start/stop the listener; Rust owns the complete authenticated
frontend contract. iOS WebViews and listeners can be replaced independently of
recording. Export presentation acknowledges dismissal or completed destination
copy before releasing the shared file lease.

Source ownership:

| Path                                    | Responsibility                                                  |
| --------------------------------------- | --------------------------------------------------------------- |
| `Sources/FloatViewBackend/application/` | Process composition and bounded OS exchange                     |
| `Sources/FloatViewBackend/bluetooth/`   | CoreBluetooth, GATT setup, permissions and test adapter         |
| `Sources/FloatViewBackend/location/`    | Core Location acquisition                                       |
| `Sources/FloatViewBackend/transport/`   | Listener lifetime and caller-owned C buffers                    |
| `Sources/FloatViewBackend/protocol/`    | Software-conformance fixture only                               |
| `ios/float-view/`                       | iOS lifecycle, native file presentation, appearance and WebView |
| `macos/`                                | macOS lifecycle, native file presentation and WebView/window    |

SQLite schemas and archive validation are embedded in Rust. Native file import
uses WebKit's scoped file input and the authenticated streaming importer.
Addresses, tokens, private identity and raw telemetry stay out of normal logs.

`just test apple` covers OS adapters, C bounds and shared-service conformance.
`just apple-packaging` checks clean macOS, iOS simulator and iOS device packages.
Choose the affected tests and packages using the root
[verification policy](../../AGENTS.md#verification). Physical BLE/background and
presentation checks need connected hardware only when the changed behavior
cannot be established in software; unrelated platforms need no regression run.

`just run macos` assembles and launches `/tmp/float-view-swift-build/FloatView.app`
with `package-macos.sh`, also used by physical macOS conformance. The app bundle
includes the permission usage descriptions and bundle identity needed by Core
Location; launching the bare executable with `swift run` omits that metadata.
To check location permission presentation, launch this app with Location Services
enabled and permission not yet requested, then use Settings → Permissions →
Location → Request access. Confirm the native prompt appears and allowing access
updates the Location grant. This OS checkpoint cannot be established by unit tests.

Discovery requests Bluetooth access before checking radio readiness: CoreBluetooth
can report an unknown radio until its manager has triggered the first permission
prompt. Reading adapter state and stopping discovery never create that manager,
so closing or quitting an unused app must not trigger a new prompt. The Connect
page distinguishes access requests, pending prompts, radio startup and scan
failures, with settings/retry actions where appropriate. Validate the first-use
prompt, denial/recovery and quit-without-requesting behavior on macOS; mocked
permission tests cannot establish actual OS prompt presentation.

## iOS App Store preparation

The GitHub `just ipa` route remains unsigned for AltStore to re-sign. Store
preparation is separate and always uses Release for the app and its embedded
Live Activity extension. No current command uploads a build or creates a tag.

### Account and signing setup

Apple Developer membership is confirmed. On 2026-10-17 the owner installed Xcode
26.0, registered the two identifiers below and created an Apple Distribution
certificate. The owner also confirmed creating the iOS App Store Connect record.
Automatic provisioning has signed the app and extension locally; complete
archive/export verification and store uploads remain pending.

1. Install a supported [Xcode release](https://developer.apple.com/xcode/resources/).
   [Apple currently requires Xcode 26+ and the iOS 26+ SDK](https://developer.apple.com/news/upcoming-requirements/?id=04282026a)
   for uploads. The deployment target remains iOS/iPadOS 17; building with a newer
   SDK does not require raising that target. Set `DEVELOPER_DIR` for individual
   commands if keeping multiple Xcode installations; the owner installed 26.0 at
   `/Applications/Xcode-26.0.0.app`, so use
   `export DEVELOPER_DIR=/Applications/Xcode-26.0.0.app/Contents/Developer`.
2. In [Certificates, Identifiers & Profiles](https://developer.apple.com/account/resources/identifiers/list),
   select the publishing team and register two **explicit App IDs**, after checking
   availability: `fail.acheron.floatview` and
   `fail.acheron.floatview.live-activity`. The second belongs to the WidgetKit
   Live Activity extension. The current implementation does not use push-token
   Live Activity updates, App Groups or iCloud; do not enable those speculatively.
   If an identifier belongs to another team, resolve it before creating the app
   record or changing source identifiers.
3. Sign into that team in Xcode Settings → Accounts. Use Manage Certificates to
   create an **Apple Distribution** identity and retain a secure private-key backup.
   Automatic archive signing may also need an Apple Development identity. The
   Developer ID Application identity for Mac downloads cannot replace either.
   [Apple explains distribution signing and profiles](https://developer.apple.com/help/account/provisioning-profiles/create-an-app-store-provisioning-profile/).
   Both bundles need profiles for the same team; automatic signing manages these.
4. In [App Store Connect → Apps](https://appstoreconnect.apple.com/apps), create
   the iOS app record using the main identifier. Select **iOS** for the current
   setup, choose the public name and primary language, and use a stable internal
   SKU (suggested `float-view-ios`). **Full Access** is suitable for the solo
   account: this setting controls invited App Store Connect users, not customers.
   The extension has no separate store listing. Adding macOS to this record later
   requires the same bundle ID and creates a
   [universal purchase](https://developer.apple.com/help/app-store-connect/create-an-app-record/add-platforms/);
   the current direct Mac app retains its separate identifier. Complete any pending
   account agreements.
   [Apple's app-record instructions](https://developer.apple.com/help/app-store-connect/create-an-app-record/add-a-new-app/)
   explain which fields can change later. Price, territories, privacy/support
   URLs and payment/tax details remain owner decisions before publication.

Keep team identifiers and private signing material out of source. Set
`APPLE_TEAM_ID` in the local environment; the tooling passes it to **both** targets
with one `DEVELOPMENT_TEAM` override. Changing the former development bundle
identifier creates a separate iOS installation/container; no old-container
migration or identity alias is provided. The shared scheme makes archive settings
reproducible on a fresh checkout and keeps lifecycle tests in its Test action.

### Local archive and export

Install the frontend dependencies (`cd frontend && npm ci`), the pinned Rust
toolchain and its `aarch64-apple-ios` target. Fetch the locked iOS Rust dependencies
before archiving (`cd backends/core && cargo fetch --locked --target aarch64-apple-ios`).
The commands rebuild and embed the shared frontend automatically.

An unsigned structural check also works with the older Xcode 16.4 toolchain and
needs no developer account. The number below is an unused local test value:

```sh
FLOAT_VIEW_VERSION=v1.0.0 FLOAT_VIEW_BUILD_NUMBER=1 \
  just ios-archive-check /tmp/float-view-ios-archive-check
```

This checks a real arm64 Release archive, matching app/extension identifiers and
versions, extension embedding and the shared HTML resource. It records source
revision, dirty state, toolchain and app-tree SHA-256 in `archive.json`. This output
cannot be exported as a store build by the tooling.

After account setup and Xcode upgrade, commit source changes and choose a fresh
output directory under the ignored `.local/ios-store/`:

```sh
# Set APPLE_TEAM_ID to the publishing team; keep its value outside Git.
# Set FLOAT_VIEW_BUILD_NUMBER to an allocated, unused native build number.
export FLOAT_VIEW_VERSION=v1.0.0
just ios-store-archive .local/ios-store/candidate-archive
just ios-store-export .local/ios-store/candidate-archive .local/ios-store/candidate-export
```

The build number must be explicit. These commands **do not allocate or reserve
one**: the shared monotonic allocator in plan task 5.1 is still required before
upload automation. Never reuse an Apple-consumed number for a corrected binary.
The version input reserves no Git tag; stable v1.0.0 still waits for acceptance.

Normal commands use installed signing assets. To let Xcode create/update profiles
using its signed-in account, explicitly invoke the relevant command with:

```sh
python3 backends/apple/ios-store.py archive .local/ios-store/candidate-archive \
  --allow-provisioning-updates
python3 backends/apple/ios-store.py export .local/ios-store/candidate-archive \
  .local/ios-store/candidate-export --allow-provisioning-updates
```

Archive and export refuse existing output directories. Store archives require
clean source and the currently required SDK; export refuses an unsigned check or
an archive changed since recording. Export uses `app-store-connect` with
`destination=export`, automatic signing and automatic version changes disabled.
Successful verification requires both bundles to have Apple distribution
signatures, the expected team/application identifiers, unexpired App Store
profiles, device arm64 binaries and no debugger entitlement. A successful export
writes `export.json` with source/version and the IPA hash; a partial directory
without that record is not a verified export. It remains local until explicitly
uploaded later.

### Remaining store work

Protected CI key import/upload, monotonic build-number allocation and actual
signed export/TestFlight processing remain open. Upload credentials are not
needed for the current sideload jobs or local structural checks. Before external
beta, finish the baseline-100 storage/startup work. Before any upload, complete
the Apple distribution-rights check and required privacy-manifest/API audit;
background-mode/usage-string review, disclosures and review materials also remain
in the public-distribution plan. Local archive success alone does not complete
those gates or validate store-delivered behavior on hardware.

## macOS release app

The **Build apps** workflow builds a Release app for Apple Silicon (`arm64`) and
Intel (`x86_64`), requiring macOS 14 or later and embedding the shared frontend.
Stable/nightly GitHub releases retain the universal ZIP alongside the IPA/APK.
Master pushes and manual master runs publish the rolling Nightly after all three
sideload builds succeed.

The app keeps bundle identifier `fail.acheron.float-view.macos`. Release packages
use a **Developer ID Application** identity, hardened runtime and a secure timestamp.
Apple's notarization service must accept the upload before the ticket is stapled
to the app. CI validates the ticket, signature, selected Apple team and Gatekeeper
assessment, then creates the final ZIP from the stapled app. A rejection, timeout,
credential failure or failed validation prevents artifact/release publication;
there is no self-signed fallback. Notarization waits at most 20 minutes and logs
the submission ID for tracing a timed-out request.

CI imports the certificate only into a temporary keychain on a disposable
GitHub-hosted macOS runner. It neither installs a custom trust root nor modifies
a developer's keychain. The temporary keychain and private files are removed
before artifact upload. Local `just run macos` builds retain ad-hoc development
signing and do not require notarization credentials.

### Release credentials

Configure the repository's Actions variable and secrets before enabling the
updated workflow. Account details and personal team identifiers stay out of source:

| Setting                        | Type             | Value                                                                                        |
| ------------------------------ | ---------------- | -------------------------------------------------------------------------------------------- |
| `APPLE_TEAM_ID`                | Actions variable | Publishing Apple Developer team ID                                                           |
| `MACOS_CODE_SIGN_P12_BASE64`   | Actions secret   | Base64 PKCS12 containing only the selected Developer ID Application identity and private key |
| `MACOS_CODE_SIGN_P12_PASSWORD` | Actions secret   | Password protecting that PKCS12                                                              |
| `APPLE_NOTARY_APPLE_ID`        | Actions secret   | Apple Account email belonging to the selected team                                           |
| `APPLE_NOTARY_PASSWORD`        | Actions secret   | Apple app-specific password, not the account password                                        |

Replace the earlier self-signed PKCS12; do not retain its custom trust policy.
Back up the selected identity and its password securely. The local provisioning
backup is ignored under `.local/macos-signing/`; GitHub secret values cannot be
read back. Export only the intended identity, never all identities in a keychain.
Certificate renewal can use Apple's normal team-based signing requirement rather
than anchoring app identity to one certificate fingerprint.

Create app-specific passwords at [Apple Account](https://account.apple.com/).
After membership or account-security changes, validate credentials again rather
than bypassing a failed notarization. Never paste credentials into issues/chat or
commit them. Use the Actions settings UI or interactive `gh secret set NAME`.

### Local release verification

Store notarization credentials interactively (the command prompts for the app-specific
password); keep email/team values local:

```sh
xcrun notarytool store-credentials float-view-notary --apple-id YOUR_APPLE_ACCOUNT_EMAIL --team-id YOUR_TEAM_ID
```

Build the frontend and copy its embedded artifact with `just build`, then run:

```sh
APPLE_TEAM_ID=YOUR_TEAM_ID CODE_SIGN_IDENTITY=YOUR_DEVELOPER_ID_FINGERPRINT \
  FLOAT_VIEW_VERSION=YOUR_VERSION FLOAT_VIEW_BUILD_NUMBER=YOUR_BUILD_NUMBER \
  bash backends/apple/build-macos-release.sh
```

The selected identity must already be in your local keychain; no certificate is
imported by this command. Set `APPLE_NOTARY_KEYCHAIN_PROFILE` to use another profile.
The build needs both pinned macOS Rust targets. The final output is
`FloatView-<version>-macos-universal.zip`. To notarize an already assembled and
correctly Developer ID-signed Release bundle, use `notarize-macos-release.sh APP ZIP`
with `APPLE_TEAM_ID` and the same profile. That command does not make an ad-hoc or
self-signed binary eligible: its Apple identity is verified before submission.

After successful signed-artifact verification, install a quarantined download on
a supported Mac and check normal launch, Bluetooth/location permission presentation,
recording and export. Switching from the old self-signed identity may require
permission regrants; software signature tests do not prove permission retention
across the change. Old downloaded ZIPs remain unnotarized; notarization applies to
newly generated packages, without replacing immutable stable release assets.

## AltStore installation

Run `just ipa` on macOS to build the frontend and iOS device app in Debug, as
`just run ios <device>` does, and package
`FloatView-YYYYMMDD-HHMMSS.ipa` in the directory where you invoke `just`, using
the current local date and time. No connected device or Xcode signing
account is required: AltStore signs the IPA when installing it. The archive omits
macOS metadata files that can confuse AltStore's app detection.

GitHub Actions builds master pushes and manual master runs as a
rolling [Nightly prerelease](https://github.com/acheronfail/float-view-mobile/releases?q=prerelease%3Atrue).
A nightly such as `v0.1.50-nightly.12.gabcdef012345` uses the next patch after the
greatest stable ancestor, the commit count since that tag and the source hash.
Unchanged successful commits are skipped. Only the current Nightly asset set is
retained. Its release tag is the full SemVer nightly version, allowing GitHub to
sort it above older stable releases. Source tags are immutable, while the same
Nightly release is updated in place. Nightly never replaces the stable Latest release.

Push an explicit `vX.Y.Z` tag on master history, or run
`just release patch|minor|major` from clean, current master, to publish a permanent
stable [GitHub release](https://github.com/acheronfail/float-view-mobile/releases).
The helper fetches tags, calculates the next version, creates an annotated tag
and pushes it without changing source. Minor and major bumps reset the patch.
Stable mobile packages use Release configuration; nightly mobile packages use
Debug. macOS packages always use Release. All three packages must succeed before
publication. Published stable assets are preserved on reruns; failed drafts can
be resumed.

Package names include the full display version, for example
`FloatView-v0.2.0.ipa` or `FloatView-v0.2.1-nightly.3.gabcdef012345.ipa`, with
corresponding APK and universal macOS ZIP names. Settings displays the same version
and commit revision. Native Apple marketing versions omit the nightly suffix;
app and extension build numbers use the independent workflow counter. These
outputs remain AltStore/sideload/self-signed packages, not app-store submissions.

Locally, `FLOAT_VIEW_VERSION=v0.2.0 FLOAT_VIEW_BUILD_NUMBER=60 just ipa` produces
a versioned package with marketing version `0.2.0` and build `60`; add
`FLOAT_VIEW_BUILD_CONFIGURATION=Release` for stable packaging. Nightly display
versions are accepted with the same separate build-number requirement. Without a
version, local builds keep timestamp filenames and display “Development build”.
Historical releases were renamed without rebuilding their binaries, so their
embedded metadata remains original.

To build master manually, open **Actions → Build apps → Run workflow**. Download
the `FloatView-ipa-<full version>` artifact from the run summary. Artifacts are
retained for 14 days; release assets remain available independently of that expiry.
The [workflow](../../.github/workflows/build-apps.yml) uses Node.js 24, macOS runners for
Apple builds and the pinned Rust toolchain. It fetches locked dependencies before
offline native builds and caches frontend downloads and Rust build outputs.
No connected device is required. AltStore signs the IPA when installing it;
macOS uses Developer ID signing and mandatory notarization.

Transfer the IPA to Files on the iPhone and select it in AltStore's My Apps tab
using the + button. If switching from a `just run ios` installation fails, export
any recordings you want to keep, then remove that installation before retrying.
Install future IPA updates through AltStore and use AltStore to refresh them.

## iOS Live Activity

A ready board connection starts a Live Activity while the app is foregrounded
and Live Activities are enabled in iOS Settings. Standalone BMS connections do
not start one. The compact/minimal Dynamic Island shows the primary gauge's
winning drive-load percentage; the expanded Island and Lock Screen also show
distance in the selected units, battery charge and pack voltage to one decimal
place. The summary uses stable headings above the values and secondary details:
Load / percentage / Duty, Motor or Battery; Distance / number / km or mi;
Battery / charge / voltage; and Temp / temperature / hottest component.
When stopped, the first section shows Board / Stopped instead of load.
Four columns switch to a two-by-two grid when space is tight or text is large.
The grid moves the footpad indicator beside Load or Board and omits the app heading.
Text scaling is bounded by role to keep the grid within ActivityKit's height limit,
with the main values scaling up to XXXL. Temperature is shown in white, rounded to
whole degrees in the selected units, beside a coloured thermometer in the Temp heading.
The icon colour matches the temperature bars: green
at 30°C or below, through orange to red at 60°C (45°C for battery cells).
Missing readings stay neutral; VoiceOver also names the sensor. Selection matches the in-app highest
temperature widget, excluding detached or five-second-old BMS observations.
In Xcode, open `ios/FloatViewLiveActivity/FloatViewLiveActivity.swift` and enable
the Canvas for Lock Screen previews of riding, stopped, imperial and unavailable
readings, plus expanded Island, stale-reading and narrow-width/default/XXXL previews.

The dark gauge palette is shared visually: duty yellow, motor pink, battery current green. Charge uses the
same estimate/controller-reported selection as the frontend power widget.

The shared service emits a coalesced native presentation effect. The iOS host
keeps only its latest value and checks it once per second, independently of the
WebView and loopback listener. It submits changed displayed numbers/colours at
most once every five seconds, comparing the widget's formatted strings rather
than raw precision or observation timestamps. Board status, footpad and freshness
transitions bypass this delay on the next worker iteration. Unchanged live
readings refresh at ten seconds so their stale deadline continues to advance.
These intervals are an experiment to reduce redundant ActivityKit submissions,
not documented iOS refresh guarantees. Readings become
unavailable after 15 seconds without telemetry. iOS controls rendering cadence
and background execution; this is not a replacement for the in-app live gauge.
Disconnect ends the activity immediately on the next worker iteration. A
connection established in the background starts its activity on the next
foreground opportunity. User dismissal is respected for that connection, and
orphaned activities are removed on process launch.

`ios/FloatViewLiveActivity/` owns the WidgetKit UI, `ios/LiveActivityShared/` the
ActivityKit content model, and `ios/float-view/live-activity/` the OS lifecycle.
The extension is embedded in both simulator and device app packages.

ActivityKit submission diagnostics use unified logging with subsystem
`fail.acheron.floatview`, category `LiveActivity`. `update_begin` records the
reason and source age; `update_end` records the await duration and source age at
completion. Log timestamps supply submission start/completion times. A source
age of `-1` means unavailable; ages are capped at one day. Logs contain no
telemetry values, identifiers or tokens and are not part of `.floatview` exports.
Completion means the ActivityKit call returned, not that the Island rendered it.

For a device comparison, run a Debug build normally for the five-second policy,
then launch the same build with Xcode scheme environment variable
`FLOAT_VIEW_LIVE_ACTIVITY_BASELINE=1` for the original one-second/raw-content
comparison. `worker_started` logs the selected mode; Release ignores the switch.
Capture device Console logs filtered to the subsystem/category, alongside a
timestamped observation of compact/expanded Island and Lock Screen behaviour.
Compare fresh runs lasting at least several minutes, with the same board,
background location setting and power conditions, without an attached debugger.
Record the iOS version and whether Low Power Mode is enabled. Correlate with
`liveactivitiesd` and `chronod` logs where available. Timely submissions with
delayed visible changes point to system presentation; gaps in submission times
or long await durations identify a different bottleneck.

`NSSupportsLiveActivitiesFrequentUpdates` remains unset: Apple documents it for
[remote push updates](https://developer.apple.com/documentation/BundleResources/Information-Property-List/NSSupportsLiveActivitiesFrequentUpdates),
while this activity uses local updates only.

Physical checkpoint: on a Dynamic Island iPhone with Live Activities allowed,
connect a board, compare load selection/colours, distance and charge with the
primary UI, then lock the phone and verify updates while BLE runs in the
background. Check expanded and compact Island layouts, stale telemetry,
disconnect/reconnect, user dismissal and disabled Live Activities. Simulator
builds and service tests do not establish this OS/background behavior.

## Spoken announcements

The shared Rust service owns load/speed eligibility and timing. The service-owned
`speech/` adapter executes bounded typed utterances on the main queue using an
installed system voice, leaving the utterance voice unset for the system default.
The default reported to apps may differ from the Spoken Content selection; voices
must be downloaded and exposed by iOS to be available to this adapter.
iOS and macOS do not expose the Voice settings action. On iOS 18.7.8, the tested
undocumented Settings links only launched Settings without navigating when it was
not already open; apparent navigation was retained Settings state. The UI includes
the manual Spoken Content (Read & Speak) → Voices path and download instructions.
The connected iPhone audibly used Lee after its download completed.
iOS declares background audio and uses a short playback voice-prompt session per
utterance. “Lower other audio while speaking” selects
ducking/spoken-content interruption or mix-only behavior; all terminal paths
stop speech and release the session. macOS uses native synthesis without iOS
audio-session APIs. Neither host changes system volume or keeps silence playing.
Calls, route changes and unavailable audio can skip an announcement; successful
call playback is not guaranteed. The main-queue adapter owns one injected
`SpeechDriver` boundary for deterministic engine/session failure tests; the native
driver retains AVSpeechSynthesizer and iOS audio-session calls. Tests establish
resource cleanup, not audible behavior on every player or route. Completed phone
observations and optional user-owned manual checks are retained in conformance
physical-checkpoint evidence.

Foreground auto-connect uses the existing OS foreground facts: iOS scene
activation/background and macOS application activation/deactivation. Closing a
macOS window ends its foreground opportunity; reopening an active window reports
foreground again. Listener or WebView recreation alone does not rearm. The
shared Rust service owns the 15-second opportunity and exact saved association
matching; native adapters execute ordinary discovery effects.
