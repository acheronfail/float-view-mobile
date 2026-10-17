# Release verification summary

## Recording policy

Keep reusable procedures, open release gates and brief build-linked outcomes in
Git. Keep detailed execution history, device/OS/firmware inventories, tester
reports, screenshots, logs, account setup and approval history in ignored local
records under `.local/ios-store/readiness/`. Raw artifacts can remain in their
existing ignored build folders. Arrange a private backup when retention matters;
ignored files are not backed up by Git.

A public result should identify the build/source or CI run, tested scope,
pass/fail/pending status and material limitations. Include sanitized actionable
failures without publishing personal device records, routes, identifiers or
contact details. Never infer physical acceptance from software checks. Preserve
existing Git history; this policy does not require rewriting earlier commits.

## Build-linked results

| Build / source | Scope and result | Limits / next step |
| --- | --- | --- |
| [Mac nightly / run 38863449266](https://github.com/acheronfail/float-view-mobile/actions/runs/38863449266) | Developer ID signing, notarization, stapling, quarantined download and Gatekeeper checks passed. Installation/permissions checkpoint passed. | Does not establish Mac App Store sandbox behavior or recording/export acceptance. |
| [iOS 1.0.0 (77) / run 38977368838](https://github.com/acheronfail/float-view-mobile/actions/runs/38977368838), [source fae17d3](https://github.com/acheronfail/float-view-mobile/tree/fae17d3206174adfd1c432bb64d7df370e9a005f) | Protected signing/export, independent IPA validation, exact-source access and packaged privacy checks passed. Read-only Apple authentication/app/build lookup passed. | No upload or Apple processing acceptance. Predates the Rust runtime notice correction and is ineligible for submission. |
| [Nightly 78 / run 38980352255](https://github.com/acheronfail/float-view-mobile/actions/runs/38980352255), [source fae1f60e](https://github.com/acheronfail/float-view-mobile/tree/fae1f60ea5709fb40f6856c2a62a9c4ce63ec414) | IPA/APK passed; master selected no store jobs. Mac notarization was accepted, but ticket stapling failed on both attempts, preventing nightly publication. | A subsequent retry fix needs successful publication. Physical iOS acceptance is pending on the nightly actually installed. |
| [Apple navigation implementation fae1aff1](https://github.com/acheronfail/float-view-mobile/commit/fae1aff18ecb00da432fdc09a838e1a35dd75b44) | Focused native policy/startup tests and Release iOS simulator privacy/support/source browser handoff passed. | Local software/simulator verification; subsequent nightly device acceptance is recorded below. |
| [iOS nightly 82 / run 38986380948](https://github.com/acheronfail/float-view-mobile/actions/runs/38986380948), [source fae1971](https://github.com/acheronfail/float-view-mobile/tree/fae1971b4f947215dad7ef962c6a1702686dae2a) | CI succeeded. Owner confirmed build 82 passed all scoped iOS device acceptance checks, including storage, Bluetooth/permissions, locked-screen recording, announcements, Live Activity, import/export and licence/browser navigation. | Sideload acceptance; detailed results remain local. Does not establish App Store signing, Apple processing or TestFlight-delivered behavior. |
| [iOS local signed preflight / source fae1814](https://github.com/acheronfail/float-view-mobile/tree/fae1814f9696b56071755d33784f0e233cc34bac) | Distribution export, app/extension signatures/profiles/entitlements, corrected full notices, manifests, permission text, development-hook exclusion and anonymous exact-source comparison passed. Release iPad simulator rotation and browser resizing checks passed. | Local test number, not an upload allocation. Does not establish hosted candidate upload, Apple processing or TestFlight delivery; detailed artifacts stay local. |
| [iOS candidate v1.0.0-rc.1, build 86 / run 38990541780](https://github.com/acheronfail/float-view-mobile/actions/runs/38990541780), [source fae1651](https://github.com/acheronfail/float-view-mobile/tree/fae16516d2565b59c747203e17712bc3affdd222) | Hosted signing/export and independent IPA validation passed: app/extension distribution signatures, profiles, entitlements, full notices, manifests, permission text, iPad orientations, source/build identity and development-hook exclusion. Anonymous exact-source comparison passed. All three sideload packages published as a separate GitHub prerelease. Protected Apple upload passed; matching receipt and independent API check confirm VALID processing and no non-exempt encryption. | Owner confirmed installation through TestFlight and all scoped iOS device checks passed. Settings shows the numeric v1.0.0 without the candidate suffix; the display correction needs a subsequent build. External beta review remains open. |
| [iOS candidate v1.0.0-rc.2, build 88 / run 38992628964](https://github.com/acheronfail/float-view-mobile/actions/runs/38992628964), [source fae11a5](https://github.com/acheronfail/float-view-mobile/tree/fae11a5d0f1b1774e340ffcaad2e7c7280b381d8) | Hosted signing/export, independent IPA distribution/package checks, anonymous exact-source comparison, all sideload builds/publication and protected Apple upload/processing passed. The exact packaged frontend displays v1.0.0-rc.2 in Settings under WebKit. | Owner confirmed build 88 installed through TestFlight and Settings displays v1.0.0-rc.2. This packaging-only correction does not reopen unaffected BLE/background acceptance from build 86. Production and TestFlight hardware/video review notes are verified saved; remaining listing work and external beta review remain open. |

## Unpublished checks

Local software, unsigned packaging and simulator checks are retained in the local
readiness record. They do not certify a signed release candidate. Publish their
brief result against the committed build after the relevant checks pass.

## Open release gates

- Repeat package/source gates for each new candidate. Build 88 passes the hosted
  export and independent package/source checks.
- Complete remaining console metadata and review materials. The owner confirmed
  completion of the iOS App Privacy questionnaire; private TestFlight contact
  fields are configured. Hardware details and video are verified in both
  production and TestFlight review notes. Final listing/assets, account declarations and external beta
  review remain open; detailed review materials stay local and in private
  console fields.
- Owner-reported TestFlight device acceptance passed for build 86; the scoped
  candidate-version display check also passed through TestFlight in build 88.
  Upload readiness
  remains enabled behind protected environment review; external beta review is
  still pending.
- Complete remaining Android/Mac-specific tasks and the candidate-gated historical
  release cleanup. No historical release/tag deletion or stable v1.0.0 tagging
  has been performed as part of this checkpoint.

The implementation checklist is 8/38 complete. Local checks do not close
cross-platform tasks or external submission gates.
