# Android privacy and Play submission audit

This is a reusable source inventory and Console worksheet, not a claim of Play
approval or physical acceptance. Detailed records stay in ignored
`.local/android-store/readiness/`. Build 120 (`v1.1.0-rc.3`, source `fae159af`)
passed signed-AAB checks and the owner reports internal-track setup complete.
The disclosure, navigation, backup and user-agent corrections described below
require a newer build. Focused verification passed: 10 permission/navigation
JVM tests, Release packaging/lint, backup XML selection tests, and a native
Android emulator test for both same-window and new-window browser handoff.
These are software/emulator results, not physical BLE or cloud-restore acceptance.

## Data and network inventory

| Surface | Implementation and disclosure |
| --- | --- |
| Boards, settings, Tunes and rides | Shared Rust service stores private local data. Bluetooth addresses remain local routing associations. There is no developer telemetry server, advertising SDK or app account. |
| Location | Optional Android LocationManager GPS/network observations. Shared policy records only eligible riding samples and stops capture on loss of permission/eligibility. Recorded routes can include precise locations. |
| Maps | Shared Leaflet UI requests OSMF HTTPS tiles for viewed routes, including imported rides. Tile coordinates/zoom expose the viewed area; IP, user-agent and HTTP metadata reach the provider. Complete archives/raw coordinate lists are not uploaded. Maps can load automatically when opening route-bearing history. |
| WebView | Bundled HTML and authenticated HTTP/WebSocket on IPv4 loopback. Network security configuration permits cleartext only for localhost/127.0.0.1. Default WebView HTTP caching is retained; the host now appends FloatView/version to its user-agent for identifiable map requests. Native request/cache observation remains a checkpoint. |
| Speech | Android's selected TTS engine is used only with an installed, non-network English voice; no eligible voice produces an actionable speech issue. No microphone or speech recognition. Engine installation/downloads and platform services have their own policies. This does not certify arbitrary third-party engine internals. |
| Notifications | Local ride notification and optional promoted live updates. No remote push server or push token. |
| Diagnostics | Bounded local structured logs and explicit user-exported troubleshooting ZIPs; no automatic upload. Public GitHub issue attachments are user-controlled and must not be represented as private support. |
| Files | System-selected import/export streams and private staging. No broad storage permission. User-selected cloud providers have their own behavior. |
| Backup | Standard Auto Backup and device transfer include the complete owned settings/ride store, preserving its marker, SQLite databases and WAL files. Cloud backup requires encryption capability; legacy rules allow encrypted backup or device transfer. Diagnostics, caches, temporary exports, coordination locks, WebView state and permission-history hints are excluded. Cloud quota is 25 MB and can prevent backup of a large store. Actual provider restore/device transfer remains unverified. |
| Payment | Store purchase is handled by Google Play; no in-app billing SDK or developer payment-data endpoint. |

Public privacy URL: <https://github.com/acheronfail/float-view-mobile/blob/master/PRIVACY.md>.
Support: <https://github.com/acheronfail/float-view-mobile/issues>.
Inspect both public URLs and in-app access on the delivered build. Android now
routes public HTTP(S) links to the external browser, including user-activated
new-window links. Only the exact local backend document and fragment routes
remain in the embedded WebView. Non-web schemes, backend API navigation and
other loopback destinations are blocked. Popup routing is bounded to one short-
lived non-rendered WebView; it does not fetch external content.

## Draft Data safety answers

Use the actual Console questions and this inventory together. This is a proposed
classification of provider processing; do not copy Apple's categories verbatim.

| Question | Draft answer and basis |
| --- | --- |
| Collect/share user data | Yes: app-controlled map requests go to OSMF. Local-only storage by itself is not off-device collection. |
| Approximate and precise location | Collected and shared, optional; viewed tile areas can reveal location at different zoom levels, and IP can reveal an approximate area. |
| Device or other IDs | Conservatively include collected/shared request IP and browser metadata where it can identify a browser/device. No advertising, installation, Bluetooth or controller identifier is sent to the tile provider. |
| App interactions | Conservatively include collected/shared map-view request activity and identifiable app user-agent used in provider usage statistics. No developer event analytics endpoint exists. |
| Purposes | App functionality; provider service security/abuse prevention and stated aggregate analytics where applicable to the selected type. No advertising, marketing, personalization or account management. |
| Ephemeral processing | No claim: provider request logs can outlive an individual tile request. |
| Encryption in transit | Yes for app-controlled external tile traffic (HTTPS). Loopback HTTP remains on device. User-directed exports/external browser actions are distinct flows. |
| Deletion request mechanism | Do not promise deletion of provider logs through Float View. No dedicated off-device deletion request mechanism is currently implemented. Local ride deletion is separate. |
| Account creation | None; account-deletion URL requirements tied to app-created accounts do not apply to this implementation. |
| Independent security certification | No; packaging/source checks are not a paid independent security certification. |

