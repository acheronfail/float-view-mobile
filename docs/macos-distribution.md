# Mac App Store preparation

Float View uses separate paid iOS and macOS app records. The native Mac store
app uses `fail.acheron.floatview.macos`; it is not the iPhone app running on
Apple silicon and does not participate in iOS universal purchase.

The notarized direct-download app keeps `fail.acheron.float-view.macos` and its
Developer ID signing/notarization route. Store work must not replace that ZIP.

## Account and signing inputs

1. Register the explicit macOS identifier `fail.acheron.floatview.macos` in
   [Apple Developer identifiers](https://developer.apple.com/account/resources/identifiers/list).
2. In [App Store Connect Apps](https://appstoreconnect.apple.com/apps), use
   **New App**, macOS only, that bundle ID, an English primary language and
   an internal SKU such as `float-view-macos`. If the existing iOS record
   prevents reusing the name, use `Float View for Mac`. Do not add macOS to the
   iOS record when separate purchases are intended. See
   [Apple's app-record guide](https://developer.apple.com/help/app-store-connect/create-an-app-record/add-a-new-app/).
3. Create a **Mac App Store Connect** distribution profile for the identifier,
   selecting the publishing team's Apple Distribution certificate. Download it
   into ignored `.local/macos-store/`. The iOS app's profile and Developer ID
   certificate do not substitute for this profile. See
   [Apple's profile instructions](https://developer.apple.com/help/account/provisioning-profiles/create-an-app-store-provisioning-profile/).
4. Create a **Mac Installer Distribution** certificate using a locally generated
   **RSA 2048** key and CSR (the portal rejects RSA 3072 for this certificate),
   then download the certificate into the same ignored folder. Retain and
   securely back up its matching private key and password. The package signer
   is distinct from the Apple Distribution app signer. See
   [Apple's package-signing guidance](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution).
5. Independently validate the downloaded profile's team, exact app identifier,
   certificate, expiry, distribution type and granted entitlements. Match the
   installer certificate to its CSR/private key before importing/exporting it.
   Keep identities, fingerprints, profiles and operational records local.

No Mac helper uploads software, creates store records or sends private keys to
GitHub. The protected signing job is selected only by explicit stable/candidate
release tags and retains the existing cross-platform CI build-number allocation.
It is disabled until the repository variable `MACOS_STORE_SIGNING_ENABLED` is
`true`. Upload has its own readiness switch and does not run on ordinary master/nightly builds.

## Store packaging and protected CI

`just mac-store-package-check <new-output-directory>` rebuilds a universal
Release app using the real store bundle ID, checks resources and full legal
notices, and records source/version/build metadata. It uses ad-hoc signing and
is not uploadable. Set `FLOAT_VIEW_VERSION` to `vX.Y.Z` or `vX.Y.Z-rc.N` and an
explicit `FLOAT_VIEW_BUILD_NUMBER`; local test numbers are never CI allocations.
Do not launch this check as a substitute for a store-delivered installation.

`python3 backends/apple/mac-store.py package <new-output-directory> --profile
<profile-path>` requires clean committed source, `APPLE_TEAM_ID`, the Apple
Distribution certificate SHA1 in `CODE_SIGN_IDENTITY` and the matching
`MAC_INSTALLER_IDENTITY` name. `CODE_SIGN_KEYCHAIN` and
`MAC_INSTALLER_KEYCHAIN` select already-provisioned keychains when needed.
It rebuilds from source, validates the distribution profile, signs the app,
checks exact entitlements and Apple's certificate chain, signs the installer
and records its SHA256. Neither command uploads software.

Both iOS and Mac use one Apple Distribution identity in the shared protected
`apple-store-signing` environment; do not duplicate its certificate/password
under platform-specific names. Profiles and readiness switches remain separate.
Before enabling CI, create that GitHub environment with
a required reviewer and release-tag deployment rules. Configure these secrets:
`APPLE_DISTRIBUTION_P12_BASE64`, `APPLE_DISTRIBUTION_P12_PASSWORD`,
`MAC_INSTALLER_P12_BASE64`, `MAC_INSTALLER_P12_PASSWORD`, and
`MAC_STORE_PROFILE_BASE64`. Set environment variables `APPLE_TEAM_ID` and
`MACOS_STORE_SIGNING_READY=true`. Only then enable the repository switch
`MACOS_STORE_SIGNING_ENABLED=true`.

The job imports both identities into one disposable hosted-runner keychain,
checks their kind/team, removes signing secrets from the child environment,
and cleans up the keychain. The profile is temporary. It signs only after a
valid shared source/version/run/build record is verified, and retains the PKG
and `package.json` artifact separately from the public direct-download ZIP.
Manual retry uses the existing Build apps workflow on an immutable release tag
with the `mac-store` input; it does not select an iOS rebuild unless requested
separately. Master/nightly requests are rejected before signing credentials are
used. A Mac job failure does not block GitHub publication or iOS delivery.

## Sandbox feasibility package

Build an isolated universal Release app with the production Bluetooth adapter,
shared runtime and embedded frontend:

```sh
export DEVELOPER_DIR=/Applications/Xcode-26.0.0.app/Contents/Developer
export FLOAT_VIEW_VERSION=vX.Y.Z-rc.N
export FLOAT_VIEW_BUILD_NUMBER=900001 # Local check only; never a store allocation.
just mac-store-sandbox-check .local/macos-store/new-sandbox-check
```

Use a real valid release/candidate version and a new output directory. This
command rebuilds the frontend and universal Rust/Swift Release executable. Its
ad-hoc-signed app is explicitly **not uploadable** and uses the separate
`fail.acheron.floatview.macos.sandbox-check` container, preserving existing app
data. The JSON record captures exact executable hash, source/dirty status,
architectures and packaging checks; it does not claim runtime acceptance.
`--arch native` is available directly through `mac-store.py` for local iteration;
it does not establish universal packaging.

The signature contains only these sandbox capabilities:

| Entitlement | Purpose |
| --- | --- |
| `com.apple.security.app-sandbox` | Required Mac App Store isolation |
| `com.apple.security.network.server` | Existing authenticated loopback listener |
| `com.apple.security.network.client` | Native WebView loopback traffic and online map tiles |
| `com.apple.security.device.bluetooth` | Native board connection |
| `com.apple.security.personal-information.location` | Optional route recording |
| `com.apple.security.files.user-selected.read-write` | User-selected import/export and Save panels |

No broad filesystem exception, app group, persisted security-scoped bookmark,
debugger entitlement or second service owner is introduced. Release builds ignore
development frontend/storage/port overrides and exclude browser/test-host modes.
Debug/conformance builds retain those controls. The Mac privacy manifest is packaged from
`backends/apple/macos/Resources/PrivacyInfo.xcprivacy`. Its collection declarations
match iOS, with an additional required-reason declaration for user-selected
files outside the container. See the [Mac privacy/map audit](macos-distribution-audit.md).

## Storage and channel switching

Direct-download data lives under `~/Library/Application Support/FloatView`.
Sandbox data lives under the app's
`~/Library/Containers/<bundle-id>/Data/Library/Application Support/FloatView`.
Foundation resolves the container automatically; do not redirect production
storage with launch arguments or copy files around to bypass sandbox boundaries.
Both routes preserve owned-storage baseline 100 and later supported data.

Use supported recording export/import to move between routes. This does not
promise automatic settings, Tunes, permission or private Bluetooth-association
transfer; review preferences and reconnect hardware in the destination app.
Keep the original installation/data until exports are verified. Close one app
before opening the other: both use the existing loopback port and cannot run
simultaneously on that port. Do not uninstall or delete supported data to force
a channel switch or downgrade.

Ad-hoc rebuilds may require fresh OS permission grants. They do not establish
permission continuity for Apple-signed or store-delivered packages.

## Verified scope and remaining work

Local universal sandboxed Release checks passed: signature/entitlement and
architecture inspection, packaged frontend/icon/privacy manifest, development
entry-point exclusion, native WebView, authenticated HTTP/WebSocket, settings
and baseline archive persistence across relaunch, baseline archive HTTP
import/export, native Save-panel export outside the container and staging-lease
cleanup. A connected ESP fixture passed scoped direct connection/recording and
unexpected-disconnect recovery with canonical RESET/READY cleanup. These are
physical Mac Bluetooth checks against simulated board firmware, not validation
on a rider board or of a store-delivered app. Detailed build hashes, fixture
inventory, timing and reports remain under ignored `.local/macos-store/`.

[Candidate v1.0.0-rc.9, build 106](https://github.com/acheronfail/float-view-mobile/actions/runs/39025585402),
source `fae1bec2`, passed all platform builds, protected signing and shared-environment
Apple delivery. Both Mac and iOS builds processed as `VALID`. Independent Mac
PKG inspection verified Apple signatures/profile/entitlements, exact source/build,
universal executable, readable installed-bundle permissions, notices, privacy
manifest and icon. The Mac build is assigned to the owner's internal TestFlight
group. Detailed receipts, hashes, screenshots and extracted evidence remain local.

The [Mac source/API and map audit](macos-distribution-audit.md) is implemented and
verified in that signed candidate. The owner confirmed the requested Mac
TestFlight installation/runtime checks passed for build 106. Detailed physical
results stay local. The owner completed the Mac listing and six screenshots;
read-only verification confirms all six processed, Utilities, a 4+ age rating,
privacy/support URLs, review-contact presence and manual release. Before review,
remove iOS-only Live Activities claims from the description/review notes, supply
a Mac hardware demonstration, and select the final production build. The saved
video currently demonstrates iPhone hardware interaction. Pricing/availability
require owner verification because the Developer API key cannot read them. The
owner will handle stable release and submission; neither has occurred.

## Shared protected Apple upload

Both Apple platforms use `apple-store-upload`, one Developer-role team API key,
and a `v*` tag-only deployment policy. The required owner approval remains on
`apple-store-signing`; it authorizes signing and subsequent selected delivery
for that run. The upload environment has no additional required reviewer.
Keep the former environment during migration until both paths are verified.

Configure `ASC_API_PRIVATE_KEY_BASE64` as its sole secret. Configure
`ASC_API_KEY_ID`, `ASC_API_ISSUER_ID`, `APPLE_TEAM_ID`, `ASC_IOS_APP_ID` and
`ASC_MAC_APP_ID` as variables. Per-platform readiness remains independent:
`IOS_STORE_UPLOAD_READY` and `MACOS_STORE_UPLOAD_READY`. The delivery jobs receive
only their selected numeric app ID. Signing and sideload jobs do not receive the
API key. Keep its download and secure backup outside public artifacts.

For a new candidate containing the Mac audit and uploader changes, first request
an explicitly read-only check on its immutable tag:

```sh
gh workflow run build-apps.yml --ref vX.Y.Z-rc.N -f mac-delivery=check --repo acheronfail/float-view-mobile
```

This selects a fresh Mac signed build and number. Review the source and approve
`apple-store-signing`. The check verifies the exact signed PKG, embedded app and
profile, notices, privacy manifest, team, source/run/build and digest; it then
queries Apple's app record and consumed build number. It never uploads and can
run while Mac upload readiness and the repository upload switch are off.
RC7 predates these audit/record changes and must not be used to bypass validation.

After that check and owner acceptance, set the environment variable
`MACOS_STORE_UPLOAD_READY=true` and repository variable
`MACOS_STORE_UPLOAD_ENABLED=true`. An explicit `just release ...` stable or
candidate tag then selects Mac upload, subject to signing approval. A deliberate
store-only retry uses the actual immutable tag with `mac-delivery=upload`;
ordinary master/nightly builds never select this job. Neither flag is required
for the existing direct-download ZIP.

Delivery verifies the PKG again, uploads once using Apple's package transport,
and polls processing to `VALID`. It does not invite testers, submit review or
release the app. The receipt records the package digest, exact source/build/run,
transport outcome and Apple processing identity for 90 days. On uncertain
transport, inspect Apple and the receipt before retrying. A rerun never blindly
resends an absent number; an existing number is accepted only with a matching
prior accepted-transport receipt from that run and binary. Corrected, invalid or
uncertain absent builds need a fresh run and number. Preserve any receipts needed
beyond Actions retention. A Mac delivery failure does not block GitHub or iOS.

After Apple processing, install through TestFlight and scope physical acceptance
to Bluetooth/location permissions, a short recording, optional speech, native
import/export and supported direct/store transfer. Keep detailed results local;
public readiness notes need only the build/source outcome and coverage limits.
