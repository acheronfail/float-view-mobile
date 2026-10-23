# Design

## Context

See [proposal.md](proposal.md) for motivation and scope. `LiveView.svelte` currently owns fixed layout and fault/Refloat-disabled warning markup. `WidgetNumbers.svelte` renders four `LiveValueBox` instances and `WidgetGauge.svelte` contains the fifth. `LiveValueBox` binds rendering, slot persistence, hold configuration and lighting actions together. `live-values.ts` already centralises names, formatting and available readings.

`LiveCharts.svelte` owns its own in-memory sample list, sequence deduplication and pruning timer. `live-charts.ts` supplies the existing 30-second/1,500-point bounds and chart geometry. The dialog only observes `latestSample`; it does not request extra telemetry. Moving history ownership changes its lifetime from dialog lifetime to live-route lifetime.

Board Profiles are bounded JSON, validated by the shared Rust product store, with revision-checked patch operations. The existing complete-profile limit is 16,384 bytes. Archive format 4 carries historical profile snapshots and uses the same profile validation. Storage already supports baseline 100 and transition 100→101; dashboard metadata must not reset either. UI checkpoints currently permit only normal/detailed mode and are process-memory data.

## Goals / Non-Goals

**Goals:** Keep data ownership explicit, share renderers rather than entire screens, and use one responsive configuration and editor across hosts. Preserve fixed Live View behaviour while introducing Custom View through bounded additive metadata.

**Non-Goals:** A widget plugin framework, generic layout engine, frontend product persistence, platform-specific frontend variants, a second native runtime, native hinge reporting, or automatic data-collection changes. See proposal non-goals for product exclusions.

## Decisions

### 1. A small typed dashboard model in Board Profiles

Add optional `customDashboard` to `BoardProfileValues`. Absent/null selects a derived initial layout; `{version: 1, widgets: []}` is an intentional empty dashboard. Use at most 24 widgets and 8,192 encoded dashboard bytes within the unchanged complete-profile bound. IDs are unique opaque strings of at most 64 ASCII identifier characters. No arbitrary labels, CSS, HTML, formulas, URLs or executable configuration are stored.

Use a discriminated union of concrete widget families:

- Reading: existing numeric field, explicit number/bar/gauge/trend presentation, size, optional range in the field's canonical units.
- Specialised visual: speed/load gauge, battery, temperatures or footpads, plus size and only options actually needed by that renderer.
- Control: existing headlights or LEDs, plus size.

Shared catalogue metadata determines compatible presentations and default ranges. Numeric fields can offer numbers and trends; offer bars/gauges only where a defensible default domain exists. User ranges are finite, ordered, bounded by documented field-appropriate limits and stored in canonical units so unit changes do not reinterpret them. Validate closed shape, known values and compatibility at the Rust boundary as well as in the editor. Publish the exact union and domain limits in `contract.ts` with matching conformance cases before wiring the editor.

Load dashboard data through `boardProfile.get`; save through the existing revision-checked update patch. Do not embed another layout collection in every telemetry snapshot. Derived defaults incorporate the existing nonempty slot selections and standard visual widgets without writing a profile on read. Live's five slots remain separate after initial derivation. Cache only for the current route/board and invalidate from successful profile updates; reconcile current revisions before saving. Never read browser storage for dashboards.

**Alternative:** Separate dashboard methods/table add dispatch and migration surface without a current need. Reusing fixed slots cannot describe size, order or multiple presentations.

### Flexible size compatibility

Extend the existing version-1 `size` union additively with `{columns, rows}` where both are integers 1–4 and unknown members are rejected. Existing recognised named sizes remain valid supported data: compact→1×1, medium→2×2, large→4×2, including their prior narrow-container clamping. Do not silently rewrite existing layouts on read. New widgets and explicit dimension changes use the object form. Explicit spans clamp only columns to available width; row intent stays independent, including 1×4 and 4×1 layouts. Existing untouched named values survive edits and archives. The additive profile JSON validation change requires no SQL, owned-storage marker or archive format transition; there is no reset or general migration framework. Both host bindings continue using the shared profile JSON owner.

### Value-first creation

