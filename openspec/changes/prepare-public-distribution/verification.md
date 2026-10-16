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


## Verified iOS distribution export — 2026-10-17

- A fresh signed archive from clean revision
  `fae1d624e5060016d3474fb671f662474913a684` passed the corrected validator with
  Xcode 26.0 / iOS SDK 26.0, marketing version `1.0.0` and local test build `1`.
  Automatic provisioning supplied the app and Live Activity development profiles;
  export then obtained the App Store profiles and used Apple distribution signing.
- Real local App Store Connect export succeeded, without uploading, under ignored
  `.local/ios-store/signing-check-20261017-export/`. Archive metadata is in sibling
  `signing-check-20261017-verified/archive.json`; export metadata records the same
  native source revision/version and final IPA hash. The certificate/private key
  remains in the local keychain; no CI credential was provisioned.
- IPA SHA-256:
  `3686f0cbd48bfccceff0b96684f519abe7e2ce9bdeb2c9f0cde7eabd66283e44`.
- Both bundles passed strict Apple distribution certificate/anchor and
  certificate-team requirements, matched profile/signature application and team
  identifiers, had unexpired App Store profiles (no device/enterprise scope),
  arm64 device executables, matching version/build numbers and no debugger grant.
  Re-extracted the IPA and repeated final distribution/team checks plus digest
  verification successfully.
- Xcode 26 produced DER-only signed entitlements. This Mac's codesign human
  display reads them, but its XML display returns no XML and an entitlements-blob
  warning. Replaced dependence on that XML display with a small Security.framework
  reader using SecCodeCopySigningInformation and its decoded entitlements
  dictionary. The real export exercised this reader for both bundles; signature
  validation remains separate and required. No legacy reader/fallback was added.
- All 18 fixture-based store-tool tests passed after adapting the signing-tool
  mock. The actual native reader compiled and passed against both real signed
  bundles. Strict OpenSpec and whitespace checks passed.
- Xcode logged a nonfatal App Store Connect store-configuration authentication
  warning during export. Local export completed; authenticated upload, protected
  CI provisioning and TestFlight processing are still unverified. No build number
  was allocated/consumed by Apple and no public/external beta was distributed.
- Group 4's public-data baseline/startup work and the licensing/privacy/API/package
  audits remain required before their planned distribution gates. Store-delivered
  device behavior remains untested by this local export.


## Shared CI numbering and protected iOS build preparation — 2026-10-17

- Owner requested a native build number tied to the CI run and shared across
  platforms. Kept the existing sole counter (`Build apps` run number + 60), with
  an explicit workflow-path guard and source/run ID/attempt/URL build records.
  Display-name changes do not reset identity; corrected binaries require new
  runs, while reruns retain the native number. No second workflow counter or
  persistent allocation service was introduced. Failed/skipped runs may leave gaps.
- Every produced platform package receives the same preparation number. Release
  notes now link it to its GitHub Actions run and retain a build-number marker;
  old retries cannot replace a higher-numbered Nightly. Build records are retained
  per preparation attempt; a store export also records its actual build job/attempt.
- Preparation nightlies explicitly target v1.0.0 and count all reachable commits
  until the first stable v1.0.0 ancestor. Old tag removal cannot change that count;
  after launch normal next-patch/count-since-stable behavior resumes. No stable
  tag or historical release cleanup occurred.
- Added a manual-master-only `ios-store` input and protected `ios-store-signing`
  build job in the same workflow. The three-package GitHub release dependency
  remains unchanged. An explicit store run requests fresh builds of all routes
  for the same native number, including unchanged source, while retaining older-run
  guards; normal unchanged master runs still skip. No App Store upload command, upload credential or automatic
  store submission was added. Owner must configure environment restrictions,
  reviewer, signing secrets and the readiness variable before activation.
- Reused the existing disposable hosted-runner keychain importer under the general
  `with-apple-signing-identity.sh` name, retaining existing Mac secret names and
  Developer ID behavior. The new certificate-family option selects exactly one
  publishing-team Apple Distribution identity. No workstation imports/custom
  trust roots are permitted. Mac and iOS selection/cleanup cases pass fixtures.
- CI profiles must match the exact app/extension, publishing team and selected
  certificate, be unexpired App Store profiles, and have no device/enterprise or
  debugger scope. Profile secret values are removed before child tools; installed
  profiles and the temporary keychain are cleaned before artifact upload. Manual
  Xcode profiles avoid needing an interactive account/API key in the build job.
- All 49 Node release-tool tests and 23 Python store-tool tests passed. Coverage
  includes shared identity, reruns/new runs, workflow rename guard, overflow,
  source/run mismatch, pre-v1 tag cleanup and launch transition, wrong profile
  facts/certificate, missing secrets and cleanup after success/failure. Actionlint
  and strict OpenSpec/whitespace checks passed. Actual hosted iOS store execution
  and its environment protection remain unverified; upload receipt/consumed-build
  checks remain required in the future upload path (tasks 5.2/5.3).
- Next focused packaging check: exercise manual signing/export locally with the
  already provisioned App Store profiles and Distribution identity, without
  simulating a hosted runner or importing a private key on the workstation.


