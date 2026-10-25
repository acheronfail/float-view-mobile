# Tasks

## 1. Bounded contract and persistence

- [x] 1.1 Define the version-1 `customDashboard` discriminated union, 24-widget/8,192-byte bounds, unique ID rules, presentation compatibility and canonical range limits in the frontend contract and shared validation; verify valid combinations and rejection of unknown members/versions, duplicate IDs, invalid ranges and both dashboard/profile capacity violations with focused frontend/Rust tests.
- [x] 1.2 Add optional dashboard read/update handling through existing revisioned Board Profiles in the product store, shared service/server and client/mock; verify atomic save, unrelated-field preservation, stale revision, missing board, operational failure and independent Live View slots through affected store/service tests and canonical direct/mock scenarios.
- [x] 1.3 Cover reopen and archive compatibility, including frozen baseline stores/archives, absent/null/empty dashboards, malformed stored data, new-board import and existing-board merge; verify supported data and historical snapshots survive, storage/archive numbers remain unchanged for the additive model, and no reset or extra persistence owner is introduced.
- [x] 1.4 Document the dashboard contract and ownership beside existing frontend/backend guidance; verify links and published limits match the tested contract and record any actual compatibility findings before continuing.

## 2. Shared widget presentation

- [x] 2.1 Extend the existing reading catalogue with compatible presentation/default-domain metadata and reusable canonical unit/range conversion; verify signed current, temperatures, percentages, missingness, invalid ranges and unit preference changes through focused tests.
- [x] 2.2 Extract presentation-only number, lighting and specialised visual renderers from slot/screen wrappers, retaining fixed Live View composition and hold configuration; verify existing number/lighting/gauge/battery/temperature/footpad behaviour with focused component tests and affected interaction coverage.
- [x] 2.3 Add the custom widget frame and typed renderer switch for number/bar/gauge/trend, battery, temperatures, footpads and lighting; verify duplicate-reading independence, size-preserved presentation, range overflow indication, unavailable BMS and reconnect dimming without layout loss.
- [x] 2.4 Document renderer, frame and catalogue responsibilities in the frontend ownership guide and remove replaced duplicate paths; verify imports follow ownership boundaries and no renderer starts persistence or independent sample collection.

## 3. Shared live-history lifetime

- [x] 3.1 Extract one live-route-owned history buffer and pruning clock from the live-chart dialog, then connect the overlay and custom trend renderers; verify 30-second/1,500-point total bounds, sequence deduplication, stale/gap behaviour, captured BMS/load semantics and shared history across view/overlay changes with focused tests.
- [x] 3.2 Add identity/generation-boundary reset and route/frontend cleanup; verify reconnect, sequence reset, transport loss and disposal release incompatible points and timers, and request-spy coverage shows no extra telemetry, session backfill or recording requests when charts are added.
- [x] 3.3 Update live-chart documentation to describe route lifetime and ephemeral restoration behaviour; verify the documented lifetime and bounds match the tested owner.

## 4. Responsive custom dashboard and navigation

- [x] 4.1 Add saved-board Custom View loading, derived defaults, intentional empty state and direct labelled Live/Detailed/Custom selection; verify no write occurs on first view, profile-read errors offer retry, guest/BMS eligibility is unchanged, and separate boards retain separate layouts in browser/mock tests.
- [x] 4.2 Implement ordered responsive placement with compact/medium/large spans, readable minimums, bounded wide layouts and stable widget keys; verify narrow portrait, short landscape, tablet/unfolded-width and desktop fixtures have readable widgets, stable reading order and no horizontal page overflow.
- [x] 4.3 Cover live viewport resize, rotation-like and split-screen-like changes in one mounted dashboard; verify configuration, widget identity, draft data and retained trend history survive and no device-specific layout is saved.
- [x] 4.4 Extend checkpoint mode validation and restoration across frontend/mock/shared service for eligible Custom View; verify same-process restoration loads current saved values, excluded edit/chart state is not restored, old normal/detailed checkpoints remain valid, and changed-board/guest fallback is safe in canonical and browser cases.
- [x] 4.5 Document view eligibility, responsive sizing and the unreported-hinge limitation; verify the browser rendering matrix describes actual supported viewport scenarios without claiming physical fold-device coverage.

## 5. Dashboard editing and gestures

- [x] 5.1 Build draft-based add/configure/remove/resize/reorder with widget previews and one modal, plus Save/Cancel and dirty-exit handling; verify cancel/no-write, atomic save, explicit empty dashboard, capacity feedback, conflicts, retry and unchanged unrelated profile fields in component/browser tests.
- [x] 5.2 Implement whole-widget edit interception and gesture arbitration using primary Pointer Events, touch hold-to-drag, pre-drag scrolling and bounded edge autoscroll; verify tap opens configuration, movement scrolls or deliberately reorders, completed drag suppresses tap, and cancellation/non-primary input never mutates or opens configuration in mouse/touch browser tests.
- [x] 5.3 Add subtle wiggle/static cues, reduced-motion handling and keyboard/assistive configuration and earlier/later ordering; verify focus containment/restoration, position announcements and every edit operation without dragging, and ensure lighting controls send no requests during editing.
- [x] 5.4 Guard drafts, in-flight saves and drag geometry against board changes, reconnects, viewport changes, Back/Escape and frontend teardown; verify same-board drafts survive ordinary reconnect, stale completions cannot affect another target, resize safely cancels/reconciles drag, and recreation never replays a save or control.
- [x] 5.5 Document tap/hold/scroll/drag behaviour, range configuration and Save/Cancel semantics in reusable frontend guidance; verify instructions match browser-tested touch and keyboard interactions.