Replace the preset gallery with search over all numeric values and a small popular-values section, with complete catalog categories available through Browse all values. Show explicit distinct estimated/controller/BMS battery names, Number/1×1 defaults, small previews and whole-card primary-pointer/keyboard activation. Keep composite visuals and controls separate. Stage the selection outside the draft; only Add widget commits it to the draft, while Cancel leaves the draft unchanged. Existing widgets retain modal Save. Show compatible Number/Bar/Gauge/Chart choices as native toggle buttons rather than hiding them in a dropdown. Illustrated shape buttons select sizes and update the same accurate preview, without separate width/height inputs. A saved non-preset size remains visible as a selected shape button.

### 2. Share display primitives, keep screen composition explicit

Extend the existing reading catalogue with presentation compatibility and range metadata. Extract presentation-only number/control/visual/chart pieces from slot- or dialog-specific wrappers. Renderers receive readings, units, freshness, presentation and size inputs; they do not save profiles or start their own history subscriptions. Controls use the existing authoritative action state and mutation methods.

A small custom widget frame owns unavailable presentation, sizing and editing interception. A typed renderer switch is sufficient; do not build dynamic registration, inheritance or a general widget framework. Live retains its fixed composition and existing hold-to-configure wrappers, consuming the extracted renderers where practical. Custom owns its dashboard editor; Detailed continues to use coherent telemetry groups. Share value semantics and formatting without forcing every screen into the same layout component.

Remove superseded warning markup, chart-dialog buffer/timer code and duplicated rendering paths after consumers use their shared owners. Keep specialised visuals specialised instead of adding unrelated conditional props to one universal component.

**Alternative:** Duplicating Live's components into Custom would fork availability, lighting and unit semantics. Turning Live into a configurable dashboard immediately expands scope and risks its established behaviour.

### 3. Responsive grid from order and size intent

Use a responsive CSS grid with ordered row placement. Use 2 logical columns on narrow containers, 4 from 20rem container width, 8 from 40rem and 12 from 72rem; permit one column below 14rem. Use rows at 55% of the column width, bounded to 3.5–5rem, with 6px gaps through an inline-size container. Measure column width and row height independently for accurate configuration previews. Custom footpad bars stay a slim 1.25rem strip, centred within their allocated footprint. Existing named compact widgets occupy 1×1 cells, medium 2×2 and large 4×2, retaining their prior narrow-container clamping. Explicit `{columns, rows}` sizes accept independent 1–4 spans and clamp only columns to available width. Four compact widgets can fill the two rows beside one medium widget. Widget heights come from cell spans rather than content-driven row stretching or fixed minimum heights. Specialised dashboard renderers respond to both allocated width and height. Battery progress bars remain fully visible, pack voltage appears only when height permits, and scale labels appear only when width also permits. Temperature readings stack or use a small grid without fixed label/detail columns. Value labels wrap instead of ellipsising, reading text respects the available height, and narrow charts reduce axis labels. Display the trend presentation as “Chart” while retaining the persisted trend identifier. Live renderers retain their existing layout. Bound the custom dashboard and its shell to 100rem, while retaining the normal Live and Detailed shell width limits.

Dense placement may put later smaller widgets into earlier visual gaps. Preserve the identity-based saved order and DOM/keyboard traversal order; packing never persists coordinates or rewrites order.

Widget renderers adapt to allocated container space, not device names or global viewport assumptions. Use container queries where supported by the project's existing baseline, with simple width-based fallback where needed. A larger number remains a number; a resized chart remains a chart. Widget IDs remain stable keys so reflow does not remount charts or lose local draft state. Portrait, landscape, unfolded and split-screen views all reuse the same order. Widgets fill the next available position in row order beside larger footprints; later widgets never backfill earlier rows. Remaining gaps are empty cells.

React to usable host viewport/safe areas. Do not infer physical hinge position from width or promise rendering across unreported occlusion. New native display-region APIs would be a separate follow-up if actual supported devices require them.

**Alternative:** Fixed coordinates or saved breakpoint layouts require conflict resolution, cross-device editing and migration policy. A freeform canvas or pixel-based masonry introduces layout state beyond the shared dense grid.

### 4. Draft editor with one configuration modal

A stationary whole-widget hold (or a labelled keyboard/assistive widget action) creates a local copy of the accepted dashboard and its profile revision. The dashboard editor owns add, configure, reorder, remove and Save/Cancel. The picker offers ready-made configurations with previews; the widget modal edits content, compatible presentation, size and applicable ranges. Its Save action updates only the draft; removal is available through the widget × button. Keyboard users reorder from the widget surface using Alt+Left/Right. Modal changes affect only the dashboard draft. Save sends one complete dashboard patch; Cancel/discard sends none.

