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
`true`. Apple upload automation remains separate work.

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
Debug/conformance builds retain those controls. The existing Apple privacy
inventory is packaged from the iOS manifest source because the Rust runtime and
shared map/speech features are common; final Mac source/API and signed-package
privacy checks remain required.

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

The distribution profile and installer certificate/key pairing are validated.
A local preflight produced an Apple Distribution-signed universal app and a
Mac Installer Distribution-signed package, with exact signed sandbox/identity
entitlements, embedded profile, icon and privacy-manifest checks passing. This
reused the existing test executable and frontend; it is not a clean release
candidate, upload authorization, or store processing/install acceptance.

The repository packaging path also passed an actual universal Release
structure/notices check and focused profile/source/release-routing/identity
boundary tests. Hosted protected signing has not yet been exercised.

Still required: clean signed release-candidate package acceptance,
exact package/source/notices audit,
Mac required-reason API and map-provider/user-agent audit, protected release-only
CI signing/upload, TestFlight processing/install acceptance, optional location
and speech checks, actual direct/store transfer checks, Mac screenshots/listing
and privacy answers, and owner-controlled review/release.