## 6. Shared warning overlay

- [x] 6.1 Move fault/Refloat-disabled warning ownership above all live views and remove gauge-local warnings; verify visibility without a gauge, no duplicates, scrolling/safe-area clearance, fault-history activation and enable-Refloat action gates in focused browser coverage.
- [x] 6.2 Integrate warning presentation with edit/configuration modal top layers and existing safety overlays; verify active warnings remain visible during editing, focus/actions obey modality, and flywheel retains its exclusive Disable interaction.
- [x] 6.3 Document warning ownership and surface precedence beside the existing interaction guide; verify there is no remaining widget-dependent warning owner.

## 7. Integration verification

- [x] 7.1 Run affected frontend tests, `npm run check`, relevant formatting/lint checks and `npm run build` sequentially; then run affected `conformance/test/*.browser.ts` with direct sandbox escalation on macOS, covering gesture cancellation, reduced motion, warning/modal precedence, maximum supported composition and the responsive matrix. Record actual outcomes and remaining limits.
- [x] 7.2 Run affected shared Rust crate/consumer tests with locked offline dependencies, relevant formatting and warnings-denied Clippy checks, plus affected canonical mock/direct conformance; verify both Apple and Android consume the same profile/checkpoint contract through existing bindings and run focused native integration checks for those changed boundaries.
- [x] 7.3 Review accepted scenarios against delivered tests and docs, run `git diff --check` and strict OpenSpec validation, and record build/source-linked outcomes without claiming physical BLE or fold-device validation from software tests. Broaden checks only for failures or demonstrated cross-cutting impact.

## 8. Durable per-board view selection

- [x] 8.1 Add validated optional `lastLiveView` to frontend/mock/shared profile handling; verify invalid values, preservation, independent boards and reopen compatibility.
- [x] 8.2 Restore per-board selection on entry and save explicit selection with bounded conflict handling, dirty-exit guards and stale-target protection; keep guests temporary and preserve checkpoint precedence.
- [x] 8.3 Cover selection persistence, failed writes, per-board restoration and dashboard editor revisions with focused unit, canonical and browser checks; update ownership documentation and validate the change.

## 9. Home-screen editing UX

- [x] 9.1 Replace explicit Edit actions with a stationary whole-widget hold that enters editing and continues dragging, preserve ordinary control taps and pre-hold scrolling, and provide keyboard entry and empty-dashboard Add.
- [x] 9.2 Add a pointer-following lifted widget, clear placement cues and animated reordering with reduced-motion support; verify cancellation, no lighting commands on hold, keyboard operation and draft/save regression coverage.
- [x] 9.3 Update interaction guidance and delta specs, run affected frontend/browser checks and strict OpenSpec validation, and record actual results.

## 10. Compact layout and direct removal

- [x] 10.1 Remove configuration move buttons, retain keyboard ordering on widget surfaces, and replace edit dots with circular direct-removal controls using primary pointer/keyboard activation and cancellation.
- [x] 10.2 Fit four compact widgets across ordinary phone widths, reduce compact padding/height and adapt compact battery/temperature presentation without changing Live View defaults.
- [x] 10.3 Verify removal isolation, draft/Cancel semantics, keyboard focus, four-column sizing and specialised renderers with affected frontend/browser checks; update specs and guidance.

## 11. Square-cell widget packing

- [x] 11.1 Replace stretched row heights with a responsive square-cell grid: compact 1×1, medium 2×2 and large 4×2 (spans clamped on narrow containers). Use dense placement so compact widgets fill the spaces beside larger widgets without changing saved order.
- [x] 11.2 Adapt widget content to bounded cells, preserving presentations, and cover square compact sizing, a 2×2 widget beside four 1×1 widgets, responsive reflow, editing and cancellation.
- [x] 11.3 Update placement specs/guidance and run affected frontend/browser verification while keeping the preview available.

## 12. Consistent colours and stationary editing chrome

- [x] 12.1 Use one semantic reading colour throughout custom number/bar/gauge/trend widgets, including authoritative load-source changes, without changing shared Live defaults.
- [x] 12.2 Keep parent-owned warnings on the live dashboard only; conceal and disable them behind menus/dialogs without relocating them or shifting dashboard geometry, retaining flywheel precedence.
- [x] 12.3 Move Add/Cancel/Save to a floating safe-area-aware toolbar, remove in-flow editing chrome, preserve scroll clearance across modes, and hide the toolbar during panels/confirmation. Verify geometry, colour changes, modal warning absence and save/discard/accessibility regressions.

