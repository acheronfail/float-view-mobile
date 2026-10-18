# Apple host

No personal development team is committed in the Xcode project. For signed
device builds, supply `DEVELOPMENT_TEAM=<team-id>` as an `xcodebuild` build-setting
override. Keep local signing/account configuration out of version control.
Unsigned packaging checks do not need a development team.

For `just run ios <device-id>`, set `APPLE_TEAM_ID` in your shell (add the
export to `~/.zshrc` to persist it outside the repository):

```sh
export APPLE_TEAM_ID=YOUR_TEAM_ID
just run ios <device-id>
```

The recipe passes it as a command-line override for both the app and Live Activity
extension without changing the project. Select your account in Xcode Settings >
Accounts, but avoid saving a team selection in the project's Signing &
Capabilities editor: that writes to the tracked `project.pbxproj` file.

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

## Embedded navigation

Both Apple WebViews stay on the backend's frontend document. Internal hash routes
are allowed; changing its origin, path or query is not. User-activated external
HTTP(S) links open through the OS in the user's default browser, including links
with a new-window target. Scripted external navigation, other URL schemes and
embedded subframes are cancelled. No additional WebView is created for popups.
Returning from the browser retains the app's current route.

## iOS App Store preparation

The GitHub `just ipa` route remains unsigned for AltStore to re-sign. Store
preparation is separate and always uses Release for the app and its embedded
Live Activity extension. These local preparation commands do not upload a build
or create a tag; protected CI delivery is described separately below.

### Account and signing setup

Configure the publishing account, app record and signing identities before
archiving. Keep account enrollment and credential provisioning history local.
See the [release verification summary](../../openspec/changes/prepare-public-distribution/verification.md)
for completed build checks; signing/export success is not TestFlight acceptance.