On conflict, retain the draft, show a reload-current-values action and require the user to reconcile; do not automatically resubmit against a newer revision. Operational failure retains the draft for retry. A save interrupted before acknowledgement has an uncertain outcome; the next read determines accepted data. Same-board reconnect preserves a living draft; a board change invalidates the editor and guards all late completions by board identity.

Store optional nullable `lastLiveView` (`normal`, `detailed`, `custom`) in the existing Board Profile JSON. Missing/null defaults to Live. Persist explicit view selection before mounting the destination, using a fresh profile revision and at most one conflict retry of only this field, preserving dashboard and other configuration. Failed saves keep the current view with actionable feedback. Board changes invalidate pending reads and selection completions; guests remain temporary. On entry, restore the eligible same-process checkpoint first, otherwise load that board’s durable selection without writing. Loading and transient reconnects must not overwrite an existing view or draft. Extend checkpoint mode with `custom` and require `live.savedDeviceId` for that mode so a replaced saved-board identity cannot inherit a previous target's presentation. Existing normal/detailed checkpoints remain valid without that optional identity. Do not include layout data, edit mode or widget surfaces. On frontend recreation restore Custom in viewing mode from current profile values. Preserve existing checkpoint size and queue limits.

**Alternative:** Autosaving every drag makes experimentation and cancellation difficult and increases profile conflicts. Restoring drafts in native checkpoints conflates transient presentation with durable product data.

### Floating editor controls and reading colours

Place Add/Cancel/Save in an absolute bottom toolbar with safe-area clearance, short labels and existing motion primitives. Its appearance never changes grid or scroll viewport geometry. Reserve bottom scroll padding in both modes; hide the toolbar while a picker, widget dialog or discard confirmation is open. Move editing/discovery instructions to assistive text rather than varying in-flow paragraph heights. Restore toolbar focus when its original trigger was removed during a panel handoff.

Derive one semantic colour with the existing live-value colour selector and inherit it throughout each custom reading widget, using currentColor for bar fills, gauge strokes and single-series charts. Let shared number/chart renderers opt into inheritance; leave their Live defaults intact. Load follows the authoritative winner immediately across all visual parts. Reading gauges devote the cell to a 270-degree dial with label, actual reading and units centred inside it; range endpoints and unavailable/out-of-range indications remain within the SVG rather than competing with a separate prominent number.

### 5. Whole-widget dragging with explicit gesture arbitration

There is no explicit Edit button in the dashboard or view menu. Empty dashboards offer Add. A stationary hold enters editing and lifts that same widget without requiring release; its inert preview follows the pointer and marks the drop destination. Neighbouring widgets shift aside in a temporary CSS order with bounded animations unless reduced motion is requested; only dropping commits draft order, and cancellation restores the original order. Hit-testing uses the final layout slots rather than the painted positions of animating neighbours, preventing oscillation during a hover.

In edit mode, the widget frame captures interaction before its normal content: short stationary release opens configuration, deliberate drag reorders, and controls never run. Show a subtle bounded wiggle and static edit border/indicator; reduced motion suppresses wiggle. Each editing widget has a separate 28px circular × badge with an opaque subtle danger tint, straddling the top-right corner with equal outward offsets and retaining a 44px target for direct draft removal. Its pointer stream bypasses whole-card gesture handling, so removal cannot open configuration, drag or activate underlying controls; movement and cancellation abort it. Restore focus to the adjacent widget, or Add if none remain.

Use a small feature-local Pointer Events gesture state machine with explicit idle, pending, scrolling and dragging states. Mouse/pen drag starts after movement tolerance. On touch, a stationary hold arms dragging anywhere on the widget; movement before that hold scrolls, and a short release configures. Provide a brief drag cue when armed. Preserve ordinary scrolling outside widget surfaces. Pointer cancellation, lost capture, visibility loss, resize and board changes cancel pending gestures; a drop suppresses the follow-up click.