## 13. Accurate size previews

- [x] 13.1 Show explicit cell dimensions instead of ambiguous size names and render configuration previews using the current dashboard’s measured cell size and shared footprint mapping, scaling down only when needed to fit.
- [x] 13.2 Verify footprint calculation, exact compact preview dimensions and existing editor interactions; document the preview’s responsive behaviour.

## 14. Gauge-first reading visualisation

- [x] 14.1 Replace the number-plus-small-arc composition with a cell-filling dial and centred reading/units, retaining semantic colours, ranges, unavailable values and overflow indication.
- [x] 14.2 Verify compact/medium gauge geometry and colour/source changes in both browsers; update presentation guidance and capture ignored screenshots.

## 15. Simpler widget configuration actions

- [x] 15.1 Remove the duplicate modal removal action and its callback, rename Apply to dashboard to Save while preserving draft-only semantics, and verify direct × removal and configuration/save regressions.

## 16. Value-first discovery and flexible footprints

- [x] 16.1 Extend the bounded version-1 dashboard size union with closed integer column/row spans (1–4 each), preserving all existing named sizes without reset or implicit writes. Update frontend/mock/shared validation, canonical contract coverage, reopen/archive checks and shared host consumers together.
- [x] 16.2 Expose independent Width/Height and visual presets, retain accurate previews, clamp explicit widths to available columns while preserving row intent, and adapt one-row numeric/battery/control/chart content to its cell height.
- [x] 16.3 Replace the preset gallery with searchable, categorised values (Number/1×1 defaults), clear visual panels/controls and whole-card activation. Stage new widgets until Add widget; Cancel must not alter the draft. Replace reading presentation dropdowns with visible compatible choices.
- [x] 16.4 Verify discovery, keyboard/touch activation, staged cancellation, supported dimension bounds, existing layout preservation, short/wide/tall packing and save/archive/public consumers; document actual outcomes.

## Workflow follow-up

- After implementation and relevant verification, sync specs and archive when requested. Preserve detailed local testing evidence under the repository's ignored paths.
- Do not create Git commits without the user's explicit authorization; this instruction overrides the repository OpenSpec commit-phase convention.

## Verification evidence

Verified on 2026-10-23 in the uncommitted working tree based on `fae15048`.
The browser checks used frontend bundle `bundle.6732a029.js` built from this
implementation. Source owners are documented in the
[frontend guide](../../../../frontend/src/lib/README.md) and
[backend guide](../../../../backends/README.md).

- Frontend: all 491 tests across 89 files passed; Svelte/TypeScript reported zero
  errors and warnings; ESLint and production build passed. Loopback-dependent
  authentication tests ran outside the sandbox.
- Canonical mock and direct Rust conformance passed, including dashboard bounds,
  profile atomicity and the custom checkpoint identity contract.
- Product-store and backend-service suites passed, including frozen baseline
  fixtures, reopen, malformed-data preservation and archive create/merge tests.
  Affected Apple FFI, Android JNI and runtime-host tests passed. Rust formatting
  and warnings-denied Clippy passed.
- Chromium and WebKit custom dashboard tests passed for responsive resize,
  whole-widget dragging, touch scrolling/cancellation, keyboard configuration,
  reduced motion, no controls during editing, range validation, revision conflicts,
  failed loads, 24-chart capacity, empty/add flows, warnings, reconnects, identity
  changes and restoration. Existing Live menu, battery and restoration regressions
  passed. Live/Detailed viewport coverage passed 144 normal and 48 detailed cases
  per engine, with ordinary-fit fixtures separated from warning-overlay scenarios.
- Conformance TypeScript checking, strict OpenSpec validation and whitespace checks
  passed. Generated screenshots remain ignored under `conformance/generated/`.

Coverage is software-based. Viewport emulation does not establish physical
fold/hinge integration; no native hinge API or firmware/Bluetooth behaviour changed.
No native app packaging, store submission, Git commit, spec sync or archive was
performed for this implementation.

### Last-view persistence follow-up

Verified the follow-up on 2026-10-23 with frontend bundle `bundle.1929bfc8.js`:
56 focused frontend tests passed; Svelte/TypeScript, ESLint and production build
passed. The product-store preference test passed for validation, independent
boards, unrelated-value preservation, null/default compatibility and reopen.
Canonical mock (revision 86) and direct Rust (revision 508) conformance passed;
affected warnings-denied Clippy passed. Chromium and WebKit persistence tests
passed for Custom/Detailed/Live restoration without a process checkpoint,
reconnect, failed-write retry and immediate dashboard save. Existing custom
composition, Live menu and process-restoration browser regressions passed.

### Home-screen editing UX follow-up

Verified on 2026-10-24 in the uncommitted working tree based on `fae1e92a`,
with frontend bundle `bundle.664484d8.js`.

