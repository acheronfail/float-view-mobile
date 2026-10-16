# iOS distribution audit

Repository audit: 17 October 2026. This records implementation evidence and open
checks; successful packaging is not Apple processing or physical beta acceptance.

## Privacy and network inventory

| Surface | Implementation and treatment |
| --- | --- |
| Bluetooth and telemetry | Native CoreBluetooth communicates with the selected VESC. Saved board identity, settings, tunes and recordings are stored by the shared Rust service in app-private SQLite files. No developer telemetry endpoint, advertising or analytics SDK is configured. |
| Route recording | Optional CoreLocation observations are recorded during rides. `CoreLocationAdapter` uses Always permission for recording that may restart with riding while locked/backgrounded; denying location does not prevent ordinary BLE telemetry. |
| Maps | Leaflet requests `https://tile.openstreetmap.org/{z}/{x}/{y}.png` for viewed recorded routes, up to zoom 19. Tiles can identify precise viewed areas; IP address, user-agent and HTTP metadata reach OSMF. The raw route/telemetry record is not uploaded. |
| Speech | AVSpeechSynthesizer speaks locally generated ride phrases; there is no microphone capture or speech-recognition upload. Platform voice downloads remain platform services. |
| Live Activity | ActivityKit local content updates, without push tokens, remote updates or a server. The extension reads only the supplied activity state. |
| Web frontend | Bundled single-file HTML talks to authenticated loopback HTTP/WebSocket endpoints. This communication remains on the same device. Only theme/accent hints are held in localStorage; product data belongs to Rust. |
| Imports/exports | User-selected imports are staged in private storage before Rust reads them. User-selected exports may contain precise route data. Shared-file copies and platform backups have separate lifetimes. |
| Support/TestFlight | Support links open public GitHub Issues. Apple provides TestFlight crash/usage/feedback information separately from app-initiated collection; the privacy policy explains this. A feedback email is still needed in App Store Connect, even when public support uses Issues. |

The public policy is [PRIVACY.md](../PRIVACY.md), linked from README and shared
Settings. Proposed console URL after pushing:
`https://github.com/acheronfail/float-view-mobile/blob/master/PRIVACY.md`.
Support URL: `https://github.com/acheronfail/float-view-mobile/issues`.
Verify both anonymously after publication before entering them in Apple metadata.

Do not answer “Data Not Collected” solely because there is no developer server.
The app manifest conservatively declares precise location and other data (IP and
HTTP request metadata) for map functionality, linked and not used for tracking.
This accounts for the tile provider's retained IP/request logs without promising
anonymity. Review the matching App Privacy answers in the console before release;
there are no advertising/tracking domains and no ATT request is needed for the
audited functionality. Purely on-device board/ride records are not listed as
off-device collection. Support and TestFlight disclosures remain explicit.

