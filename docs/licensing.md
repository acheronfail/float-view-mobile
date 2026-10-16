# Licensing and attribution

Float View's project-owned code is **AGPL-3.0-or-later**, copyright © 2026
acheronfail. [LICENSE](../LICENSE) contains the full terms and [NOTICE](../NOTICE)
identifies the project and artwork. This does not replace third-party terms.
Vendor submodules and source-derived fixtures retain their upstream licences.

The rider artwork is adapted from [“onewheel” by Luis Prado, Noun Project](https://thenounproject.com/icon/onewheel-4260841/),
image 4260841, using the attribution route under [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/).
See [Noun Project's licence options](https://help.thenounproject.com/hc/en-us/articles/200509798-What-licenses-do-you-offer-for-icons).
The source SVG and generated app/loading/map artwork retain that licence, with
colour, background, silhouette and format changes noted in NOTICE. A royalty-free
label is not evidence of a purchased attribution waiver. Do not claim exclusive
ownership of the underlying artwork or an endorsement by its creator.

## Bundled notices

[THIRD_PARTY_NOTICES.txt](../THIRD_PARTY_NOTICES.txt) includes a conservative native
dependency inventory (including build tools/macros), client JavaScript, generated
Lucide/Feather icons, Android Release dependencies and rider attribution. Settings
bundles the project licence and notices directly into the shared frontend, so all
native packages and sideload routes can display them without internet access.
Settings opens the complete documents on a dedicated scrollable licences route.
The generated inventory also includes pinned Rust standard-library,
compiler-builtins and math-runtime notices from LICENSES/. The frontend build
checks actually bundled client package versions; iOS archive/export checks
require the complete current legal texts in the packaged HTML.

The Android POM inventory identifies licence grants, including inherited licences;
it is not a substitute for collecting any additional copyright/NOTICE material
from exact upstream source packages. Complete that final-package review before
store submission. The generator rejects missing licences and unreviewed Android
licence families; review component-specific and embedded notices, not just SPDX
expressions. The committed Lucide notice was retrieved from the upstream licence
on 2026-10-16 and includes MIT attribution for Feather-derived icons. Review this
against the selected icon data when changing its dependency version.

After installing the locked frontend and native dependencies, capture the resolved
Android Release graph (Java 17 or 21 and the Android SDK are required):

```sh
cd backends/android
./gradlew --offline --no-daemon :app:dependencies --configuration releaseRuntimeClasspath > /tmp/float-view-android-dependencies.txt
cd ../..
python3 tools/generate-license-notices.py --android-report /tmp/float-view-android-dependencies.txt
python3 tools/generate-license-notices.py --android-report /tmp/float-view-android-dependencies.txt --check
```

The collector uses offline Cargo metadata for the shipped targets, local registry
licence files, installed frontend packages checked against package-lock.json, and
cached Maven POMs. It does not download packages. Full licence texts are deduplicated
by content while retaining the component-to-document mapping. Review regenerated
output with dependency changes. Build the frontend and check the notices surface
after changing the collector, legal text or bundled components.

## Source delivery and store submissions

Keep each binary's exact source revision, dependency lockfiles, native/firmware
source provenance and required build inputs/scripts available with the release.
Settings links to the recorded source revision for clean identified builds; local
dirty builds link to the repository without claiming to identify their changes.
GitHub source archives omit submodule contents, so check any required submodule or
upstream material separately. Access to the latest branch alone does not establish
corresponding-source delivery for an older binary.

Before store submission, verify the final IPA/APK/AAB/Mac package notices and
corresponding-source availability. Resolve Apple's distribution terms against all
applicable rights. This repository does not silently grant a store exception or
waive any third-party obligations. Any additional permission for owner-controlled
code needs an explicit owner decision.

### Official Apple distribution

On 2026-10-17 the owner confirmed ownership of project-authored code and
permission to distribute official binaries through Apple TestFlight and the
App Store under Apple's applicable terms. Public source remains
AGPL-3.0-or-later; LICENSE has no general store exception. This is a project
audit note, not a document required by Apple. It grants no rights to separately
credited dependencies, artwork or future outside contributions.

The locked notice inventory identifies permissive software licences and CC BY
artwork, with no third-party GPL/LGPL/AGPL runtime dependency identified.
Research vendor submodules and source-derived test fixtures are not copied into
the Release app. Final package notices and exact source availability must still
be verified; new copyleft code or outside contributions need a fresh rights review.

The [iOS package/source audit](ios-package-source-audit.md) records build 77's
actual notices, all 95 iOS Cargo inventory entries and anonymous exact-source
verification. It found the Rust runtime notice omission, now fixed for the next
candidate. A fresh tagged candidate must pass the new packaging check; the audit
does not declare the older binary ready for submission or close Android/Mac work.