- Five focused dashboard frontend tests passed. Svelte/TypeScript reported no
  errors or warnings; affected ESLint, production build and conformance TypeScript
  checking passed. The build retains the existing large-bundle advisory.
- Chromium and WebKit dashboard and last-view browser tests passed. Coverage
  includes hold-to-edit without releasing, a pointer-following preview, neighbour
  placement before drop, ordinary single lighting taps, no lighting commands on
  hold, Chromium touch entry and early scrolling, cancelled preview restoration,
  keyboard entry/configuration, reduced motion, animated mouse reordering,
  Save/Cancel, conflicts, empty/Add and per-board selection restoration.
- Strict OpenSpec validation and whitespace checks passed. Screenshot evidence
  remains ignored under `conformance/generated/custom-dashboard/`.

### Compact layout and direct removal follow-up

Verified on 2026-10-24 in the uncommitted working tree based on `fae1e92a`,
with frontend bundle `bundle.91b76e6f.js`.

- All 23 focused frontend tests passed; Svelte/TypeScript reported zero errors
  and warnings; affected ESLint, formatting, production build and conformance
  TypeScript checking passed. The existing bundle-size advisory remains.
- Eight browser tests passed across Chromium and WebKit: custom dashboard,
  last-view restoration, Live battery configuration and Live/BMS temperatures.
  Custom coverage verifies four compact widgets in one row at a 390px viewport,
  unclipped compact readings/lighting/battery/temperature content, touch removal,
  right-button and movement cancellation, draft-only changes, discard restoration,
  keyboard removal/focus and Alt+Left ordering without move buttons. Normal-motion
  drag preview now hit-tests final layout slots to avoid moving-neighbour wobble.
- Shared Live renderer defaults remain covered by the battery/temperature browser
  cases. Those tests use ephemeral backend ports and explicit fixture bootstraps
  so they can run alongside the development preview without using its state.
- Strict OpenSpec validation and whitespace checks passed. Generated screenshots
  remain under ignored `conformance/generated/custom-dashboard/`.

### Square-cell packing follow-up

Verified on 2026-10-24 in the uncommitted working tree based on `fae1e92a`,
with final frontend bundle `bundle.aa29a505.js`.

- Ten focused dashboard/chart/battery unit tests passed. Svelte/TypeScript,
  affected ESLint/formatting, production build and conformance TypeScript checks
  passed; the existing bundle-size advisory remains.
- Chromium and WebKit custom-dashboard checks passed for exact square compact
  cells, a medium 2×2 tile beside four compact 1×1 tiles, responsive non-overlap,
  dense placement without saved-order mutation, keyboard removal/reordering,
  cancellation and save/conflict flows. Compact trend/bar/gauge content remains
  within cell bounds; the square temperature fixture includes all four BMS
  readings. Mixed-layout screenshots are ignored under conformance/generated/.
- Shared Live charts, per-board view restoration and Live/BMS temperatures passed
  in both engines. The charts test now uses an ephemeral port and explicit mock
  bootstrap so the development preview can remain running, and checks its Done
  and existing warning actions separately.
- Strict OpenSpec validation and whitespace checks passed. No native, contract,
  persistence-format or firmware changes were introduced by this follow-up.

### Colours, warning scope and floating toolbar follow-up

Verified on 2026-10-24 in the uncommitted working tree based on `fae1e92a`,
with frontend bundle `bundle.beeb6673.js`.

- All 25 focused dashboard/reading/chart tests passed. Svelte/TypeScript,
  affected ESLint/formatting, production build and conformance TypeScript checks
  passed. The existing bundle-size advisory remains.
- Ten browser cases passed across Chromium and WebKit: custom dashboards, durable
  view selection, Live charts, Ride location and Board actions. Added coverage
  verifies identical widget coordinates on hold-to-edit, toolbar absence during
  widget configuration, semantic text/visual colour equality for bar/gauge/trend
  across all three authoritative load sources, and no warning presenters in
  configuration, Board actions, Ride location, charts or fault-history dialogs.
- Dashboard warning visibility returns on dismissal; existing action gates,
  lighting, confirmation, focus and save/discard flows remain covered. Menu and
  location tests use explicit isolated mock bootstraps and ephemeral ports so the
  live development preview remains available.
- Strict OpenSpec validation and whitespace checks passed. No Git commits, native
  packaging or firmware/storage changes were part of this follow-up.

### Accurate size preview follow-up

Verified on 2026-10-24 in the uncommitted working tree based on `fae1e92a`,
with frontend bundle `bundle.f022d26d.js`.

- Three focused composition/footprint tests passed. Svelte/TypeScript, affected
  ESLint/formatting, production build and conformance TypeScript checks passed.
- Four Chromium/WebKit dashboard and last-view cases passed, including exact
  1×1 and 2×2 editor previews matching dashboard cell dimensions. Grid placement
  and preview share one footprint mapping. The preview’s absolute inner frame
  avoids WebKit adding content padding to an aspect-ratio-derived height.
- Strict OpenSpec validation and whitespace checks passed. This follow-up changes
  preview presentation only; the persisted three-size model remains unchanged.
  Independent bounded column/row spans were discussed as the next design step.

