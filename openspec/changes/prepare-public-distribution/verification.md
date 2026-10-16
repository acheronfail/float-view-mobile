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

Outstanding:

- Activate and verify the updated GitHub workflow; signing and notarization
  credential settings now exist but must be exercised by an actual hosted run.
- Verify a published download on a clean supported Mac, and the affected signing
  transition's Bluetooth/location permission retention or regrant, recording and
  export. Local signing/Gatekeeper checks are not this OS/device checkpoint.
- Mac App Store sandbox/provisioning, privacy requirements and submission remain
  separate subsequent work. Existing self-signed assets were not replaced.
