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

## Unpublished checks

Local software, unsigned packaging and simulator checks are retained in the local
readiness record. They do not certify a signed release candidate. Publish their
brief result against the committed build after the relevant checks pass.

## Open release gates

- Verify corrected notices, source access, manifests and development-hook
  exclusions in the fresh signed release candidate.
- Complete remaining console metadata and review materials. The owner confirmed
  completion of the iOS App Privacy questionnaire; private TestFlight contact
  fields are configured. Public beta copy and hardware review evidence remain open.
- Validate actual upload, Apple processing and TestFlight-delivered behavior.
  Upload readiness remains disabled; no Apple upload or tester invitation is
  recorded as complete.
- Complete remaining Android/Mac-specific tasks and the candidate-gated historical
  release cleanup. No historical release/tag deletion or stable v1.0.0 tagging
  has been performed as part of this checkpoint.

The implementation checklist remains 7/38 complete. Local checks do not close
cross-platform tasks or external submission gates.