1. Install a supported [Xcode release](https://developer.apple.com/xcode/resources/).
   [Apple currently requires Xcode 26+ and the iOS 26+ SDK](https://developer.apple.com/news/upcoming-requirements/?id=04282026a)
   for uploads. The deployment target remains iOS/iPadOS 17; building with a newer
   SDK does not require raising that target. Set `DEVELOPER_DIR` for individual
   commands if keeping multiple Xcode installations. For example, if installed at
   `/Applications/Xcode-26.0.0.app`, use
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
one**: official CI builds use the single Build apps workflow counter plus 60,
shared across all platforms and the optional store job. Local test numbers are
not upload allocations. Never reuse an Apple-consumed number for a corrected binary.
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
profiles, device arm64 binaries and no debugger entitlement. Signed entitlements
are read through Security.framework, including Xcode 26 DER-only signatures; the
macOS 15 `codesign` XML display cannot read that representation. A successful export
writes `export.json` with source/version and the IPA hash; a partial directory
without that record is not a verified export. It remains local until explicitly
uploaded later.

### Protected CI store builds

The existing **Build apps** workflow owns one native number (`run_number + 60`)
for all platforms. Stable `vX.Y.Z` and numbered candidate `vX.Y.Z-rc.N` tags pushed by `just release ...` select
Release App Store signing and protected delivery alongside the three sideload
packages. Master pushes/manual master runs and nightly tags never select store
jobs, even if store inputs are requested. GitHub publication depends only on the
three sideload packages; store signing/delivery remain independent.

Each run retains a `build-record-<attempt>` artifact with source, semantic/native
version, run number, run ID, attempt and CI URL. Store `export.json` embeds that
record plus the actual build job/attempt and IPA hash. The store artifact name
includes its build number and attempt. These records link native build `N` back
to workflow run number `N - 60`; release notes also link the build to its CI run.
A rerun keeps its native number. Use a new workflow run for corrected binaries;
the delivery job queries Apple before considering a retry of that number.

Configure [the repository's environments](https://github.com/acheronfail/float-view-mobile/settings/environments)
before enabling this job. Use owner review and a tag-only `v*` deployment policy
for `apple-store-signing`. The preparation and Python
validators additionally require an exact stable `vX.Y.Z` or candidate `vX.Y.Z-rc.N` ref, excluding nightlies.
The iOS and native Mac jobs share this protected signing environment and one
`APPLE_DISTRIBUTION_P12_BASE64` / `APPLE_DISTRIBUTION_P12_PASSWORD` pair.
Provisioning profiles and readiness switches remain platform-specific; Mac
installer signing uses a separate identity. Keep secrets scoped to this environment. The job refuses to proceed until environment variable
`IOS_STORE_SIGNING_READY` is `true`; set it only after protection and secrets
are configured. The existing repository `APPLE_TEAM_ID` variable is reused.

Export **only** the selected Apple Distribution identity and private key as an
encrypted PKCS12 in Keychain Access → My Certificates: right-click the Apple
Distribution certificate with its private key, choose Export, and save it as
`.local/ios-store/ci-signing/distribution.p12`. Choose an export password for the
interactive secret prompt below. Create/download two **App Store Connect**
distribution profiles in the [Apple developer portal](https://developer.apple.com/account/resources/profiles/list),
selecting that certificate and the registered app/extension IDs. These profiles
contain no device list; development/ad-hoc and Xcode-managed profiles are rejected.
To create them in the portal:

1. Open Profiles and click **+**. Under Distribution choose **App Store Connect**.
2. Select `fail.acheron.floatview`, then the Apple Distribution certificate created
   for this team. Name it **Float View App Store**, generate it and download it.
3. Repeat for `fail.acheron.floatview.live-activity`, selecting the same certificate.
   Name it **Float View Live Activity App Store**, generate it and download it.

Xcode's automatic local export creates managed profiles, which Xcode refuses in
this manual CI route. Keep those managed profiles for automatic local signing;
create the two portal-managed profiles separately. Place the downloaded files
under ignored `.local/ios-store/ci-signing/` as `app.mobileprovision` and
`live-activity.mobileprovision`, then set the secrets yourself. First run this
interactive password command on its own and enter the PKCS12 export password:

```sh
gh secret set APPLE_DISTRIBUTION_P12_PASSWORD --env apple-store-signing --repo acheronfail/float-view-mobile
```

Then run the file uploads and enable readiness:

```sh
base64 < .local/ios-store/ci-signing/distribution.p12 | \
  gh secret set APPLE_DISTRIBUTION_P12_BASE64 --env apple-store-signing --repo acheronfail/float-view-mobile
base64 < .local/ios-store/ci-signing/app.mobileprovision | \
  gh secret set IOS_APP_PROFILE_BASE64 --env apple-store-signing --repo acheronfail/float-view-mobile
base64 < .local/ios-store/ci-signing/live-activity.mobileprovision | \
  gh secret set IOS_EXTENSION_PROFILE_BASE64 --env apple-store-signing --repo acheronfail/float-view-mobile
gh variable set IOS_STORE_SIGNING_READY --body true --env apple-store-signing --repo acheronfail/float-view-mobile
```

The password command prompts interactively; supply the password used for that
PKCS12 export. No private keys are uploaded by the agent. Keep a secure backup.
Only the explicitly selected protected job receives these secrets; normal
GitHub releases and local builds continue without them. No App Store Connect
API key is needed for this manual-profile build route.

`just release candidate` pushes a numbered prerelease tag for TestFlight before
the stable release; `just release patch|minor|major` retains stable publication
after candidate acceptance. Both select protected store jobs. To rebuild only store artifacts for an existing release, select its
actual stable or candidate tag (replace `vX.Y.Z` below); never use master:

```sh
gh workflow run build-apps.yml --ref vX.Y.Z -f ios-store=true --repo acheronfail/float-view-mobile
```

CI imports the distribution identity into a temporary keychain on a disposable
hosted Mac. It verifies each profile's app/team, expiry, distribution type and
authorized certificate before installing it. Secret environment values are
removed before Xcode runs. Both profiles and the temporary keychain are cleaned
up before artifact upload. The build uses manual profiles and exports with
automatic version changes disabled; no interactive account or cloud provisioning
is needed. Renewal means replacing the matching certificate/profiles together.
The same importer still handles the Mac Developer ID route with its existing
secret names; there is no change to the Mac signer or trust policy.

### Protected Apple delivery

The shared `apple-store-upload` environment has no required reviewer and keeps a
tag-only `v*` deployment policy. The single owner approval on
`apple-store-signing` authorizes signing and subsequent delivery for that run.
Exact stable/candidate tags are validated before credentials are used. Only the
delivery step receives the App Store Connect team
key; signing and sideload jobs do not receive it. A Developer-role team key can
access the team's apps, so keep its one-time `.p8` download and backups private.
Store `ASC_API_PRIVATE_KEY_BASE64` as an environment secret and `ASC_API_KEY_ID`,
`ASC_API_ISSUER_ID`, and the numeric `ASC_IOS_APP_ID` and `ASC_MAC_APP_ID` as
environment variables. Each delivery job maps its app ID to the helper's `ASC_APP_ID`.
The repository's `APPLE_TEAM_ID` is reused. Validate authentication, exact-app
lookup and consumed-number queries with a read-only delivery check before upload.

For an existing stable or candidate release, request a fresh build and read-only check
using its actual tag (replace `vX.Y.Z`):

```sh
gh workflow run build-apps.yml --ref vX.Y.Z -f ios-store=true -f ios-delivery=check --repo acheronfail/float-view-mobile
```

Approve the single `apple-store-signing` environment request for that exact source.
After successful signing and artifact verification, delivery proceeds automatically.
`check` independently verifies the exported signatures, profiles, privacy
manifests, version and digest, authenticates to Apple, checks the numeric app
record's bundle ID, and queries whether Apple already has that build number.
It never uploads. Existing published stable/candidate downloads remain immutable, so a
store-only retry can have a newer native number than those earlier downloads.
The original release run gives all four packages the same number. Signing/export
artifacts and delivery receipts in this public repository are not confidential.

Keep `IOS_STORE_UPLOAD_READY` unset until package, privacy and supported-storage
checks are accepted. Complete hardware instructions/video before external beta
review; they need not block the initial owner-only TestFlight upload. Then the owner can set it to `true`
in `apple-store-upload`. A new `just release ...` tag selects upload, subject to
the single signing approval; a deliberate retry uses the existing stable or candidate tag and
`ios-delivery=upload`. This makes a fresh candidate, uploads it once, and polls Apple
processing. It does not invite testers or submit beta/App Store review.

Each attempt retains a JSON receipt with source, version, native number, IPA
digest, run/attempt, transport status and Apple's build/processing identity for
90 days. It contains no private key or bearer token. A transport failure is
uncertain: inspect Apple and the receipt before retrying. Reruns never blindly
resubmit an absent number. An already received build is trusted only with a
matching earlier accepted-transport receipt from the same run and binary.
Rejected or corrected binaries need a fresh workflow run and number. Pending
processing is not successful delivery; check Apple before acting. Download
receipts needed for longer retention before Actions artifacts expire.

Only GitHub publication is serialized across runs. A store environment waiting
for approval therefore does not hold subsequent nightly builds in a queue;
existing older-run guards still prevent stale nightly publication.

### Mac App Store preparation

The native Mac app will use the separate paid record and bundle ID
`fail.acheron.floatview.macos`. The Developer ID direct-download app retains its
current identity. See [Mac distribution preparation](../../docs/macos-distribution.md)
for account/profile/installer-certificate setup, sandbox capability rationale,
the local `just mac-store-sandbox-check` command and data-transfer limits.

### Remaining store work

The protected CI environments and validated manual profiles are ready.
[Build 77](https://github.com/acheronfail/float-view-mobile/actions/runs/38977368838)
passed signing/export and independent downloaded-IPA verification, including
both bundles' signatures/profiles and the new privacy manifests. It includes the
baseline-100 storage work. The separately approved read-only delivery check
passed on attempt 2: Apple authentication and the exact app lookup succeeded,
and Apple returned no existing build for 1.0.0 (77). No upload occurred; actual
transport and Apple processing remain unverified. A future upload action creates
a fresh candidate and number. Corrected source needs a new stable or candidate release tag;
never move an existing published tag. The owner recorded permission for official Apple
distribution;
third-party notices and source obligations retain their own terms. The privacy
audit and Xcode aggregate report are documented in
[the iOS distribution audit](../../docs/ios-distribution-audit.md).
Finish its remaining gates, including storage/failure presentation checks,
corresponding-source/package notices, beta metadata and supported-hardware review
materials. Local archive success does not establish Apple's acceptance or
store-delivered behavior on hardware.

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
Before v1.0.0, a nightly uses `v1.0.0-nightly.<count>.g<hash>` with the count
of all reachable repository commits. After v1.0.0, it uses the next patch after
the greatest stable ancestor and counts commits since that tag.
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
downloads remain AltStore IPAs, signed APKs and notarized Developer ID Mac ZIPs.
Only stable or numbered candidate release tags select the separate store signing/delivery jobs;
master/nightly runs never do. Signing alone does not upload; protected delivery
requires its own review and readiness.

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
with the main values scaling up to XXXL. Temperature uses the primary text colour, rounded to
whole degrees in the selected units, beside a coloured thermometer in the Temp heading.
The icon colour matches the temperature bars: green
at 30°C or below, through orange to red at 60°C (45°C for battery cells).
Missing readings stay neutral; VoiceOver also names the sensor. Selection matches the in-app highest
temperature widget, excluding detached or five-second-old BMS observations.
In Xcode, open `ios/FloatViewLiveActivity/FloatViewLiveActivity.swift` and enable
the Canvas for Lock Screen previews of riding, stopped, imperial and unavailable
readings, plus expanded Island, stale-reading and narrow-width/default/XXXL previews.
The light preview also covers riding, stopped, stale and large-text layouts.

The Lock Screen card follows the app's light/dark preference (including System),
with adaptive text, background and footpad colours. The native ActivityKit worker
forwards the resolved appearance because iOS 26 can report a wallpaper-based dark
appearance to the widget even in system light mode. Appearance changes bypass
the telemetry coalescing delay on the next worker iteration. While iOS suspends
the app, updates wait until the host can run again. Load colours match the frontend gauge palettes:
light mode uses darker amber, rose and green; dark mode uses yellow, pink and mint.
The Dynamic Island keeps the dark palette against its black surface. Charge uses the
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
iOS and macOS do not expose the Voice settings action. Use the manual Spoken
Content (Read & Speak) → Voices path and download instructions; undocumented
Settings links do not reliably navigate to the required screen.
iOS declares background audio and uses a short playback voice-prompt session per
utterance. “Lower other audio while speaking” selects
ducking/spoken-content interruption or mix-only behavior; all terminal paths
stop speech and release the session. macOS uses native synthesis without iOS
audio-session APIs. Neither host changes system volume or keeps silence playing.
Calls, route changes and unavailable audio can skip an announcement; successful
call playback is not guaranteed. The main-queue adapter owns one injected
`SpeechDriver` boundary for deterministic engine/session failure tests; the native
driver retains AVSpeechSynthesizer and iOS audio-session calls. Tests establish
resource cleanup, not audible behavior on every player or route. Brief phone
outcomes and reusable procedures are retained in conformance physical-checkpoint
summaries; detailed device results remain local.

Foreground auto-connect uses the existing OS foreground facts: iOS scene
activation/background and macOS application activation/deactivation. Closing a
macOS window ends its foreground opportunity; reopening an active window reports
foreground again. Listener or WebView recreation alone does not rearm. The
shared Rust service owns the 15-second opportunity and exact saved association
matching; native adapters execute ordinary discovery effects.