OSMF is a third party, without an established processor-only arrangement with
this project. Do not claim the service-provider sharing exception. Do not assume
a user expects map-area sharing simply because they opened history: maps can
appear automatically. Opening public web links and explicit user-directed file
exports are separate from these app-controlled tile requests.

Purely local telemetry, board identifiers, route databases, settings and local
logs do not by themselves require off-device collection declarations. No
app-authored flow collects contacts, microphone recordings, messages, advertising
IDs, installed-app inventories or financial details. Reassess if support uploads,
network speech, SDKs or other data paths change.

## Location and foreground services

The current product records location when a saved connected board reports riding.
A new riding segment can start while the UI is backgrounded; reconnect/permission
changes can also require location capability to resume. Android and the shared
policy explicitly distinguish foreground-only from background authorization.
Removing ACCESS_BACKGROUND_LOCATION alone would stop background capture under
that policy. Keep it for this submission and explain the feature accurately.
A foreground-service-only design is a separate minimum-permission alternative
requiring changes to eligibility/recovery and relevant device checks; existing
code requiring a permission does not itself prove that no narrower design exists.

The native disclosure now precedes first foreground/background location requests
and settings recovery, with an explicit Continue/Open settings action and Cancel.
It describes location, background/locked-screen ride recording, optionality and
local records. Foreground disclosure also explains online map requests and the
actual precise-location/Location-switch requirement for Android board discovery.
It is not merely a system rationale shown after denial. Verify its presentation
on Android 10's runtime background grant and Android 11+'s settings path in the
new delivered build. Do not claim this correction is in build 120.

| Foreground type | Reviewer explanation | Impact of delay/interruption |
| --- | --- | --- |
| connectedDevice | User selects/connects a Bluetooth VESC board; service supports continuing telemetry, ride recording and board connection with the UI backgrounded. | Missing/discontinuous telemetry or recording and delayed board interaction. |
| location | User enables route recording and grants location; eligible rides capture route points while locked/backgrounded. | Gaps in route and inaccurate history; missed points cannot be reconstructed. |
| mediaPlayback | User enables spoken announcements; locally generated ride phrases are spoken through an installed offline voice while the UI is backgrounded. | User misses requested ride announcements; delayed phrases can be stale. |

Service starts when connected or recording, with types adjusted for granted
location and enabled speech. The native fallback preserves connected-device work
if location foreground promotion is not eligible. It is not a boot-launched
continuous tracker or silent-audio keepalive. No full-screen intent is declared.
Play's foreground-service form needs descriptions, interruption impact and an
Android video showing each claimed feature. A phone/Mac Apple review video is
not evidence of these Android permission and notification flows.

## Other Console fields and closed-test preparation

Owner selected ages **13 and up**: 13–15, 16–17 and 18+, excluding under-13 groups.
This is the target audience, separate from the IARC content-rating questionnaire.
Draft answers: no ads, no in-app accounts/sign-in, no app-hosted public user
content/chat, no news/government/financial service, and no medical/health feature
claims. Hardware is required for live readings; disclose that in reviewer access
instructions alongside Settings and imported history available without hardware.
Answer the actual rating questions from app content; do not invent a rating.

Before moving to closed testing:

1. Owner completes privacy/Data safety, access, target audience/rating, ads and
   applicable sensitive-permission/foreground-service declarations.
2. Produce a newer candidate containing the disclosure/map identifier changes;
   verify exact signed package and delivered Android behavior.
3. Verify external links and the new disclosure on the delivered build. Backup
   rule selection is software-tested; retain provider restore/device-transfer
   verification as a separate coverage limit.
4. Record an Android video showing disclosure, permission grant, board connection,
   optional route recording, background notification and spoken announcements.
   The Test announcement control uses the same speech pipeline and can produce
   an audible example without waiting for a natural alert. With a board connected
   and announcements enabled, show a phrase continuing after backgrounding if
   feasible. A foreground-only preview does not alone prove background playback;
   the reviewer decides whether the submitted demonstration is sufficient.
5. Create the closed track, select countries and tester email list/Google Group,
   provide a feedback channel and invite enough real testers to maintain at least
   12 continuously opted in for 14 days. Gather real usage feedback.
6. Internal testers must opt out of internal testing before joining the closed
   track. Paid closed-test apps normally require purchase; arrange an appropriate
   Play-supported access/promo route if testers will not purchase. Do not change
   the app permanently to free merely to recruit testers.
7. Verify actual opt-ins and track status in Console; creating a group or sending
   an invitation does not start everyone's 14-day qualification period. Production
   access remains a separate application and approval.

Sources: [Data safety](https://support.google.com/googleplay/android-developer/answer/10787469),
[background location](https://support.google.com/googleplay/android-developer/answer/9799150),
[foreground services](https://support.google.com/googleplay/android-developer/answer/13392821),
[testing tracks](https://support.google.com/googleplay/android-developer/answer/9845334),
[personal-account testing](https://support.google.com/googleplay/android-developer/answer/14151465),
[OSMF privacy](https://osmfoundation.org/wiki/Privacy_Policy) and
[tile usage](https://operations.osmfoundation.org/policies/tiles/).