Touch-action must be chosen before pointerdown: changing `pan-y` to `none` after a hold does not reliably transfer an existing browser scroll gesture. Widget surfaces use `touch-action: none` with the helper forwarding pre-drag vertical movement to the dashboard scroll container; bound drag edge autoscroll to that container and stop it on cancellation/disposal. Outside editing the frame also arbitrates the hold, forwarding early vertical movement to the scroll owner. It intercepts pointerdown before nested control handlers register; only a short stationary release invokes the target button’s native click path once. Keyboard/assistive activation keeps native button semantics. Test this arbitration in touch browser coverage before considering it complete. Avoid duplicating mouse/touch handlers or relying on desktop HTML drag-and-drop for mobile.

Keyboard/assistive users open the same modal through native button semantics and reorder from labelled widget surfaces using Alt+Left/Right, with described shortcuts, with focus restoration and announced position changes. No operation requires dragging. Keep telemetry updates out of gesture state so frequent snapshots do not interrupt interaction.

**Alternative:** Always treating vertical movement as a drag prevents scrolling. A handle would reduce gesture ambiguity but conflicts with the agreed whole-widget surface. No new dependency is planned; adopt one only if a focused spike demonstrates it replaces substantial gesture/accessibility complexity and document that trade-off first.

### 6. Dashboard-only warnings

The live route owns fault/Refloat-disabled warning state above all three dashboard views, independent of widget selection. The single presenter remains in its original dashboard owner. While an ordinary menu, dialog or confirmation owns interaction, hide the presenter and make it inert; never prepend it into another surface. Keep its measured clearance stable so opening a panel cannot move the dashboard behind it. Restore visibility after the last modal closes. Authoritative flywheel mode retains its exclusive overlay and suppresses ordinary warnings.

Use a bounded DOM/resize observer for modal visibility and warning clearance because feature-local configuration dialogs are not all route-owned. Make attribute updates idempotent. Existing enable/fault actions retain their authoritative gates on the dashboard.

### 7. One route-scoped live-history owner

Extract sample admission and pruning from `LiveCharts.svelte` into one frontend owner created by the live route, shared by all trend renderers and the overlay. Retain the existing 30-second/1,500-point total budget and stale/gap semantics, captured per-point BMS values, unit conversion and authoritative load values. Widgets select series from this buffer; they do not each retain source samples or create collection timers. Bound visible work through the 24-widget cap and render only mounted views.

Collect only delivered fresh samples while the live screen is mounted and ready. Reset on changed device/saved-board/tracking identity, ready-generation boundaries and decreasing sample sequence; deduplicate equal sequence values. Conservatively clearing on leaving ready state and on frontend transport loss prevents crossing generations without adding a public generation API. Never reinterpret historical points with current controller limits. Keep one pruning clock, stop it on disposal, and release history on route exit or WebView destruction. No backfill from session queries, browser storage or native persistence.

**Alternative:** Dialog-local history loses data on every overlay close; widget-local buffers multiply memory and admission logic. Native history ownership or background collection is unnecessary for this ephemeral display.

## Risks / Trade-offs

- Touch drag/scroll ambiguity → Define the gesture states above, provide configuration-based ordering, and cover tap/hold/move/cancel/scroll on narrow and short layouts with touch-enabled browser tests.
- Range metadata and specialised gauges can drift → Centralise reading semantics; preserve existing load/battery owners and test unit changes, signed current and out-of-range values.
- Additive profiles can still exceed existing limits → Enforce both dashboard and whole-profile limits before commit and test maximal dashboard plus other profile fields.
- Many charts can be expensive even with one buffer → Bound widget count, share sample admission/pruning, avoid per-widget timers and test the maximum supported composition.
- Reflow during dragging can invalidate geometry → Cancel/reconcile safely on container resize; keep draft ordering independent of measured coordinates.
- Ordinary modals must stay free of warning panels → Test hidden/inert dashboard warnings, restoration after dismissal, focus and flywheel precedence.
- Width responsiveness cannot discover a physical hinge → State this limit and test usable viewport changes; add native work only for demonstrated device requirements.

## Migration Plan

