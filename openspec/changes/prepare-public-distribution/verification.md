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

Owner-reported installation and permissions checkpoint — 2026-10-16:

- In response to the outstanding installation/permission-transition checkpoint,
  the owner reported: “I tested, gatekeeper passed, the permissions all worked
  as well”. This closes the direct-distribution notarization checkpoint (3.4).
- Device model, macOS version, clean-install conditions and whether permissions
  were retained or requested again were not specified. Recording/export results
  were not separately reported; this confirmation does not establish those checks
  or Mac App Store sandbox acceptance.

Outstanding:

- Mac App Store sandbox/provisioning, privacy requirements and submission remain
  separate subsequent work. Existing self-signed assets were not replaced.


## iOS archive/signing preparation — 2026-10-17

Scope: tasks 1.1/1.4 (readiness inventory), 5.3 (local archive/export scaffolding)
and 5.6 (documentation). Overall implementation remains 4/38 tasks complete;
actual signed export and upload have not been performed.

- Owner confirmed Apple membership earlier, and confirmed today that iOS app IDs,
  App Store Connect record and Apple Distribution identity have not been created.
  No account resources or credentials were created, imported or uploaded here.
- Existing IDs are `fail.acheron.float-view` and
  `fail.acheron.float-view.live-activity`; both have deployment target iOS 17.
  Availability/ownership must be checked in the publishing team before creating
  the App Store Connect record. Team is supplied to both targets by one override;
  no personal team identifier is committed.
- Added a shared scheme with a Release Archive action and retained lifecycle
  Test action. Added independent local unsigned-check, signed-archive and
  App Store Connect export commands; none uploads or tags a release.
- A real unsigned arm64 Release archive succeeded with Xcode 16.4 / SDK 18.5
  after rerunning outside the Codex sandbox (the first run could not launch
  Xcode's build-script sandbox). App and Live Activity extension were embedded,
  used matching marketing/build versions `1.0.0`/`1`, retained iOS 17 deployment,
  and contained the shared frontend. This local test value is not an allocated
  or uploaded store build number. The archive records its source revision,
  dirty state, toolchain and app-tree hash; it is explicitly an unsigned check.
- Store paths require Xcode 26+ / SDK 26+, explicit version/build metadata,
  a clean checkout for signed archiving and a publishing team. Profile updates
  are opt-in. Export stays local, disables automatic version changes, refuses
  unsigned/modified archives and verifies distribution signatures plus each
  bundle's profile/team/application/debugger facts before recording IPA evidence.
- The local Xcode 16.4 toolchain cannot produce a currently uploadable store
  build. Upgrade Xcode, register the two explicit IDs, create the distribution
  identity/profiles and app record before actual signed verification. Apple
  privacy/API, licensing/distribution-rights and production-entry-point audits
  remain open. Baseline-100/storage/startup work remains required before external
  beta; no external testers or public release are enabled by these commands.

Checks completed:

- All 18 Python store-tool boundary tests passed, including rejected development,
  enterprise, expired and wrong-team profiles; signature/debugger/application-ID
  failure; missing or mismatched extension; unsupported SDK; missing version/build;
  dirty source; changed/unsigned archive; local-only export and IPA digest recording.
  Signed tests use mocked OS tools and fixture profiles, not real Apple signing.
- All 39 existing Node release-tooling tests passed. The Python tests now run
  alongside them in GitHub preparation without accessing Apple credentials.
- The existing `just ipa` command passed in Release with the shared scheme and
  produced an unsigned AltStore IPA. ZIP integrity and app/extension version/ID
  inspection passed. No store credential is required by that route.
- A direct signed-archive invocation with the installed SDK refused before signing
  with the expected Xcode 26+/SDK 26+ message. This Mac runs macOS 15.6; no Xcode
  installation or global toolchain selection was changed.
- Shared scheme XML, workflow YAML, whitespace and strict OpenSpec validation
  passed. No Swift/Rust/product behavior changed; no new device behavior is claimed.


## Registered iOS identifiers and Xcode 26 — 2026-10-17

- Owner reported installing Xcode 26.0, registering explicit IDs
  `fail.acheron.floatview` / `fail.acheron.floatview.live-activity` and creating
  an Apple Distribution certificate. App Store Connect record creation is in
  progress; certificate/private-key usability and profiles have not yet been
  exercised by a signed export.
- Updated app/extension and lifecycle-test target identifiers, launch/development
  and conformance tooling, iOS Bluetooth restoration namespace, Live Activity
  logging subsystem and archive validation to the registered IDs. The former
  development iOS bundle identity is not retained as an alias or migration path;
  installation under the new ID has its own private container. Direct macOS
  bundle identity and Developer ID/notarization requirements are unchanged.
- Explicit `DEVELOPER_DIR` selected the installed Xcode 26.0 (build 17A324) with
  iOS SDK 26.0, without changing global Xcode selection. A real unsigned Release
  archive passed with both registered bundle IDs, embedded extension, matching
  `1.0.0`/`1` versions, arm64 binaries and iOS 17 deployment. This remains an
  unsigned structural check using an unallocated local build number.
- All 18 store-tool tests and 14 focused conformance development/target tests
  passed. Conformance TypeScript checking, whitespace and strict OpenSpec
  validation passed. Lifecycle/BLE behavior on a device and an actual signed
  export remain for the affected iOS distribution checks.
- App-record guidance: iOS only for the current setup; suggested internal SKU
  `float-view-ios`; Full Access for the solo App Store Connect account. Adding
  macOS to the same record would use the same bundle ID and universal purchase;
  this remains a later store-distribution choice, not a change to direct Mac IDs.


## Initial iOS signing attempt — 2026-10-17

- Owner confirmed creating the iOS App Store Connect record. The selected Apple
  Distribution identity is valid and available locally; no private material was
  exported or uploaded.
- Automatic provisioning successfully produced an Xcode-signed archive for the
  app and extension using the publishing team. The subsequent custom validator
  exposed an argument-format bug: inline `codesign -R` expressions require a
  leading `=`; otherwise codesign treats the expression as a file path.
- Corrected both archive and export requirement arguments. The 18 fixture tests
  pass with the mock now enforcing this real CLI requirement syntax. Direct
  verification of the first archive against the Apple anchor/publishing team
  passed outside the Codex sandbox; inside the sandbox trust evaluation returned
  `CSSMERR_TP_NOT_TRUSTED`. Apple trust services were used normally; no trust-root
  override was installed.
- A fresh clean-source archive and real distribution export follow this fix.
  Initial build number `1` remains a local unallocated test value and no upload
  or release has occurred.
