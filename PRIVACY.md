# Float View privacy policy

Updated 17 October 2026.

Float View is an open-source app maintained by acheronfail. It connects to your
VESC devices, displays telemetry and optionally records rides and routes. There
is no Float View account, advertising, tracking SDK or developer-operated
telemetry upload service.

## Data on your device

Float View stores saved boards and Bluetooth associations, settings, battery and
motor configuration, Refloat tunes, ride telemetry and optional location records
in the app's private storage. Bluetooth device identifiers are used to recognise
and reconnect to your boards. Location recording is optional and requires your
permission. When enabled, it can continue during a ride while the app is in the
background or your screen is locked. It is not required for ordinary telemetry.

These records are not automatically uploaded to the developer. Your operating
system may include app data in its backups or device transfers according to its
settings and platform rules. Float View does not provide its own cloud backup.

## Online maps

Viewing a recorded route loads map images directly from the OpenStreetMap
Foundation's tile service. Requests include the map tile coordinates and zoom
level, your IP address and normal HTTP request metadata. The requested tiles
can reveal the area you are viewing, including precise locations at high zoom.
The app does not send the complete recording or a list of route coordinates to
the tile service. Map requests are made when viewing maps, rather than to
record a ride.

The provider may process and retain request logs under its
[privacy policy](https://osmfoundation.org/wiki/Privacy_Policy). Its tile servers
use a global network of caches. Avoid opening maps if you do not want to make
these requests. Telemetry and route recording do not require tile downloads.
Map imagery is best-effort and is not provided as an offline download service.

## Speech and platform services

Optional spoken announcements use the operating system's text-to-speech service.
Float View does not record microphone audio or provide speech recognition.
Voice downloads and, on Android, any network processing by your selected speech
engine are governed by the platform or engine provider's settings and policies.
Live Activities and notifications display ride information through the platform;
Float View does not use remote push notifications or a push server.

## Files you share and support requests

Exports can contain board information, ride telemetry and precise route data.
Files you choose to export, share or import are handled by your selected file
provider or recipient under their own terms. Exported copies are not removed
when you delete the original recording in Float View.

Support is available through
[GitHub Issues](https://github.com/acheronfail/float-view-mobile/issues).
Issues and attachments in this repository are public. Do not post private route
files, addresses, credentials or other sensitive information. GitHub processes
account and request data under its own privacy policy. Opening source, support
or licence links also makes requests to the relevant website.

## TestFlight testing

Apple processes TestFlight installation, usage, crash and feedback information
and makes certain testing information available to the developer. Feedback,
screenshots and crash reports may include personal information. See
[TestFlight & Privacy](https://www.apple.com/legal/privacy/data/en/test-flight/).
The developer uses information received through TestFlight to investigate issues
and improve Float View, not for advertising or sale. Do not include private
route data in feedback unless you intend to share it for that investigation.

## Retention and your choices

Local data remains until you delete it using the app, clear its storage or remove
the installation, subject to platform backup and restore behavior. You can turn
off route recording, revoke Bluetooth/location permission in system settings and
delete recordings. Revoking a permission does not delete previously recorded
data. Keep your own exports before uninstalling or changing installation routes.

Development builds before the supported public baseline may reset earlier
development data as described in the README. This does not delete exported
copies, reset operating-system permissions or erase third-party backups.

The developer retains support and TestFlight information while needed to
investigate reported problems and maintain the project; public GitHub history
and third-party retention are controlled by those services. Contact the
maintainer through GitHub Issues for privacy questions without posting sensitive
details. This document will be updated when the app's data handling changes.