### Gauge-first visualisation follow-up

Verified on 2026-10-24 in the uncommitted working tree based on `fae1e92a`,
with frontend bundle `bundle.11584265.js`.

- All 21 focused reading/composition tests passed. Svelte/TypeScript, affected
  ESLint/formatting, production build and conformance TypeScript checks passed.
- Chromium and WebKit custom-dashboard checks passed, including compact and medium
  cell-filling gauge geometry, centred SVG reading text and colour equality across
  duty/motor/battery winners. Existing range configuration, preview, save/discard,
  missing-reading and interaction coverage remains passing.
- Visually inspected compact and medium gauge screenshots saved under ignored
  conformance/generated/custom-dashboard/. The dial uses a 270-degree arc with
  centred label/value/units, endpoint ranges and explicit unavailable/out-of-range
  presentation, without a separate large number above it.
- Strict OpenSpec validation and whitespace checks passed. Shared Live gauge,
  persisted dashboard shape and backend contract were unchanged.

### Simplified configuration actions follow-up

Verified on 2026-10-24 with frontend bundle `bundle.e5091ed7.js`.
Chromium and WebKit dashboard checks passed, including modal Save, absence of
Remove widget/Apply to dashboard, direct × removal, range validation and
save/conflict/discard flows. Svelte/TypeScript, affected ESLint/formatting,
production build, conformance TypeScript, strict OpenSpec and whitespace checks
passed. Modal Save still updates the draft; dashboard Save persists it.

### Value-first creation and flexible sizing follow-up

Verified on 2026-10-24 in the uncommitted working tree based on `fae1e92a`,
with final frontend bundle `bundle.a50ddd7d.js`.

- All 27 focused frontend tests passed: closed dimension validation (all 16
  accepted column/row pairs and malformed rejection), legacy footprint mapping,
  full reading-catalogue coverage, search/source distinctions and reading values.
  Svelte/TypeScript, affected frontend ESLint/formatting, production build and
  conformance TypeScript checks passed. The existing bundle-size advisory remains.
- Chromium and WebKit custom-dashboard and last-view cases passed for searchable
  value-first creation, whole-card touch and keyboard activation, secondary-input
  rejection, staged cancellation without draft changes or writes, explicit display
  choices, shape presets, 4×1 battery strips, 1×4 height on narrower containers,
  persisted/reloaded dimensions and untouched named-size preservation.
- Product-store and backend-service tests passed, including frozen baseline
  stores/archives, all 16 shapes, invalid atomic updates, mixed old/new footprint
  reopen and archive round trips. Apple FFI and Android JNI consumer tests passed.
  Locked/offline warnings-denied Clippy and Rust formatting checks passed.
- Canonical mock (isolated ephemeral port, revision 86) and direct shared Rust
  conformance (revision 508) passed for mixed named/explicit footprints and closed
  dimension validation. Both native hosts keep the shared JSON profile owner;
  native adapters have no separate dashboard schema or layout interpretation.
- Strict OpenSpec validation and whitespace checks passed. No schema/marker or
  archive-format reset, native packaging, physical-device validation, Git commit,
  PR edit or online publication was performed.

The long-running development preview was refreshed after detecting a cached old
mock validator. Its accepted settings and board layout were restored and checked
for equality. A rejected validation probe confirmed the refreshed validator accepts
explicit dimensions while leaving the live profile unchanged. The value-first
picker was reopened for review.

## 17. Denser dashboard rows

- [x] Reduce grid rows independently from column widths (55%, bounded to 56–80px), retaining span-based packing and unchanged saved dimensions.
- [x] Measure width and height independently for configuration previews. Keep custom footpad bars at 20px and fit compact battery/temperature contents within shorter rows.
- [x] Update layout unit and browser assertions for rectangular cells and verify frontend checks.

Density follow-up verification: three focused layout unit tests passed; Svelte
checks, targeted ESLint/formatting, production build and strict OpenSpec validation
passed. Chromium and WebKit dashboard browser tests both passed with independent
preview dimensions, dense mixed-size packing and compact trend/bar/gauge content.

## 18. Audit widget content across flexible sizes

- [x] Make dashboard battery and temperature content adapt to measured width and height, avoiding clipped bars, fixed-width detail columns and overlapping scales.
- [x] Fit reading titles, values, unavailable labels and chart axes in narrow/short tiles; rename the visible Trend choice to Chart without changing persistence.
- [x] Audit all 16 sizes at phone and desktop widths, with/without BMS data, and every reading label in Chromium/WebKit; verify frontend checks and affected Live panels.

Widget content audit verification: all 16 sizes passed at 280, 390, 844
(landscape height 390), and 1280px widths in Chromium and WebKit, with and
without BMS/charging states. Every reading label passed in 1×1; battery and
temperature configuration previews passed every size independently. The dashboard
interaction, Live battery configuration and Live BMS temperature browser suites
passed (8 tests); one earlier WebKit battery-dialog timeout passed on sequential
retry. Focused frontend units passed (23 tests), Svelte/type checks, targeted
ESLint/formatting, production build, strict OpenSpec and whitespace checks passed.

