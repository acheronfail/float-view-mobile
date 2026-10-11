# Design

## Context

See proposal.md. Five persisted widget slots already drive a shared picker, and the live action model already defines authoritative lighting eligibility. Both native hosts share the Rust product-store settings catalogue.

## Goals / Non-Goals

Keep five slots and one lighting mutation path. Preserve existing saved choices; no storage reset, aliases or migrations. No new dependencies or OS/firmware changes.

## Decisions

- Extend the closed slot catalogue with empty, headlights and leds; change defaults in the contract and Rust store together. Retain the five-string settings shape.
- A primary pointer held for 600 ms opens the picker; release before then taps lighting only. Moving over 10 px, cancellation, lost capture, blur, hiding or unmount cancels. Holding never also toggles; synthetic clicks are guarded. Numeric/empty keyboard activation opens the picker. Lighting Enter/Space toggles, while Shift+Enter or the context-menu key opens configuration; expose a screen-reader-only configure button as an alternate assistive action.
- Reuse a shared lighting presentation derived from authoritative connection/control state for both the box and picker. Unknown/unavailable lighting shows text rather than a false off switch. Busy/reconnecting/read-only lighting cannot mutate but remains configurable. Do not optimistically flip or persist runtime state.
- Remove lighting items from the menu action generator. Add Charts… only for an active saved session, using the shared session viewer, and remove only the Live placement of CurrentRideButton.

## Risks / Trade-offs

- Long holds can collide with scrolling/context menus: cancel on movement and suppress native selection/context menu for boxes; retain touch panning and explicit keyboard configuration.
- A setting or connection can change during a gesture: cancel a held gesture when its target field or connection identity changes and recheck authoritative eligibility at release.
- Unknown state must remain configurable: do not disable the entire box solely because its lighting action is unavailable.

## Verification

Focused hold/lighting tests, shared settings persistence tests and consumer checks, canonical mock/direct settings coverage, browser desktop/touch interaction and menu/current-ride regressions. Both native hosts use the changed shared settings owner with unchanged bindings; no physical checkpoint is required for this UI and catalogue change.
