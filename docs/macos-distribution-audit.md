# Mac privacy and map audit

This supplements the [iOS inventory](ios-distribution-audit.md) for the native
sandboxed Mac app. The shared frontend, Rust service, telemetry storage and map
provider are the same. The separate Mac App Store record needs its own saved
App Privacy answers; iOS answers do not populate it automatically.

## Data and network behavior

There is no account, developer telemetry endpoint, advertising SDK or tracking
SDK. Native Bluetooth telemetry, recordings, settings and optional route samples
remain in local app-owned storage until the user exports them. OS backups and
user-selected exports are covered by the repository [privacy policy](../PRIVACY.md).
The store container differs from the direct-download storage root; use supported
export/import rather than moving private storage files between them.

Opening a route map requests HTTPS tiles from OpenStreetMap. Tile coordinates
reveal the displayed area; the provider also receives the connection IP address
and request metadata. These requests bypass no privacy disclosure merely because
they originate in WebKit. The Mac worksheet therefore matches the conservative
iOS declarations, including the provider's aggregate usage analytics:

| App Privacy answer | Mac declaration |
| --- | --- |
| Precise Location | App Functionality and Analytics; linked to user; no tracking |
| Other Data Types | App Functionality and Analytics; linked to user; no tracking |
| Tracking | No |

These declarations concern map-provider processing. Local ride telemetry is not
uploaded to the developer. Reassess the worksheet if the provider or network
behavior changes. See [Apple's disclosure definitions](https://developer.apple.com/app-store/app-privacy-details/).

The shared map uses the standard HTTPS XYZ endpoint and visible contributor
attribution. It does not bulk-download or offer offline tile packs. The native
Mac WebView appends `FloatView/<version>` to its user agent and uses persistent
WebKit HTTP caching. Controlled native WebKit checks verify the identifying
user agent, absence of cache-bypass headers and reuse of a cacheable tile across
WebView recreation. Chromium/WebKit browser checks intercept synthetic requests;
they do not contact the live tile service. These checks support the
[OSM tile policy](https://operations.osmfoundation.org/policies/tiles/) audit.

Mac navigation callbacks use the SDK's actor/sendable completion signatures so
WebKit actually invokes the policy delegate. Native registration and shared
navigation-policy tests verify the backend-document boundary: external user
links open the external browser and embedded navigation remains restricted.

## Native differences and required-reason APIs

Mac uses CoreBluetooth, optional CoreLocation and local AVSpeechSynthesizer
output. It does not ship the iOS Live Activity extension, use iOS audio-session
background modes, request a microphone or add speech recognition. Permission
requests remain driven by the relevant user action. Store entitlements remain
the bounded capabilities listed in [Mac distribution preparation](macos-distribution.md).

The shipped Swift/Rust source and Release imports were audited for required-reason
API categories. The Mac bundle includes its own manifest:

| Category | Declared reasons and actual use |
| --- | --- |
| System boot time | `35F9.1`: elapsed-time measurement for service/timing behavior |
| Disk space | `E174.1`: low-space recording prevention and user-visible recovery |
| File timestamps | `C617.1`: owned databases/cache/staging; `3B52.1`: native Open/Save access to explicitly user-selected files outside the container |

File timestamps are not transmitted off-device. There is no project use of the
UserDefaults or active-keyboard categories. Keep the source/symbol audit and
manifest checks current when dependencies or native file behavior change. Reason
definitions are in [Apple's required-reason documentation](https://developer.apple.com/documentation/bundleresources/app-privacy-configuration/nsprivacyaccessedapitypes/nsprivacyaccessedapitypereasons).

The Mac Info.plist declares `ITSAppUsesNonExemptEncryption=false`, matching the
iOS rationale: platform HTTPS and Bluetooth security plus standard hashes, with
no custom data encryption or bundled TLS stack in the locked runtime graph.
See [Apple's TestFlight export-compliance guidance](https://developer.apple.com/help/app-store-connect/test-a-beta-version/provide-export-compliance-information-for-beta-builds/).

## Package and acceptance boundary

Packaging verifies the source-matching Mac manifest, export declaration, full
notices, production entry-point exclusion and exact sandbox/profile entitlements.
The protected uploader repeats these checks against the downloaded signed PKG.
A manifest or local software check does not establish Apple's processing or a
store-delivered permission/recording result. Upload a new candidate containing
these changes, inspect its signed resources, wait for Apple processing and then
perform the scoped TestFlight installation checks. Detailed local evidence and
device results remain under ignored `.local/macos-store/`.