## 19. Match normal Live box styling

- [x] Match compact custom number layout, typography and corner radius to the normal Live box.
- [x] Share muted, centred uppercase title styling across custom renderers and their previews.
- [x] Verify computed title styling against normal Live boxes, all sizes/previews and affected shared Live panels in Chromium/WebKit; run relevant frontend checks.

Shared styling verification: computed title font, weight, colour, tracking,
alignment and casing match normal Live boxes across all custom renderers.
Compact values use the normal 14px cap. Chromium/WebKit size and interaction
suites passed (4 tests), including all 16 sizes at four viewport widths, BMS/charging
variants, every reading label and battery/temperature editor previews. Shared
Live battery and temperature suites passed (4 tests). Frontend type checks,
targeted ESLint/formatting, production build and strict OpenSpec validation passed.

## 20. Discoverable large gauge footprints

- [x] Offer illustrated 3×3 and 4×4 presets for reading and combined gauges.
- [x] Default newly added gauges to 3×3 while preserving existing widgets and subsequent explicit size choices.
- [x] Verify catalog defaults, new reading/composite gauge flows, preset pointer/keyboard activation, existing editing and frontend checks.

Gauge preset verification: catalog units (3 tests), frontend Svelte checks,
targeted lint/formatting, production build and conformance TypeScript checks
passed. Chromium/WebKit dashboard tests (2 tests) passed for new gauge defaults,
both large presets, pointer/keyboard activation, repeated Gauge selection,
cancellation without adding, and existing widget editing. Strict OpenSpec and
whitespace validation passed.

## 21. Use expanded widget space

- [x] Give multi-row temperature/battery panels a readable expanded layout rather than centring unchanged compact content.
- [x] Audit controls, footpads, numbers, bars, gauges and charts; use bounded expansion where extra space is useful.
- [x] Verify visible 4×1→4×2 growth, all dimensions/previews and shared Live regressions in Chromium/WebKit, plus frontend checks.

Expanded layout verification: Chromium/WebKit assert more than 20% visual growth
from 4×1 to 4×2 for temperature/battery readings and bars, lighting, footpads,
number/bar readings, both gauge types and charts. All 16 footprints at four
viewport widths, BMS/charging variants and battery/temperature previews pass.
Shared Live battery/temperature browser checks passed on sequential retry after
concurrent autosave/dialog timing failures (6 audit/Live tests); dashboard
interaction tests also passed (2 tests). Svelte checks, targeted ESLint/formatting,
production build, conformance TypeScript and strict OpenSpec validation passed.

## 22. Simple size buttons and readable charts

- [x] Add a 3×4 gauge preset; remove Width/Height inputs and retain saved non-preset sizes as selected shape buttons.
- [x] Improve dashboard chart labels, formatting, plot spacing and line weight with a sparse sparkline layout for one-row widgets.
- [x] Verify preset keyboard/pointer activation, 3×4 selection, preserved uncommon footprints, readable chart geometry and all-size/Live chart regressions in both browsers; run frontend checks.

Icon sizing/chart verification: Chromium and WebKit dashboard/size tests passed
(4 tests), covering 3×4 activation, icon-only configuration, retained 1×4 stored
footprints, all 16 persisted dimensions, preset previews and readable external
chart labels/stronger traces. Normal Live chart tests passed in both browsers
(2 tests). Five focused chart units, Svelte checks, targeted lint/formatting,
production build, conformance TypeScript, strict OpenSpec and whitespace checks
passed. Chart axes use muted labels while current values and traces share the
reading's semantic colour.

## 23. Zero-origin signed bars

- [x] Draw signed reading bars from zero in the appropriate direction with a visible baseline and correct asymmetric/custom-range placement.
- [x] Verify negative/zero/positive, missing and clipped readings, one-sided scales and browser rendering; run focused frontend checks.

Signed bar verification: five focused layout/bar unit tests passed, including
symmetric and asymmetric ranges, missing/zero values, endpoint clamping and
positive-/negative-only ranges. Chromium/WebKit dashboard tests passed (2 tests),
with explicit negative/zero/positive fill direction and zero-marker assertions.
Svelte, targeted lint/formatting, production build, conformance TypeScript, strict
OpenSpec and whitespace checks passed.

## 24. Larger chart presets

- [x] Offer charts the shared 3×3, 3×4 and 4×4 presets, retaining their current size until explicit selection.
- [x] Verify all three selections with pointer/keyboard browser coverage and cancellation without adding a widget.

Verification: Chromium/WebKit dashboard tests passed (2 tests), frontend Svelte
checks, targeted lint/formatting, production build, conformance TypeScript, strict
OpenSpec and whitespace checks passed.

## 25. Expand the wide dashboard grid