1. Introduce the optional validated profile union, checkpoint enum extension and equivalent frontend/mock/shared implementation. Keep the five-slot setting and existing views intact.
2. Verify additive JSON storage with existing markers and archive format 4: missing field defaults, current-profile reopen, immutable historical snapshots and archive create/merge behaviour. No SQL schema or marker bump is planned; if investigation discovers an actual incompatible transition, revise this change before implementing an explicit upgrade. Never reset supported data.
3. Extract shared renderers/history/warnings with focused regression coverage, then connect the editor and Custom View. Ship frontend and shared native contract changes together through existing host bindings.
4. Run affected frontend, shared Rust, direct/mock and native contract checks, then document limits. No physical BLE checkpoint is introduced by dashboard rendering. Real fold-device gesture checks may supplement software evidence without claiming hinge support.

Rollback is a normal compatible software update: do not delete `customDashboard` to hide the feature and do not rewrite session snapshots or supported storage markers. Verify older profile readers preserve the additive key before claiming a downgrade path; unsupported downgrades follow the repository's existing compatibility policy.

### Shared Live box styling

Custom compact readings use the normal Live box's centred layout, 14px value
cap, muted 10px semibold uppercase title and 8px corner radius. A shared title
stylesheet keeps number, control, chart, battery, temperature, footpad and gauge
labels consistent. Larger reading values retain bounded scaling. Composite
widgets centre their title above their content; gauges retain their title within
the dial. Dense temperature layouts keep their heading visible and put units in
the heading when individual row space is limited.

Gauge configuration adds 3×3, 3×4 and 4×4 illustrated presets; their icons show
the full footprint while the shorter presets retain two-row icons. New combined
speed/load gauges start at 3×3. Selecting Gauge for a newly added reading suggests
3×3 once, then respects subsequent size choices. Existing widget sizes are retained.

### Expanded widget content

Widgets distinguish short one-row intent from expanded multi-row intent while
still adapting to their measured width and height. Expanded temperature panels
use larger reading text, thicker bars and spaced rows (or a BMS reading grid);
expanded battery panels grow their value and bar. Custom lighting controls and
footpads gain bounded visual size when there is space. Keep compact layouts and
shared title typography, and retain space-filling gauge/chart and number scaling.
Editor previews apply the same fitting rules at their effective dimensions.

Dashboard charts use readable HTML axis labels outside the data plot, compact
range formatting, a stronger two-pixel trace and higher-contrast labels. Narrow
layouts omit intermediate ticks and the Y gutter when space is insufficient;
one-row widgets use a sparkline with a readable current value. Accessible chart
labels retain the displayed range, including when axes are omitted.

Reading bars on ranges crossing zero originate at zero rather than the lower
range endpoint. Negative fills extend left, positive fills right, and a zero
marker remains visible. Zero follows its true position on asymmetric ranges;
nonnegative ranges retain lower-endpoint fills and negative-only ranges fill
leftward from the upper endpoint. Missing and zero signed readings have no fill.

Reading charts share the gauge 3×3, 3×4 and 4×4 presets. Selecting Chart retains
the current size until the user explicitly chooses a shape.

Widget configuration omits redundant size/preview explanations and draft-save
helper copy, retaining semantic section labels for accessibility. Range bounds
appear only while a custom range is being edited; validation errors remain visible.

Delete badges sit clear of widget headings. Permanent top/edge clearance keeps
them visible without moving the grid when editing starts. Edge clearance respects
safe-area insets. Use a solid tinted canvas background, a stronger glyph and
hover/pressed feedback rather than showing widget text through the badge.

A lifted widget's placeholder is always the active drop destination, with one
solid outline and no secondary hover/approval state. Keep the latest preview
when the pointer moves over a gap or outside another card, and commit that
preview on pointer release. Explicit cancellation, blur, resize and disposal
still discard the preview. Hide placeholder contents while keeping its footprint
so its border/fill are readable without duplicating the lifted widget.

The delete target extends inward below/left of the visible badge so its full
44px bounds fit the existing scroll clearance. It must not increase horizontal
scroll width or cause keyboard focus to pan the dashboard sideways.

Render editing/drop borders as a low-layer pseudo-element rather than a parent
outline. Keep widget frames free of animation-created stacking contexts; animate
their contents and border pseudo-elements together, suppressing both on lifted previews. This lets
all delete badges paint above both their own and neighbouring tile borders.

The floating edit toolbar uses a theme-accent border/halo and soft accent-coloured
shadow above a neutral depth shadow, preserving its position and solid readable
controls. Border/content wiggles share their animation timing; reduced-motion
mode disables both and drop placeholders remain steady.
