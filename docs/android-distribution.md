# Android distribution signing

Keep the application ID `fail.acheron.floatview` and preserve the existing
official APK signing identity when enrolling in Play App Signing. Keep
nightly/stable APK downloads available independently of Google Play.

## Key roles

| Identity | Purpose | Custody |
| --- | --- | --- |
| App-signing key | Signs official sideload APKs and APKs delivered by Play | Retain our existing key and secure backup; enroll an encrypted copy with Google |
| Play upload key | Signs AAB files sent to Play; does not sign installed Play APKs | Separate private key with a secure backup and a protected store-signing environment |
| Local debug key | Local development only | Never enroll this identity with Play |

The existing alias `float-view-ci` is a dedicated signing identity, not the
standard Android debug key. A Debug build configuration and a debug signing
certificate are different things. Store artifacts must nevertheless use Release
configuration.

Do not replace existing APK-signing secrets with upload-key material. Google
verifies AAB uploads against the upload certificate, then signs delivered APKs
with the app-signing key. Choosing a Google-generated app-signing key would
prevent independently built official APKs from using that same private key.
See [Android signing guidance](https://developer.android.com/studio/publish/app-signing).

## Before enrollment

1. Inspect the existing keystore certificate: private-key entry, algorithm,
   validity, and SHA-256 certificate fingerprint. Compare it with an actual
   published APK using `apksigner verify --verbose --print-certs`.
2. Verify the backed-up private key can sign a disposable artifact. Verify the
   separate upload key too; do not mistake a signed test JAR for a valid AAB.
3. Back up both keystores and their passwords securely outside the checkout and
   computer. GitHub Actions secrets cannot be downloaded to recover a lost key.
4. Export both public certificates in PEM format. Keep operational fingerprints,
   key paths, passwords, raw audit output and enrollment files in ignored local
   storage; never commit private keys or encrypted private-key exports.

Use password prompts or password-file options when inspecting/exporting keys;
do not put literal passwords in shell commands, chat or logs. For example:

```sh
keytool -list -v -keystore <keystore-path>
keytool -exportcert -rfc -keystore <keystore-path> \
  -alias <key-alias> -file <public-certificate.pem>
apksigner verify --verbose --print-certs <published.apk>
```

## Play Console enrollment

After account verification and app creation:

1. Open the app's **App integrity / App signing** settings. Select the option
   to supply an existing app-signing key, rather than generate a new identity.
   Console labels can change; inspect the offered enrollment options first.
2. Follow Google's current PEPK instructions to export/encrypt the existing
   app-signing key using the Console-supplied encryption inputs. Download PEPK
   only from Google's enrollment flow. Keep the encrypted export local and
   upload it only for this app's Play App Signing enrollment.
3. Register the separate upload certificate when offered. If enrollment initially
   uses the app-signing certificate for uploads, complete the separate-upload-key
   registration before configuring CI to use it.
4. Compare the Console's app-signing and upload certificate SHA-256 fingerprints
   with the independently recorded local certificates. They must match their
   respective roles and differ from one another.
5. Retain a private enrollment record. Enrollment is not established by creating
   keys locally. Do not request a signing-key upgrade or rotate keys as part of
   initial setup without considering sideload compatibility.

See [Google's Play App Signing procedure](https://support.google.com/googleplay/android-developer/answer/9842756).

## Store delivery and installation checks

The AAB build/upload path remains separate work: Release configuration, the
existing CI-derived `versionCode`, protected signing/upload jobs selected only
by explicit stable/candidate release tags, package/source audit, native 16 KB
page-size checks, and internal-track upload. A JAR signing check does not verify
any of these. Store credentials must not be required for local source builds or
ordinary APK publication.

After enrollment, verify Play-delivered APKs carry the expected app-signing
certificate. Test sideload-to-Play and Play-to-sideload replacement on a device
using increasing version codes, checking that supported settings and recordings
remain available without uninstalling. Equal signatures alone do not establish
successful replacement: version, package/split and OS update policies also apply.
Nightlies can have higher version codes than the store version; wait for a newer
compatible build rather than forcing a downgrade or deleting supported data.
Community builds signed with another key cannot ordinarily replace official
installations. Export recordings before any intentional uninstall.

## Verified scope

The [RC6 APK](https://github.com/acheronfail/float-view-mobile/releases/tag/v1.0.0-rc.6)
passes APK signature verification and matches the retained official app-signing
certificate. Both retained app-key and separate upload-key signing checks pass.
Play enrollment, signed AAB validation and Play-delivered route-switch checks
remain open. The owner confirmed separate backups of both the original
app-signing key/password and the new upload key/password. Detailed records are
local.