- [x] Allow the custom view to use a bounded 100rem shell and increase to 6/8/10 columns at 40/64/88rem container widths.
- [x] Verify fold, desktop and ultrawide column counts, unchanged stored order/spans, no overlap/overflow, all-size content, and normal Live width limits in Chromium/WebKit; run frontend checks.

Wide-grid verification: Chromium/WebKit dashboard and size suites passed
(4 tests), asserting 2/4/6/8/10 columns through 280–2560px viewports, bounded
custom width, stable order with no resize writes, no overlap/overflow, and normal
Live retaining its 1024px cap. All 16 widget footprints passed at five viewport
widths with BMS/charging variants. Reviewed the ultrawide screenshot. Svelte,
targeted lint/formatting, production build, conformance TypeScript, strict
OpenSpec and whitespace checks passed.

## 26. Predictable packing and quieter editing

- [x] Use 4/8/12 column wide layouts and ordered row placement so later widgets do not backfill old holes.
- [x] Remove redundant size/preview/draft-save helper copy; show bounds only when custom ranges are being edited.
- [x] Increase the delete badge to 36px with a subtle danger tint and retain the 44px pointer target and keyboard semantics.
- [x] Verify visual order, mixed-size packing, breakpoint counts, all-size layouts, deletion and configuration regressions in both browsers; run frontend checks.

Packing/editor verification: Chromium/WebKit dashboard and all-size browser
suites passed (4 tests), including visual row order at 2/4/8/12 columns, mixed
2×2/four-1×1 packing, 36px danger badges, pointer/keyboard deletion and cancellation,
drag reordering, no redundant helper copy and existing range validation. Svelte,
targeted ESLint/formatting, production build, conformance TypeScript, strict
OpenSpec and whitespace checks passed.

## 27. Delete badge placement

- [x] Move opaque 28px danger badges clear of widget titles, retain 44px targets, and reserve permanent top/edge clearance with safe-area handling.
- [x] Verify badge/title separation, pointer/keyboard deletion and cancellation, editing stability and all-size regressions in both browsers; run frontend checks.

Delete badge verification: Chromium/WebKit dashboard and size suites passed
(4 tests), asserting 28px opaque danger badges, 44px targets, title clearance,
unchanged widget positions on entering editing, and primary/secondary/keyboard
deletion and pointer cancellation. All 16 widget footprints passed at five widths
with BMS/charging variants. Reviewed the editing screenshot. Svelte, targeted
ESLint/formatting, production build, conformance TypeScript, strict OpenSpec and
whitespace validation passed.

## 28. Drop into the visible placeholder

- [x] Treat the visible placeholder as the active destination throughout dragging and commit its latest position on release, including over gaps.
- [x] Use one solid, readable placeholder without a second hover-confirmation style.
- [x] Verify pointer/touch gap release, cancellation, drag/edit activation and frontend checks in Chromium/WebKit.

Placeholder drop verification: Chromium/WebKit dashboard browser tests passed
(2 tests), with mouse release over a grid gap and a real Chromium touch
hold/move/gap/release stream committing the visible preview. Explicit touch
cancellation still restores the original order; keyboard ordering and existing
control/edit/delete regressions pass. Svelte, targeted ESLint/formatting,
production build, conformance TypeScript, strict OpenSpec and whitespace checks
passed.

## 29. Align corner badges

- [x] Replace the uneven floating offset with equal outward offsets from the top-right corner, retaining heading clearance and the touch target.
- [x] Adjust edge clearance without changing grid width and remove duplicated badge styles.

Verification: Chromium/WebKit dashboard tests passed (2 tests), including
measured equal corner offsets, heading clearance and deletion/drag regressions.
Reviewed the updated editing screenshot. Svelte, targeted lint/formatting, build,
conformance TypeScript, strict OpenSpec and whitespace checks passed.

## 30. Prevent clipping from delete targets

- [x] Keep delete hit targets inside scroll clearance without moving the visible corner badges or resizing the grid.
- [x] Verify no horizontal overflow or sideways focus scrolling at phone, fold, desktop and ultrawide widths.

Verification: Chromium/WebKit dashboard tests passed (2 tests), including focus
on every delete button at four widths, contained target bounds, zero horizontal
scroll and existing deletion/drag/cancel behavior. Live preview DOM measurements
confirmed equal scroll/client widths and scrollLeft zero after right-edge focus;
reviewed and saved a preview screenshot. Svelte, targeted lint/formatting, build,
conformance TypeScript, strict OpenSpec and whitespace checks passed.

## 31. Keep borders behind delete badges

- [x] Replace parent editing outlines with layered border pseudo-elements.
- [x] Move wiggle animation to widget contents so neighbouring frames cannot occlude delete badges; retain solid drop placeholders and steady lifted previews.
- [x] Verify badge layering with motion enabled, plus drag/delete/cancel and frontend checks.

Verification: Chromium/WebKit dashboard tests passed (2 tests), asserting border
layering and hit testing through the badge's lower-right area above neighbouring
widgets with wiggle enabled. Reviewed and saved a live close-up showing clean
border occlusion. Svelte, targeted lint/formatting, build, conformance TypeScript,
strict OpenSpec and whitespace checks passed.

