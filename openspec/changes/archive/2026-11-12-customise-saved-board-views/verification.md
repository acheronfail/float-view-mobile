# Verification

- `npm --prefix frontend run check`: no errors or warnings.
- `npm --prefix frontend run build`: passed, including embedded-artifact checks; existing single-bundle size advisory remains.
- Focused Vitest: 97 tests passed across backend state, UI restoration, custom dashboard composition, widget catalogue, live policies and live values.
- Conformance TypeScript `tsc --noEmit`: passed.
- Chromium and WebKit: all 12 cases passed across `board-views-editor.browser.ts`, `custom-dashboard.browser.ts`, `last-live-view.browser.ts`, `live-value-search.browser.ts`, `navigation.browser.ts` and `theme-widget.browser.ts`. Local browser bundles with existing compatibility libraries were used outside the sandbox for loopback access.
- Editor coverage includes no connection/local Bluetooth association, explicit target isolation during another connection, disconnect while editing, Live failure/retry, Custom conflict/reload, dirty switch/native Back, keyboard/touch/non-primary/cancelled input, runtime-action suppression, defaults without writes and target removal.
- Updated pre-existing regression selectors for the current hold guidance and Faults and errors labels; their earlier labels already differed from the unchanged UI.
- Touched frontend files passed Prettier and ESLint; changed conformance files passed Prettier. `git diff --check` passed.
- Inspected phone Live/Custom and desktop Custom screenshots under ignored `conformance/generated/board-views-editor/`. Both views show the selected board name and configured artwork with labelled example data; narrow layouts retain vertical access without horizontal page scrolling.

Canonical app-navigation formatting reports six pre-existing missing scenario blank lines; the added requirements and all change artifacts are formatted.

OpenSpec change and custom-dashboard canonical validation passed without issues. App-navigation validation passed with three existing requirement-length warnings; the complete pre-change canonical content remains unchanged.

No native boundary, wire shape or storage format changed.

## Header-only UI refinement

The requested follow-up removes the artwork card and visible guidance row from the editor. Board identity and preview context now occupy the existing page title and subtitle; canonical navigation requirements reflect this superseding presentation. Frontend type checks, build and focused ESLint pass, as do conformance TypeScript and the expanded Chromium/WebKit editor journeys. Twelve screenshots per engine capture saved-board selection, board history/menu, Live selection and its automatic save, Custom widget selection/configuration, draft review, Save/return and reopening the accepted composition. Screenshots remain in ignored `conformance/generated/board-views-editor/*/journey/`.

## Preview menu refinement

Configure board precedes Customise views in both entry menus. The editor reads the saved Live/Custom preference, with absent/Detailed preferences opening Live, and replaces its content selector with a header dropdown containing only switching to the other view and Back. Back routing remains available to native navigation without a separate visible header row. Header menus sit above dashboard edit controls and below modal panels.

Frontend type checks, build, focused ESLint/Prettier, conformance TypeScript and `git diff --check` pass. All four editor/navigation browser cases pass in Chromium and WebKit. The editor cases cover defaults, keyboard/touch switching, menu Back and native Back discard guards, absence of connected actions and exact healthy Live viewport x/y/width/height preservation. Updated screenshots include `03b-default-custom.png` and `03c-preview-menu.png`. Canonical navigation validation passes with the same three pre-existing length warnings.
