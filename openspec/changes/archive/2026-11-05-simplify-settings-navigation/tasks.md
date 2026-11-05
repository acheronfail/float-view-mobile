# Tasks

## 1. Settings destinations

- [x] 1.1 Replace the flat page with seven destinations, focused views and routed headers/back controls; verify category navigation, invalid slugs, checkpoint mapping and return focus/position in unit/browser tests.
- [x] 1.2 Group recording controls with advanced disclosure and relocate export options; update affected recording, appearance, transfer, diagnostic, licence and permission browser tests and verify saves/actions.

## 2. Announcement clarity

- [x] 2.1 Retain guarded accessible Off / On controls and group/disclose alert options without changing persistence; verify pointer cancellation, non-primary, keyboard, saved values and audio tests.

## 3. Integration and visual review

- [x] 3.1 Run Svelte checks, affected unit tests, formatting/lint, build and affected Chromium/WebKit browser tests; record results and fix regressions.
- [x] 3.2 Capture and inspect phone home/recording/announcements and narrow, light/dark, larger-text screenshots; fix visible issues and deliver review images.

## Workflow follow-up

- Sync the app-navigation delta, validate and archive the completed change locally.

## Verification evidence

- Frontend Svelte check: zero errors/warnings; affected ESLint and Prettier checks pass; production build passes (existing single-bundle size warning).
- Focused navigation, checkpoint, context and activation unit tests: 32 passed.
- Affected Chromium/WebKit browser suite: 34 passed across 15 files, including settings navigation, announcements, appearance, backup/import, diagnostics, licences, permissions, ride continuity, focus, motion, subtitles, storage and live-menu coverage.
- Final phone-title adjustment: settings-navigation browser tests rerun in both engines, including intact title words at 320px with 150% text; both passed. Conformance TypeScript check and git diff whitespace check pass.
- Screenshots inspected for home, recording, announcements, dark theme and narrow larger-text layouts; all eight Settings pages captured in light/dark and at larger text. Long titles and descriptions refined following inspection. Review PNGs are local artifacts outside the source tree.
- Broader navigation.browser.ts: Chromium passes; WebKit reports a session-data access-control page error. Reproduced the same failure against an unchanged HEAD frontend built independently in a temporary directory; unrelated to this Settings change.
- OpenSpec canonical validation retains three unchanged pre-existing long-requirement warnings in Home and live connection flow, Routed session browsing and Routed saved-board configuration. Changed/new requirements have no validation warnings.

### Visual review revision

- Restored labelled Off / On segmented controls throughout Settings at user request, retaining the new destinations and conditional alert options. Deleted the superseded switch component and reused the existing guarded SegmentedControl.
- Six affected browser cases across settings-navigation, announcements and segmented-controls pass in Chromium/WebKit. Final spacing/screenshot checks rerun in both engines and pass, including 320px/150% text. Svelte, affected ESLint/Prettier, conformance TypeScript, build and whitespace checks pass. Canonical/delta specs reconciled with the chosen controls; canonical validation retains only the same three unrelated existing warnings.

### Flat section review revision

- Removed surrounding cards from announcement types, Audio & timing, Test announcements, ride-location recording and Advanced recording. Retained shared Off / On controls; matched numeric input surfaces and separated groups with spacing/dividers.
- Settings navigation/layout and announcement browser checks pass in Chromium/WebKit (four cases). Added an explicit short-viewport check that Audio & timing sticks below the page header and spans the full content width. Light/dark, narrow and larger-text screenshots refreshed and inspected. Svelte, affected formatting/lint, conformance TypeScript, production build and whitespace checks pass.

### Duo navigation verification

- Updated the existing side-controls browser test for the new category routes. All seven Settings sections verify one correctly positioned, centre-hit-testable back link in the measured native-CI Duo safe region, return by button, and accepted interactive edge-back acknowledgement. Licences returns through About; geometry fallback preserves focus; navigation retains one backend connection. Chromium/WebKit both pass; conformance TypeScript, formatting and whitespace checks pass. Captured a local Duo-geometry Settings screenshot. This is shared-frontend geometry/bridge replay, not a fresh physical-device gesture check.

### Navigation and presentation review revision

- Reordered Announcements before Recording. Styled Advanced recording as a sticky section heading with a rotating 24px chevron and retained native keyboard disclosure. Replaced the underlined Location permissions text with an internal navigation row showing its destination and chevron.
- Removed the routine announcement save-success message while retaining failure, progress and test feedback. Added a titled Announcement voice section wherever voice guidance or a voice chooser is shown, including iOS hints and missing-offline-voice guidance.
- About uses the shared sticky Float View heading. Build metadata derives revision visibility from the existing validated version grammar: stable/RC versions hide the SHA, development/nightly builds show it when available. The revision remains in metadata and source links.
- Eight affected Chromium/WebKit browser cases pass across settings-navigation, announcements, legal-notices and side-controls; final disclosure sizing/About checks rerun in both engines and pass. Eight Vite configuration/build-metadata unit cases pass, including development/stable/RC/nightly classification and malformed-version rejection. Svelte, TypeScript, affected formatting/lint, build and whitespace checks pass. Reordered canonical/delta category lists remain aligned; canonical validation retains the same three unrelated warnings. Screenshots refreshed and inspected.

### Full subsection visual pass

- Removed the separators between individual announcement types, retaining spacing and the shared sticky section headings.
- Refreshed screenshots for all seven Settings subsections, including light/dark, narrow/150% text, enabled announcements, expanded advanced recording and iOS voice guidance. Inspected all subsection layouts and returned prioritised recommendations in chat.
- Chromium/WebKit settings-navigation/layout checks both pass; Svelte (zero errors/warnings), affected Prettier/ESLint, production build and whitespace checks pass.

### Accepted subsection simplifications

- Led Connection & permissions with compact permission rows and moved Reconnecting below. Details for missing/limited/pending access and relevant guidance open initially; granted rows expand on demand. Added browser checks for preserved disclosure across snapshots and automatic recovery disclosure after revocation, with no OS request from disclosure.
- Renamed backup actions/confirmation/progress to Create backup and Restore backup. Placed Clear app data in its own final section after CSV preferences, retaining the same confirmation, merge, reset and idle protections.
- Led Ride recording with location capture and moved minimum distance into Advanced recording. Shortened alert labels to Speed, Load, Footpad and Battery voltage, with Repeat every (seconds) and explicit scope guidance. Help & diagnostics was not edited.
- All 18 affected browser cases pass across seven files in Chromium/WebKit: announcements, backup/restore/clear and re-import, permission states and recovery, ride continuity, settings layout and Duo navigation. The backup test was rerun after trimming heading whitespace in its ordering assertion. Svelte (zero errors/warnings), affected formatting/lint, conformance TypeScript, production build and whitespace checks pass. Default, advanced, dark and 150% text screenshots inspected for all four changed sections.