## Manual-profile prerequisite and signing environment — 2026-10-17

- The real local manual-signing check selected the existing App Store profiles
  and Distribution identity but Xcode refused the profiles: they are automatically
  managed (`IsXcodeManaged=true`), while CI uses manual signing. No signed manual
  candidate/export was produced and no upload occurred. The previous successful
  automatic local export remains valid evidence for that separate route.
- Added an explicit CI preflight rejection of Xcode-managed profiles and included
  it in the negative profile fixture cases. All 23 signing-tool tests pass; the
  owner needs to create/download two manual App Store Connect profiles in the
  developer portal before the manual/hosted check can proceed. No profile bytes
  were modified and no private key was exported.
- Created GitHub environment `ios-store-signing` with the authenticated owner as
  required reviewer, self-review allowed for the solo workflow, and a custom
  deployment policy permitting only branch `master`. Read-back confirmed the
  required-reviewer and branch-policy protection types and `master` allowlist.
  Signing secrets/readiness were not set and no workflow was dispatched.
- Owner requested profile-creation instructions; exact app/extension IDs, suggested
  profile names and secret commands are in the Apple guide. Manual signing and
  hosted iOS store execution remain pending those inputs. Upload authentication,
  consumed-build checks, privacy/rights/data-baseline gates remain separate work.


## Portal-created profiles and manual export verified — 2026-10-17

- Owner supplied two portal-created profiles in the repository root after macOS
  privacy controls blocked agent access to Downloads. Validated their exact app
  and extension IDs, publishing team, unexpired App Store scope, disabled debugger
  grant, non-Xcode-managed status and authorization of the available Distribution
  certificate. Moved them to ignored `.local/ios-store/ci-signing/` as
  `app.mobileprovision` and `live-activity.mobileprovision`, with mode 0600.
- Installed only these two validated profiles into Xcode's user profile directory,
  preserving existing profiles. No profile bytes were changed, no private key was
  exported and no GitHub secret was uploaded by the agent.
- A clean-source Release archive and local manual App Store Connect export passed
  from revision `fae12fa47e564b0222db74636826aac21ebb8eac`, using Xcode 26.0 / SDK
  26.0, version `1.0.0` and unallocated local test build `1`. Both signatures and
  provisioning/entitlement checks passed without `-allowProvisioningUpdates`.
- Export metadata and IPA remain under ignored
  `.local/ios-store/manual-portal-profile-check-20261017/export/`.
  IPA SHA-256: `39224948060d8acc1d292d73755198b187756784bade894552e9618c93723bc4`.
  Digest verification passed, and both embedded provisioning profiles match the
  staged portal files byte for byte. This confirms the manual route planned for
  CI, separately from the earlier automatic local export.
- Documentation/strict OpenSpec/whitespace checks passed. No application source
  changed in this checkpoint. Hosted CI signing still needs the owner-exported
  encrypted PKCS12, its password, both profile secrets and readiness activation.
  No workflow was dispatched, no Apple build number was consumed, and upload/
  TestFlight/device validation remains open.


## Protected hosted iOS signing verified — 2026-10-17

- Owner configured all four environment signing secrets and enabled readiness.
  Read-back verified secret names only, required owner review and the master-only
  deployment policy. The owner approved the reviewed run before signing started.
- Pushed reviewed source `fae1a0beaa821e2bb1c45d722fc98425333946ce` and started
  [run 38972744150](https://github.com/acheronfail/float-view-mobile/actions/runs/38972744150).
  Canceled the duplicate push run; the explicit manual run built all four routes.
  Preparation, AltStore IPA, APK, notarized universal Mac, protected iOS store
  signing and rolling Nightly publication all succeeded. Preparation passed all
  49 Node release-tool tests and 23 Python store-tool tests.
- The shared record allocates native build `75` from workflow run number `15`
  plus the retained offset `60`. Nightly is `v1.0.0-nightly.622.gfae1a0beaa82`;
  the separate store IPA is Release version `1.0.0`/build `75` for both bundles,
  retaining iOS 17 minimum support. Hosted Xcode was 26.6 (17F113), SDK 26.5.
- Downloaded `FloatView-ios-store-75-attempt-1` and matched source, clean status,
  shared run identity and IPA SHA-256 against export metadata:
  `cce2e53a070c6a7d8f3abb385fe37d3d9d18323f6080118b2daaebdd7c698981`.
  Independent local verification of the downloaded app and Live Activity extension
  passed strict Apple distribution signatures, expected publishing team, profile
  expiry/distribution scope, signed entitlements, identifiers, arm64 platform,
  versions and shared frontend checks. Signing material remains outside Git.
- The rolling Nightly has all three sideload assets, expected source/version and
  build-75 run link. The store IPA is retained separately as an Actions artifact,
  not attached to the GitHub release. Artifacts in this public repository are not
  confidential; signing credentials are not included in the uploaded artifact.
- No Apple upload or TestFlight processing/distribution occurred. Upload auth and
  receipt/consumed-number checks, licensing/privacy audits, public-data baseline
  and store-delivered physical validation remain open. Tasks 5.2/5.3 remain partial;
  the overall plan remains 5/38 complete.