OSMF requires visible attribution, application identification, normal HTTP cache
behavior and no bulk/offline tile downloads. The existing map retains visible
attribution and uses normal WebKit HTTP caching; the iOS user-agent now includes
FloatView/version. No prefetch/offline feature or cache-bypass headers exist.
Check actual provider headers/cache behavior on an iOS device before completing
the map-service checkpoint. OSMF offers no availability guarantee. Sources:
[tile policy](https://operations.osmfoundation.org/policies/tiles/),
[provider privacy policy](https://osmfoundation.org/wiki/Privacy_Policy).

## Required-reason APIs and manifests

The host app's `PrivacyInfo.xcprivacy` covers its Swift code and statically linked
Rust/SQLite implementation. There are no external Swift packages or embedded
third-party binary SDK frameworks. The frontend bundles Svelte/Leaflet; dev tools
and research vendor submodules are not copied into the app. Rust dependencies
come from the locked Cargo graph, including rusqlite/libsqlite3-sys and standard
library filesystem/time implementations.

| Category | Reason | Actual use |
| --- | --- | --- |
| SystemBootTime | `35F9.1` | SharedService timer deadlines/ordering, permission timeouts, speech cooldowns, ActivityKit update duration and Rust Instant timers. Exported timings are elapsed between app events, rather than raw system boot times. |
| DiskSpace | `E174.1` | SharedService checks available capacity; shared policy prevents recording/maintenance writes when space is insufficient. Raw available-byte facts stay inside the local runtime. |
| FileTimestamp | `C617.1` | Rust/SQLite stat/fstat/lstat and app-private schema-cache metadata/eviction, staging and file bounds. User imports are copied into private staging before Rust reads them. |

The separate Live Activity extension has its own manifest with no collection,
tracking or app-authored required-reason API calls. It renders supplied ActivityKit
content, including SwiftUI timer views; it does not link the Rust service or host
adapters. No direct UserDefaults or active-keyboard API use was found. Web
localStorage is not NSUserDefaults. Do not add speculative reasons for APIs that
the app does not use. The archive/export validator requires both packaged
manifests to match the reviewed source manifests.

Sources: Apple's [required-reason API list](https://developer.apple.com/documentation/bundleresources/app-privacy-configuration/nsprivacyaccessedapitypes/nsprivacyaccessedapitype),
[manifest packaging guidance](https://developer.apple.com/documentation/bundleresources/adding-a-privacy-manifest-to-your-app-or-third-party-sdk),
[collection categories](https://developer.apple.com/documentation/bundleresources/app-privacy-configuration/nsprivacycollecteddatatypes/nsprivacycollecteddatatype).
A real unsigned Release archive passed with Xcode 26.0/SDK 26.0; both manifests
were bundled, host file-stat/disk-space symbols were present and no required-reason
symbols were found in the extension. The Release executable contains no WebView
inspection selector. Xcode Organizer's aggregate Privacy Report was generated
and reviewed from the baseline-100 Release archive on 2026-10-17. It lists
Precise Location and Other Data Types for app functionality, linked to the user,
with no tracking, matching the reviewed manifest. The report is retained under
ignored `.local/ios-store/baseline-archive-check-20261017/`.
The next signed CI export must pass the same packaged-manifest checks; resolve
any Apple processing findings. Local inspection does not establish Apple's acceptance.

## Permissions, background execution and encryption

- `bluetooth-central` supports active BLE telemetry/reconnection while backgrounded.
- `location` supports the optional route recording described above. Usage strings
  distinguish foreground and background recording. Always permission is requested
  through an explicit user recovery flow, not merely at app launch.
- `audio` supports user-enabled spoken ride announcements with an audio session;
  it is not a silent-audio keepalive. There is no microphone/recording declaration.
- NSSupportsLiveActivities is enabled; frequent remote updates, push notification,
  App Group and iCloud capabilities are not requested.
- Release WebViews no longer enable Safari inspection. Fixture adapters and
  performance/reset hooks remain behind build conditions. Mac launch overrides
  still need their separately tracked production-configuration work.

The iOS Info.plist declares `ITSAppUsesNonExemptEncryption=false`: this app uses
platform HTTPS for maps, platform Bluetooth security and standard hashes (SHA-1
for WebSocket handshakes, SHA-256 for schema/content identity), with no custom
data-encryption implementation or bundled TLS stack in the locked iOS graph.
This declares no **non-exempt** encryption, rather than claiming HTTPS uses none.
Re-audit whenever dependencies/networking change and answer any applicable
territory-specific questions in App Store Connect. Source:
[Apple's beta export-compliance guidance](https://developer.apple.com/help/app-store-connect/test-a-beta-version/provide-export-compliance-information-for-beta-builds/).

## Gates still open before TestFlight

- Baseline 100 and software preservation/startup checks are implemented; complete
  the scoped device/container and failure/retry presentation checkpoint before
  inviting external users. See storage-baseline.md and verification.md.
- Finish the iOS corresponding-source/package-notice check and document the
  owner's recorded official Apple distribution permission alongside AGPL source
  licensing. Credited dependencies and artwork retain their own terms.
- Run the prepared protected read-only delivery check to verify authentication,
  exact-app lookup and consumed-build queries. Upload credentials are configured
  separately from signing; upload readiness remains disabled.
- Verify a fresh signed CI candidate, then validate Apple processing and
  encryption status when upload is authorized. Build 75 predates these changes.
- Fill beta description, feedback email, what-to-test notes and any required beta
  review contact privately in App Store Connect. First tester is the owner;
  invited external users come later and need the applicable external beta review.
- Provide real supported-hardware pairing/use instructions and review evidence;
  test the delivered build's BLE, optional route permissions/background capture,
  announcements, Live Activity, import/export and restart on the affected device.
