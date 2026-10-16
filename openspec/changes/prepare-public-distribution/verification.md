# Verification

## Developer ID notarization — 2026-10-16

Implemented current direct-download macOS signing/notarization before Mac App
Store packaging. Account identity, team and credentials are configured outside
source. The selected Developer ID identity was exported alone to an encrypted,
ignored local backup; only its public identity metadata was inspected. The owner
installed that identity/password and team variable in GitHub Actions manually;
setting names and update timestamps were checked without retrieving secret values.

Software checks:

- All 37 release-tooling tests passed, including the initial 13 notarization/signing
  tests. The focused 13-test suite also passed after submission-ID validation.
- Tests cover accepted, invalid/rejected, timed-out and malformed submissions;
  signature, stapling and Gatekeeper failure; preserving a previous output archive;
  selected-team certificate filtering; workstation-import refusal; disposable
  keychain cleanup; credential removal before submitting.
- Shell syntax checks passed for build, import and notarization scripts.
- The first hosted preparation run found a macOS-only base64 decoding flag in the
  Linux-executed wrapper tests; replaced that operation with strict portable
  Python decoding rather than skipping the certificate-selection tests. The
  expanded 38-test release suite passes locally, including malformed certificate
  input rejection and validation of decoded bytes before the mock import.
- Workflow YAML parsed successfully. Whitespace and strict OpenSpec checks passed.
- The frontend production build passed before embedding its HTML.

Actual Apple-service and artifact checks:

- Built a Release universal arm64/x86_64 app from the current source with local
  notarization-workflow edits (base revision fae1b658b). This was an unpublished
  verification candidate, not a v1.0.0 release or replacement of existing assets.
- Signed with the selected Developer ID Application identity, hardened runtime
  (`flags=0x10000(runtime)`) and a secure timestamp.
- Apple accepted submission `2f6f01c6-308f-4514-8433-7664544e9e2c`.
- Stapling, ticket validation, strict signature validation and local Gatekeeper
  assessment passed; Gatekeeper reported `source=Notarized Developer ID`.
- Rebuilt the ZIP after stapling, checked ZIP integrity, extracted it into a fresh
  temporary directory and applied quarantine. Both architectures, signature,
  stapled ticket and Gatekeeper acceptance passed on that extracted copy.

Hosted workflow and published download:

- Pushed implementation and portable-decoding fix through revision
  `fae1192cd97f77f74cf51de96d3b49dd30f61afe`.
- [GitHub Actions run 38863449266](https://github.com/acheronfail/float-view-mobile/actions/runs/38863449266)
  passed preparation (38 release-tooling tests), iOS, Android, macOS and release
  publication. This exercised the owner-provisioned signing and notary credentials
  on the hosted macOS runner.
- Apple accepted hosted submission `eeacd242-b2d2-4e40-812f-6678efdeb9d5`.
- Published [nightly v0.3.1-nightly.8.gfae1192cd97f](https://github.com/acheronfail/float-view-mobile/releases/tag/v0.3.1-nightly.8.gfae1192cd97f)
  with the universal macOS ZIP alongside the IPA and APK.
- Downloaded the published macOS ZIP and matched its SHA-256 to the GitHub asset
  digest: `352f7b0ec252186e447eccbf237f2a02c670faa466f8f7fe4aaf8c2b56bce876`.
- Fresh extraction with quarantine applied passed both-architecture inspection,
  strict Apple Developer ID signature validation, stapled-ticket validation and
  Gatekeeper assessment (`source=Notarized Developer ID`). These checks ran on
  the existing development Mac, not a clean-machine permission-transition test.

Outstanding:

- Verify a published download on a clean supported Mac, and the affected signing
  transition's Bluetooth/location permission retention or regrant, recording and
  export. Local signing/Gatekeeper checks are not this OS/device checkpoint.
- Mac App Store sandbox/provisioning, privacy requirements and submission remain
  separate subsequent work. Existing self-signed assets were not replaced.