## 32. Accentuate the toolbar and restore border wiggle

- [x] Add a theme-accent halo, border tint and layered shadow to the floating toolbar.
- [x] Animate border pseudo-elements in sync with widget contents while keeping badge layers above adjacent borders; retain steady drop/lift previews and reduced-motion behavior.
- [x] Verify synchronized animation and reduced motion, badge layering, toolbar shadow and interaction regressions.

Verification: Chromium/WebKit dashboard browser tests passed (2 tests), including
matching border/content animation names and rotation, reduced-motion suppression
of both, and badge hit testing above neighbours. Svelte, targeted lint/formatting,
production build, conformance TypeScript, strict OpenSpec and whitespace checks
passed. Visually checked the accent shadow in live previews and saved a close-up
from an isolated preview without interrupting the user's active tab.

## 33. Discoverable press and lift

- [x] Keep 600ms holds to enter editing and shorten subsequent editing holds to 300ms, preserving immediate desktop drag.
- [x] Show cancellable press feedback and a raised, accent-outlined lift; honour reduced motion and remove duplicate delete badges from the lift.
- [x] Verify exact hold boundaries, feedback cleanup, real touch scrolling and existing drag/delete behavior in Chromium/WebKit; run frontend checks.

Verification: Chromium and WebKit dashboard browser suites passed, including exact
600ms/300ms boundaries and cancellation cleanup; Chromium also exercised real touch
input. Svelte checks, targeted lint/formatting, production build, conformance
TypeScript, strict OpenSpec validation and whitespace checks passed.

## 34. Stable drag placement

- [x] Add spatial hysteresis after each accepted placement, accounting for scroll travel and clearing on cancellation.
- [x] Verify mixed-size reflow, small pointer jitter, deliberate reversal, drop and cancellation in Chromium/WebKit with normal and reduced motion; run frontend checks.

Verification: The regression reproduced placement oscillation in both browsers
before the fix. Chromium and WebKit now pass mixed-size jitter/reversal checks at
390px and 1280px with normal and reduced motion, plus the existing dashboard drag,
touch, drop and cancellation suite. Exact hold-time tests now pause the installed
clock to exclude wall-clock drift. Svelte checks, targeted ESLint/Prettier, production
build, conformance TypeScript, strict OpenSpec and whitespace checks passed.

### 34.1. Stable insertion boundary for large neighbours

- [x] Retain the hovered neighbour and its original insertion midpoint across reflow, with directional before/after placement and a reversal dead band.
- [x] Reproduce the recording's continued-forward-travel failure, then verify two/four-row footprints, midpoint reversal and existing mouse/touch/drop/cancel behavior in both browsers.

Verification: Continued forward travel reproduced the remaining two-state oscillation
in Chromium and WebKit against the previous 20px-only fix. Both browser suites now
pass with the retained midpoint, two/four-row footprints, repeated jitter and
intentional midpoint reversals; Chromium exercises real touch streams with
frame-delivered input. Svelte checks, targeted ESLint/Prettier, production build,
conformance TypeScript, strict OpenSpec validation and whitespace checks passed.

## 35. Preview layout parity

- [x] Render the preview at its actual dashboard footprint and uniformly scale it to fit the overlay, preserving responsive content and labels.
- [x] Verify chart preview/dashboard label and logical size parity at phone, tablet and desktop widths in Chromium/WebKit, with frontend checks.

Verification: Both browsers reproduced missing preview axis labels before the fix.
Chromium and WebKit now pass label visibility and internal layout parity checks at
390px, 768px and 1280px, including fitting inside the overlay. Existing dashboard
interaction coverage, Svelte checks, targeted ESLint/Prettier, production build,
conformance TypeScript, strict OpenSpec validation and whitespace checks passed.

## 36. Compact chart label gutter

- [x] Size the vertical-axis gutter to its labels and share the plot column with the time axis, replacing the fixed 3.5rem gutter.
- [x] Verify label spacing/alignment in preview and dashboard across viewport sizes, and run affected frontend checks.

Verification: Chromium and WebKit dashboard suites passed, including bounded
unused axis space and time/plot alignment in previews and dashboard charts at
390px, 768px and 1280px. Inspected the phone chart screenshot. Svelte checks,
targeted ESLint/Prettier, production build, conformance TypeScript, strict OpenSpec
validation and whitespace checks passed.

## 37. Scrolled dashboard separation

- [x] Show a subtle top divider and downward shadow only while dashboard content is scrolled; keep the indicator out of layout and pointer handling.
- [x] Verify top/scrolled/restored states and unchanged header geometry in Chromium/WebKit; run affected frontend checks.

Verification: Chromium and WebKit dashboard suites passed, covering hidden/visible/
restored divider states, fixed scroll-edge position, zero layout height, pointer
transparency and unchanged header geometry. Inspected the scrolled screenshot.
Svelte checks, targeted ESLint/Prettier, production build, conformance TypeScript,
strict OpenSpec validation and whitespace checks passed.
